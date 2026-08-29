//! What threads do to MCF's own engine, on a real model (B-366, B-384, F52).
//!
//! **Why this is a prototype and not a surface.** B65 forbids MCF's own engine
//! from reporting a speed, and the type system holds it: there is no `timing`
//! on `Run<StandIn>`. That prohibition is about what MCF *publishes* — a
//! throughput figure from a deliberately naive kernel would be a measurement of
//! the kernel, and somebody would read it as a measurement of the model. It is
//! not a prohibition on MCF knowing how long its own code takes, which is a
//! fact about MCF and belongs in [`findings.md`](../../../doc/findings.md) with
//! the other facts about MCF (F8, F12, F52 all measured exactly this way).
//!
//! **What is measured.** One model, one prompt, one seed, one token budget, run
//! at each of several thread counts. Three things come back:
//!
//! 1. **Did the answer move?** Every run's token sequence is compared against
//!    the one-thread run's. B-366's whole claim is that it cannot, and this is
//!    that claim asked of a real model rather than of a fixture.
//! 2. **How much faster.** Reported as the median of the repeats against the
//!    one-thread median, because a mean over a distribution with a floor and no
//!    ceiling is not the number anybody wants (B56).
//! 3. **How much noisier.** The middle half of the repeats, as a percentage of
//!    the median — the same quantity `mcf-prototype-timing-noise` reports, so
//!    that these numbers sit beside F52's table rather than needing their own
//!    interpretation. F52 is the standing evidence that thread count moves
//!    noise in directions nobody predicts.
//!
//! **The counts are interleaved.** Every repeat runs every count before any
//! count is repeated, so a machine that got busier partway through spreads that
//! across all of them instead of penalizing whichever ran last. The load
//! average is read at the start and the end and printed, because F52's
//! contaminated reading is the failure this format exists to make visible.
//!
//! **And the second question, which the first one raises.** A partition costs
//! what it costs to start the workers, and a product small enough pays that
//! cost for nothing. `products` sweeps matrix shapes against thread counts and
//! reports where partitioning begins to pay — the number a threshold needs, and
//! one that has to be measured on the machine rather than reasoned about.
//!
//! ```text
//! cargo run --release -p mcf-prototype-thread-scaling -- <model.gguf> [tokens] [repeats]
//! cargo run --release -p mcf-prototype-thread-scaling -- products [repeats]
//! ```

use std::time::Instant;

use mcf_core::hardware::load_average;
use mcf_standin::llama::{Loaded, load};
use mcf_standin::sample::Settings;
use mcf_standin::session::{Request, generate};
use mcf_standin::threads::Threads;
use mcf_standin::{gguf, tokenizer::Vocabulary};

/// The thread counts tried, unless the machine reports fewer.
const COUNTS: [usize; 7] = [1, 2, 4, 8, 16, 24, 32];

/// The prompt. Short on purpose: what is being measured is the per-token cost
/// of the forward pass, and a long prompt buys more cache and less signal.
const PROMPT: &str = "The capital of France is";

fn main() -> std::process::ExitCode {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    if arguments.first().is_some_and(|first| first == "products") {
        let repeats: usize = arguments.get(1).and_then(|a| a.parse().ok()).unwrap_or(9);
        return products(repeats);
    }
    let Some(path) = arguments.first() else {
        eprintln!(
            "usage: mcf-prototype-thread-scaling <model.gguf> [tokens] [repeats]\n\
             \n\
             Runs one model at several thread counts and reports what changed: the answer \
             (which must not), the speed, and the spread."
        );
        return std::process::ExitCode::FAILURE;
    };
    let tokens: usize = arguments.get(1).and_then(|a| a.parse().ok()).unwrap_or(16);
    let repeats: usize = arguments.get(2).and_then(|a| a.parse().ok()).unwrap_or(5);

    let Ok(bytes) = std::fs::read(path) else {
        eprintln!("could not read {path}");
        return std::process::ExitCode::FAILURE;
    };
    let file = match gguf::parse(&bytes) {
        Ok(file) => file,
        Err(failure) => {
            eprintln!("{path} is not a model MCF reads: {failure}");
            return std::process::ExitCode::FAILURE;
        }
    };
    let vocabulary = match Vocabulary::read(&file) {
        Ok(vocabulary) => vocabulary,
        Err(failure) => {
            eprintln!("the vocabulary would not read: {failure}");
            return std::process::ExitCode::FAILURE;
        }
    };
    let prompt = match vocabulary.encode(PROMPT, true) {
        Ok(prompt) => prompt,
        Err(failure) => {
            eprintln!("the prompt would not segment: {failure}");
            return std::process::ExitCode::FAILURE;
        }
    };

    let loading = Instant::now();
    let model = match load(&file, &bytes) {
        Ok(model) => model,
        Err(failure) => {
            eprintln!("the model would not load: {failure}");
            return std::process::ExitCode::FAILURE;
        }
    };
    let loaded_in = loading.elapsed();

    let available = Threads::what_the_machine_reports();
    let counts: Vec<usize> = COUNTS
        .into_iter()
        .filter(|count| *count <= available.count())
        .collect();

    println!("model    {path}");
    println!("machine  {}", available.describe());
    println!(
        "loaded   {:.1} s (dequantized once, then reused)",
        loaded_in.as_secs_f64()
    );
    println!("prompt   {PROMPT:?} — {} token(s)", prompt.len());
    println!("run      {tokens} token(s), greedy, seed 0, × {repeats} repeats, interleaved");
    println!("load     {} at the start", said_load());
    println!();

    let measured = match sweep(model, &prompt, &counts, tokens, repeats) {
        Ok(measured) => measured,
        Err(reason) => {
            eprintln!("{reason}");
            return std::process::ExitCode::FAILURE;
        }
    };

    report(&counts, &measured, tokens);
    println!("load     {} at the end", said_load());
    std::process::ExitCode::SUCCESS
}

/// Where a partition begins to pay, measured on one product at a time.
///
/// **Why this is asked separately.** A forward pass is dozens of products of
/// several shapes, so a model measurement says only whether the *mixture* pays.
/// A threshold has to be set on a single product, and the shape that decides it
/// is the shape of the smallest product a model performs — which for a
/// half-billion-parameter model is a few hundred rows of a few hundred columns.
///
/// The same product is computed repeatedly and the median taken. Each row of
/// the table is one shape; each column a thread count.
// As `report`: the division renders a fixed-point number.
#[allow(clippy::integer_division)]
fn products(repeats: usize) -> std::process::ExitCode {
    /// Shapes spanning what a forward pass actually performs, from an attention
    /// projection on a small model to an output projection on a large one.
    const SHAPES: [(usize, usize); 8] = [
        (576, 576),
        (1_536, 576),
        (2_048, 2_048),
        (5_632, 2_048),
        (4_096, 4_096),
        (11_008, 4_096),
        (32_000, 2_048),
        (49_152, 576),
    ];

    let available = Threads::what_the_machine_reports();
    let counts: Vec<usize> = COUNTS
        .into_iter()
        .filter(|count| *count <= available.count())
        .collect();

    println!("machine  {}", available.describe());
    println!("run      one product per cell, x {repeats} repeats, median reported");
    println!("load     {} at the start", said_load());
    println!();
    print!("{:>16}", "rows x columns");
    for count in &counts {
        print!("{count:>10}");
    }
    println!("{:>12}", "best");

    for (rows, columns) in SHAPES {
        let mut noise = Noise::seeded(u64::try_from(rows).unwrap_or(1));
        let matrix = noise.values(rows.saturating_mul(columns));
        let vector = noise.values(columns);

        let mut medians = Vec::new();
        let mut definition: Option<Vec<f32>> = None;
        for count in counts.iter().copied() {
            let threads = Threads::stated(count);
            let mut taken = Vec::new();
            for _ in 0..repeats {
                let started = Instant::now();
                let produced =
                    mcf_standin::ops::matmul_vec_across(&matrix, &vector, rows, columns, threads);
                taken.push(u64::try_from(started.elapsed().as_micros()).unwrap_or(u64::MAX));
                match &definition {
                    None => definition = Some(produced),
                    Some(first) => {
                        if !same_bits(first, &produced) {
                            eprintln!("THE ANSWER MOVED at {rows}x{columns}, {count} thread(s)");
                            return std::process::ExitCode::FAILURE;
                        }
                    }
                }
            }
            medians.push(median(&taken).unwrap_or(0));
        }

        print!("{rows:>10} x{columns:>4}");
        for value in &medians {
            print!("{value:>9}u");
        }
        let best = medians
            .iter()
            .copied()
            .enumerate()
            .filter(|(_, value)| *value > 0)
            .min_by_key(|(_, value)| *value)
            .and_then(|(slot, value)| {
                let one = medians.first().copied().unwrap_or(0);
                counts
                    .get(slot)
                    .map(|count| (*count, one.saturating_mul(100).checked_div(value.max(1))))
            });
        match best {
            Some((count, Some(hundredths))) => {
                println!(
                    "{:>12}",
                    format!("{count}t {}.{:02}x", hundredths / 100, hundredths % 100)
                );
            }
            _ => println!("{:>12}", "-"),
        }
    }
    println!();
    println!("Microseconds. `best` is the count with the lowest median and its speedup over one.");
    println!("load     {} at the end", said_load());
    std::process::ExitCode::SUCCESS
}

/// Two answers, compared bit for bit — the property, asked of every cell above.
fn same_bits(left: &[f32], right: &[f32]) -> bool {
    left.len() == right.len()
        && left
            .iter()
            .zip(right.iter())
            .all(|(one, other)| one.to_bits() == other.to_bits())
}

/// Weights for the product sweep, dense and of mixed magnitude.
struct Noise(u64);

impl Noise {
    const fn seeded(seed: u64) -> Self {
        Self(seed | 1)
    }

    fn next(&mut self) -> f32 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1);
        let bits = u32::try_from(self.0 >> 40).unwrap_or(0);
        let half = u16::try_from(bits & 0xFFFF).unwrap_or(0);
        (f32::from(half) - 32_768.0) / 32_768.0
    }

    fn values(&mut self, count: usize) -> Vec<f32> {
        (0..count).map(|_| self.next()).collect()
    }
}

/// The one-minute load average, or that the machine would not say (A7).
///
/// It decides nothing here — F3 established that a load average answers a
/// different question than the one a quiet machine needs asking. It is printed
/// because F52's contaminated reading was one where the load moved *during* the
/// measurement, and a number at each end is what makes that visible.
fn said_load() -> String {
    load_average().known().map_or_else(
        || "unknown — this machine does not publish one".to_owned(),
        |average| format!("{} (thousandths of a core)", average.0),
    )
}

/// What a sweep produced: per count, every repeat's duration in milliseconds,
/// and the answer that count gave.
struct Measured {
    /// One row per thread count, in the order they were asked for.
    durations: Vec<Vec<u64>>,
    /// What each count said, once it has said anything.
    answers: Vec<Option<Vec<usize>>>,
}

/// Every count, every repeat, interleaved — and the answer each one gave.
///
/// Interleaved so that a machine which got busier partway through spreads that
/// across every count rather than penalizing whichever ran last (F52's
/// contaminated reading is what this ordering is against).
fn sweep(
    mut model: Loaded,
    prompt: &[usize],
    counts: &[usize],
    tokens: usize,
    repeats: usize,
) -> Result<Measured, String> {
    let mut durations: Vec<Vec<u64>> = vec![Vec::new(); counts.len()];
    let mut answers: Vec<Option<Vec<usize>>> = vec![None; counts.len()];

    for repeat in 0..repeats {
        for (slot, count) in counts.iter().copied().enumerate() {
            // `across` consumes and returns, so the model is moved through the
            // loop rather than reloaded: dequantizing it again per cell would
            // measure the loader.
            model = model.across(Threads::stated(count));
            let started = Instant::now();
            let said = run_once(&model, prompt, tokens)
                .map_err(|reason| format!("the run failed at {count} thread(s): {reason}"))?;
            let took = u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX);
            if let Some(row) = durations.get_mut(slot) {
                row.push(took);
            }
            match answers.get(slot) {
                Some(Some(before)) if *before != said => {
                    return Err(format!(
                        "THE ANSWER MOVED at {count} thread(s), repeat {repeat}: {before:?} \
                         then {said:?}"
                    ));
                }
                Some(None) => {
                    if let Some(slot) = answers.get_mut(slot) {
                        *slot = Some(said);
                    }
                }
                _ => {}
            }
        }
        eprintln!("  repeat {} of {repeats} done", repeat.saturating_add(1));
    }
    Ok(Measured { durations, answers })
}

/// One generation, returning what was said.
fn run_once(model: &Loaded, prompt: &[usize], tokens: usize) -> Result<Vec<usize>, String> {
    let marked = generate(
        model,
        "prototype, thread-scaling",
        &Request {
            prompt: prompt.to_vec(),
            limit: tokens,
            settings: Settings::Greedy,
            seed: 0,
            stop: Vec::new(),
        },
    )
    .map_err(|failure| failure.to_string())?;
    Ok(marked.value().observed().tokens.clone())
}

/// The table, and the three things it says.
///
/// **Every quantity here is an integer.** A speedup is in hundredths and a
/// spread is in tenths of a percent, computed by whole-number arithmetic on
/// whole-number milliseconds. That is the workspace's habit rather than a
/// preference of this file: floating point is admitted where a format is made
/// of it, and nowhere else.
// The two divisions below are the rendering of a fixed-point number into a
// decimal point, which is what `mcf_core::hardware::contention` does for the
// same reason: both operands are bounded and the division *is* the conversion.
#[allow(clippy::integer_division)]
fn report(counts: &[usize], measured: &Measured, tokens: usize) {
    let (durations, answers) = (&measured.durations, &measured.answers);
    let one = durations.first().and_then(|row| median(row)).unwrap_or(0);
    let first_answer = answers.first().and_then(Option::as_ref);

    println!(
        "{:>8}  {:>10}  {:>12}  {:>12}  {:>9}  {:>7}",
        "threads", "median", "per token", "middle half", "speedup", "answer"
    );
    for (slot, count) in counts.iter().copied().enumerate() {
        let Some(row) = durations.get(slot) else {
            continue;
        };
        let Some(middle) = median(row) else { continue };
        let spread = middle_half_in_tenths(row);
        let per_token = u64::try_from(tokens)
            .ok()
            .and_then(|tokens| middle.checked_div(tokens));
        // Hundredths of a times: 250 is 2.50×.
        let speedup = middle
            .checked_div(1)
            .and_then(|middle| one.saturating_mul(100).checked_div(middle.max(1)));
        let agreed = match (answers.get(slot).and_then(Option::as_ref), first_answer) {
            (Some(said), Some(first)) if said == first => "same",
            (Some(_), Some(_)) => "MOVED",
            _ => "-",
        };
        println!(
            "{count:>8}  {middle:>7} ms  {:>9} ms  {:>11}  {:>9}  {agreed:>7}",
            per_token.map_or_else(|| "-".to_owned(), |value| value.to_string()),
            spread.map_or_else(
                || "too few".to_owned(),
                |tenths| format!("{}.{}%", tenths / 10, tenths % 10)
            ),
            speedup.map_or_else(
                || "-".to_owned(),
                |hundredths| format!("{}.{:02}x", hundredths / 100, hundredths % 100)
            ),
        );
    }
    println!();
    println!(
        "The `answer` column is the claim: B-366 says a thread count cannot change what a\n\
         model says, and every row above ran the same prompt at the same seed."
    );
    println!(
        "`middle half` is the interquartile range as a percentage of the median - the same\n\
         quantity F52 reports, so these sit beside that table."
    );
}

/// The median of a set of durations, which is an order statistic and not a
/// mean: a wall time has a floor and no ceiling (B56).
fn median(values: &[u64]) -> Option<u64> {
    if values.is_empty() {
        return None;
    }
    let mut sorted = values.to_vec();
    sorted.sort_unstable();
    sorted.get(sorted.len().checked_div(2)?).copied()
}

/// The middle half of the repeats, in tenths of a percent of the median.
///
/// `None` below four repeats, because a quartile of three numbers is not a
/// quartile and a spread nobody can compute is not a spread of zero (A7).
fn middle_half_in_tenths(values: &[u64]) -> Option<u64> {
    if values.len() < 4 {
        return None;
    }
    let mut sorted = values.to_vec();
    sorted.sort_unstable();
    let quarter = sorted.len().checked_div(4)?;
    let low = sorted.get(quarter).copied()?;
    let high = sorted
        .get(sorted.len().saturating_sub(quarter).saturating_sub(1))
        .copied()?;
    let middle = median(values)?.max(1);
    high.saturating_sub(low)
        .saturating_mul(1000)
        .checked_div(middle)
}
