//! `mcf explain`: what a model says about itself, and what MCF would do with it
//! (B-038, §3.15, A21).
//!
//! **Three columns, and the third is the one that matters.** Everything MCF
//! shows about a model is one of three things: what the *file declares*, what
//! *MCF verified*, and what *MCF chose*. §3.15 forbids hidden choices and A21
//! forbids treating a declaration as a fact, so the honest surface is one that
//! says which of the three each line is — rather than a table of values whose
//! provenance the reader has to guess.
//!
//! **It is also the place MCF says what it cannot tell you.** §VI's ease is
//! bought by choosing defaults without prompting, and §6.5 forbids inventing an
//! objective to justify them. So the defaults are shown with their source, and
//! the questions MCF has no basis to answer — which quantization is best here,
//! what it costs, how fast it runs — are named as unanswered rather than
//! answered badly. That list shrinks as M3 probes and M5 measures; today it is
//! most of the interesting questions, and saying so is the difference between
//! an instrument and a demo (C7).
//!
//! **Nothing here runs the model.** Reading a file's own description is cheap
//! and safe; loading its weights is neither, and an explanation that quietly
//! loaded four gigabytes would be a surprise where §3.11 asks for a decision.

use std::path::Path;

use mcf_core::digest::sha256;
use mcf_hub::store;
use mcf_standin::gguf::{self, Model, TensorKind, Value};

use crate::Response;
use crate::run;

/// Explains a model: what it says, what MCF read, what MCF would choose.
pub(crate) fn run(model: &str) -> Response {
    let path = match run::resolve(model) {
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
                text: run::ambiguous(model, &found),
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

    let file = match gguf::parse(&bytes) {
        Ok(file) => file,
        Err(failure) => {
            return Response {
                text: format!(
                    "{}\n  MCF reads GGUF; what a vendored engine would accept is B-320's \
                     question",
                    crate::say::refusal(
                        &format!("{} is not a model file MCF can read", path.display()),
                        &failure
                    )
                ),
                served: false,
            };
        }
    };

    Response {
        text: explain(&path, &bytes, &file),
        served: true,
    }
}

/// The three columns, and then what MCF cannot say.
fn explain(path: &Path, bytes: &[u8], file: &Model) -> String {
    let mut lines = vec![format!("{}", path.display()), String::new()];

    lines.push("WHAT THE FILE DECLARES ABOUT ITSELF  (A21: declared, not verified)".to_owned());
    for (key, shown) in declared(file) {
        lines.push(format!("  {key:<38}{shown}"));
    }

    lines.push(String::new());
    lines.push("WHAT MCF READ FROM THE BYTES  (verified here, now)".to_owned());
    lines.push(format!(
        "  {:<38}{} bytes",
        "size on this disk",
        bytes.len()
    ));
    lines.push(format!("  {:<38}{}", "sha256", sha256(bytes).hex()));
    lines.push(format!(
        "  {:<38}{} tensors, {}",
        "weights",
        file.tensors.len(),
        quantizations(file)
    ));
    if let Ok(provenance) = store::provenance_of(path) {
        lines.push(format!("  {:<38}{}", "came from", provenance.origin()));
        // The terms, where somebody is deciding whether to run it (§III,
        // B-023). What MCF says about a licence is what was declared and which
        // family the identifier puts it in — never whether a particular use is
        // allowed, which is a legal judgement about a specific person, and a
        // tool that guessed would be worse than one that stays quiet.
        lines.push(format!(
            "  {:<38}{}",
            "terms",
            mcf_hub::licence::describe(provenance.licence().known())
                .trim_start_matches("licence: ")
        ));
    } else {
        lines.push(format!(
            "  {:<38}{}",
            "came from", "nothing beside it says (A7)"
        ));
        // Said rather than left out: an absent line reads as *no restrictions*,
        // which is the one thing MCF must not imply about somebody else's
        // model (A7, §III).
        lines.push(format!(
            "  {:<38}{}",
            "terms", "unknown — nothing beside it states any (A7)"
        ));
    }

    lines.push(String::new());
    lines.push("WHAT MCF WOULD CHOOSE IF ASKED TO RUN IT  (§3.15: no hidden choices)".to_owned());
    for (what, value, source) in chosen() {
        // Three columns with the last one wrapped under itself. A space after
        // the value rather than a wider column: a value that exactly filled the
        // width used to run into its source with nothing between them, and
        // widening only moves where that happens.
        let mut under = wrapped(source, 56).into_iter();
        lines.push(format!(
            "  {what:<38}{value:<22} {}",
            under.next().unwrap_or_default()
        ));
        for line in under {
            lines.push(format!("  {:<38}{:<22} {line}", "", ""));
        }
    }

    lines.push(String::new());
    lines.push("WHAT MCF CANNOT TELL YOU, AND WHY".to_owned());
    for (question, why) in unanswered() {
        lines.push(format!("  {question}"));
        // Wrapped, because the rest of this report is: a paragraph that runs
        // past a terminal's width is one somebody stops reading, and what is
        // in these is the honest half of a defaults screen (C7).
        for line in wrapped(why, 88) {
            lines.push(format!("    {line}"));
        }
    }

    lines.join("\n")
}

/// Breaks a paragraph at word boundaries, at most `width` characters a line.
///
/// Counted in characters rather than bytes: these sentences contain § and — ,
/// and a wrap that counted bytes would break lines short for no reason a reader
/// could see.
fn wrapped(text: &str, width: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let mut line = String::new();
    for word in text.split_whitespace() {
        let would_be = line.chars().count() + 1 + word.chars().count();
        if !line.is_empty() && would_be > width {
            lines.push(core::mem::take(&mut line));
        }
        if !line.is_empty() {
            line.push(' ');
        }
        line.push_str(word);
    }
    if !line.is_empty() {
        lines.push(line);
    }
    lines
}

/// What the file says about itself, in the order a reader wants it.
///
/// Every one of these is the file's claim. MCF has read the bytes that state
/// them and has verified nothing about whether they describe the weights —
/// which is `hub.metadata.deceptive`'s whole subject, and why `mcf pull`
/// compares the two (B-022).
fn declared(file: &Model) -> Vec<(&'static str, String)> {
    let architecture = file.architecture().unwrap_or("unstated").to_owned();
    let mut shown = vec![
        ("format version", file.version.to_string()),
        ("architecture", architecture.clone()),
    ];
    for (key, label) in [
        ("block_count", "blocks"),
        ("embedding_length", "embedding width"),
        ("attention.head_count", "attention heads"),
        ("attention.head_count_kv", "key/value heads"),
        ("context_length", "context length"),
        ("feed_forward_length", "feed-forward width"),
    ] {
        let full = format!("{architecture}.{key}");
        shown.push((
            label,
            match file.get(&full) {
                Some(Value::Integer(number)) => number.to_string(),
                Some(other) => format!("{other:?}"),
                None => "the file does not say".to_owned(),
            },
        ));
    }
    shown.push((
        "vocabulary",
        match file.get("tokenizer.ggml.tokens") {
            Some(Value::List(tokens)) => format!("{} tokens", tokens.len()),
            _ => "the file does not say".to_owned(),
        },
    ));
    shown.push((
        "publisher's name for it",
        file.get("general.name")
            .and_then(Value::as_text)
            .unwrap_or("unstated")
            .to_owned(),
    ));
    shown
}

/// How the weights are encoded, counted by kind.
///
/// The *file's* quantization rather than a name somebody gave it: a repository
/// calls a file `Q4_K_M` and what is inside it is whatever is inside it, which
/// is the same distinction A21 draws everywhere else.
fn quantizations(file: &Model) -> String {
    let mut counted: Vec<(String, usize)> = Vec::new();
    for tensor in &file.tensors {
        let name = match tensor.kind {
            TensorKind::F32 => "F32".to_owned(),
            TensorKind::F16 => "F16".to_owned(),
            TensorKind::Q4_0 => "Q4_0".to_owned(),
            TensorKind::Q4_1 => "Q4_1".to_owned(),
            TensorKind::Q8_0 => "Q8_0".to_owned(),
            TensorKind::Unknown(number) => format!("a type MCF does not read ({number})"),
            // `TensorKind` is non-exhaustive: a type added later is one this
            // surface has not been taught to name, and saying so beats calling
            // it something it is not (A7).
            other => format!("{other:?}"),
        };
        match counted.iter_mut().find(|(kind, _)| *kind == name) {
            Some((_, count)) => *count = count.saturating_add(1),
            None => counted.push((name, 1)),
        }
    }
    counted
        .iter()
        .map(|(kind, count)| format!("{count}×{kind}"))
        .collect::<Vec<_>>()
        .join(", ")
}

/// What MCF would choose, and where each choice comes from.
///
/// Short, because MCF chooses little: no placement to decide and no
/// quantization to pick between. The engine is the one real choice now
/// (B-032), and it is made by a stated rule rather than a preference: the
/// provisioned engine where exactly one is here, MCF's own otherwise, and
/// `--engine` overrides either. What is here is what `mcf run` would use, and
/// every line names where it is written down so that a reader can go and
/// disagree with it.
fn chosen() -> Vec<(&'static str, String, &'static str)> {
    let engine = match crate::models::default_root()
        .and_then(|models| models.parent().map(std::path::Path::to_path_buf))
        .map(|home| mcf_serve::adapters::only_one(mcf_serve::adapters::provisioned_llama(&home)))
    {
        Some(Ok(Some(llama))) => (
            format!(
                "provisioned llama.cpp @{}, from {}",
                llama.commit.get(..12).unwrap_or(&llama.commit),
                llama.prefix.display()
            ),
            "the one engine provisioned on this machine (B-032, D39); `--engine stand-in` \
             asks for MCF's own instead",
        ),
        Some(Err(_)) => (
            "refused: more than one llama.cpp is provisioned".to_owned(),
            "MCF will not choose between builds; `mcf provision --list` shows them (§3.15)",
        ),
        _ => (
            "MCF's own stand-in".to_owned(),
            "nothing is provisioned here (D39); `mcf provision llama.cpp` would change this line",
        ),
    };
    vec![
        ("engine", engine.0, engine.1),
        (
            "sampler",
            "greedy".to_owned(),
            "MCF default, stated in crates/mcf-cli/src/run.rs",
        ),
        (
            "seed",
            "0 unless --seed says".to_owned(),
            "a condition of the answer (D19)",
        ),
        (
            "token budget",
            format!("{} unless --limit says", run::TOKENS),
            "tokens rather than seconds, because a stand-in is slow by design (B49)",
        ),
        (
            "context for planning",
            format!("{} tokens", crate::pull::PLANNING_CONTEXT),
            "what `mcf pull` plans against, stated in crates/mcf-cli/src/pull.rs",
        ),
        (
            "placement",
            "the processor".to_owned(),
            "MCF's stand-in has no accelerator path (D31, §3.2)",
        ),
    ]
}

/// The questions MCF has no basis to answer, each with the reason.
///
/// This list is the honest half of a defaults screen. §6.5 forbids inventing an
/// objective, so *which is best* has no answer here; §3.4 makes a number
/// without conditions meaningless, so *how fast* has none either. Both shrink
/// when the milestones that earn them land, and neither is filled in before.
fn unanswered() -> Vec<(&'static str, &'static str)> {
    vec![
        (
            "Which quantization should I run?",
            "MCF has measured none of them here. The objective that would decide it is DEC-002 \
             and the measurements are M5–M7 (§6.5).",
        ),
        (
            "How fast is it on this machine?",
            "Unmeasured. Through MCF's own stand-in it is unanswerable in principle — B65 \
             forbids a speed from it (D31). Through a provisioned engine it is answerable and \
             nobody has: that is M5's `mcf bench`, under conditions and with its uncertainty.",
        ),
        (
            "What is it good at?",
            "M6's laboratories, gated on capabilities M3 verifies. Nothing here has been probed, \
             and a model card's claims are declarations rather than facts (§3.18).",
        ),
        (
            "Will it fit in memory?",
            "`mcf pull <repository>` answers that for every variant a repository publishes, from \
             the model's own configuration and this machine's free memory (B-213).",
        ),
    ]
}

#[cfg(test)]
mod tests;
