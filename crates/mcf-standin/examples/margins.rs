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
            let rank = ranked
                .iter()
                .position(|(id, _)| id == next)
                .unwrap_or(usize::MAX);
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

fn print_logprobs(
    step: usize,
    margin: f32,
    logits: &[f32],
    ranked: &[(usize, f32)],
    asked: &[usize],
    chosen_before: &[usize],
) {
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
    let before: Vec<String> = chosen_before.iter().map(usize::to_string).collect();
    println!(
        "{{\"step\":{step},\"margin\":{margin},\"chosen_before\":[{}],\"logprobs\":{{{}}}}}",
        before.join(","),
        fields.join(",")
    );
}

fn flatten(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}
