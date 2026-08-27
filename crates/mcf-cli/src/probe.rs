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

/// Probes a model.
pub(crate) fn run(model: &str) -> Response {
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

    // This probe's observation is *why generation ended*, and only MCF's own
    // engine reports it: a subprocess engine prints text and exits. So the
    // probe asks for the stand-in by name and the conditions say so — which is
    // D42's point that a probe result belongs to an engine and not to a model.
    let engine = format!("MCF's own stand-in, through the daemon at {build}");
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
            Some("stand-in"),
        )
    };
    let probed = probes::chat_template(&path, &bytes, TRIALS, BUDGET, &engine, &mut trial);

    let mut lines = vec![
        format!("probed {}", path.display()),
        String::new(),
        format!("  {}", probed.method.name),
        format!("    asks     {}", probed.method.asks),
        format!("    decides  {}", probed.method.decides),
        String::new(),
    ];
    match &probed.outcome {
        Outcome::Observed(addressed) => lines.extend(observed(addressed)),
        Outcome::Inconclusive { because } => {
            lines.push(format!("    INCONCLUSIVE — {because}"));
            lines.push(
                "    which licenses nothing: MCF configures no differently than before, and \
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
    lines.push(String::new());
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
fn observed(addressed: &Addressed) -> Vec<String> {
    let mut lines = vec!["    answered, then ended at the model's own stop token:".to_owned()];
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
                "   (ended without saying anything {quiet} of {})",
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
                format!("   turn ran {least} token(s)")
            }
            (Some(least), Some(middle), Some(most)) => {
                format!("   turn ran {least}-{most} token(s), middle {middle}")
            }
            _ => String::new(),
        };
        lines.push(format!(
            "      {name:<18} {ended} of {}{mark}{note}{span}",
            addressed.of
        ));
    }
    lines.push(String::new());
    lines.push(format!("    best     {}", addressed.best));

    // The divergence (B-058): what the file said against what the model did.
    let raw_is_best = addressed.best == "raw";
    let best_count = addressed
        .stopped
        .iter()
        .find(|(name, _)| *name == addressed.best)
        .map_or(0, |(_, ended)| *ended);
    lines.push(match (addressed.declared_a_template, raw_is_best) {
        (true, true) => format!(
            "    DIVERGENCE the file declares a chat template, and the model ended its turn \
             {best_count} of {} times addressed *raw* while the addressing built from that \
             template ended none. That is a disagreement between the file and the model worth \
             a person's attention — and it is not a licence to address this model raw: what \
             this probe measures is whether a turn ends, which may be measuring something \
             else here (A21, D42)",
            addressed.of
        ),
        (true, false) => format!(
            "    agrees   the file declares a chat template and the model ends its turns under \
             {}, which is what MCF would address it as — `mcf run` sends raw text today (§3.8)",
            addressed.best
        ),
        (false, false) => format!(
            "    DIVERGENCE the file declares no chat template, and the model ends its turns \
             under {} — a capability its own metadata does not mention (§3.18)",
            addressed.best
        ),
        (false, true) => {
            "    agrees   no template declared, and raw is what it answers to".to_owned()
        }
    });
    lines
}
