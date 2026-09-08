use std::path::Path;

use mcf_hub::store;
use mcf_standin::gguf::{self, Model, TensorKind};
use mcf_standin::recommended::Recommendation;

use crate::Response;
use crate::models;

pub(crate) fn run(model: &str) -> Response {
    let path = match models::resolve_named(model) {
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
                text: models::ambiguous(model, &found),
                served: false,
            };
        }
    };

    let Some(bytes) = crate::models::read_prefix(&path) else {
        return Response {
            text: format!(
                "mcf: {} could not be read as a model\n  MCF grew its read to the whole file                  and still could not find a GGUF directory in it",
                path.display()
            ),
            served: false,
        };
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
        text: explain(&path, &file),
        served: true,
    }
}

pub(crate) fn json(model: &str) -> Response {
    let path = match models::resolve_named(model) {
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
                text: models::ambiguous(model, &found),
                served: false,
            };
        }
    };
    let asked = mcf_serve::control::Request::Anatomy {
        model: path.to_string_lossy().into_owned(),
    };
    match crate::hosting::ask(&asked) {
        Ok(answer) => Response {
            text: answer.to_line(),
            served: true,
        },
        Err(why) => Response {
            text: why,
            served: false,
        },
    }
}

fn explain(path: &Path, file: &Model) -> String {
    let mut lines = vec![format!("{}", path.display()), String::new()];

    lines.push("WHAT THE FILE DECLARES ABOUT ITSELF  (declared, not verified)".to_owned());
    for (key, shown) in header::declared(file) {
        lines.push(format!("  {key:<38}{shown}"));
    }

    lines.push(String::new());
    lines.push("WHAT MCF READ FROM THE BYTES  (verified here, now)".to_owned());
    let held = std::fs::metadata(path).map_or(0, |about| about.len());
    lines.push(format!("  {:<38}{held} bytes", "size on this disk"));
    if mcf_hub::store::part_of_a_set(path).is_some()
        && let Ok(whole) = mcf_hub::store::bytes_of_the_whole(path)
        && whole != held
    {
        lines.push(format!(
            "  {:<38}{whole} bytes — this file is one part of the model, and every part is \
             loaded together",
            "the whole set",
        ));
    }
    let digest = mcf_core::integrity::digest_of(path)
        .map_or_else(|_| "unreadable".to_owned(), |held| held.hex());
    lines.push(format!("  {:<38}{digest}", "sha256"));
    lines.push(format!(
        "  {:<38}{} tensors, {}",
        "weights",
        file.tensors.len(),
        quantizations(file)
    ));
    lines.extend(anatomy::counted(file));
    if let Ok(provenance) = store::provenance_of(path) {
        lines.push(format!("  {:<38}{}", "came from", provenance.origin()));
        lines.push(format!(
            "  {:<38}{}",
            "terms",
            mcf_hub::licence::describe(provenance.licence().known())
                .trim_start_matches("licence: ")
        ));
    } else {
        lines.push(format!("  {:<38}{}", "came from", "nothing beside it says"));
        lines.push(format!(
            "  {:<38}{}",
            "terms", "unknown — nothing beside it states any"
        ));
    }

    lines.push(String::new());
    lines.extend(anatomy::agreed(file));
    lines.push(String::new());
    lines.extend(anatomy::costed(file));
    lines.push(String::new());
    lines.extend(anatomy::spoken(file));

    lines.push(String::new());
    lines.push("WHAT MCF WOULD CHOOSE IF ASKED TO RUN IT  (no hidden choices)".to_owned());
    for (what, value, source) in chosen(path, file) {
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
        for line in wrapped(&why, 88) {
            lines.push(format!("    {line}"));
        }
    }

    lines.join("\n")
}

pub(crate) fn wrapped(text: &str, width: usize) -> Vec<String> {
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

fn language_cost(path: &Path) -> String {
    let Some(vocabulary) = crate::models::read_prefix(path)
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
            costs.push((sample.language, 0, sample.text.chars().count()));
            continue;
        };
        costs.push((sample.language, tokens.len(), sample.text.chars().count()));
    }
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

fn sampler(file: &Model) -> (String, String) {
    match mcf_standin::recommended::read(file) {
        Recommendation::Declared { sampling, keys } => (
            sampling.to_string(),
            format!(
                "declared by the file ({}) and unverified here. Each request states its own \
                 temperature and seed; how a draw above nought is truncated is the engine's \
                 reading of the file until MCF states that too",
                keys.join(", ")
            ),
        ),
        Recommendation::NoneDeclared => (
            "greedy".to_owned(),
            "MCF's own, because this file recommends none: no sampler key in its metadata under \
             either namespace, and a conversion repository publishes no generation_config.json \
             either. Stated in crates/mcf-cli/src/run.rs"
                .to_owned(),
        ),
    }
}

pub(crate) fn quantizations(file: &Model) -> String {
    let mut counted: Vec<(String, usize)> = Vec::new();
    for tensor in &file.tensors {
        let name = match tensor.kind {
            TensorKind::F32 => "F32".to_owned(),
            TensorKind::F16 => "F16".to_owned(),
            TensorKind::BF16 => "BF16".to_owned(),
            TensorKind::Q4_0 => "Q4_0".to_owned(),
            TensorKind::Q4_1 => "Q4_1".to_owned(),
            TensorKind::Q8_0 => "Q8_0".to_owned(),
            TensorKind::Unknown(number) => format!("a type MCF does not read ({number})"),
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

fn resolved_here(
    path: &Path,
    file: &Model,
) -> core::result::Result<mcf_serve::engines::Choice, String> {
    let Some(home) = crate::models::default_root()
        .and_then(|models| models.parent().map(std::path::Path::to_path_buf))
    else {
        return Err("MCF could not find where its own components live".to_owned());
    };
    let bytes = mcf_hub::store::bytes_of_the_whole(path)
        .map_err(|_| "this model could not be measured on this disk".to_owned())?;
    let Some(architecture) = file.architecture() else {
        return Err("this file's header does not say what architecture it is".to_owned());
    };
    let Some(trained) = file
        .get(&format!("{architecture}.context_length"))
        .and_then(mcf_standin::gguf::Value::as_integer)
        .and_then(|held| u64::try_from(held).ok())
    else {
        return Err(
            "this file's header does not say how long a conversation it was trained for".to_owned(),
        );
    };
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
    mcf_serve::engines::resolve(&engines, bytes, cache, trained).map_err(|refused| refused.says())
}

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
                "MCF will not choose between builds; `mcf provision --list` shows them",
            ),
            _ => (
                "MCF's own stand-in".to_owned(),
                "nothing is provisioned here; `mcf provision llama.cpp` would change this line",
            ),
        }
    }
}

fn engine_identity() -> String {
    let found = crate::models::default_root()
        .and_then(|models| models.parent().map(std::path::Path::to_path_buf))
        .map(|home| mcf_serve::adapters::only_one(mcf_serve::adapters::provisioned_llama(&home)));
    match found {
        Some(Ok(Some(llama))) => format!(
            "provisioned {} @{}",
            llama.component,
            llama.commit.get(..12).unwrap_or(&llama.commit)
        ),
        _ => "stand-in".to_owned(),
    }
}

fn derived(path: &Path) -> Option<mcf_serve::configured::Addressing> {
    derived_all(path).addressing
}

fn derived_all(path: &Path) -> mcf_serve::configured::Derived {
    crate::models::default_root()
        .and_then(|models| models.parent().map(std::path::Path::to_path_buf))
        .map(|home| mcf_serve::configured::read_derived(&home, path))
        .unwrap_or_default()
}

fn chosen(path: &Path, file: &Model) -> Vec<(&'static str, String, String)> {
    let engine_now = engine_identity();
    let resolution = resolved_here(path, file);
    let engine = engine_row(path, resolution.as_ref().ok());
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
            "a condition of the answer".to_owned(),
        ),
        (
            "context for planning",
            format!("{} tokens", mcf_hub::offer::PLANNING_CONTEXT),
            "what `mcf pull` plans against, stated in crates/mcf-cli/src/pull.rs".to_owned(),
        ),
        match &resolution {
            Ok(choice) => (
                "placement",
                choice.device.name.clone(),
                format!(
                    "worked out from this model's own header and what the device has free: \
                     {} tokens of context fit there. `mcf settings <model>` shows every \
                     setting it would run under",
                    choice.context
                ),
            ),
            Err(why) => (
                "placement",
                "Unknown".to_owned(),
                format!("MCF could not work out where this would run: {why}"),
            ),
        },
    ]
}

fn unanswered(path: &Path) -> Vec<(&'static str, String)> {
    let probed = derived(path).is_some();
    vec![
        (
            "Which quantization should I run?",
            "MCF has measured none of them here. The objective that would decide it is DEC-002 \
             and the measurements are M5–M7 (§6.5)."
                .to_owned(),
        ),
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

mod anatomy;
mod header;

#[cfg(test)]
mod tests;
