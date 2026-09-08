use std::path::Path;

use crate::probes::{self, Addressed};
use mcf_core::probe::Outcome;

const TRIALS: usize = 5;

const BUDGET: usize = 320;

const FROM: usize = 32;

const CEILING: usize = 1024;

const TOOL_BUDGET: usize = 160;

const THINKING_BUDGET: usize = 400;

const STRUCTURED_BUDGET: usize = 800;

fn resolved_engine(at: &Places<'_>, asked: &str) -> String {
    if asked == "stand-in" {
        return "stand-in".to_owned();
    }
    if let Some(resolved) = at.resolved {
        return resolved.to_owned();
    }
    let home = at.models.parent().unwrap_or(at.models);
    match crate::adapters::only_one(crate::adapters::provisioned_llama(home)) {
        Ok(Some(llama)) => format!(
            "provisioned {} @{}",
            llama.component,
            llama.commit.get(..12).unwrap_or(&llama.commit)
        ),
        _ => asked.to_owned(),
    }
}

fn whichever_is_here(home: &std::path::Path) -> &'static str {
    match crate::adapters::provisioned_llama(home) {
        crate::adapters::Found::One(_) | crate::adapters::Found::Several(_) => "provisioned",
        crate::adapters::Found::None => "stand-in",
    }
}

#[derive(Clone, Copy)]
pub struct Places<'a> {
    pub socket: &'a Path,
    pub models: &'a Path,
    pub resolved: Option<&'a str>,
    pub gone: Option<&'a dyn Fn() -> bool>,
}

impl std::fmt::Debug for Places<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Places")
            .field("socket", &self.socket)
            .field("models", &self.models)
            .field("resolved", &self.resolved)
            .finish_non_exhaustive()
    }
}

impl Places<'_> {
    fn asker_gone(&self) -> bool {
        self.gone.is_some_and(|gone| gone())
    }

    fn trial(
        &self,
        model: &Path,
        prompt: &str,
        pieces: Option<&[mcf_standin::tokenizer::Piece]>,
        budget: usize,
        engine: Option<&str>,
    ) -> crate::probes::Trial {
        self.spoken(model, prompt, pieces, budget, engine).trial
    }

    fn spoken(
        &self,
        model: &Path,
        prompt: &str,
        pieces: Option<&[mcf_standin::tokenizer::Piece]>,
        budget: usize,
        engine: Option<&str>,
    ) -> crate::probes::Spoken {
        if self.asker_gone() {
            return crate::probes::Spoken {
                trial: crate::probes::Trial::CouldNotTell(crate::served::CLIENT_LEFT.to_owned()),
                text: String::new(),
                answer: None,
                engine_ran: None,
                window_ran: None,
            };
        }
        crate::probes::spoken(self.socket, model, prompt, pieces, budget, engine)
    }

    fn counted(
        &self,
        model: &Path,
        text: &str,
        engine: Option<&str>,
    ) -> Result<crate::probes::Counted, String> {
        if self.asker_gone() {
            return Err(crate::served::CLIENT_LEFT.to_owned());
        }
        crate::probes::counted(self.socket, model, text, engine)
    }

    fn accepts(
        &self,
        model: &Path,
        filler: usize,
        length: usize,
        engine: Option<&str>,
    ) -> crate::probes::Accepted {
        if self.asker_gone() {
            return crate::probes::Accepted::CouldNotTell(crate::served::CLIENT_LEFT.to_owned());
        }
        crate::probes::accepts(self.socket, model, filler, length, engine)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Asked<'a> {
    pub engine: Option<&'a str>,
    pub apply: bool,
    pub up_to: Option<usize>,
    pub only: &'a [String],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Step {
    pub name: &'static str,
    pub count: usize,
    pub of: usize,
}

impl Step {
    #[must_use]
    pub fn to_value(&self) -> mcf_record::json::Value {
        use mcf_record::json::Value;
        Value::map([
            ("name", Value::text(self.name.to_owned())),
            ("count", Value::Integer(as_integer(self.count))),
            ("of", Value::Integer(as_integer(self.of))),
        ])
    }
}

pub const PROBES: [&str; 9] = [
    "chat-template",
    "context",
    "stop-conditions",
    "tool-calls",
    "structured-output",
    "thinking",
    "language-cost",
    "embedding",
    "vision",
];

const THROUGH_THE_ADDRESSING: [&str; 3] = ["tool-calls", "structured-output", "thinking"];

#[must_use]
pub fn planned(only: &[String]) -> Vec<&'static str> {
    if only.is_empty() {
        return PROBES.to_vec();
    }
    let wants = |name: &str| only.iter().any(|asked| asked == name);
    let needs_addressing = THROUGH_THE_ADDRESSING.iter().any(|name| wants(name));
    PROBES
        .into_iter()
        .filter(|name| wants(name) || (*name == "chat-template" && needs_addressing))
        .collect()
}

pub fn run(
    at: &Places<'_>,
    path: &Path,
    asked: &Asked<'_>,
    say: &mut dyn FnMut(&Step, &[String]) -> bool,
) -> Result<Vec<String>, String> {
    let Some(bytes) = read_prefix(path) else {
        return Err(format!(
            "{} could not be read as a model: MCF grew its read to the whole file and still \
             could not find a GGUF directory in it",
            path.display()
        ));
    };
    let Some(build) = probes::describe_engine(at.socket) else {
        return Err(format!(
            "nothing is listening on {}: a probe asks a model to do the thing, which needs an \
             engine (D42)",
            at.socket.display()
        ));
    };
    let home = at.models.parent().unwrap_or(at.models);
    let engine_asked: &str = asked.engine.unwrap_or_else(|| whichever_is_here(home));
    let engine = format!(
        "{}, through the daemon at {build}",
        resolved_engine(at, engine_asked)
    );
    let mut trial = |pieces: &[mcf_standin::tokenizer::Piece], budget: usize| {
        at.trial(path, "", Some(pieces), budget, Some(engine_asked))
    };
    let plan = planned(asked.only);
    let of = plan.len();
    let mut probed: Option<mcf_core::probe::Probed<Addressed>> = None;
    for (count, name) in plan.into_iter().enumerate() {
        let step = Step {
            name,
            count: count.saturating_add(1),
            of,
        };
        if !say(&step, &[]) {
            break;
        }
        let best = probed
            .as_ref()
            .and_then(|probed| probed.outcome.observed())
            .and_then(|addressed: &Addressed| addressed.best_addressing.as_ref());
        let lines = match name {
            "chat-template" => {
                let found =
                    probes::chat_template(path, &bytes, TRIALS, BUDGET, &engine, &mut trial);
                let mut lines = addressing_lines(&found, path, &engine);
                let applied = if asked.apply {
                    Some(apply_addressing(home, path, &found, &engine))
                } else {
                    against_what_was_applied(home, path, &found, &engine)
                };
                if let Some(said) = applied {
                    lines.push(String::new());
                    lines.extend(said);
                }
                probed = Some(found);
                lines
            }
            "context" => context_lines(at, path, &bytes, &engine, engine_asked, asked.up_to),
            "stop-conditions" => {
                stopping_lines(home, at, path, &bytes, &engine, engine_asked, asked.apply)
            }
            "tool-calls" => tool_lines(at, path, &bytes, &engine, engine_asked, best),
            "structured-output" => structured_lines(at, path, &bytes, &engine, engine_asked, best),
            "thinking" => thinking_lines(
                home,
                at,
                path,
                &bytes,
                &engine,
                engine_asked,
                best,
                asked.apply,
            ),
            "language-cost" => language_lines(at, path, &engine, engine_asked),
            "embedding" => embedding_lines(path, &bytes),
            "vision" => vision_lines(home, path, &bytes),
            _ => Vec::new(),
        };
        if !say(&step, &lines) {
            break;
        }
    }
    let mut closing = declined_lines();
    closing.push(
        "  Nothing was configured. A probe writes what it observed; changing how MCF addresses \
         this model is an act somebody takes, and it is recorded (D42, D43)."
            .to_owned(),
    );
    Ok(closing)
}

pub const RECORDED: [&str; 9] = [
    "chat-template",
    "usable-context",
    "stop-conditions",
    "tool-calling",
    "structured-output",
    "thinking",
    "language-cost",
    "embedding",
    "vision",
];

#[must_use]
pub fn recorded_said(body: &mcf_record::json::Value) -> String {
    use mcf_record::json::Value;
    if let Some(said) = crate::examine::recorded_said(body) {
        return said;
    }
    let text = |key: &str| body.get(key).and_then(Value::as_text);
    let figure = |key: &str| body.get(key).and_then(Value::as_integer);
    match text("method") {
        Some("chat-template") => match (text("best_addressing"), figure("ended_their_turn")) {
            (Some(best), Some(ended)) => {
                format!("addressed as {best}; ended its turn in {ended} trial(s)")
            }
            _ => "could not tell".to_owned(),
        },
        Some("stop-conditions") => match (
            figure("stopped_of"),
            figure("trials"),
            figure("longest_tokens"),
        ) {
            (Some(stopped), Some(trials), Some(longest)) => format!(
                "ended its own turn in {stopped} of {trials} trial(s), the longest {longest} token(s)"
            ),
            _ => "could not tell".to_owned(),
        },
        Some("usable-context") => match (figure("accepted_tokens"), figure("declared_tokens")) {
            (Some(accepted), Some(declared)) => {
                format!("accepted {accepted} of {declared} declared token(s)")
            }
            _ => text("because").map_or_else(|| "could not tell".to_owned(), str::to_owned),
        },
        Some("tool-calling") => match (
            figure("well_formed"),
            figure("malformed"),
            figure("no_call"),
        ) {
            (Some(well), Some(malformed), Some(none)) => format!(
                "{}called well-formed in {well} trial(s), {malformed} malformed, {none} without a call{}",
                if matches!(body.get("declared_support"), Some(Value::Bool(false))) {
                    "declares no support; "
                } else {
                    ""
                },
                text("best_offering")
                    .map_or_else(String::new, |best| format!("; best offered {best}"))
            ),
            _ => "could not tell".to_owned(),
        },
        Some("structured-output") => {
            match (figure("conformed"), figure("departed"), figure("no_object")) {
                (Some(conformed), Some(departed), Some(none)) => format!(
                    "conformed in {conformed} trial(s), {departed} departed, {none} gave no object{}",
                    text("best_framing")
                        .map_or_else(String::new, |best| format!("; best framed {best}"))
                ),
                _ => "could not tell".to_owned(),
            }
        }
        Some("thinking") => match (figure("opened"), figure("closed"), figure("trials")) {
            (Some(0), _, Some(trials)) => format!("did not think in {trials} trial(s)"),
            (Some(opened), Some(closed), Some(trials)) => format!(
                "thought in {opened} of {trials} trial(s), closed it in {closed}{}",
                figure("before_the_answer_tokens").map_or_else(String::new, |before| format!(
                    "; {before} token(s) before the answer"
                ))
            ),
            _ => "could not tell".to_owned(),
        },
        Some("language-cost") => match (text("cheapest_language"), text("dearest_language")) {
            (Some(cheapest), Some(dearest)) => {
                format!("cheapest in {cheapest}, dearest in {dearest}")
            }
            _ => "could not tell".to_owned(),
        },
        Some("vision") => match body.get("answers_differ") {
            Some(Value::Bool(true)) => "told a circle from a triangle".to_owned(),
            Some(Value::Bool(false)) => "did not tell a circle from a triangle".to_owned(),
            _ => "could not tell".to_owned(),
        },
        Some("embedding") => match figure("width") {
            Some(width) => format!(
                "embeds at width {width}{}",
                match body.get("identical_twice") {
                    Some(Value::Bool(true)) => ", the same vector twice",
                    Some(Value::Bool(false)) => ", a different vector the second time",
                    _ => "",
                }
            ),
            None => "does not embed".to_owned(),
        },
        _ => recorded_figures(body),
    }
}

fn recorded_figures(body: &mcf_record::json::Value) -> String {
    use mcf_record::json::Value;
    let Value::Map(fields) = body else {
        return "recorded".to_owned();
    };
    let said: Vec<String> = fields
        .iter()
        .filter(|(key, _)| !matches!(key.as_str(), "model" | "method" | "engine"))
        .filter_map(|(key, value)| match value {
            Value::Integer(held) => Some(format!("{} {held}", key.replace('_', " "))),
            Value::Bool(held) => Some(format!("{} {held}", key.replace('_', " "))),
            Value::Text(held) if held.len() <= 40 => {
                Some(format!("{} {held}", key.replace('_', " ")))
            }
            _ => None,
        })
        .take(4)
        .collect();
    if said.is_empty() {
        "recorded".to_owned()
    } else {
        said.join(", ")
    }
}

#[must_use]
pub fn read_prefix(path: &Path) -> Option<Vec<u8>> {
    use std::io::Read as _;

    let held = std::fs::metadata(path).map_or(0, |meta| meta.len());
    for cap in [16_u64 << 20, 256 << 20, u64::MAX] {
        let take = cap.min(held);
        let mut prefix = Vec::new();
        std::fs::File::open(path)
            .and_then(|handle| handle.take(take).read_to_end(&mut prefix))
            .ok()?;
        if mcf_standin::gguf::parse(&prefix).is_ok() {
            return Some(prefix);
        }
        if take >= held {
            return None;
        }
    }
    None
}

fn record_configured(
    model: &Path,
    addressing: &crate::configured::Addressing,
) -> Result<std::path::PathBuf, mcf_core::Failure> {
    use mcf_record::json::Value;
    let Some(path) = mcf_record::journal::default_path() else {
        return Err(mcf_core::Failure::new(
            mcf_core::failure::Category::RecordUnwritable,
            mcf_core::failure::Attribution::Machine,
            mcf_core::failure::Disposition::Refused,
            mcf_core::failure::Subsystem::new("mcf-serve::probes::run"),
            "there is nowhere to record the configuration",
        ));
    };
    let body = Value::map([
        ("model", Value::text(model.display().to_string())),
        ("addressing", addressing.to_value()),
        ("was", Value::text("raw text, MCF's default")),
    ]);
    let mut journal = mcf_record::journal::Journal::open(&path)?;
    journal.append(&mcf_record::journal::Entry::new(
        mcf_record::journal::EntryKind::ModelConfigured,
        mcf_core::time::Timestamp::now(),
        body,
    ))?;
    Ok(path)
}

fn record_probed_context(
    model: &Path,
    context: &probes::Context,
    engine: &str,
) -> Result<std::path::PathBuf, mcf_core::Failure> {
    use mcf_record::json::Value;
    let mut body = vec![
        (
            "declared_tokens",
            Value::Integer(as_integer(context.declared)),
        ),
        (
            "accepted_tokens",
            Value::Integer(as_integer(context.accepted)),
        ),
        (
            "asked_up_to_tokens",
            Value::Integer(as_integer(context.ceiling)),
        ),
    ];
    if let Some(because) = &context.because {
        body.push(("because", Value::text(because.clone())));
    }
    record_probed(model, probes::USABLE_CONTEXT.name, engine, body)
}

pub(crate) fn record_probed(
    model: &Path,
    method: &str,
    engine: &str,
    fields: Vec<(&'static str, mcf_record::json::Value)>,
) -> Result<std::path::PathBuf, mcf_core::Failure> {
    use mcf_record::json::Value;
    let Some(path) = mcf_record::journal::default_path() else {
        return Err(mcf_core::Failure::new(
            mcf_core::failure::Category::RecordUnwritable,
            mcf_core::failure::Attribution::Machine,
            mcf_core::failure::Disposition::Refused,
            mcf_core::failure::Subsystem::new("mcf-serve::probes::run"),
            "there is nowhere to record what the probe observed",
        ));
    };
    let mut body = vec![
        ("model", Value::text(model.display().to_string())),
        ("method", Value::text(method.to_owned())),
        ("engine", Value::text(engine.to_owned())),
    ];
    body.extend(fields);
    let mut journal = mcf_record::journal::Journal::open(&path)?;
    journal.append(&mcf_record::journal::Entry::new(
        mcf_record::journal::EntryKind::ModelProbed,
        mcf_core::time::Timestamp::now(),
        Value::map(body),
    ))?;
    Ok(path)
}

pub(crate) fn record_readings(
    model: &Path,
    method: &str,
    engine: &str,
    conditions: Vec<(&str, mcf_record::json::Value)>,
    rows: &[mcf_record::readings::Reading],
) -> Result<std::path::PathBuf, mcf_core::Failure> {
    let Some(path) = mcf_record::journal::default_path() else {
        return Err(mcf_core::Failure::new(
            mcf_core::failure::Category::RecordUnwritable,
            mcf_core::failure::Attribution::Machine,
            mcf_core::failure::Disposition::Refused,
            mcf_core::failure::Subsystem::new("mcf-serve::probes::run"),
            "there is nowhere to record the readings",
        ));
    };
    let body = mcf_record::readings::run_body(
        &model.display().to_string(),
        method,
        engine,
        conditions,
        rows,
    );
    let mut journal = mcf_record::journal::Journal::open(&path)?;
    journal.append(&mcf_record::journal::Entry::new(
        mcf_record::journal::EntryKind::Readings,
        mcf_core::time::Timestamp::now(),
        body,
    ))?;
    Ok(path)
}

#[allow(clippy::too_many_arguments, reason = "one part's fields, each named")]
pub(crate) fn record_part(
    model: &Path,
    method: &str,
    engine: &str,
    conditions: Vec<(&str, mcf_record::json::Value)>,
    rows: &[mcf_record::readings::Reading],
    run: &str,
    part: u64,
    ended: Option<&str>,
) -> Result<std::path::PathBuf, mcf_core::Failure> {
    let Some(path) = mcf_record::journal::default_path() else {
        return Err(mcf_core::Failure::new(
            mcf_core::failure::Category::RecordUnwritable,
            mcf_core::failure::Attribution::Machine,
            mcf_core::failure::Disposition::Refused,
            mcf_core::failure::Subsystem::new("mcf-serve::probes::run"),
            "there is nowhere to record the readings",
        ));
    };
    let body = mcf_record::readings::part_body(
        &model.display().to_string(),
        method,
        engine,
        conditions,
        rows,
        run,
        part,
        ended,
    );
    let mut journal = mcf_record::journal::Journal::open(&path)?;
    journal.append(&mcf_record::journal::Entry::new(
        mcf_record::journal::EntryKind::Readings,
        mcf_core::time::Timestamp::now(),
        body,
    ))?;
    Ok(path)
}

fn rows_recorded(
    model: &Path,
    method: &str,
    engine: &str,
    rows: &[mcf_record::readings::Reading],
) -> String {
    if rows.is_empty() {
        return String::new();
    }
    match record_readings(model, method, engine, Vec::new(), rows) {
        Ok(_) => format!("  {} reading(s) recorded", rows.len()),
        Err(why) => format!("  READINGS NOT RECORDED: {why}"),
    }
}

fn reading(
    dims: &[(&str, mcf_record::json::Value)],
    metric: &str,
    value: usize,
    unit: &str,
) -> mcf_record::readings::Reading {
    mcf_record::readings::Reading::new(dims, metric, as_integer(value), unit)
}

fn counted_rows(
    dimension: &str,
    metric: &str,
    per: &[(String, usize)],
) -> Vec<mcf_record::readings::Reading> {
    per.iter()
        .map(|(name, count)| {
            reading(
                &[(dimension, mcf_record::json::Value::text(name.clone()))],
                metric,
                *count,
                "count",
            )
        })
        .collect()
}

fn as_integer(held: usize) -> i64 {
    i64::try_from(held).unwrap_or(i64::MAX)
}

fn addressing_lines(
    probed: &mcf_core::probe::Probed<Addressed>,
    path: &std::path::Path,
    engine: &str,
) -> Vec<String> {
    let mut lines = vec![
        format!("  {}", probed.method.name),
        format!(" asks {}", probed.method.asks),
        format!(" decides  {}", probed.method.decides),
        String::new(),
    ];
    match &probed.outcome {
        Outcome::Observed(addressed) => {
            lines.extend(observed(addressed));
            lines.push(recorded(record_probed(
                path,
                probed.method.name,
                engine,
                addressing_fields(addressed),
            )));
            let mut rows = counted_rows("addressing", "stopped", &addressed.stopped);
            rows.extend(counted_rows("addressing", "silent", &addressed.silent));
            for (name, lengths) in &addressed.lengths {
                for (trial, length) in lengths.iter().enumerate() {
                    rows.push(reading(
                        &[
                            ("addressing", mcf_record::json::Value::text(name.clone())),
                            ("trial", mcf_record::json::Value::Integer(as_integer(trial))),
                        ],
                        "turn_tokens",
                        *length,
                        "tokens",
                    ));
                }
            }
            lines.push(rows_recorded(path, probed.method.name, engine, &rows));
        }
        Outcome::Inconclusive { because } => {
            lines.push(format!(" INCONCLUSIVE — {because}"));
            lines.push(
                " which licenses nothing: MCF configures no differently than before, and \
                 this is not a negative result (D42, §3.18)"
                    .to_owned(),
            );
        }
    }
    lines.push(String::new());
    lines.push(format!(
        "  {} trial(s), {} token(s) spent",
        probed.trials, probed.tokens
    ));
    lines.push(format!("  under: {}", probed.conditions));
    lines
}

fn tool_fields(
    calling: &crate::probes::tools::Calling,
) -> Vec<(&'static str, mcf_record::json::Value)> {
    vec![
        (
            "well_formed",
            mcf_record::json::Value::Integer(counted(&calling.well_formed)),
        ),
        (
            "malformed",
            mcf_record::json::Value::Integer(counted(&calling.malformed)),
        ),
        (
            "no_call",
            mcf_record::json::Value::Integer(counted(&calling.no_call)),
        ),
        (
            "trials_per_offering",
            mcf_record::json::Value::Integer(i64::try_from(calling.of).unwrap_or(i64::MAX)),
        ),
        (
            "declared_support",
            mcf_record::json::Value::Bool(calling.declared.claims_support()),
        ),
        (
            "declared_form",
            mcf_record::json::Value::text(calling.declared.form.as_str()),
        ),
        (
            "best_offering",
            calling
                .best
                .clone()
                .map_or(mcf_record::json::Value::Null, mcf_record::json::Value::text),
        ),
    ]
}

fn per_framing(structured: &crate::probes::structured::Structured) -> Vec<String> {
    let at = |per: &[(String, usize)], name: &String| {
        per.iter()
            .find(|(other, _)| other == name)
            .map_or(0, |(_, count)| *count)
    };
    structured
        .conformed
        .iter()
        .map(|(name, good)| {
            format!(
                "   {name}: {good} conformed, {} departed, {} produced no object, {} still \
                 going when the budget ran out, of {}",
                at(&structured.departed, name),
                at(&structured.no_object, name),
                at(&structured.unfinished, name),
                structured.of
            )
        })
        .collect()
}

fn structured_fields(
    structured: &crate::probes::structured::Structured,
) -> Vec<(&'static str, mcf_record::json::Value)> {
    vec![
        (
            "conformed",
            mcf_record::json::Value::Integer(counted(&structured.conformed)),
        ),
        (
            "departed",
            mcf_record::json::Value::Integer(counted(&structured.departed)),
        ),
        (
            "no_object",
            mcf_record::json::Value::Integer(counted(&structured.no_object)),
        ),
        (
            "with_extra_keys",
            mcf_record::json::Value::Integer(
                i64::try_from(structured.with_extra).unwrap_or(i64::MAX),
            ),
        ),
        (
            "trials_per_framing",
            mcf_record::json::Value::Integer(i64::try_from(structured.of).unwrap_or(i64::MAX)),
        ),
        (
            "best_framing",
            structured
                .best
                .clone()
                .map_or(mcf_record::json::Value::Null, mcf_record::json::Value::text),
        ),
    ]
}

fn stopping_fields(
    stopping: &crate::probes::Stopping,
) -> Vec<(&'static str, mcf_record::json::Value)> {
    vec![
        (
            "stopped_of",
            mcf_record::json::Value::Integer(i64::try_from(stopping.stopped).unwrap_or(i64::MAX)),
        ),
        (
            "trials",
            mcf_record::json::Value::Integer(i64::try_from(stopping.of).unwrap_or(i64::MAX)),
        ),
        (
            "longest_tokens",
            mcf_record::json::Value::Integer(i64::try_from(stopping.longest).unwrap_or(i64::MAX)),
        ),
        (
            "before_the_answer_tokens",
            stopping
                .before
                .map_or(mcf_record::json::Value::Null, |before| {
                    mcf_record::json::Value::Integer(i64::try_from(before).unwrap_or(i64::MAX))
                }),
        ),
        (
            "default_budget",
            mcf_record::json::Value::Integer(
                i64::try_from(stopping.default_budget).unwrap_or(i64::MAX),
            ),
        ),
    ]
}

fn addressing_fields(addressed: &Addressed) -> Vec<(&'static str, mcf_record::json::Value)> {
    vec![
        (
            "best_addressing",
            mcf_record::json::Value::text(addressed.best.clone()),
        ),
        (
            "trials_per_addressing",
            mcf_record::json::Value::Integer(i64::try_from(addressed.of).unwrap_or(i64::MAX)),
        ),
        (
            "ended_their_turn",
            mcf_record::json::Value::Integer(counted(&addressed.stopped)),
        ),
        (
            "said_nothing",
            mcf_record::json::Value::Integer(counted(&addressed.silent)),
        ),
    ]
}

fn language_lines(
    at: &Places<'_>,
    path: &std::path::Path,
    engine: &str,
    asked: &str,
) -> Vec<String> {
    let mut count = |text: &str| at.counted(path, text, Some(asked));
    let probed = crate::probes::language::language_cost(path, engine, &mut count);
    let mut lines = vec![
        format!("  {}", probed.method.name),
        format!(" asks {}", probed.method.asks),
        format!(" decides  {}", probed.method.decides),
        String::new(),
    ];
    match &probed.outcome {
        Outcome::Observed(spend) => {
            lines.push(" observed".to_owned());
            for cost in &spend.costs {
                lines.push(format!(
                    "   {:<9} {:>3} token(s) for {:>3} character(s) — {} of the English",
                    cost.language,
                    cost.tokens,
                    cost.characters,
                    per_cent(cost.against_english_ppm)
                ));
            }
            for (language, why) in &spend.unencodable {
                lines.push(format!(
                    "   {language:<9} could not be counted, which is a fact about the file \
                     rather than about the language: {why}"
                ));
            }
            lines.push(format!(" counted by {}", spend.read_by));
            lines.push(recorded(record_probed(
                path,
                probed.method.name,
                engine,
                language_fields(spend),
            )));
            let mut rows = Vec::new();
            for cost in &spend.costs {
                let dims = [("language", mcf_record::json::Value::text(cost.language))];
                rows.push(reading(&dims, "tokens", cost.tokens, "tokens"));
                rows.push(reading(&dims, "characters", cost.characters, "count"));
            }
            lines.push(rows_recorded(path, probed.method.name, engine, &rows));
            lines.push(format!(
                " observed   this vocabulary spends most on {} and least on {}",
                spend.dearest, spend.cheapest
            ));
            lines.push(
                " which is a cost and NOT a grade: tokens are context, budget and time, and a \
                 model can be excellent at a language its vocabulary spells expensively (F81)"
                    .to_owned(),
            );
        }
        Outcome::Inconclusive { because } => {
            lines.push(format!(" INCONCLUSIVE — {because}"));
            lines.push(" which licenses nothing, and is not a negative result".to_owned());
        }
    }
    lines.push(String::new());
    lines.push(format!(
        "  {} sample(s), {} token(s) generated — none: this counts, it does not generate",
        probed.trials, probed.tokens
    ));
    lines.push(format!("  under: {}", probed.conditions));
    lines.push(String::new());
    lines
}

fn language_fields(
    spend: &crate::probes::language::Spend,
) -> Vec<(&'static str, mcf_record::json::Value)> {
    vec![
        (
            "dearest_language",
            mcf_record::json::Value::text(spend.dearest.to_owned()),
        ),
        (
            "cheapest_language",
            mcf_record::json::Value::text(spend.cheapest.to_owned()),
        ),
        (
            "tokens_per_language",
            mcf_record::json::Value::Map(
                spend
                    .costs
                    .iter()
                    .map(|cost| {
                        (
                            cost.language.to_owned(),
                            mcf_record::json::Value::Integer(
                                i64::try_from(cost.tokens).unwrap_or(i64::MAX),
                            ),
                        )
                    })
                    .collect(),
            ),
        ),
        (
            "counted_by",
            mcf_record::json::Value::text(spend.read_by.clone()),
        ),
    ]
}

#[allow(
    clippy::too_many_arguments,
    reason = "one argument per thing the section is about: where, what, through which engine, under which addressing, and whether to act"
)]
fn thinking_lines(
    home: &std::path::Path,
    at: &Places<'_>,
    path: &std::path::Path,
    bytes: &[u8],
    engine: &str,
    asked: &str,
    addressing: Option<&crate::probes::Addressing>,
    apply: bool,
) -> Vec<String> {
    let under = thinking_turn(bytes, addressing);
    let addressing = under.as_ref().map(|held| &held.addressing);
    let mut ask = |budget: usize| {
        let spoken = match addressing {
            Some(held) => at.spoken(
                path,
                "",
                Some(&held.wrap(crate::probes::thinking::QUESTION)),
                budget,
                Some(asked),
            ),
            None => at.spoken(
                path,
                crate::probes::thinking::QUESTION,
                None,
                budget,
                Some(asked),
            ),
        };
        (spoken.trial, spoken.text)
    };
    let probed = crate::probes::thinking::thinking(
        path,
        bytes,
        TRIALS,
        THINKING_BUDGET,
        engine,
        under
            .as_ref()
            .and_then(|held| held.opened_by_turn.as_deref()),
        &mut ask,
    );

    let mut lines = vec![
        format!("  {}", probed.method.name),
        format!(" asks {}", probed.method.asks),
        format!(" decides  {}", probed.method.decides),
        String::new(),
    ];
    match &probed.outcome {
        Outcome::Observed(spends) => {
            lines.extend(thinking_under(under.as_ref()));
            lines.extend(thinking_observed(spends));
            let mut fields = thinking_fields(spends);
            fields.push((
                "under",
                addressing.map_or(mcf_record::json::Value::Null, |held| {
                    mcf_record::json::Value::text(held.name.clone())
                }),
            ));
            lines.push(recorded(record_probed(
                path,
                probed.method.name,
                engine,
                fields,
            )));
            let rows = vec![
                reading(&[], "trials", spends.trials, "count"),
                reading(&[], "opened", spends.opened, "count"),
                reading(&[], "closed", spends.closed, "count"),
                reading(
                    &[],
                    "longest_inside_tokens",
                    spends.longest_inside,
                    "tokens",
                ),
                reading(&[], "longest_turn_tokens", spends.longest_turn, "tokens"),
            ];
            lines.push(rows_recorded(path, probed.method.name, engine, &rows));
            lines.push(String::new());
            lines.push(thinking_verdict(spends));
            if apply {
                lines.push(String::new());
                lines.extend(raise_budget(home, path, &probed, spends, engine));
            }
        }
        Outcome::Inconclusive { because } => {
            lines.push(format!(" INCONCLUSIVE — {because}"));
            lines.push(" which licenses nothing, and is not a negative result".to_owned());
        }
    }
    lines.push(String::new());
    lines.push(format!(
        "  {} trial(s), {} token(s) spent",
        probed.trials, probed.tokens
    ));
    lines.push(format!("  under: {}", probed.conditions));
    lines.push(String::new());
    lines
}

fn thinking_turn(
    bytes: &[u8],
    addressing: Option<&crate::probes::Addressing>,
) -> Option<crate::probes::thinking::Under> {
    let held = addressing?;
    let pairs = mcf_standin::gguf::parse(bytes)
        .ok()
        .and_then(|file| {
            mcf_standin::tokenizer::Tokens::read(&file)
                .ok()
                .map(|tokens| crate::probes::thinking::pairs(&file, &tokens))
        })
        .unwrap_or_default();
    Some(crate::probes::thinking::under(held, &pairs))
}

fn thinking_under(under: Option<&crate::probes::thinking::Under>) -> Vec<String> {
    let Some(under) = under else {
        return vec![" under    the bare question, since no addressing was found".to_owned()];
    };
    let mut lines = vec![format!(" under    {}", under.addressing.name)];
    if under.changed {
        lines.push(
            "          the addressing on file ends in the marker that closes the thinking, so \
             a turn under it could never open one; the same addressing is asked with the \
             opening marker in its place, which is the other form this file's template \
             writes, and the closed form is still what MCF would address it as"
                .to_owned(),
        );
    } else if under.opened_by_turn.is_some() {
        lines.push(
            "          the addressing ends in the opening marker, so the turn itself opened it \
             and what is counted is how long the model stayed inside"
                .to_owned(),
        );
    }
    lines
}

fn thinking_observed(spends: &crate::probes::thinking::Spends) -> Vec<String> {
    let mut lines = vec![format!(
        " markers  this file holds {} it could be inside{}",
        spends.available.len(),
        match spends.available.first() {
            Some(first) => format!(", the first of them {first}"),
            None => String::new(),
        }
    )];
    if !spends.unpairable.is_empty() {
        lines.push(format!(
            "          and {} it could not pair, whose insides are not measured here{}",
            spends.unpairable.len(),
            match spends.unpairable.first() {
                Some(first) => format!(" — {first} among them"),
                None => String::new(),
            }
        ));
    }
    match &spends.used {
        Some(marker) => lines.push(format!(
            " {marker} opened in {} of {} turn(s), closed in {}",
            spends.opened, spends.trials, spends.closed
        )),
        None => lines.push(format!(" none of them opened in {} turn(s)", spends.trials)),
    }
    if spends.longest_turn > 0 {
        lines.push(format!(
            " the longest turn that finished ran {} token(s){}",
            spends.longest_turn,
            before_the_answer(spends.before_in_longest)
        ));
    }
    lines
}

fn thinking_verdict(spends: &crate::probes::thinking::Spends) -> String {
    let still_going = spends.opened.saturating_sub(spends.closed);
    if still_going > 0 {
        return format!(
            " DIVERGENCE {still_going} turn(s) were still inside their marker when the budget \
             of {} ran out. What was measured there is the budget and not the model, and MCF \
             allows 32 tokens unless told otherwise",
            spends.budget
        );
    }
    if spends.closed > 0 {
        return format!(
            " VERIFIED   a turn spends up to {} word(s) inside its marker before the answer \
             begins, so a budget that does not allow for them measures the budget",
            spends.longest_inside
        );
    }
    " VERIFIED   no turn opened a marker, so an answer here begins at the first token and \
     needs no allowance beyond itself"
        .to_owned()
}

fn thinking_fields(
    spends: &crate::probes::thinking::Spends,
) -> Vec<(&'static str, mcf_record::json::Value)> {
    vec![
        (
            "marker",
            spends
                .used
                .clone()
                .map_or(mcf_record::json::Value::Null, mcf_record::json::Value::text),
        ),
        (
            "opened",
            mcf_record::json::Value::Integer(as_count(spends.opened)),
        ),
        (
            "closed",
            mcf_record::json::Value::Integer(as_count(spends.closed)),
        ),
        (
            "trials",
            mcf_record::json::Value::Integer(as_count(spends.trials)),
        ),
        (
            "longest_inside_words",
            mcf_record::json::Value::Integer(as_count(spends.longest_inside)),
        ),
        (
            "longest_turn_tokens",
            mcf_record::json::Value::Integer(as_count(spends.longest_turn)),
        ),
        (
            "before_the_answer_tokens",
            spends
                .before_in_longest
                .map_or(mcf_record::json::Value::Null, |before| {
                    mcf_record::json::Value::Integer(as_count(before))
                }),
        ),
        (
            "budget",
            mcf_record::json::Value::Integer(as_count(spends.budget)),
        ),
    ]
}

fn as_count(held: usize) -> i64 {
    i64::try_from(held).unwrap_or(-1)
}

fn vision_lines(home: &std::path::Path, path: &std::path::Path, bytes: &[u8]) -> Vec<String> {
    let tool = crate::engines::discover(home)
        .into_iter()
        .find_map(|engine| engine.tool("llama-mtmd-cli").map(|at| (engine.commit, at)));
    let projector = crate::projector::beside(path);

    let Some((commit, binary)) = tool else {
        return vision_result_lines(
            path,
            &crate::probes::vision::without_a_tool(path),
            "no engine with a tool that takes an image",
        );
    };

    let engine = format!("provisioned llama.cpp @{commit}, driven with an image");
    let mut look = |image: &[u8], question: &str| {
        let projector = projector.as_ref()?;
        let at = std::env::temp_dir().join(format!("mcf-probe-{}.png", std::process::id()));
        std::fs::write(&at, image).ok()?;
        let spoke = std::process::Command::new(&binary)
            .arg("-m")
            .arg(path)
            .arg("--mmproj")
            .arg(projector)
            .arg("--image")
            .arg(&at)
            .arg("-p")
            .arg(question)
            .arg("-n")
            .arg("40")
            .arg("--no-warmup")
            .arg("--seed")
            .arg("41")
            .output();
        let _gone = std::fs::remove_file(&at);
        let spoke = spoke.ok()?;
        let said = String::from_utf8_lossy(&spoke.stdout).trim().to_owned();
        if said.is_empty() {
            return None;
        }
        let spent = said.split_whitespace().count();
        Some((said, spent))
    };

    let probed =
        crate::probes::vision::vision(path, bytes, projector.as_deref(), &engine, &mut look);
    vision_result_lines(path, &probed, &engine)
}

fn vision_fields(
    sees: &crate::probes::vision::Sees,
) -> Vec<(&'static str, mcf_record::json::Value)> {
    vec![
        (
            "architecture",
            mcf_record::json::Value::text(sees.declared.architecture.clone()),
        ),
        (
            "projector",
            sees.declared
                .projector
                .clone()
                .map_or(mcf_record::json::Value::Null, mcf_record::json::Value::text),
        ),
        (
            "answers_differ",
            mcf_record::json::Value::Bool(sees.answers_differ),
        ),
        (
            "about_triangle",
            mcf_record::json::Value::text(sees.about_triangle.clone()),
        ),
        (
            "about_circle",
            mcf_record::json::Value::text(sees.about_circle.clone()),
        ),
    ]
}

fn vision_result_lines(
    path: &std::path::Path,
    probed: &mcf_core::probe::Probed<crate::probes::vision::Sees>,
    engine: &str,
) -> Vec<String> {
    let mut lines = vec![
        format!("  {}", probed.method.name),
        format!(" asks {}", probed.method.asks),
        format!(" decides  {}", probed.method.decides),
        String::new(),
    ];
    match &probed.outcome {
        Outcome::Observed(sees) => {
            lines.push(format!(
                " declared  the file says it is {}{}",
                sees.declared.architecture,
                match &sees.declared.projector {
                    Some(at) => format!(", and a projector was found at {at}"),
                    None => ", and no projector was found beside it".to_owned(),
                }
            ));
            lines.push(format!(
                "       shown {}, it said: {}",
                crate::probes::vision::Shape::Triangle.said(),
                sees.about_triangle
            ));
            lines.push(format!(
                "       shown {}, it said: {}",
                crate::probes::vision::Shape::Circle.said(),
                sees.about_circle
            ));
            lines.push(recorded(record_probed(
                path,
                probed.method.name,
                &probed.conditions.to_string(),
                vision_fields(sees),
            )));
            let rows = vec![reading(
                &[],
                "answers_differ",
                usize::from(sees.answers_differ),
                "bool",
            )];
            lines.push(rows_recorded(path, probed.method.name, engine, &rows));
            lines.push(String::new());
            if sees.answers_differ {
                lines.push(
                    " VERIFIED   two different pictures produced two different answers, so the \
                     image reached the model. Whether it is RIGHT about either is a graded \
                     task and not this (§XIII)"
                        .to_owned(),
                );
            } else {
                lines.push(
                    " REFUSED    two different pictures produced the same answer, so the image \
                     did not reach the model here. Every answer this artifact gives about a \
                     picture is a text-only answer, and a measurement taken through it is of a \
                     different thing than it is named for (§3.8)"
                        .to_owned(),
                );
            }
        }
        Outcome::Inconclusive { because } => {
            lines.push(format!(" INCONCLUSIVE — {because}"));
            lines.push(" which licenses nothing, and is not a negative result".to_owned());
        }
    }
    lines.push(String::new());
    lines.push(format!(
        "  {} trial(s), {} word(s) spent",
        probed.trials, probed.tokens
    ));
    lines.push(format!("  under: {}", probed.conditions));
    lines.push(String::new());
    lines
}

fn embedding_lines(path: &std::path::Path, bytes: &[u8]) -> Vec<String> {
    let mut ask = |text: &str| probes::embedding::measured(path, text);
    let engine = crate::probes::embedding::in_process();
    let probed = crate::probes::embedding::embedding(path, bytes, &engine, &mut ask);
    let mut lines = vec![
        format!("  {}", probed.method.name),
        format!(" asks {}", probed.method.asks),
        format!(" decides  {}", probed.method.decides),
        String::new(),
    ];
    match &probed.outcome {
        Outcome::Observed(embeds) => {
            lines.push(format!(
                " declared  the file says it is {}{}",
                embeds.declared.architecture,
                match embeds.declared.width {
                    Some(width) => format!(", embedding width {width}"),
                    None => ", declaring no embedding width".to_owned(),
                }
            ));
            lines.push(format!(
                " observed  a vector of width {} came back for {} token(s), and asking twice \
                 produced {}",
                embeds.width,
                embeds.tokens,
                if embeds.identical_twice {
                    "the same vector to the last bit"
                } else {
                    "TWO DIFFERENT VECTORS, which MCF's own engine cannot do correctly"
                }
            ));
            lines.push(recorded(record_probed(
                path,
                probed.method.name,
                &engine,
                vec![
                    (
                        "width",
                        mcf_record::json::Value::Integer(
                            i64::try_from(embeds.width).unwrap_or(i64::MAX),
                        ),
                    ),
                    (
                        "identical_twice",
                        mcf_record::json::Value::Bool(embeds.identical_twice),
                    ),
                ],
            )));
            let rows = vec![
                reading(&[], "width", embeds.width, "count"),
                reading(
                    &[],
                    "identical_twice",
                    usize::from(embeds.identical_twice),
                    "bool",
                ),
                reading(&[], "tokens", embeds.tokens, "tokens"),
            ];
            lines.push(rows_recorded(path, probed.method.name, &engine, &rows));
            if embeds
                .declared
                .width
                .is_some_and(|width| width != embeds.width)
            {
                lines.push(format!(
                    " DIVERGENCE  the file declares {:?} and the vector that came back is {} \
                     wide (A21, B-058)",
                    embeds.declared.width, embeds.width
                ));
            }
        }
        Outcome::Inconclusive { because } => {
            lines.push(format!(" INCONCLUSIVE — {because}"));
        }
    }
    lines.push(String::new());
    lines.push(format!("  under: {}", probed.conditions));
    lines.push(String::new());
    lines
}

fn declined_lines() -> Vec<String> {
    let mut lines = vec![
        "  not probed, and why".to_owned(),
        " a modality MCF does not mention is one a reader assumes it checked".to_owned(),
        String::new(),
    ];
    for held in crate::probes::declined::DECLINED {
        lines.push(format!("   {} — DECLINED", held.modality));
        lines.push(format!("     looked for  {}", held.looked_for));
        lines.push(format!("     because     {}", held.because));
        lines.push(format!("     needs       {} ({})", held.needs, held.until));
    }
    lines.push(String::new());
    lines
}

#[allow(
    clippy::integer_division,
    reason = "a percentage to one decimal place, from integers, as `Headroom` and `compare` \
              render theirs"
)]
fn per_cent(ppm: u64) -> String {
    format!("{}.{}%", ppm / 10_000, (ppm % 10_000) / 1_000)
}

fn recorded(written: Result<std::path::PathBuf, mcf_core::Failure>) -> String {
    match written {
        Ok(journal) => format!(" recorded in {}", journal.display()),
        Err(failure) => format!(
            " BUT NOT RECORDED — {failure}; a measurement nobody can find later is the same as \
             one not taken (A1, A2)"
        ),
    }
}

fn counted(per: &[(String, usize)]) -> i64 {
    i64::try_from(
        per.iter()
            .map(|(_, count)| *count)
            .fold(0_usize, usize::saturating_add),
    )
    .unwrap_or(i64::MAX)
}

fn tool_verdict(calling: &crate::probes::tools::Calling) -> String {
    match &calling.best {
        Some(best) => {
            format!(" VERIFIED   this model emits well-formed calls, best under {best}")
        }
        None if calling.declared.claims_support() => " DIVERGENCE  the file claims tool support \
             and no trial produced a well-formed call. That is a disagreement between what the \
             artifact says and what it did, which is a finding rather than an error (A21, \
             B-058) — and it is not proof the model cannot: MCF chose how to describe the tool, \
             and that choice is a condition of this answer"
            .to_owned(),
        None => {
            " observed   no well-formed call, and the file claimed none. The two agree.".to_owned()
        }
    }
}

fn tool_lines(
    at: &Places<'_>,
    path: &std::path::Path,
    bytes: &[u8],
    engine: &str,
    asked: &str,
    addressing: Option<&crate::probes::Addressing>,
) -> Vec<String> {
    let mut ask = |pieces: &[mcf_standin::tokenizer::Piece], budget: usize| {
        let spoken = at.spoken(path, "", Some(pieces), budget, Some(asked));
        let answered = spoken.answered().to_owned();
        (spoken.trial, answered)
    };
    let probed = crate::probes::tools::tool_calling(
        path,
        bytes,
        addressing,
        TRIALS,
        TOOL_BUDGET,
        engine,
        &mut ask,
    );

    let mut lines = vec![
        format!("  {}", probed.method.name),
        format!(" asks {}", probed.method.asks),
        format!(" decides  {}", probed.method.decides),
        String::new(),
    ];
    match &probed.outcome {
        Outcome::Observed(calling) => {
            let declared = &calling.declared;
            lines.push(format!(
                " declared  the file {} tool support{}",
                if declared.claims_support() {
                    "claims"
                } else {
                    "claims no"
                },
                if declared.markers.is_empty() {
                    String::new()
                } else {
                    format!(" — its vocabulary carries {}", declared.markers.join(", "))
                }
            ));
            lines.push(format!(
                " declared  its template writes a call as {}",
                declared.form.as_str()
            ));
            lines.push(" observed".to_owned());
            for (name, good) in &calling.well_formed {
                let bad = calling
                    .malformed
                    .iter()
                    .find(|(other, _)| other == name)
                    .map_or(0, |(_, count)| *count);
                let none = calling
                    .no_call
                    .iter()
                    .find(|(other, _)| other == name)
                    .map_or(0, |(_, count)| *count);
                lines.push(format!(
                    "   {name}: {good} well formed, {bad} malformed, {none} no call, of {}",
                    calling.of
                ));
            }
            for reason in &calling.reasons {
                lines.push(format!("   what came out — {reason}"));
            }
            lines.push(recorded(record_probed(
                path,
                probed.method.name,
                engine,
                tool_fields(calling),
            )));
            let mut rows = counted_rows("offering", "well_formed", &calling.well_formed);
            rows.extend(counted_rows("offering", "malformed", &calling.malformed));
            rows.extend(counted_rows("offering", "no_call", &calling.no_call));
            rows.push(reading(&[], "trials_per_offering", calling.of, "count"));
            lines.push(rows_recorded(path, probed.method.name, engine, &rows));
            lines.push(tool_verdict(calling));
        }
        Outcome::Inconclusive { because } => {
            lines.push(format!(" INCONCLUSIVE — {because}"));
            lines.push(" which licenses nothing, and is not a negative result".to_owned());
        }
    }
    lines.push(String::new());
    lines.push(format!(
        "  {} trial(s), {} token(s) spent",
        probed.trials, probed.tokens
    ));
    lines.push(format!(
        "  a trial had {TOOL_BUDGET} token(s): a model that did not call within that is \
         reported as not having called within it, never as unable to (A7)"
    ));
    lines.push(format!("  under: {}", probed.conditions));
    lines.push(String::new());
    lines
}

fn structured_lines(
    at: &Places<'_>,
    path: &std::path::Path,
    bytes: &[u8],
    engine: &str,
    asked: &str,
    addressing: Option<&crate::probes::Addressing>,
) -> Vec<String> {
    let mut ask = |pieces: &[mcf_standin::tokenizer::Piece], budget: usize| {
        let spoken = at.spoken(path, "", Some(pieces), budget, Some(asked));
        let answered = spoken.answered().to_owned();
        (spoken.trial, answered)
    };
    let probed = crate::probes::structured::structured_output(
        path,
        bytes,
        addressing,
        TRIALS,
        STRUCTURED_BUDGET,
        engine,
        &mut ask,
    );

    let mut lines = vec![
        format!("  {}", probed.method.name),
        format!(" asks {}", probed.method.asks),
        format!(" decides  {}", probed.method.decides),
        String::new(),
    ];
    match &probed.outcome {
        Outcome::Observed(structured) => {
            lines.push(
                " declared  nothing: a model file makes no claim about producing a shape, so \
                 there is no declaration for this to agree or disagree with (A7)"
                    .to_owned(),
            );
            lines.push(" observed".to_owned());
            lines.extend(per_framing(structured));
            for reason in &structured.reasons {
                lines.push(format!("   what came out — {reason}"));
            }
            if structured.with_extra > 0 {
                lines.push(format!(
                    "   {} conforming trial(s) also carried keys nobody asked for, which is \
                     recorded and is not a failure",
                    structured.with_extra
                ));
            }
            lines.push(recorded(record_probed(
                path,
                probed.method.name,
                engine,
                structured_fields(structured),
            )));
            let mut rows = counted_rows("framing", "conformed", &structured.conformed);
            rows.extend(counted_rows("framing", "departed", &structured.departed));
            rows.extend(counted_rows("framing", "no_object", &structured.no_object));
            rows.extend(counted_rows(
                "framing",
                "unfinished",
                &structured.unfinished,
            ));
            rows.push(reading(&[], "with_extra", structured.with_extra, "count"));
            rows.push(reading(&[], "trials_per_framing", structured.of, "count"));
            lines.push(rows_recorded(path, probed.method.name, engine, &rows));
            match &structured.best {
                Some(best) => lines.push(format!(
                    " VERIFIED   this model produces the shape it is asked for, best under \
                     {best} — and how often is the answer, not whether: {} of {} trial(s) \
                     conformed",
                    structured.conforming(),
                    structured.of.saturating_mul(structured.conformed.len())
                )),
                None => lines.push(
                    " observed   no trial produced the shape asked for. That is a fact about \
                     this model under three framings MCF chose, which are conditions of the \
                     answer — not a claim that it cannot (A7)"
                        .to_owned(),
                ),
            }
            let cut = counted(&structured.unfinished);
            if cut > 0 {
                lines.push(format!(
                    " and {cut} trial(s) were still going when the budget ran out, which is \
                     MCF interrupting the model rather than the model declining (A7)"
                ));
            }
        }
        Outcome::Inconclusive { because } => {
            lines.push(format!(" INCONCLUSIVE — {because}"));
            lines.push(" which licenses nothing, and is not a negative result".to_owned());
        }
    }
    lines.push(String::new());
    lines.push(format!(
        "  {} trial(s) per framing, {} token(s) spent",
        probed.trials, probed.tokens
    ));
    lines.push(format!(
        "  a trial had {STRUCTURED_BUDGET} token(s): a model that did not finish an object \
         within that is reported as not having, never as unable to (A7)"
    ));
    lines.push(format!("  under: {}", probed.conditions));
    lines.push(String::new());
    lines
}

fn against_what_was_applied(
    home: &std::path::Path,
    path: &std::path::Path,
    probed: &mcf_core::probe::Probed<Addressed>,
    engine: &str,
) -> Option<Vec<String>> {
    use crate::configured::Since;

    let stored = crate::configured::read(home, path)?;

    let mut said = vec![format!("  applied  {}", stored.provenance())];

    match &probed.outcome {
        Outcome::Observed(addressed) if addressed.best == stored.name => {
            said.push(
                "  agrees this run measured the same addressing that is applied, so the configuration is not merely old — it is confirmed"
                    .to_owned(),
            );
        }
        Outcome::Observed(addressed) => {
            said.push(format!(
                "  DIVERGENCE what is applied is {}, and this run measured {} as best. MCF's answer would now differ; applying it is an act: `mcf probe --apply`",
                stored.name, addressed.best
            ));
        }
        Outcome::Inconclusive { .. } => {
            said.push(
                "  unchanged  this run could not tell, which is not a disagreement with what is applied and does not license undoing it: an inconclusive probe leaves the capability where it was"
                    .to_owned(),
            );
        }
    }

    let build = mcf_core::build_identity::BuildIdentity::current().to_string();
    if let Since::ConditionsMoved(moved) = crate::configured::since(&stored, engine, &build) {
        for one in moved {
            said.push(format!(
                "  moved the {} it was taken through is not the one in force: was {}, now {}",
                one.what, one.was, one.now
            ));
        }
        said.push(
            " which does not mean the answer changed — two engines agreed on this question when it was measured — only that the evidence was gathered elsewhere"
                .to_owned(),
        );
    }
    Some(said)
}

fn apply_addressing(
    home: &std::path::Path,
    path: &std::path::Path,
    probed: &mcf_core::probe::Probed<Addressed>,
    engine: &str,
) -> Vec<String> {
    let Outcome::Observed(addressed) = &probed.outcome else {
        return vec![
            "  NOT APPLIED — the probe could not tell, and a configuration MCF cannot account \
             for is worse than none (D42, A7)"
                .to_owned(),
        ];
    };
    let Some(chosen) = &addressed.best_addressing else {
        return vec![
            "  NOT APPLIED — the probe named an addressing it cannot hand over, which is a \
             defect in MCF rather than an answer about this model"
                .to_owned(),
        ];
    };
    if chosen.pieces_before.is_empty() && chosen.pieces_after.is_empty() {
        return vec![
            "  NOT APPLIED — the addressing observed is raw, which is what MCF does already. \
             Writing that down would put a probe's provenance on a default and make it look \
             derived (A21)"
                .to_owned(),
        ];
    }

    let at = mcf_core::time::Timestamp::now();
    let addressing = crate::configured::Addressing {
        name: chosen.name.clone(),
        before: chosen.pieces_before.clone(),
        after: chosen.pieces_after.clone(),
        probe: probed.method.name.to_owned(),
        at: at.to_string(),
        build: mcf_core::build_identity::BuildIdentity::current().to_string(),
        conditions: engine.to_owned(),
    };
    match crate::configured::write(home, path, &addressing) {
        Err(failure) => vec![format!("  NOT APPLIED — {failure}")],
        Ok(written) => {
            let recorded = record_configured(path, &addressing);
            let mut said = vec![
                format!("  APPLIED  {}", addressing.provenance()),
                format!(" written to {}", written.display()),
                " `mcf run` addresses this model that way from now on, and every account says so"
                    .to_owned(),
                " measurements taken before and after this are not comparable — the conditions changed, and MCF says so rather than assuming"
                    .to_owned(),
            ];
            match recorded {
                Ok(journal) => said.push(format!(" recorded in {}", journal.display())),
                Err(failure) => said.push(format!(
                    " BUT NOT RECORDED — {failure}; a change nobody can find later is \
                     the silent part D43 forbids"
                )),
            }
            said
        }
    }
}

fn stopping_lines(
    home: &std::path::Path,
    at: &Places<'_>,
    path: &std::path::Path,
    bytes: &[u8],
    engine: &str,
    asked: &str,
    apply: bool,
) -> Vec<String> {
    if crate::probes::gguf_of(bytes).is_err() {
        return Vec::new();
    }
    let addressing = crate::configured::read(home, path);

    let mut ask = |question: &str, budget: usize| {
        let mut pieces = addressing
            .as_ref()
            .map_or_else(Vec::new, |held| held.before.clone());
        pieces.push(mcf_standin::tokenizer::Piece::Text(question.to_owned()));
        if let Some(held) = addressing.as_ref() {
            pieces.extend(held.after.iter().cloned());
        }
        at.trial(path, "", Some(&pieces), budget, Some(asked))
    };
    let probed = probes::stop_conditions(
        path,
        TRIALS,
        FROM,
        CEILING,
        crate::control::DEFAULT_LIMIT,
        engine,
        &mut ask,
    );

    let mut lines = vec![
        format!("  {}", probed.method.name),
        format!(" asks {}", probed.method.asks),
        format!(" decides  {}", probed.method.decides),
        String::new(),
    ];
    match &probed.outcome {
        Outcome::Observed(stopping) => {
            lines.push(format!(
                " ended its own turn in {} of {} trials, the longest running {} token(s){}",
                stopping.stopped,
                stopping.of,
                stopping.longest,
                before_the_answer(stopping.before)
            ));
            lines.push(recorded(record_probed(
                path,
                probed.method.name,
                engine,
                stopping_fields(stopping),
            )));
            let mut rows = vec![
                reading(&[], "longest_tokens", stopping.longest, "tokens"),
                reading(&[], "stopped", stopping.stopped, "count"),
                reading(&[], "trials", stopping.of, "count"),
                reading(&[], "ceiling_tokens", stopping.ceiling, "tokens"),
            ];
            if let Some(before) = stopping.before {
                rows.push(reading(&[], "before_answer_tokens", before, "tokens"));
            }
            lines.push(rows_recorded(path, probed.method.name, engine, &rows));
            lines.push(String::new());
            if stopping.longest > stopping.default_budget {
                lines.push(format!(
                    " DIVERGENCE MCF allows {} tokens unless told otherwise, and this model's turns run to {}. Every answer past that is cut off by MCF rather than finished by the model, which measures the budget and not the model",
                    stopping.default_budget, stopping.longest
                ));
            } else {
                lines.push(format!(
                    " agrees MCF's {} tokens is enough for this model's turns, the longest of which ran {}",
                    stopping.default_budget, stopping.longest
                ));
            }
            if apply {
                lines.push(String::new());
                lines.extend(apply_budget(home, path, &probed, stopping, engine));
            }
        }
        Outcome::Inconclusive { because } => {
            lines.push(format!(" INCONCLUSIVE — {because}"));
            lines.push(
                " which licenses nothing: MCF configures no differently than before, and this is not a negative result"
                    .to_owned(),
            );
        }
    }
    lines.push(String::new());
    lines.push(format!(
        "  {} trial(s), {} token(s) spent",
        probed.trials, probed.tokens
    ));
    lines.push(format!("  under: {}", probed.conditions));
    lines.push(String::new());
    lines
}

fn apply_budget(
    home: &std::path::Path,
    path: &std::path::Path,
    probed: &mcf_core::probe::Probed<crate::probes::Stopping>,
    stopping: &crate::probes::Stopping,
    engine: &str,
) -> Vec<String> {
    if stopping.longest <= stopping.default_budget {
        return vec![
            "  NOT APPLIED — MCF's default is already enough, and writing it down would put a probe's provenance on a default"
                .to_owned(),
        ];
    }
    let budget = crate::configured::Budget {
        tokens: stopping.longest,
        before: stopping.before,
        probe: probed.method.name.to_owned(),
        at: mcf_core::time::Timestamp::now().to_string(),
        build: mcf_core::build_identity::BuildIdentity::current().to_string(),
        conditions: engine.to_owned(),
    };
    match crate::configured::write_budget(home, path, &budget) {
        Err(failure) => vec![format!("  NOT APPLIED — {failure}")],
        Ok(_written) => vec![
            format!("  APPLIED  {}", budget.provenance()),
            " `mcf run` allows this model that many tokens unless --limit says otherwise, and the account says where the number came from"
                .to_owned(),
        ],
    }
}

fn before_the_answer(before: Option<usize>) -> String {
    match before {
        Some(before) => format!(", and up to {before} of a turn spent thinking before the answer"),
        None => String::new(),
    }
}

fn raise_budget(
    home: &std::path::Path,
    path: &std::path::Path,
    probed: &mcf_core::probe::Probed<crate::probes::thinking::Spends>,
    spends: &crate::probes::thinking::Spends,
    engine: &str,
) -> Vec<String> {
    let on_file = crate::configured::read_derived(home, path).budget;
    let covers = on_file
        .as_ref()
        .map_or(crate::control::DEFAULT_LIMIT, |held| held.tokens);
    if spends.longest_turn == 0 {
        return vec![
            "  NOT APPLIED — no turn here ended at the model's own stop token, so there is no turn to set a budget to"
                .to_owned(),
        ];
    }
    if spends.longest_turn <= covers {
        return vec![format!(
            "  NOT RAISED — the budget of {covers} {} covers the longest turn that thought here, {}",
            match on_file {
                Some(held) => format!("the {} probe set", held.probe),
                None => "MCF allows unless told otherwise".to_owned(),
            },
            spends.longest_turn
        )];
    }
    let budget = crate::configured::Budget {
        tokens: spends.longest_turn,
        before: spends.before_in_longest,
        probe: probed.method.name.to_owned(),
        at: mcf_core::time::Timestamp::now().to_string(),
        build: mcf_core::build_identity::BuildIdentity::current().to_string(),
        conditions: engine.to_owned(),
    };
    match crate::configured::write_budget(home, path, &budget) {
        Err(failure) => vec![format!("  NOT APPLIED — {failure}")],
        Ok(_written) => vec![
            format!("  APPLIED  {}", budget.provenance()),
            format!(
                " the budget of {covers} {} was shorter than a turn that thinks before it answers; `mcf run` allows this model the longer one unless --limit says otherwise. It is the longest turn seen on this probe's question and bounds no other: a thought is the size of its question, and one that takes more will run out inside the marker all the same",
                match on_file {
                    Some(held) => format!("the {} probe set", held.probe),
                    None => "MCF allows unless told otherwise".to_owned(),
                }
            ),
        ],
    }
}

fn context_lines(
    at: &Places<'_>,
    path: &std::path::Path,
    bytes: &[u8],
    engine: &str,
    asked: &str,
    up_to: Option<usize>,
) -> Vec<String> {
    let Ok(file) = crate::probes::gguf_of(bytes) else {
        return Vec::new();
    };
    let Some(declared) = crate::probes::declared_context(&file) else {
        return Vec::new();
    };
    let Some(filler) = crate::probes::a_filler_token(&file) else {
        return Vec::new();
    };

    let mut ask = |length: usize| at.accepts(path, filler, length, Some(asked));
    let ceiling = crate::probes::ceiling_of(declared, up_to);
    let projection = projected(ceiling, &mut ask);
    let projected_from = projection.as_ref().map(|_sentence| RATE_SAMPLE);
    if let Some(sentence) = &projection {
        eprintln!("  {sentence}");
    }
    let probed = probes::usable_context(path, declared, up_to, engine, &mut ask);

    let mut lines = vec![
        format!("  {}", probed.method.name),
        format!(" asks {}", probed.method.asks),
        format!(" decides  {}", probed.method.decides),
        String::new(),
    ];
    if let Some(sentence) = projection {
        lines.push(format!(" {sentence}"));
        lines.push(String::new());
    }
    match &probed.outcome {
        Outcome::Observed(context) => {
            lines.push(format!(" declared {} token(s)", context.declared));
            if context.ceiling.saturating_add(1) < context.declared {
                lines.push(format!(
                    " asked up to {} token(s), as --up-to said: the file's claim itself was not asked",
                    context.ceiling
                ));
            }
            lines.push(format!(
                " accepted {} token(s) of prompt, with one left to generate",
                context.accepted
            ));
            lines.push(String::new());
            lines.push(recorded(record_probed_context(path, context, engine)));
            let rows = vec![
                reading(&[], "declared_tokens", context.declared, "tokens"),
                reading(&[], "asked_up_to_tokens", context.ceiling, "tokens"),
                reading(&[], "accepted_tokens", context.accepted, "tokens"),
            ];
            lines.push(rows_recorded(
                path,
                probes::USABLE_CONTEXT.name,
                engine,
                &rows,
            ));
            if context.accepted.saturating_add(1) >= context.declared {
                lines.push(
                    " agrees the file's claim holds: every token it declares but one is taken as prompt, and the one left over is the answer"
                        .to_owned(),
                );
            } else if context.accepted == context.ceiling {
                lines.push(format!(
                    " agrees as far as it was asked: {} tokens were taken whole. The {} the file declares were not asked for, and this run settles nothing about them",
                    context.accepted, context.declared
                ));
            } else {
                lines.push(format!(
                    " DIVERGENCE the file declares {} tokens and this engine on this machine takes {}. A prompt planned against the declaration would be refused, or worse, quietly shortened — which is a measurement of a different prompt",
                    context.declared, context.accepted
                ));
                if let Some(because) = &context.because {
                    lines.push(format!(" the engine's own words: {because}"));
                }
            }
        }
        Outcome::Inconclusive { because } => {
            lines.push(format!(" INCONCLUSIVE — {because}"));
            lines.push(
                " which licenses nothing: MCF configures no differently than before, and this is not a negative result"
                    .to_owned(),
            );
        }
    }
    lines.push(String::new());
    lines.push(format!(
        "  {} trial(s), {} token(s) spent",
        probed.trials, probed.tokens
    ));
    if projected_from.is_some() {
        lines.push(format!(
            "  and 1 trial of {RATE_SAMPLE} token(s) to time the reading, not counted above"
        ));
    }
    lines.push(format!("  under: {}", probed.conditions));
    lines.push(String::new());
    lines
}

const RATE_SAMPLE: usize = 512;

fn projected(
    ceiling: usize,
    ask: &mut dyn FnMut(usize) -> crate::probes::Accepted,
) -> Option<String> {
    use crate::probes::Accepted;
    use mcf_core::time::{Clock as _, SystemClock};
    if ceiling <= RATE_SAMPLE {
        return None;
    }
    let clock = SystemClock;
    let began = clock.now();
    let read = match ask(RATE_SAMPLE) {
        Accepted::Read(read) if read == RATE_SAMPLE => read,
        Accepted::Read(_) | Accepted::Refused(_) | Accepted::CouldNotTell(_) => return None,
    };
    let nanos = clock.now().saturating_duration_since(began).as_nanos();
    Some(
        crate::probes::Projection {
            sample: read,
            nanos,
            target: ceiling,
        }
        .sentence(),
    )
}

fn observed(addressed: &Addressed) -> Vec<String> {
    let mut lines = vec![" answered, then ended at the model's own stop token:".to_owned()];
    for (name, ended) in &addressed.stopped {
        let mark = if *name == addressed.best { " ←" } else { "" };
        let quiet = addressed
            .silent
            .iter()
            .find(|(other, _)| other == name)
            .map_or(0, |(_, times)| *times);
        let note = if quiet > 0 {
            format!(
                " (ended without saying anything {quiet} of {})",
                addressed.of
            )
        } else {
            String::new()
        };
        let ran = addressed
            .lengths
            .iter()
            .find(|(other, _)| other == name)
            .map(|(_, ran)| ran.as_slice())
            .unwrap_or_default();
        let middle = ran.get(ran.len().wrapping_div(2));
        let span = match (ran.first(), middle, ran.last()) {
            (Some(least), _, Some(most)) if least == most => {
                format!(" turn ran {least} token(s)")
            }
            (Some(least), Some(middle), Some(most)) => {
                format!(" turn ran {least}-{most} token(s), middle {middle}")
            }
            _ => String::new(),
        };
        lines.push(format!(
            " {name:<18} {ended} of {}{mark}{note}{span}",
            addressed.of
        ));
    }
    lines.push(String::new());
    lines.push(format!(" best {}", addressed.best));

    let raw_is_best = addressed.best == "raw";
    let best_count = addressed
        .stopped
        .iter()
        .find(|(name, _)| *name == addressed.best)
        .map_or(0, |(_, ended)| *ended);
    lines.push(match (addressed.declared_a_template, raw_is_best) {
        (true, true) => format!(
            " DIVERGENCE the file declares a chat template, and the model ended its turn \
             {best_count} of {} times addressed *raw* while the addressing built from that \
             template ended none. That is a disagreement between the file and the model worth \
             a person's attention — and it is not a licence to address this model raw: what \
             this probe measures is whether a turn ends, which may be measuring something \
             else here (A21, D42)",
            addressed.of
        ),
        (true, false) => format!(
            " agrees the file declares a chat template and the model ends its turns under \
             {}, which is what MCF would address it as once somebody applies it; until then \
             `mcf run` sends the words raw, or framed by the engine where a switch asks it to",
            addressed.best
        ),
        (false, false) => format!(
            " DIVERGENCE the file declares no chat template, and the model ends its turns \
             under {} — a capability its own metadata does not mention (§3.18)",
            addressed.best
        ),
        (false, true) => " agrees no template declared, and raw is what it answers to".to_owned(),
    });
    lines
}

#[cfg(test)]
mod tests {
    #![allow(clippy::panic, clippy::expect_used, clippy::unwrap_used)]

    use super::{PROBES, planned};

    #[test]
    fn the_plan_is_the_probes_named_with_what_they_need() {
        assert_eq!(planned(&[]), PROBES.to_vec());
        assert_eq!(
            planned(&["context".to_owned(), "vision".to_owned()]),
            vec!["context", "vision"]
        );
        assert_eq!(
            planned(&["thinking".to_owned(), "tool-calls".to_owned()]),
            vec!["chat-template", "tool-calls", "thinking"]
        );
        assert_eq!(
            planned(&["chat-template".to_owned(), "thinking".to_owned()]),
            vec!["chat-template", "thinking"]
        );
        assert!(planned(&["no-such-probe".to_owned()]).is_empty());
    }

    #[test]
    fn a_recorded_finding_reads_back_as_its_figures() {
        use mcf_record::json::Value;
        let stopping = Value::map([
            ("method", Value::text("stop-conditions")),
            ("stopped_of", Value::Integer(5)),
            ("trials", Value::Integer(5)),
            ("longest_tokens", Value::Integer(51)),
        ]);
        assert_eq!(
            super::recorded_said(&stopping),
            "ended its own turn in 5 of 5 trial(s), the longest 51 token(s)"
        );
        let bare = Value::map([("method", Value::text("chat-template"))]);
        assert_eq!(super::recorded_said(&bare), "could not tell");
        let other = Value::map([
            ("method", Value::text("something-new")),
            ("model", Value::text("m")),
            ("english_tokens", Value::Integer(16)),
        ]);
        assert_eq!(super::recorded_said(&other), "english tokens 16");
        let thinking = Value::map([
            ("method", Value::text("thinking")),
            ("opened", Value::Integer(5)),
            ("closed", Value::Integer(5)),
            ("trials", Value::Integer(5)),
            ("before_the_answer_tokens", Value::Integer(242)),
        ]);
        assert_eq!(
            super::recorded_said(&thinking),
            "thought in 5 of 5 trial(s), closed it in 5; 242 token(s) before the answer"
        );
        let vision = Value::map([
            ("method", Value::text("vision")),
            ("answers_differ", Value::Bool(true)),
        ]);
        assert_eq!(
            super::recorded_said(&vision),
            "told a circle from a triangle"
        );
        assert_eq!(super::RECORDED.len(), super::PROBES.len());
    }
}
