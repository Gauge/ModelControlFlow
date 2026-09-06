//! The probes as one run (`mcf probe`, the window's capabilities card): asking a model to do the thing (B-051, B-052, D42, §3.18, §X).
//!
//! **What it is for.** §X's reason is §3.8's: a misconfigured model is a
//! measurement error. This is the command that finds the misconfiguration by
//! experiment rather than by reading a field — and that reports what it found
//! without configuring anything, because D42 makes a probe result a
//! measurement and never a default.
//!
//! **What it prints.** What was asked, what every candidate did across its
//! trials, what it cost in tokens, and the conditions the answer holds under.
//! A divergence between what the file *declared* and what the model *did* is
//! called out, because B-058 is right that it is often the most useful thing
//! MCF can say.

use std::path::Path;

use crate::probes::{self, Addressed};
use mcf_core::probe::Outcome;

/// How many trials each addressing gets.
///
/// Five, because the observation is a count and a count of one is an anecdote
/// — and because a model that stops sometimes is a different fact from one
/// that always does, which is exactly what §3.18 wants a probe to be able to
/// say.
const TRIALS: usize = 5;

/// The token budget one trial gets.
///
/// Enough for a short answer and its end-of-turn token, short enough that a
/// model which will not stop is found out quickly. Stated rather than tuned:
/// a probe that could not decide within it says so (D42's third state).
const BUDGET: usize = 320;

/// Where the stop-condition probe starts doubling.
///
/// Small on purpose: most turns are short, and a first budget large enough for
/// the worst case is spent on every trial (B49).
const FROM: usize = 32;

/// And where it stops. Reaching this is *not within this many tokens*, never
/// *never* (A7).
const CEILING: usize = 1024;

/// The token budget one tool-calling trial gets.
///
/// Smaller than [`BUDGET`], because a tool call is short — a name and one
/// argument — and the probe runs two offerings by five trials, which on MCF's
/// own engine is minutes of generation per hundred tokens allowed.
///
/// **What it costs is a condition, not a threshold.** A model that would have
/// called on its two-hundredth token is reported as not having called *within
/// this many*, which is the same distinction the stop-condition probe's
/// ceiling makes (A7): never write *never* for something only observed not to
/// have happened yet.
const TOOL_BUDGET: usize = 160;

/// What a thinking trial is given.
///
/// Deliberately generous: the probe asks how much of a turn happens before the
/// answer does, and a budget that cut the turn short would measure the budget —
/// which is the defect it exists to find (F106).
const THINKING_BUDGET: usize = 400;

/// The token budget one structured-output trial gets.
///
/// **Eight hundred, and the number was measured rather than chosen** (F106).
/// It was two hundred, on the reasoning that an object carrying a
/// twenty-five-character sentence needs a little room — and on the first real
/// model, two of three framings never finished inside it. The same probe at
/// eight hundred, same model, same engine:
///
/// | framing | at 200 | at 800 |
/// |---|---|---|
/// | described in words | 0 conformed, 5 unfinished | **5 conformed** |
/// | a schema | 5 conformed | 5 conformed |
/// | an example filled in | 0 conformed, 5 unfinished | 0 conformed, **5 finished with no object** |
///
/// The first row is a budget that was measuring itself. The third is what a
/// large enough budget buys: at two hundred that framing was *interrupted*, and
/// at eight hundred it is a real observation about the model — it finishes its
/// turn and produces no object when shown an example.
///
/// Still a condition rather than a threshold: a model that would have closed
/// its brace on the thousandth token is reported as not having closed it
/// *within this many* (A7), which is what `Attempt::Unfinished` counts.
const STRUCTURED_BUDGET: usize = 800;

/// Probes a model.
/// The engine, named the way a later comparison can use.
///
/// `provisioned` is not an engine; a particular build at a particular commit
/// is. The name a caller types is resolved to that before it is recorded, so
/// that *the same engine* and *another build of it* can be told apart (D43,
/// §3.4).
fn resolved_engine(at: &Places<'_>, asked: &str) -> String {
    if asked == "stand-in" {
        return "stand-in".to_owned();
    }
    // The build the daemon resolved for this model, where it said: with
    // more than one build here, the daemon chooses by the device, and the
    // record names the one that answered rather than the word asked for.
    if let Some(resolved) = at.resolved {
        return resolved.to_owned();
    }
    let home = at.models.parent().unwrap_or(at.models);
    match crate::adapters::only_one(crate::adapters::provisioned_llama(home)) {
        // The component's own name, not a literal: a run on `llama.cpp-cuda`
        // and one on `llama.cpp` recorded the same identity while their paths
        // said otherwise, and a comparison between two names for one engine
        // reports a moved condition that did not move (F45, A6).
        Ok(Some(llama)) => format!(
            "provisioned {} @{}",
            llama.component,
            llama.commit.get(..12).unwrap_or(&llama.commit)
        ),
        _ => asked.to_owned(),
    }
}

/// The engine a probe uses when the caller does not name one.
///
/// The provisioned server where there is one, MCF's own otherwise.
fn whichever_is_here(home: &std::path::Path) -> &'static str {
    // Any provisioned build is an engine: which of several serves is the
    // daemon's choice, by the device the model resolves to (B-471), and a
    // probe that fell to MCF's own engine because two builds stood here
    // read a model the daemon would have served (F185).
    match crate::adapters::provisioned_llama(home) {
        crate::adapters::Found::One(_) | crate::adapters::Found::Several(_) => "provisioned",
        crate::adapters::Found::None => "stand-in",
    }
}

/// Where a probe run reaches the daemon and the store, and how it learns
/// that nobody is waiting for it any more.
#[derive(Clone, Copy)]
pub struct Places<'a> {
    /// The daemon's control socket, which every trial goes through.
    pub socket: &'a Path,
    /// The model store; the derived configurations live beside it.
    pub models: &'a Path,
    /// The engine the daemon resolved for this model, named as the record
    /// names it, where the daemon said; `None` lets the run name the one
    /// provisioned build where there is exactly one.
    pub resolved: Option<&'a str>,
    /// Whether the asker has gone, asked before every trial: a run cut
    /// short spends no further generation on nobody (B-468). `None` is a
    /// run nobody can leave.
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
    /// Whether the asker has gone.
    fn asker_gone(&self) -> bool {
        self.gone.is_some_and(|gone| gone())
    }

    /// One trial, unless the asker has gone.
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

    /// One trial keeping what the model said, unless the asker has gone.
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
            };
        }
        crate::probes::spoken(self.socket, model, prompt, pieces, budget, engine)
    }

    /// One text counted, unless the asker has gone.
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

    /// One length tried, unless the asker has gone.
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

/// What was asked of the run.
#[derive(Debug, Clone, Copy)]
pub struct Asked<'a> {
    /// Which engine to ask through, where the caller named one.
    pub engine: Option<&'a str>,
    /// Whether to apply what was observed, which is an act (D43).
    pub apply: bool,
    /// The longest prompt the usable-context probe may ask for (B-461).
    pub up_to: Option<usize>,
    /// Which probes to run, by name; empty is all of them.
    pub only: &'a [String],
}

/// One probe about to run, or just run: which, and which of how many.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Step {
    /// The probe's name, as [`PROBES`] lists it.
    pub name: &'static str,
    /// Which this is, counting from one.
    pub count: usize,
    /// How many the run asks in all.
    pub of: usize,
}

impl Step {
    /// The step as the daemon sends it.
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

/// Every probe the run can make, in the order it makes them. The chat
/// template goes first because the probes after it are asked through the
/// addressing it measures.
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

/// The probes that are asked through the addressing the chat-template
/// probe finds, so asking for one of them asks for that first.
const THROUGH_THE_ADDRESSING: [&str; 3] = ["tool-calls", "structured-output", "thinking"];

/// Which probes a run makes, in order: every one where none is named,
/// else the ones named, with the chat template ahead of any that is asked
/// through its addressing. A name that is none of them is ignored, so the
/// caller that reads a wire says what it could not read.
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

/// Runs the probes on one model, saying which is about to run and what
/// each found as it lands (B-478, D50).
///
/// `say` is told each step twice — before the probe runs, with no lines,
/// and after, with what it found — and answers whether to go on; a
/// listener that has gone stops the run at the next probe (B-468). What
/// comes back is the closing lines: the probes MCF declines, and that
/// nothing was configured unless it was asked for.
///
/// # Errors
///
/// The file could not be read as a model, or nothing is listening on the
/// socket, each in a sentence.
pub fn run(
    at: &Places<'_>,
    path: &Path,
    asked: &Asked<'_>,
    say: &mut dyn FnMut(&Step, &[String]) -> bool,
) -> Result<Vec<String>, String> {
    // **Enough of the file to answer, not the file** (F145, F140, B-372).
    let Some(bytes) = read_prefix(path) else {
        return Err(format!(
            "{} could not be read as a model: MCF grew its read to the whole file and still \
             could not find a GGUF directory in it",
            path.display()
        ));
    };
    // A probe needs an engine, and which engine is a condition (D42) — so the
    // trials go through the daemon exactly as a generation does, and the
    // engine that served them is named in the result.
    let Some(build) = probes::describe_engine(at.socket) else {
        return Err(format!(
            "nothing is listening on {}: a probe asks a model to do the thing, which needs an \
             engine (D42)",
            at.socket.display()
        ));
    };
    let home = at.models.parent().unwrap_or(at.models);
    // The provisioned server where there is one, MCF's own otherwise; the
    // same verdict through both, thirteen seconds against two hundred and
    // sixty-seven (F39, B29).
    let engine_asked: &str = asked.engine.unwrap_or_else(|| whichever_is_here(home));
    // The engine as it *resolves*, not as it was asked for (F45).
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

/// The record's method names, in the order the run makes them: what a
/// model's page lists its findings by (B-483).
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

/// One probe's recorded finding as a sentence, from the fields the record
/// keeps of it: the same figures the report printed, read back (A1).
#[must_use]
pub fn recorded_said(body: &mcf_record::json::Value) -> String {
    use mcf_record::json::Value;
    // A measurement's finding reads back through its own module (D52).
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

/// A finding as its figures, in the order they were written, leaving out
/// what names the run rather than what it found: the reading for a probe
/// this sentence-maker does not know.
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

/// Enough of a model file to parse its directory: sixteen megabytes, then
/// more, then all of it, the same bounded read `mcf run` and `mcf bench`
/// use (B-372).
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

/// Writes what the operator applied: the addressing, and what it was before
/// (D43, A1).
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
        // What MCF did before, so that the entry says what *changed* and not
        // only what is now true (A1).
        ("was", Value::text("raw text, MCF's default (§3.8)")),
    ]);
    let mut journal = mcf_record::journal::Journal::open(&path)?;
    journal.append(&mcf_record::journal::Entry::new(
        mcf_record::journal::EntryKind::ModelConfigured,
        mcf_core::time::Timestamp::now(),
        body,
    ))?;
    Ok(path)
}

/// Writes what a probe observed about a model's usable context (B-386,
/// B-055).
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
        // The longest length that was asked for, so that a run capped by
        // --up-to reads back as a run that did not test the claim (B-461).
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

/// Writes what any probe observed (B-386, B-054, D42, A1): one writer,
/// because two would eventually disagree about what a probe result is (F79).
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

/// Writes a run's readings: one row a figure, in the one schema every
/// diagnostic shares (D54, B-511).
///
/// # Errors
///
/// Nowhere to record, or the record could not be written.
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

/// Writes a probe's rows beside its finding, and says so in a line
/// (D54, B-513).
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

/// One reading, for the probes' rows.
fn reading(
    dims: &[(&str, mcf_record::json::Value)],
    metric: &str,
    value: usize,
    unit: &str,
) -> mcf_record::readings::Reading {
    mcf_record::readings::Reading::new(dims, metric, as_integer(value), unit)
}

/// A per-name count as rows under one dimension.
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

/// A count as the record's integer, saturating rather than wrapping.
fn as_integer(held: usize) -> i64 {
    i64::try_from(held).unwrap_or(i64::MAX)
}

/// The chat-template probe, and what it found (B-051, B-052).
///
/// **Its own function for the reason the others have one**: a probe that is
/// rendered inline in the command is a probe no check watching *renderers* can
/// see, and this one was the last that did not record what it observed
/// (B-386, F106). It is first in the output because the probes after it are
/// asked *through* the addressing it measures.
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
            // From the observation, before anything is said about what it
            // means (A9, F106).
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

/// What the record keeps of a tool-calling observation.
///
/// The totals across offerings, plus which offering did best: the per-offering
/// counts are what a reader on a terminal needs and the totals are what a later
/// query can compare, and the best names the condition the total was reached
/// under (A6).
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
        // The form the file's template writes, so that a later reading of
        // this entry knows which shape the trials asked for (B-453, A6).
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

/// What each framing did, one line each.
///
/// Every count beside every other, because a framing that conformed nought
/// times means something different depending on whether its trials ended in
/// prose or were cut off, and a reader who has to hold two lists in their head
/// to find that out will not (F106, A1).
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

/// What the record keeps of a structured-output observation.
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

/// What the record keeps of a stop-condition observation.
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

/// What the record keeps of a chat-template observation.
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

/// What each language costs this model's vocabulary (B-057, B-379, F81).
///
/// **It generates nothing**: the answer is a count, and the count is taken by
/// the daemon through the tokenizer of the engine that generates for this
/// model — so a vocabulary MCF's own tokenizer refuses is still counted when
/// a provisioned engine can read it, and the same reading that frames a turn
/// is the one that costs it (B-442, B-441, F158). The engine is a condition
/// because its tokenizer took part (D42), and the report names which one
/// counted.
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
            lines.push(
                " which licenses nothing, and is not a negative result (D42, §3.18)".to_owned(),
            );
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

/// What the record keeps of a language-cost observation.
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

/// How much of a turn happens before the answer does (B-421).
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
    // Through the addressing this run just measured: a turn put to a model the
    // way it was NOT trained is a turn that says nothing about where its
    // tokens go. An addressing that closes the thinking itself is asked with
    // it opened instead, or the probe would measure the closing and not the
    // model.
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
            // From the observation, before either verdict (A9, F106).
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

/// The addressing the turns go under: the one on file, or where that one
/// closes the thinking itself, the same addressing with it opened.
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

/// Which addressing the turns were put under, and why it is not the one on
/// file where it is not.
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

/// The observation itself, before anything is said about what it means.
fn thinking_observed(spends: &crate::probes::thinking::Spends) -> Vec<String> {
    let mut lines = vec![format!(
        " markers  this file holds {} it could be inside{}",
        spends.available.len(),
        match spends.available.first() {
            Some(first) => format!(", the first of them {first}"),
            None => String::new(),
        }
    )];
    // What could not be paired is said, so that *nothing was found* is
    // never read as *nothing was there* (A7).
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
    // In the engine's count rather than in words, because it is the number a
    // budget is in: what the longest turn that finished ran to, thought and
    // answer together, on a question that asks for a reason (F172).
    if spends.longest_turn > 0 {
        lines.push(format!(
            " the longest turn that finished ran {} token(s){}",
            spends.longest_turn,
            before_the_answer(spends.before_in_longest)
        ));
    }
    lines
}

/// What the observation means, said after it is recorded (A9).
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

/// What the record keeps of a thinking observation.
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

/// A count as the record holds numbers.
fn as_count(held: usize) -> i64 {
    i64::try_from(held).unwrap_or(-1)
}

/// Whether an image reaches this model at all (B-057, B-320).
///
/// **Through the provisioned tool that takes one.** MCF's own engine implements
/// text transformers and nothing else, so this question could not be put at all
/// until a provisioned engine carried `llama-mtmd-cli`. Where it does not, the
/// probe says the tool is missing rather than that the model cannot see: the
/// difference between those two is the confusion A21 exists to prevent.
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
        // The picture goes to a file because the tool takes a path: it is
        // written under the model's own directory so that a run leaves nothing
        // anywhere else, and removed when the turn is over.
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
            // The seed is held still so that what differs between the two
            // turns is the picture and nothing else (D19).
            .arg("--seed")
            .arg("41")
            .output();
        let _gone = std::fs::remove_file(&at);
        let spoke = spoke.ok()?;
        let said = String::from_utf8_lossy(&spoke.stdout).trim().to_owned();
        if said.is_empty() {
            return None;
        }
        // What it spent, in words rather than identifiers: the tool does not
        // report a token count, and a figure MCF made up would be one nobody
        // measured (A7).
        let spent = said.split_whitespace().count();
        Some((said, spent))
    };

    let probed =
        crate::probes::vision::vision(path, bytes, projector.as_deref(), &engine, &mut look);
    vision_result_lines(path, &probed, &engine)
}

/// What the record keeps of a vision observation.
///
/// The two answers themselves, because the verdict is a comparison of them and
/// a reader who cannot see both cannot check it. What is *not* kept is any
/// judgement about whether either was right (§XIII).
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

/// The vision probe, as a reader meets it.
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
            // From the observation, before either verdict: *the image did
            // not get in* is as much a measurement as *it did* (A9, F106).
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
            // The rows name the engine the way every other probe's do,
            // rather than carrying the whole conditions sentence (B-542).
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
            // D42's third state, §3.18: the citation belongs here and in the
            // record. The screen gets what it means.
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

/// Whether this artifact produces an embedding, of what width (B-057).
fn embedding_lines(path: &std::path::Path, bytes: &[u8]) -> Vec<String> {
    let mut ask = |text: &str| probes::embedding::measured(path, text);
    // Not the daemon's engine: this path is in-process, and a condition that
    // names an engine which did not participate is F102's defect (A21, §3.4).
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
                    "TWO DIFFERENT VECTORS, which MCF's own engine cannot do correctly (A19)"
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

/// The modalities MCF does not probe, and why (B-057's second half).
///
/// Printed with the probes rather than in a manual, because the reader who
/// needs it is the one who has just read four answers and is about to assume
/// the silence means the rest was fine.
fn declined_lines() -> Vec<String> {
    let mut lines = vec![
        "  not probed, and why".to_owned(),
        " a modality MCF does not mention is one a reader assumes it checked (A7)".to_owned(),
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

/// A parts-per-million as a person reads it.
///
/// Integer arithmetic, as everywhere else a ratio is rendered here: the crates
/// hold no floating-point number and a rendering is not a reason to introduce
/// one (A6). The division truncates, which is what a percentage to one place
/// is.
#[allow(
    clippy::integer_division,
    reason = "a percentage to one decimal place, from integers, as `Headroom` and `compare` \
              render theirs"
)]
fn per_cent(ppm: u64) -> String {
    format!("{}.{}%", ppm / 10_000, (ppm % 10_000) / 1_000)
}

/// What a surface says about a write that may not have happened.
///
/// One sentence in one place: a probe that printed *recorded* for a write that
/// failed would be the silent failure A2 forbids, and three spellings of that
/// sentence would eventually include one that forgot (F79).
fn recorded(written: Result<std::path::PathBuf, mcf_core::Failure>) -> String {
    match written {
        Ok(journal) => format!(" recorded in {}", journal.display()),
        Err(failure) => format!(
            " BUT NOT RECORDED — {failure}; a measurement nobody can find later is the same as \
             one not taken (A1, A2)"
        ),
    }
}

/// The total across every framing or offering, for the record.
///
/// The per-framing counts are what a reader needs and the total is what a
/// later query can compare; both are kept, because a total that hid which
/// framing produced it would be a number without its conditions (A6).
fn counted(per: &[(String, usize)]) -> i64 {
    i64::try_from(
        per.iter()
            .map(|(_, count)| *count)
            .fold(0_usize, usize::saturating_add),
    )
    .unwrap_or(i64::MAX)
}

/// What the counts come to, in a sentence.
///
/// A model that called is verified; one whose file claimed tools and never
/// called is a divergence rather than an error, and one that claimed none
/// and called none is two readings agreeing (A21, B-058).
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

/// The tool-calling probe, and what it found (B-053).
///
/// **Asked through the addressing this run just measured**, because a model
/// spoken to in a way it does not recognise emits nothing recognisable and the
/// probe would report *no call* about a turn nobody would ever send. That makes
/// this probe depend on the chat-template one, which is the right dependency:
/// §X calls a misconfigured model a measurement error, and a tool-call
/// observation taken under a wrong addressing is exactly that.
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
        // The answer and not the thought before it: a model that drafts its
        // call inside its thinking is read where it made the call (F171).
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
            // The declaration first, and marked as a declaration: A21's three
            // states are only kept apart if the reader can see which is which.
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
            // The form its template writes, which is what the offerings
            // were built from: a model asked for a shape its family does not
            // use is a model asked the wrong question (B-453).
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
            // Every reason, not a summary of them: what a model actually
            // emitted is the thing somebody debugging needs (A1).
            for reason in &calling.reasons {
                lines.push(format!("   what came out — {reason}"));
            }
            // B-386's rule, which held for one probe of four: a probe that
            // prints and does not write leaves a measurement nobody can find
            // later (A1, F106). Whichever way it came out (A9).
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
            lines.push(
                " which licenses nothing, and is not a negative result (D42, §3.18)".to_owned(),
            );
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

/// The structured-output probe, and what it found (B-054).
///
/// **Asked through the addressing this run just measured**, for the reason
/// [`tool_lines`] is: a model spoken to in a way it does not recognise emits
/// nothing recognisable, and a conformance figure taken under a wrong
/// addressing is the measurement error §X is about.
///
/// **There is no declaration to diverge from.** A file says nothing about
/// whether it emits JSON on request — there is no metadata field for it — so
/// unlike the tool probe this one has only the observed half, and says so
/// rather than leaving a reader to wonder which half is missing (A21, A7).
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
        // The answer and not the thought before it: a model that drafts its
        // call inside its thinking is read where it made the call (F171).
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
            // Every reason, not a summary: what a model actually emitted is
            // the thing somebody debugging a shape needs (A1).
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
            lines.push(
                " which licenses nothing, and is not a negative result (D42, §3.18)".to_owned(),
            );
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

/// What a decided probe says.
/// What this run says about a configuration somebody applied earlier (D43,
/// B-058).
///
/// D43's divergence is *MCF's answer would now differ*, and there are two ways
/// to learn it, which are not the same kind of knowledge and are not reported
/// as though they were.
///
/// The **conditions** can be compared with no trials at all: the configuration
/// records the engine and build it was taken through. That they have moved
/// does not mean the answer has — F39 measured two engines agreeing on this
/// exact question — so it is reported as *this was learned somewhere else*,
/// with what to run, and never as a disagreement.
///
/// The **answer** is compared only because this run has just measured it. That
/// is evidence, and it is the one that says the configuration is wrong.
fn against_what_was_applied(
    home: &std::path::Path,
    path: &std::path::Path,
    probed: &mcf_core::probe::Probed<Addressed>,
    engine: &str,
) -> Option<Vec<String>> {
    use crate::configured::Since;

    let stored = crate::configured::read(home, path)?;

    let mut said = vec![format!("  applied  {}", stored.provenance())];

    // The evidence first, because it is the one that can say *wrong*.
    match &probed.outcome {
        Outcome::Observed(addressed) if addressed.best == stored.name => {
            said.push(
                "  agrees this run measured the same addressing that is applied, so the configuration is not merely old — it is confirmed (A21)"
                    .to_owned(),
            );
        }
        Outcome::Observed(addressed) => {
            said.push(format!(
                "  DIVERGENCE what is applied is {}, and this run measured {} as best. MCF's answer would now differ, which is the case D43 is about — applying it is an act: `mcf probe --apply` (§3.11, D43)",
                stored.name, addressed.best
            ));
        }
        Outcome::Inconclusive { .. } => {
            said.push(
                "  unchanged  this run could not tell, which is not a disagreement with what is applied and does not license undoing it: an inconclusive probe leaves the capability where it was (A7, D42)"
                    .to_owned(),
            );
        }
    }

    // Then the conditions, which are a fact about where the evidence came from
    // rather than a fact about the model.
    let build = mcf_core::build_identity::BuildIdentity::current().to_string();
    if let Since::ConditionsMoved(moved) = crate::configured::since(&stored, engine, &build) {
        for one in moved {
            said.push(format!(
                "  moved the {} it was taken through is not the one in force: was {}, now {}",
                one.what, one.was, one.now
            ));
        }
        said.push(
            " which does not mean the answer changed — two engines agreed on this question when it was measured (F39) — only that the evidence was gathered elsewhere (A21)"
                .to_owned(),
        );
    }
    Some(said)
}

/// The act D43 requires: somebody read what the probe observed and said *do
/// that* (B-059).
///
/// Three answers, and only one of them writes anything.
///
///   - **Observed, and not raw.** The addressing is written down with the
///     probe that found it, the moment, the build and the conditions, and the
///     act goes on the record. `mcf run` addresses the model that way
///     afterwards, and every account says so.
///   - **Observed, and raw.** There is nothing to apply: raw is what MCF does
///     already, and writing a configuration that changes nothing would put a
///     provenance on a default and make it look derived (A21).
///   - **Inconclusive.** Refused. D42 makes *could not tell* a first-class
///     outcome precisely so that it cannot become a configuration, and this is
///     the place that rule has to hold or it holds nowhere.
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
                " measurements taken before and after this are not comparable — the conditions changed, and MCF says so rather than assuming (D43, §3.4)"
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

/// How long this model's turns run, against the budget MCF would give it
/// (B-056).
///
/// It is asked through whatever addressing was *applied*, because that is how
/// the model will actually be spoken to — asking it raw would measure a turn
/// nobody will ever ask for. A model addressed wrongly does not stop at any
/// budget, which is the chat-template probe's business and is why this one
/// says so rather than reporting an enormous number.
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
        // The turn as it will really be sent: through what was applied, or
        // unwrapped where nothing was — as markers and text either way, for
        // the tokenizer of the engine that answers to read (B-442). The
        // question travels as a turn rather than as a prompt because a
        // prompt is routed to the engine that takes a command line and
        // cannot say why it stopped, so the probe would report inconclusive
        // on every unconfigured model — most of them — for a reason that is
        // MCF's plumbing rather than the model's behaviour (B-376, F48).
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
            // Recorded from the observation, before anything is said about
            // what it means: a record written out of the verdict branch is a
            // record of MCF's interpretation rather than of what happened
            // (B-386, A9, F106).
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
                    " DIVERGENCE MCF allows {} tokens unless told otherwise, and this model's turns run to {}. Every answer past that is cut off by MCF rather than finished by the model, which measures the budget and not the model (§3.8)",
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
                " which licenses nothing: MCF configures no differently than before, and this is not a negative result (D42, §3.18)"
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

/// The act, for a budget.
///
/// The value applied is the longest turn observed, not an average and not a
/// margin on top. An average cuts off half the answers; a margin is a number
/// MCF invented, and §3.15 has no room for one. What is claimed is exactly
/// what was measured: *this many tokens were enough for every turn that
/// finished here*.
fn apply_budget(
    home: &std::path::Path,
    path: &std::path::Path,
    probed: &mcf_core::probe::Probed<crate::probes::Stopping>,
    stopping: &crate::probes::Stopping,
    engine: &str,
) -> Vec<String> {
    if stopping.longest <= stopping.default_budget {
        return vec![
            "  NOT APPLIED — MCF's default is already enough, and writing it down would put a probe's provenance on a default (A21)"
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

/// The share of a turn that came before its answer, for a page line.
fn before_the_answer(before: Option<usize>) -> String {
    match before {
        Some(before) => format!(", and up to {before} of a turn spent thinking before the answer"),
        None => String::new(),
    }
}

/// The act, for a budget the thinking probe found wanting (B-466).
///
/// The stop-conditions probe asks for a river's name and sets the budget to
/// the longest turn that answered; on a model that thinks, that turn is
/// mostly thought about a river's name, and the budget runs out inside the
/// marker on the first question that asks for a reason (F172). The thinking
/// probe asks for one. Where its longest finished turn is longer than what
/// is on file — or than MCF's default, where nothing is — the budget is
/// raised to it, under this probe's own provenance: the value is measured
/// here, and it says so.
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

/// The usable context against the declared one (B-055).
///
/// Its own section rather than its own command: the two probes ask different
/// questions of one model, and D42's framework is one result each, both
/// carrying their own conditions.
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
    // Told before it is spent (B-461): the trial of the ceiling can take
    // hours on a processor, and the person waiting is owed the hours in
    // advance. The rate is read over a short prompt and the projection goes
    // to the error stream now, where the report goes to the output at the
    // end; it is repeated in the report so that the page carries it too.
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
            // B-386: written down, whichever way it came out. *Agrees* is as
            // much a measurement as *diverges* (A9), and it is written *here*,
            // from the observation, rather than after the branch that decides
            // what it means — a record produced out of the verdict is a record
            // of the verdict (F106).
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
            // The declared context is the whole budget, not the prompt's share
            // of it, so a prompt one shorter is agreement rather than a
            // divergence — reporting that off-by-one would be reporting
            // arithmetic (B-055, F42).
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
                    " DIVERGENCE the file declares {} tokens and this engine on this machine takes {}. A prompt planned against the declaration would be refused, or worse, quietly shortened — which is a measurement of a different prompt (§3.8, A21)",
                    context.declared, context.accepted
                ));
                if let Some(because) = &context.because {
                    lines.push(format!(" the engine's own words: {because}"));
                }
            }
            // B-386: written down, whichever way it came out. *Agrees* is as
            // much a measurement as *diverges*, and a record that kept only
            // the surprising half could not answer *what does this machine
            // take* (A1, A9).
        }
        Outcome::Inconclusive { because } => {
            lines.push(format!(" INCONCLUSIVE — {because}"));
            lines.push(
                " which licenses nothing: MCF configures no differently than before, and this is not a negative result (D42, §3.18)"
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

/// How many identifiers the rate is read over. Long enough that the engine
/// is reading a prompt rather than starting up, short enough to cost
/// seconds on a processor.
const RATE_SAMPLE: usize = 512;

/// The projection's sentence, from a short prompt timed first; `None` where
/// the engine could not be timed reading it, or the ceiling is no longer
/// than the sample — a trial of seconds needs no forecast.
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
        // The silence is printed beside the count rather than under it: a
        // reader has to be able to see that "0 of 5" means the model kept
        // talking and "0 of 5, silent 5" means it would not start (F38).
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
        // How long the finished turns ran. Already observed, and the thing a
        // stop-condition question is asked of (B-056).
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

    // The divergence (B-058): what the file said against what the model did.
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

    /// No name is every probe, in order; names are those probes; and a
    /// probe asked through the addressing brings the chat template ahead
    /// of it, once.
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

    /// A recorded finding reads back as the figures it kept, and one with
    /// its figures missing says it could not tell rather than inventing.
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
