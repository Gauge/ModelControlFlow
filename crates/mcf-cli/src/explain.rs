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
use mcf_core::time::{Duration, Monotonic};
use mcf_hub::store;
use mcf_standin::gguf::{self, Model, TensorKind, Value};
use mcf_standin::recommended::Recommendation;

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
    for (what, value, source) in chosen(path, file) {
        // Three columns, and *both* of the last two wrapped under themselves.
        // The value was not wrapped once, on the reasoning that a value is
        // short — true of every row until a derived configuration arrived
        // carrying its own provenance, which is a sentence (F45). A value that
        // overruns pushes its source onto the same line and the table stops
        // being one.
        let mut held = wrapped(&value, 22).into_iter();
        let mut under = wrapped(&source, 56).into_iter();
        lines.push(
            format!(
                "  {what:<38}{:<22} {}",
                held.next().unwrap_or_default(),
                under.next().unwrap_or_default()
            )
            .trim_end()
            .to_owned(),
        );
        loop {
            let (left, right) = (held.next(), under.next());
            if left.is_none() && right.is_none() {
                break;
            }
            lines.push(
                format!(
                    "  {:<38}{:<22} {}",
                    "",
                    left.unwrap_or_default(),
                    right.unwrap_or_default()
                )
                .trim_end()
                .to_owned(),
            );
        }
    }

    lines.push(String::new());
    lines.push("WHAT MCF CANNOT TELL YOU, AND WHY".to_owned());
    for (question, why) in unanswered(path) {
        lines.push(format!("  {question}"));
        // Wrapped, because the rest of this report is: a paragraph that runs
        // past a terminal's width is one somebody stops reading, and what is
        // in these is the honest half of a defaults screen (C7).
        for line in wrapped(&why, 88) {
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
    // A paragraph that already has its own lines keeps them. A table wrapped
    // as prose is a table destroyed, and one of these answers is a table
    // (B-379) — a reader cannot compare thirteen figures that have been run
    // together into a paragraph.
    if text.contains('\n') {
        return text.lines().map(str::to_owned).collect();
    }
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
/// What this machine's own history says a request here would take (B-214,
/// PR3, A20).
///
/// **An estimate, and it says so in the sentence rather than only in the type.**
/// A20 permits the middle answer and then draws the line absolutely: an
/// estimate can never be mistaken for a measurement. The type wall is
/// `Estimate`'s; what this owes is that a reader meets the word.
///
/// Where there is no history to read between, the answer is the one that was
/// here before — *unmeasured, and here is what would measure it*. A band with
/// nothing under it would be worse than no band.
fn how_fast(path: &Path) -> String {
    let held = crate::history::read();
    let bytes = std::fs::metadata(path).map_or(0, |meta| meta.len());
    // The budget `mcf run` would use, which is what this page describes. A
    // projection at a budget nothing on this page mentions would be an answer
    // to a question the reader did not ask.
    let budget = run::TOKENS;
    let projected = mcf_bench::project::band(
        &held.points,
        bytes,
        u32::try_from(budget).unwrap_or(u32::MAX),
    );
    let unmeasured = "Unmeasured. Through MCF's own stand-in it is unanswerable in principle — \
                      B65 forbids a speed from it (D31). Through a provisioned engine it is \
                      answerable: `mcf bench <a> --against <b>` takes it, under conditions and \
                      with its uncertainty.";
    match projected {
        Ok(band) => format!(
            "Unmeasured *for this file*. From {} comparison arm(s) this machine has measured at \
             {budget} tokens, a request here would probably take {} — which is an ESTIMATE \
             read between two measured sizes, and A20 forbids it standing beside a measurement \
             or being promoted into one. It was {}, which is a condition of the estimate and \
             not a footnote (B-385, §3.4). `mcf bench` measures it.{}",
            held.points.len(),
            millisecond_band(band.band()),
            band.rested_on(),
            if held.unreadable == 0 {
                String::new()
            } else {
                format!(
                    " ({} earlier arm(s) could not be used: their files are not here now.)",
                    held.unreadable
                )
            }
        ),
        Err(why) => format!("{unmeasured} There is no projection either: {why}."),
    }
}

/// What each declared language costs on this vocabulary (B-379, DEC-002).
///
/// **A fact about the vocabulary, not about the model or the language.** A
/// vocabulary that spells a script expensively spends more context, more time
/// and more money on the same meaning — and says nothing whatever about how
/// well the model handles it. The wording keeps those apart deliberately,
/// because *expensive* reads as *bad* to a reader who is not being careful.
///
/// **A ratio against the cheapest** rather than a rank: what a reader needs is
/// *this costs 2.6 times that*, which is the number that compounds. And the
/// sentence travels with the result, because a different sentence gives
/// different figures and a ratio with no text behind it is not reproducible
/// (§3.4, §II).
///
/// No generation and no judgement (§3.15).
fn language_cost(path: &Path) -> String {
    let Some(vocabulary) = crate::bench::read_prefix(path)
        .and_then(|bytes| mcf_standin::gguf::parse(&bytes).ok())
        .and_then(|file| mcf_standin::tokenizer::Vocabulary::read(&file).ok())
    else {
        return "Unanswerable: this file carries no vocabulary MCF can read, and a cost per \
                language is a question about a vocabulary (A7)."
            .to_owned();
    };
    let vocabulary = &vocabulary;
    let mut costs: Vec<(&'static str, usize, usize)> = Vec::new();
    for sample in mcf_standin::languages::DECLARED {
        let Ok(tokens) = vocabulary.encode(sample.text, false) else {
            // A vocabulary that cannot represent a script at all is a fact
            // about it, and dropping the row would hide the strongest result
            // this can produce (A1, A7).
            costs.push((sample.language, 0, sample.text.chars().count()));
            continue;
        };
        costs.push((sample.language, tokens.len(), sample.text.chars().count()));
    }
    // **Total tokens, not tokens per character.** Per-character is the
    // arithmetic the register asked for and it answers the wrong question: a
    // script that writes the same meaning in nineteen characters looks
    // expensive per character while costing fewer tokens outright. What
    // compounds — context, money, time — is the total for the same meaning,
    // and these sentences carry the same meaning by construction, which is why
    // a carefully translated parallel text was chosen over one MCF wrote. The
    // character count stays on the line, because it is what makes the two
    // readings separable by anyone who wants the other one.
    let cheapest = costs
        .iter()
        .map(|(_, tokens, _)| *tokens)
        .filter(|tokens| *tokens > 0)
        .min()
        .unwrap_or(0)
        .max(1);

    let mut lines = vec![
        "One sentence, in each of a declared set of languages, as this vocabulary spells it:"
            .to_owned(),
        String::new(),
    ];
    costs.sort_by_key(|(_, tokens, _)| *tokens);
    for (language, tokens, letters) in &costs {
        if *tokens == 0 {
            lines.push(format!(
                "  {language:<22} this vocabulary cannot represent that text at all"
            ));
            continue;
        }
        let times = tokens.saturating_mul(10).checked_div(cheapest).unwrap_or(0);
        lines.push(format!(
            "  {language:<22} {tokens:>3} token(s) for {letters:>3} character(s) — {}.{}x the \
             cheapest here",
            times.wrapping_div(10),
            times.wrapping_rem(10)
        ));
    }
    // Wrapped here rather than by the renderer: this answer keeps its own
    // lines so that the table survives (see `wrapped`), which means the prose
    // in it has to wrap itself.
    let note = format!(
        "The sentence is {}, so every line above is the same meaning. What differs is what \
         this vocabulary spends on it — a property of the file, not a claim about the model's \
         fluency, and nothing here rates it (§3.15, DEC-002).",
        mcf_standin::languages::SOURCE
    );
    lines.push(String::new());
    for line in wrapped(&note, 74) {
        lines.push(format!("  {line}"));
    }
    lines.join("\n")
}

/// A band as a person reads it, without a float (A6).
///
/// Milliseconds and tenths from integer arithmetic. The word *estimate* is in
/// the sentence around it rather than here, because a reader who sees only a
/// range of numbers has been shown a measurement (A20).
fn millisecond_band(held: &mcf_core::measurement::Estimate<Duration<Monotonic>>) -> String {
    let tenths = |at: Duration<Monotonic>| {
        let held = at.as_nanos().wrapping_div(100_000);
        format!("{}.{}", held.wrapping_div(10), held.wrapping_rem(10))
    };
    format!(
        "between {} and {} ms",
        tenths(held.low()),
        tenths(held.high())
    )
}

/// Whose choice the sampler is (B60, B-281).
///
/// B60 has MCF adopt what the artifact recommends rather than imposing a house
/// style, and mark the adoption *declared, unverified* (A21). So the first
/// thing this does is **look**: at the file's own metadata, which is the only
/// source an offline machine has. Where the file recommends something, that is
/// what MCF would use, and the line says which key it was read from — *the
/// artifact says so* is not checkable and *this key says so* is.
///
/// Where it recommends nothing — which is the ordinary case, and was true of
/// six of six files examined for [findings.md](../../../doc/findings.md) F63 —
/// MCF's own choice is named **as MCF's**, with the reason it had to make one.
/// A house choice that says it is a house choice is a condition a reader can
/// weigh; one that does not is the hidden default §3.15 forbids.
fn sampler(file: &Model) -> (String, String) {
    match mcf_standin::recommended::read(file) {
        Recommendation::Declared { sampling, keys } => (
            sampling.to_string(),
            format!(
                "declared by the file ({}) and unverified here — adopted because the publisher \
                 knows what this model was trained for (A21, B60)",
                keys.join(", ")
            ),
        ),
        Recommendation::NoneDeclared => (
            "greedy".to_owned(),
            "MCF's own, because this file recommends none: no sampler key in its metadata, and a \
             conversion repository publishes no generation_config.json either (B60, F63). Stated \
             in crates/mcf-cli/src/run.rs"
                .to_owned(),
        ),
    }
}

/// The *file's* quantization rather than a name somebody gave it: a repository
/// calls a file `Q4_K_M` and what is inside it is whatever is inside it, which
/// is the same distinction A21 draws everywhere else.
pub(crate) fn quantizations(file: &Model) -> String {
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
/// What MCF would actually run this model on, worked out here the way the
/// daemon works it out.
///
/// **This screen said the wrong thing about the two choices that matter.** The
/// engine row read *more than one is provisioned and MCF will not choose*, and
/// the placement row read *the processor — MCF's stand-in has no accelerator
/// path*. Both came from an older path that predates engine resolution: MCF
/// does choose, by arithmetic over the model's own header and what each device
/// has free, and on this machine it chooses a graphics card. A defaults screen
/// that is wrong about the default is worse than no defaults screen (§3.15,
/// B-038, F133).
///
/// The same `mcf_serve::engines::resolve` the daemon calls, so the two cannot
/// disagree. No daemon is started to ask: resolution is arithmetic over this
/// machine, and `mcf explain` reads rather than runs.
fn resolved_here(path: &Path, file: &Model) -> Option<mcf_serve::engines::Choice> {
    let home = crate::models::default_root()
        .and_then(|models| models.parent().map(std::path::Path::to_path_buf))?;
    let bytes = std::fs::metadata(path).ok()?.len();
    let architecture = file.architecture()?;
    let trained = file
        .get(&format!("{architecture}.context_length"))
        .and_then(mcf_standin::gguf::Value::as_integer)
        .and_then(|held| u64::try_from(held).ok())?;
    let cache = mcf_serve::engines::cache_bytes_per_token(file);
    let free = mcf_core::hardware::Machine::read().memory.available;
    let free = match free {
        mcf_core::attested::Attested::Known(bytes) => Some(bytes.0),
        mcf_core::attested::Attested::Unknown => None,
    };
    let engines: Vec<(mcf_serve::engines::Engine, Vec<mcf_serve::engines::Device>)> =
        mcf_serve::engines::discover(&home)
            .into_iter()
            .map(|engine| {
                let devices = engine.devices(free).unwrap_or_default();
                (engine, devices)
            })
            .collect();
    mcf_serve::engines::resolve(&engines, bytes, cache, trained).ok()
}

/// The engine row: what MCF resolved, or what it would fall back to.
fn engine_row(
    path: &Path,
    resolution: Option<&mcf_serve::engines::Choice>,
) -> (String, &'static str) {
    let _ = path;
    if let Some(choice) = resolution {
        (
            choice.engine.clone(),
            "what MCF resolved for this model on this machine: the build that can compute on \
             the device below, chosen by arithmetic rather than by preference. `--engine \
             stand-in` asks for MCF's own instead",
        )
    } else {
        match crate::models::default_root()
            .and_then(|models| models.parent().map(std::path::Path::to_path_buf))
            .map(|home| {
                mcf_serve::adapters::only_one(mcf_serve::adapters::provisioned_llama(&home))
            }) {
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
        }
    }
}

/// The engine in force, named the way a configuration records it.
///
/// The same spelling `mcf probe` writes down, because a comparison between two
/// names for one engine reports a moved condition that did not move (F45).
fn engine_identity() -> String {
    let found = crate::models::default_root()
        .and_then(|models| models.parent().map(std::path::Path::to_path_buf))
        .map(|home| mcf_serve::adapters::only_one(mcf_serve::adapters::provisioned_llama(&home)));
    match found {
        // The component's own name, not a literal: a run on `llama.cpp-cuda`
        // and one on `llama.cpp` recorded the same identity while their paths
        // said otherwise, and a comparison between two names for one engine
        // reports a moved condition that did not move (F45, A6).
        Some(Ok(Some(llama))) => format!(
            "provisioned {} @{}",
            llama.component,
            llama.commit.get(..12).unwrap_or(&llama.commit)
        ),
        _ => "stand-in".to_owned(),
    }
}

/// What somebody applied to this model, if anybody did (D43, B-059).
fn derived(path: &Path) -> Option<mcf_serve::configured::Addressing> {
    derived_all(path).addressing
}

/// Everything somebody applied to this model.
fn derived_all(path: &Path) -> mcf_serve::configured::Derived {
    crate::models::default_root()
        .and_then(|models| models.parent().map(std::path::Path::to_path_buf))
        .map(|home| mcf_serve::configured::read_derived(&home, path))
        .unwrap_or_default()
}

fn chosen(path: &Path, file: &Model) -> Vec<(&'static str, String, String)> {
    let engine_now = engine_identity();
    // What MCF would actually use, by the same arithmetic the daemon does.
    // Where that answers it *is* the answer, because it is the answer the
    // daemon will act on; what follows is for a machine with nothing
    // provisioned, where there is nothing to resolve.
    let resolution = resolved_here(path, file);
    let engine = engine_row(path, resolution.as_ref());
    // How MCF will address this model. It is the first row because it is the
    // one that changed under M3, and because a page whose job is to have no
    // hidden choices (§3.15) must not omit the choice somebody made
    // deliberately (D43).
    let addressed = derived(path).map_or_else(
        || {
            (
                "raw text, MCF's default".to_owned(),
                "nothing has been applied here; `mcf probe <model>` asks the model how it wants \
                 to be addressed, and `--apply` acts on the answer (§3.8, D42)",
            )
        },
        |held| {
            let build = mcf_core::build_identity::BuildIdentity::current().to_string();
            let still = matches!(
                mcf_serve::configured::since(&held, &engine_now, &build),
                mcf_serve::configured::Since::ConditionsHold
            );
            (
                held.provenance(),
                if still {
                    "applied by somebody, on a probe's evidence, under the conditions in force \
                     here (D43, B-059)"
                } else {
                    "applied by somebody, on a probe's evidence gathered under conditions that \
                     have since moved — `mcf probe <model>` says whether the answer moved with \
                     them (D43, A21)"
                },
            )
        },
    );

    let sampler = sampler(file);
    vec![
        ("engine", engine.0.clone(), engine.1.to_owned()),
        ("addressed as", addressed.0, addressed.1.to_owned()),
        ("sampler", sampler.0, sampler.1),
        (
            "seed",
            "0 unless --seed says".to_owned(),
            "a condition of the answer (D19)".to_owned(),
        ),
        derived_all(path).budget.map_or_else(
            || {
                (
                    "token budget",
                    format!("{} unless --limit says", run::TOKENS),
                    "tokens rather than seconds, because a stand-in is slow by design (B49); \
                     nothing has been applied here, and `mcf probe` measures how long this \
                     model's turns actually run (B-056)"
                        .to_owned(),
                )
            },
            |budget| {
                (
                    "token budget",
                    format!("{} unless --limit says", budget.tokens),
                    "the longest turn this model was seen to finish, applied by somebody on a \
                     probe's evidence — MCF's own default would have cut its answers off \
                     (§3.8, D43)"
                        .to_owned(),
                )
            },
        ),
        (
            "context for planning",
            format!("{} tokens", mcf_hub::offer::PLANNING_CONTEXT),
            "what `mcf pull` plans against, stated in crates/mcf-cli/src/pull.rs".to_owned(),
        ),
        match &resolution {
            Some(choice) => (
                "placement",
                choice.device.name.clone(),
                format!(
                    "worked out from this model's own header and what the device has free: \
                     {} tokens of context fit there. `mcf settings <model>` shows every \
                     setting it would run under",
                    choice.context
                ),
            ),
            // A7: MCF could not work it out is not *the processor*. Which it
            // is decides where every figure about this model comes from.
            None => (
                "placement",
                "Unknown".to_owned(),
                "MCF could not work out where this would run: either no engine is \
                 provisioned, or this file's header does not say how it is shaped"
                    .to_owned(),
            ),
        },
    ]
}

/// The questions MCF has no basis to answer, each with the reason.
///
/// This list is the honest half of a defaults screen. §6.5 forbids inventing an
/// objective, so *which is best* has no answer here; §3.4 makes a number
/// without conditions meaningless, so *how fast* has none either. Both shrink
/// when the milestones that earn them land, and neither is filled in before.
fn unanswered(path: &Path) -> Vec<(&'static str, String)> {
    // What has been probed is a fact about *this model*, and saying "nothing
    // here has been probed" to somebody who probed it yesterday is the kind of
    // stale sentence that makes a reader stop believing the rest of the page
    // (A1, A21).
    let probed = derived(path).is_some();
    vec![
        (
            "Which quantization should I run?",
            "MCF has measured none of them here. The objective that would decide it is DEC-002 \
             and the measurements are M5–M7 (§6.5)."
                .to_owned(),
        ),
        ("How fast is it on this machine?", how_fast(path)),
        ("What does each language cost here?", language_cost(path)),
        (
            "What is it good at?",
            if probed {
                "M6's laboratories, gated on capabilities M3 verifies. What *has* been probed \
                 here is how this model wants to be addressed — `mcf probe` shows it, and the \
                 line above says what was applied. A model card's other claims are declarations \
                 rather than facts (§3.18)."
                    .to_owned()
            } else {
                "M6's laboratories, gated on capabilities M3 verifies. Nothing here has been \
                 probed, and a model card's claims are declarations rather than facts (§3.18). \
                 `mcf probe <model>` asks."
                    .to_owned()
            },
        ),
        (
            "Will it fit in memory?",
            "`mcf pull <repository>` answers that for every variant a repository publishes, from \
             the model's own configuration and this machine's free memory (B-213)."
                .to_owned(),
        ),
    ]
}

#[cfg(test)]
mod tests;
