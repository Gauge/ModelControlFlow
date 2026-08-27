//! How close the winning token was, at every step of a generation (B-368, A19).
//!
//! **What this is for.** When MCF and a reference implementation produce
//! different text from the same model and the same prompt, there are two
//! possible causes and they call for opposite responses. Either MCF computed
//! something wrong — a defect — or the two top tokens were so close that any
//! difference in summation order picks a different winner, which is not a
//! defect in either implementation and never will be.
//!
//! The margin between the best and second-best logit is what distinguishes
//! them. A divergence where the margin is a hundredth of a logit is a coin
//! landing differently; a divergence where the margin is two logits is
//! somebody's arithmetic being wrong.
//!
//!   cargo run -p mcf-standin --example margins -- <model.gguf> "some text" [steps]
//!   cargo run -p mcf-standin --example margins -- <model.gguf> "some text" [steps] --against "<text>"
//!
//! `--against` answers the oracle's question directly: at which step does MCF's
//! generation stop being a prefix of the reference's text, and what was the
//! margin *there*? The minimum margin anywhere in a generation explains
//! nothing about a divergence at its first token — a broken `Q3_K` decoder
//! diverged at step 0 with a margin of 0.45 and was excused by a 0.02 five
//! tokens later (F32). One line is printed: `diverged at <step> margin <m>`,
//! or `agreed` when the whole generation is a prefix of the reference.

#[allow(
    clippy::too_many_lines,
    reason = "a diagnostic's main is the sequence of things it loads and the one loop it runs; \
              splitting it would put the loading in one function and what is loaded for in \
              another"
)]
fn main() -> std::process::ExitCode {
    let mut arguments = std::env::args().skip(1);
    let (Some(path), Some(text)) = (arguments.next(), arguments.next()) else {
        eprintln!("usage: margins <model.gguf> \"some text\" [steps]");
        return std::process::ExitCode::FAILURE;
    };
    let mut steps: usize = 10;
    let mut against: Option<String> = None;
    while let Some(argument) = arguments.next() {
        if argument == "--against" {
            against = arguments.next();
        } else if let Ok(count) = argument.parse() {
            steps = count;
        }
    }

    let bytes = match std::fs::read(&path) {
        Ok(bytes) => bytes,
        Err(error) => {
            eprintln!("{path}: {error}");
            return std::process::ExitCode::FAILURE;
        }
    };
    let file = match mcf_standin::gguf::parse(&bytes) {
        Ok(file) => file,
        Err(failure) => {
            eprintln!("{path}: {failure}");
            return std::process::ExitCode::FAILURE;
        }
    };
    let vocabulary = match mcf_standin::tokenizer::Vocabulary::read(&file) {
        Ok(vocabulary) => vocabulary,
        Err(failure) => {
            eprintln!("{path}: {failure}");
            return std::process::ExitCode::FAILURE;
        }
    };
    let model = match mcf_standin::llama::load(&file, &bytes) {
        Ok(model) => model,
        Err(failure) => {
            eprintln!("{path}: {failure}");
            return std::process::ExitCode::FAILURE;
        }
    };
    let prompt = match vocabulary.encode(&text, true) {
        Ok(prompt) => prompt,
        Err(failure) => {
            eprintln!("{path}: {failure}");
            return std::process::ExitCode::FAILURE;
        }
    };

    let mut cache = mcf_standin::llama::Cache::for_model(&model.shape);
    let mut position = 0;
    let mut logits = Vec::new();
    for token in &prompt {
        logits = match model.forward(*token, position, &mut cache) {
            Ok(logits) => logits,
            Err(failure) => {
                eprintln!("{failure}");
                return std::process::ExitCode::FAILURE;
            }
        };
        position += 1;
    }

    if against.is_none() {
        println!("step  margin      chosen                    runner-up");
    }
    let reference = against.unwrap_or_default();
    let mut produced = String::new();
    for step in 0..steps {
        let mut ranked: Vec<(usize, f32)> = logits.iter().copied().enumerate().collect();
        // Highest first, and the lower identifier on a tie — the same order the
        // sampler uses, so this reports the choice that was actually made.
        ranked.sort_by(|left, right| {
            right
                .1
                .partial_cmp(&left.1)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| left.0.cmp(&right.0))
        });
        let (Some(best), Some(second)) = (ranked.first().copied(), ranked.get(1).copied()) else {
            break;
        };
        let margin = best.1 - second.1;
        if reference.is_empty() {
            println!(
                "{step:>4}  {:>9.5}  {:>8} {:<16?} {:>8} {:?}",
                margin,
                best.0,
                vocabulary.decode(&[best.0]),
                second.0,
                vocabulary.decode(&[second.0])
            );
        } else {
            produced.push_str(&vocabulary.decode(&[best.0]));
            // Whitespace is flattened on both sides the way the oracle's text
            // comparison flattens it, so a newline against a space is not a
            // divergence here that the text comparison would not see.
            let mine = flatten(&produced);
            let theirs = flatten(&reference);
            if !theirs.starts_with(&mine) {
                println!("diverged at {step} margin {margin:.5}");
                return std::process::ExitCode::SUCCESS;
            }
        }

        logits = match model.forward(best.0, position, &mut cache) {
            Ok(logits) => logits,
            Err(failure) => {
                eprintln!("{failure}");
                return std::process::ExitCode::FAILURE;
            }
        };
        position += 1;
    }
    if !reference.is_empty() {
        println!("agreed");
    }
    std::process::ExitCode::SUCCESS
}

/// Runs of whitespace as one space, ends trimmed — the oracle's own rule.
fn flatten(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}
