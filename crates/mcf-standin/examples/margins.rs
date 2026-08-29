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
//!
//! `--logprobs-of 12,345,6789` prints, at the parting step (or at step 0 when
//! there is no `--against`), one JSON line holding MCF's log-softmax for those
//! tokens and for its own top eight: `{"step":N,"margin":M,"logprobs":{"12":
//! -0.031,…}}`. That is what a comparison of *distributions* reads — the
//! reference's top tokens scored by MCF, so that the two can be put side by
//! side in one unit that does not depend on how confident either model is
//! (B-373).

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
    let mut logprobs_of: Vec<usize> = Vec::new();
    let mut forced: Vec<usize> = Vec::new();
    let mut dump_at: Vec<usize> = Vec::new();
    while let Some(argument) = arguments.next() {
        if argument == "--against" {
            against = arguments.next();
        } else if argument == "--dump-at" {
            dump_at = arguments
                .next()
                .unwrap_or_default()
                .split(',')
                .filter_map(|id| id.trim().parse().ok())
                .collect();
        } else if argument == "--forced" {
            forced = arguments
                .next()
                .unwrap_or_default()
                .split(',')
                .filter_map(|id| id.trim().parse().ok())
                .collect();
        } else if argument == "--logprobs-of" {
            logprobs_of = arguments
                .next()
                .unwrap_or_default()
                .split(',')
                .filter_map(|id| id.trim().parse().ok())
                .collect();
        } else if let Ok(count) = argument.parse() {
            steps = count;
        }
    }
    let wants_logprobs = !logprobs_of.is_empty();

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

    // Teacher forcing: MCF is made to read the reference's own tokens rather
    // than its own, so that the two can still be compared *after* they have
    // parted. Greedy generation stops being comparable at the first
    // disagreement — everything downstream is a different sentence — and that
    // is the whole reason this mode exists: whether MCF's agreement with the
    // reference decays with position cannot be asked any other way, and
    // position is exactly where a rotary or cache defect would live (B-377).
    if !forced.is_empty() {
        for next in &forced {
            let mut ranked: Vec<(usize, f32)> = logits.iter().copied().enumerate().collect();
            ranked.sort_by(|left, right| {
                right
                    .1
                    .partial_cmp(&left.1)
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .then_with(|| left.0.cmp(&right.0))
            });
            let (Some(best), Some(second)) = (ranked.first().copied(), ranked.get(1).copied())
            else {
                break;
            };
            // Where MCF's own choice is not the reference's, the rank it gave
            // the reference's token says how far apart they are: rank 1 with a
            // hair of margin is a coin, rank 900 is not.
            let rank = ranked
                .iter()
                .position(|(id, _)| id == next)
                .unwrap_or(usize::MAX);
            // The whole distribution where it was asked for, so that two
            // engines disagreeing about which token is best can be compared on
            // what they *both* think of the same twenty (B-373, B-377).
            if dump_at.contains(&position) {
                let largest = best.1;
                let mut total = 0.0_f32;
                for value in &logits {
                    total += (value - largest).exp();
                }
                let offset = largest + total.ln();
                let top: Vec<String> = ranked
                    .iter()
                    .take(20)
                    .map(|(id, value)| format!("\"{id}\":{:.5}", value - offset))
                    .collect();
                println!("{{\"dump\":{position},\"logprobs\":{{{}}}}}", top.join(","));
            }
            // Whether MCF's own choice is the model's end of turn. A
            // reference run with `ignore_eos` — which is how a comparison
            // reaches past a sliding window at all — is forbidden to stop, so
            // where MCF stops the two are being asked different questions and
            // the disagreement is the flag rather than the engine. F40's
            // largest apparent defect, by an order of magnitude, was exactly
            // this. It is reported rather than hidden so that a reader can see
            // how many positions were set aside (A1).
            let mine_is_stop = vocabulary.ending == Some(best.0);
            println!(
                "{{\"position\":{},\"forced\":{},\"mine\":{},\"agreed\":{},\"rank_of_forced\":{},\"margin\":{:.5},\"mine_is_stop\":{}}}",
                position,
                next,
                best.0,
                best.0 == *next,
                rank,
                best.1 - second.1,
                mine_is_stop
            );
            logits = match model.forward(*next, position, &mut cache) {
                Ok(logits) => logits,
                Err(failure) => {
                    eprintln!("{failure}");
                    return std::process::ExitCode::FAILURE;
                }
            };
            position += 1;
        }
        return std::process::ExitCode::SUCCESS;
    }

    if against.is_none() && !wants_logprobs {
        println!("step  margin      chosen                    runner-up");
    }
    let reference = against.unwrap_or_default();
    let mut produced = String::new();
    let mut chosen: Vec<usize> = Vec::new();
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
        // The distribution at this step, where it was asked for: at the
        // parting step under `--against`, at step 0 otherwise.
        let parts_here = if reference.is_empty() {
            step == 0
        } else {
            produced.push_str(&vocabulary.decode(&[best.0]));
            !flatten(&reference).starts_with(&flatten(&produced))
        };
        if wants_logprobs && parts_here {
            print_logprobs(step, margin, &logits, &ranked, &logprobs_of, &chosen);
            return std::process::ExitCode::SUCCESS;
        }
        chosen.push(best.0);
        if wants_logprobs {
            // Keep going to the parting step; nothing else is printed.
        } else if reference.is_empty() {
            println!(
                "{step:>4}  {:>9.5}  {:>8} {:<16?} {:>8} {:?}",
                margin,
                best.0,
                vocabulary.decode(&[best.0]),
                second.0,
                vocabulary.decode(&[second.0])
            );
        } else if parts_here {
            // Whitespace is flattened on both sides the way the oracle's text
            // comparison flattens it, so a newline against a space is not a
            // divergence here that the text comparison would not see.
            println!("diverged at {step} margin {margin:.5}");
            return std::process::ExitCode::SUCCESS;
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
    if wants_logprobs {
        println!("{{\"agreed\":true}}");
    } else if !reference.is_empty() {
        println!("agreed");
    }
    std::process::ExitCode::SUCCESS
}

/// One JSON line: the step, the margin, and MCF's log-softmax for the tokens
/// asked about and for its own top eight.
fn print_logprobs(
    step: usize,
    margin: f32,
    logits: &[f32],
    ranked: &[(usize, f32)],
    asked: &[usize],
    chosen_before: &[usize],
) {
    // log-softmax, computed the stable way: subtract the maximum first.
    let largest = ranked.first().map_or(0.0, |(_, value)| *value);
    let mut total = 0.0_f32;
    for value in logits {
        total += (value - largest).exp();
    }
    let normalizer = largest + total.ln();
    let mut wanted: Vec<usize> = asked.to_vec();
    for (id, _) in ranked.iter().take(8) {
        if !wanted.contains(id) {
            wanted.push(*id);
        }
    }
    let mut fields: Vec<String> = Vec::new();
    for id in wanted {
        if let Some(logit) = logits.get(id) {
            fields.push(format!("\"{id}\":{}", logit - normalizer));
        }
    }
    // The tokens MCF chose before this step: a distribution at step N is only
    // comparable with another's at step N if both reached it through the same
    // tokens, and text agreement does not guarantee that (F34).
    let before: Vec<String> = chosen_before.iter().map(usize::to_string).collect();
    println!(
        "{{\"step\":{step},\"margin\":{margin},\"chosen_before\":[{}],\"logprobs\":{{{}}}}}",
        before.join(","),
        fields.join(",")
    );
}

/// Runs of whitespace as one space, ends trimmed — the oracle's own rule.
fn flatten(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}
