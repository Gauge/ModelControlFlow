//! `mcf probe`: asking a model to do the thing (B-051, B-052, D42, §3.18, §X).
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

use mcf_core::probe::Outcome;
use mcf_serve::probes::{self, Addressed};

use crate::Response;
use crate::run::{ambiguous, resolve};

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

/// Probes a model.
/// The engine, named the way a later comparison can use.
///
/// `provisioned` is not an engine; a particular build at a particular commit
/// is. The name a caller types is resolved to that before it is recorded, so
/// that *the same engine* and *another build of it* can be told apart (D43,
/// §3.4).
fn resolved_engine(asked: &str) -> String {
    if asked == "stand-in" {
        return "stand-in".to_owned();
    }
    let found = crate::models::default_root()
        .and_then(|models| models.parent().map(std::path::Path::to_path_buf))
        .map(|home| mcf_serve::adapters::only_one(mcf_serve::adapters::provisioned_llama(&home)));
    match found {
        Some(Ok(Some(llama))) => format!(
            "provisioned llama.cpp @{}",
            llama.commit.get(..12).unwrap_or(&llama.commit)
        ),
        _ => asked.to_owned(),
    }
}

/// The engine a probe uses when the caller does not name one.
///
/// The provisioned server where there is one, MCF's own otherwise.
fn whichever_is_here() -> &'static str {
    let provisioned = crate::models::default_root()
        .and_then(|models| models.parent().map(std::path::Path::to_path_buf))
        .map(|home| mcf_serve::adapters::only_one(mcf_serve::adapters::provisioned_llama(&home)));
    match provisioned {
        Some(Ok(Some(_))) => "provisioned",
        _ => "stand-in",
    }
}

#[allow(
    clippy::too_many_lines,
    reason = "one command, written as what it does in order: resolve, probe, report, apply"
)]
pub(crate) fn run(model: &str, engine: Option<&str>, apply: bool) -> Response {
    let path = match resolve(model) {
        Ok(Some(path)) => path,
        Ok(None) => {
            return Response {
                text: format!(
                    "mcf: there is no model at {model}\n  `mcf list` says what this machine is \
                     holding; a path to a file works too"
                ),
                served: false,
            };
        }
        Err(found) => {
            return Response {
                text: ambiguous(model, &found),
                served: false,
            };
        }
    };

    let bytes = match std::fs::read(&path) {
        Ok(bytes) => bytes,
        Err(error) => {
            return Response {
                text: format!("mcf: {} could not be read\n  {error}", path.display()),
                served: false,
            };
        }
    };

    // A probe needs an engine, and which engine is a condition (D42) — so the
    // trials go through the daemon exactly as a generation does, and the
    // engine that served them is named in the result.
    let Some(socket) = crate::serve::socket_path() else {
        return Response {
            text: "mcf: there is nowhere to look for a daemon, and a probe needs an engine to \
                   ask (D42)"
                .to_owned(),
            served: false,
        };
    };
    let Some(build) = probes::describe_engine(&socket) else {
        return Response {
            text: format!(
                "mcf: nothing is listening on {}\n  a probe asks a model to do the thing, \
                 which needs an engine: `mcf serve` starts one (D42)",
                socket.display()
            ),
            served: false,
        };
    };

    // A probe result belongs to the engine it was taken through, not to the
    // model (D42), so whichever answers is named in the conditions.
    //
    // The default is the provisioned server where there is one, and it was
    // changed on evidence rather than on the speed alone: the same probe run
    // through both engines returns the same verdict — the same best
    // addressing, the same counts, the same silences — and the server answers
    // in thirteen seconds where MCF's own takes two hundred and sixty-seven
    // (F39). A twentyfold difference decides which is usable; the agreement
    // is what makes changing the default honest rather than convenient (B29).
    //
    // Where there is no provisioned engine, MCF's own answers, as before.
    let asked: &str = match engine {
        Some(named) => named,
        None => whichever_is_here(),
    };
    // The engine as it *resolves*, not as it was asked for. A configuration
    // records this and `mcf explain` compares against it, and the two have to
    // be the same kind of name or the comparison invents a moved condition
    // out of two spellings of one engine — which it did (F45). A different
    // provisioned build is a genuinely different condition and is only
    // visible if the commit is part of the name.
    let engine = format!("{}, through the daemon at {build}", resolved_engine(asked));
    let mut trial = |identifiers: &[usize], budget: usize| {
        probes::trial(
            &socket,
            &path,
            // Empty on purpose: the turn travels as identifiers, and the
            // probe varies the question between trials. A prompt here would
            // be recorded as the thing asked and would be wrong for four
            // trials in five (A1).
            "",
            Some(identifiers),
            budget,
            Some(asked),
        )
    };
    let probed = probes::chat_template(&path, &bytes, TRIALS, BUDGET, &engine, &mut trial);
    let applied = if apply {
        Some(apply_addressing(&path, &probed, &engine))
    } else {
        // Not applying is not the same as not looking. If something was
        // applied before, this run has just re-measured the thing that set it,
        // and saying nothing about whether the two agree would be throwing
        // away the comparison D43 asks for (B-058).
        against_what_was_applied(&path, &probed, &engine)
    };

    let mut lines = vec![format!("probed {}", path.display()), String::new()];
    lines.extend([
        format!("  {}", probed.method.name),
        format!(" asks {}", probed.method.asks),
        format!(" decides  {}", probed.method.decides),
        String::new(),
    ]);
    match &probed.outcome {
        Outcome::Observed(addressed) => lines.extend(observed(addressed)),
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
    if let Some(said) = applied {
        lines.push(String::new());
        lines.extend(said);
    }
    lines.push(String::new());

    lines.extend(context_lines(&socket, &path, &bytes, &engine, asked));
    lines.extend(stopping_lines(
        &socket, &path, &bytes, &engine, asked, apply,
    ));
    lines.push(
        "  Nothing was configured. A probe writes what it observed; changing how MCF addresses \
         this model is an act somebody takes, and it is recorded (D42, D43)."
            .to_owned(),
    );

    Response {
        text: lines.join("\n"),
        served: true,
    }
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
    path: &std::path::Path,
    probed: &mcf_core::probe::Probed<Addressed>,
    engine: &str,
) -> Option<Vec<String>> {
    use mcf_serve::configured::Since;

    let home = crate::models::default_root()
        .and_then(|models| models.parent().map(std::path::Path::to_path_buf))?;
    let stored = mcf_serve::configured::read(&home, path)?;

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
    if let Since::ConditionsMoved(moved) = mcf_serve::configured::since(&stored, engine, &build) {
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

    let Some(home) = crate::models::default_root()
        .and_then(|models| models.parent().map(std::path::Path::to_path_buf))
    else {
        return vec!["  NOT APPLIED — there is nowhere to write it".to_owned()];
    };
    let at = mcf_core::time::Timestamp::now();
    let addressing = mcf_serve::configured::Addressing {
        name: chosen.name.clone(),
        before: chosen.pieces_before.clone(),
        after: chosen.pieces_after.clone(),
        probe: probed.method.name.to_owned(),
        at: at.to_string(),
        build: mcf_core::build_identity::BuildIdentity::current().to_string(),
        conditions: engine.to_owned(),
    };
    match mcf_serve::configured::write(&home, path, &addressing) {
        Err(failure) => vec![format!("  NOT APPLIED — {failure}")],
        Ok(written) => {
            let recorded = crate::log::record_configured(path, &addressing);
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
    socket: &std::path::Path,
    path: &std::path::Path,
    bytes: &[u8],
    engine: &str,
    asked: &str,
    apply: bool,
) -> Vec<String> {
    let Ok(file) = mcf_serve::probes::gguf_of(bytes) else {
        return Vec::new();
    };
    let Ok(vocabulary) = mcf_standin::tokenizer::Vocabulary::read(&file) else {
        return Vec::new();
    };
    let home = crate::models::default_root()
        .and_then(|models| models.parent().map(std::path::Path::to_path_buf));
    let addressing = home
        .as_ref()
        .and_then(|home| mcf_serve::configured::read(home, path));

    let mut ask = |question: &str, budget: usize| {
        // The turn as it will really be sent: through what was applied, or
        // raw where nothing was.
        let identifiers = addressing.as_ref().and_then(|held| {
            let mut pieces = held.before.clone();
            pieces.push(mcf_standin::tokenizer::Piece::Text(question.to_owned()));
            pieces.extend(held.after.iter().cloned());
            vocabulary.addressed(&pieces)
        });
        // Where nothing was applied the question still travels as identifiers,
        // just unwrapped. Sending it as *text* routes it to the engine that
        // takes a command line and cannot say why it stopped, so the probe
        // would report inconclusive on every unconfigured model — most of
        // them — for a reason that is MCF's plumbing rather than the model's
        // behaviour (B-376, F48).
        let identifiers = identifiers.or_else(|| vocabulary.encode(question, true).ok());
        match identifiers {
            Some(identifiers) => {
                probes::trial(socket, path, "", Some(&identifiers), budget, Some(asked))
            }
            None => probes::trial(socket, path, question, None, budget, Some(asked)),
        }
    };
    let probed = probes::stop_conditions(
        path,
        TRIALS,
        FROM,
        CEILING,
        crate::run::TOKENS,
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
                " ended its own turn in {} of {} trials, the longest running {} token(s)",
                stopping.stopped, stopping.of, stopping.longest
            ));
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
                lines.extend(apply_budget(path, &probed, stopping, engine));
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
    path: &std::path::Path,
    probed: &mcf_core::probe::Probed<mcf_serve::probes::Stopping>,
    stopping: &mcf_serve::probes::Stopping,
    engine: &str,
) -> Vec<String> {
    if stopping.longest <= stopping.default_budget {
        return vec![
            "  NOT APPLIED — MCF's default is already enough, and writing it down would put a probe's provenance on a default (A21)"
                .to_owned(),
        ];
    }
    let Some(home) = crate::models::default_root()
        .and_then(|models| models.parent().map(std::path::Path::to_path_buf))
    else {
        return vec!["  NOT APPLIED — there is nowhere to write it".to_owned()];
    };
    let budget = mcf_serve::configured::Budget {
        tokens: stopping.longest,
        probe: probed.method.name.to_owned(),
        at: mcf_core::time::Timestamp::now().to_string(),
        build: mcf_core::build_identity::BuildIdentity::current().to_string(),
        conditions: engine.to_owned(),
    };
    match mcf_serve::configured::write_budget(&home, path, &budget) {
        Err(failure) => vec![format!("  NOT APPLIED — {failure}")],
        Ok(_written) => vec![
            format!("  APPLIED  {}", budget.provenance()),
            " `mcf run` allows this model that many tokens unless --limit says otherwise, and the account says where the number came from"
                .to_owned(),
        ],
    }
}

/// The usable context against the declared one (B-055).
///
/// Its own section rather than its own command: the two probes ask different
/// questions of one model, and D42's framework is one result each, both
/// carrying their own conditions.
fn context_lines(
    socket: &std::path::Path,
    path: &std::path::Path,
    bytes: &[u8],
    engine: &str,
    asked: &str,
) -> Vec<String> {
    let Ok(file) = mcf_serve::probes::gguf_of(bytes) else {
        return Vec::new();
    };
    let Some(declared) = mcf_serve::probes::declared_context(&file) else {
        return Vec::new();
    };
    let Some(filler) = mcf_serve::probes::a_filler_token(&file) else {
        return Vec::new();
    };

    let mut ask =
        |length: usize| mcf_serve::probes::accepts(socket, path, filler, length, Some(asked));
    let probed = probes::usable_context(path, declared, engine, &mut ask);

    let mut lines = vec![
        format!("  {}", probed.method.name),
        format!(" asks {}", probed.method.asks),
        format!(" decides  {}", probed.method.decides),
        String::new(),
    ];
    match &probed.outcome {
        Outcome::Observed(context) => {
            lines.push(format!(" declared {} token(s)", context.declared));
            lines.push(format!(
                " accepted {} token(s) of prompt, with one left to generate",
                context.accepted
            ));
            lines.push(String::new());
            // The declared context is the whole budget, not the prompt's share
            // of it, so a prompt one shorter is agreement rather than a
            // divergence — reporting that off-by-one would be reporting
            // arithmetic (B-055, F42).
            if context.accepted.saturating_add(1) >= context.declared {
                lines.push(
                    " agrees the file's claim holds: every token it declares but one is taken as prompt, and the one left over is the answer"
                        .to_owned(),
                );
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
            lines.push(
                match crate::log::record_probed_context(path, context, engine) {
                    Ok(journal) => format!(" recorded in {}", journal.display()),
                    Err(failure) => format!(
                        " BUT NOT RECORDED — {failure}; a measurement nobody can find later is the \
                     same as one not taken (A1, A2)"
                    ),
                },
            );
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
             {}, which is what MCF would address it as — `mcf run` sends raw text today (§3.8)",
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
