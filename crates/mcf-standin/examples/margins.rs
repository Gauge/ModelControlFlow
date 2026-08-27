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

fn main() -> std::process::ExitCode {
    let mut arguments = std::env::args().skip(1);
    let (Some(path), Some(text)) = (arguments.next(), arguments.next()) else {
        eprintln!("usage: margins <model.gguf> \"some text\" [steps]");
        return std::process::ExitCode::FAILURE;
    };
    let steps: usize = arguments
        .next()
        .and_then(|value| value.parse().ok())
        .unwrap_or(10);

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

    println!("step  margin      chosen                    runner-up");
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
        println!(
            "{step:>4}  {:>9.5}  {:>8} {:<16?} {:>8} {:?}",
            best.1 - second.1,
            best.0,
            vocabulary.decode(&[best.0]),
            second.0,
            vocabulary.decode(&[second.0])
        );

        logits = match model.forward(best.0, position, &mut cache) {
            Ok(logits) => logits,
            Err(failure) => {
                eprintln!("{failure}");
                return std::process::ExitCode::FAILURE;
            }
        };
        position += 1;
    }
    std::process::ExitCode::SUCCESS
}
