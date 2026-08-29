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
    if arguments.first().is_some_and(|first| first == "ladder") {
        return ladder(arguments.get(1..).unwrap_or_default());
    }
    if arguments.first().is_some_and(|first| first == "shapes") {
        let Some(path) = arguments.get(1) else {
            eprintln!("usage: mcf-prototype-thread-scaling shapes <model.gguf>");
            return std::process::ExitCode::FAILURE;
        };
        return shapes(path);
    }
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

/// What a token costs on MCF's own engine, across the models this machine holds
/// (B-384).
///
/// **The question B-384 asks is not how fast the engine is.** It is *how large
/// a model this engine can usefully read* — and "usefully" has to be tied to a
/// purpose or it is a preference. The purpose is the one that justifies the
/// engine existing at all: A19 and D31 put it here to be checked against an
/// independent implementation, so the number that matters is how long that
/// cross-check takes. F49's comparison is a hundred and twenty positions, so
/// the last column is what that would cost on each model.
///
/// Every model is loaded, timed over a fixed token budget at the thread count
/// the machine reports, and reported with its parameter count — so the shape of
/// the relationship is visible rather than assumed to be linear.
fn ladder(paths: &[String]) -> std::process::ExitCode {
    /// How many tokens each model generates. Small, because the largest model
    /// in a ladder decides how long the whole thing takes and the per-token
    /// cost is what is wanted.
    const TOKENS: usize = 4;
    /// How many positions F49's cross-check compares — what the last column
    /// projects (F49, B-368).
    const CROSS_CHECK_POSITIONS: u64 = 120;

    if paths.is_empty() {
        eprintln!("usage: mcf-prototype-thread-scaling ladder <model.gguf>...");
        return std::process::ExitCode::FAILURE;
    }
    let threads = Threads::what_the_machine_reports();
    println!("machine  {}", threads.describe());
    println!("run      {TOKENS} token(s) after a {PROMPT:?} prompt, greedy, seed 0");
    println!("load     {} at the start", said_load());
    println!();
    println!(
        "{:>52}{:>14}{:>12}{:>10}{:>12}{:>14}",
        "model", "elements", "multiplied", "dequantized", "per token", "120 positions"
    );

    for path in paths {
        println!("{}", one_rung(path, threads, TOKENS, CROSS_CHECK_POSITIONS));
    }
    println!();
    println!(
        "`120 positions` is F49's cross-check projected from the measured per-token cost -\n\
         an estimate and labelled one (A20), not a measurement of a run that happened."
    );
    println!("load     {} at the end", said_load());
    std::process::ExitCode::SUCCESS
}

/// One model of the ladder — every column that could be filled, and the reason
/// for any that could not.
///
/// **A rung that did not run still reports what was read.** A4: a partial
/// outcome is an outcome. The size of a model MCF refuses is exactly the
/// interesting thing about it — the reference model this project is named
/// around is refused for its *architecture*, and a table that printed only the
/// refusal would have hidden that its size was never the blocker.
fn one_rung(path: &str, threads: Threads, tokens: usize, positions: u64) -> String {
    let mut row = Rung::new(short(path));
    let Ok(bytes) = std::fs::read(path) else {
        return row.refused("the file could not be read");
    };
    let file = match gguf::parse(&bytes) {
        Ok(file) => file,
        Err(failure) => return row.refused(&failure.to_string()),
    };
    // Tensor elements, not "parameters" as a publisher counts them: every
    // number in every tensor the file carries, embedding table included, which
    // is what the engine multiplies and stores. A model sold as 15M reads as
    // 24M here because nine million of them are its vocabulary.
    if let Some(dequantized) = file.dequantized_bytes() {
        row.elements = dequantized.checked_div(4);
        row.dequantized = Some(dequantized);
    }

    let vocabulary = match Vocabulary::read(&file) {
        Ok(vocabulary) => vocabulary,
        Err(failure) => return row.refused(&failure.to_string()),
    };
    let prompt = match vocabulary.encode(PROMPT, true) {
        Ok(prompt) => prompt,
        Err(failure) => return row.refused(&failure.to_string()),
    };

    let loading = Instant::now();
    let model = match load(&file, &bytes) {
        Ok(model) => model.across(threads),
        Err(failure) => return row.refused(&failure.to_string()),
    };
    row.loaded = Some(loading.elapsed().as_secs_f64());
    // What a forward pass multiplies: everything the file carries, less the
    // embedding table, which is indexed rather than multiplied.
    let shape = &model.shape;
    let table = u64::try_from(shape.vocabulary.saturating_mul(shape.embedding)).unwrap_or(0);
    row.multiplied = row.elements.map(|all| all.saturating_sub(table));

    let started = Instant::now();
    if let Err(reason) = run_once(&model, &prompt, tokens) {
        return row.refused(&reason);
    }
    let took = u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX);
    row.per_token = u64::try_from(tokens)
        .ok()
        .and_then(|tokens| took.checked_div(tokens));
    row.projected = row.per_token.map(|each| each.saturating_mul(positions));
    row.render()
}

/// One row of the ladder, filled as far as the model got.
struct Rung {
    name: String,
    elements: Option<u64>,
    /// The elements a forward pass actually multiplies against.
    ///
    /// **Everything except the embedding table.** A token's embedding is one
    /// *row* read out of that table, not a product against all of it — and for
    /// a small model the table is most of the file. `stories15M` carries 24
    /// million elements of which 9 million are vocabulary, so a cost per
    /// element computed from the file's total says the small models are more
    /// expensive per element than the large ones, which is an artefact of the
    /// denominator rather than a fact about the engine.
    ///
    /// The output projection *is* multiplied, and where a file ties it to the
    /// embedding table it is the same tensor read twice — so it is counted once
    /// here, as work done rather than as bytes held.
    multiplied: Option<u64>,
    dequantized: Option<u64>,
    loaded: Option<f64>,
    per_token: Option<u64>,
    projected: Option<u64>,
}

impl Rung {
    fn new(name: String) -> Self {
        Self {
            name,
            elements: None,
            multiplied: None,
            dequantized: None,
            loaded: None,
            per_token: None,
            projected: None,
        }
    }

    /// The columns that were filled, and then why the rest were not.
    fn refused(&self, why: &str) -> String {
        format!("{}\n{:>44}  {why}", self.render(), "")
    }

    fn render(&self) -> String {
        format!(
            "{:>44}{:>11}{:>12}{:>12}{:>9}{:>11}{:>12}",
            self.name,
            self.elements
                .and_then(|value| value.checked_div(1_000_000))
                .map_or_else(|| "-".to_owned(), |value| format!("{value}M")),
            self.multiplied
                .and_then(|value| value.checked_div(1_000_000))
                .map_or_else(|| "-".to_owned(), |value| format!("{value}M")),
            self.dequantized
                .and_then(|value| value.checked_div(1_000_000))
                .map_or_else(|| "-".to_owned(), |value| format!("{value}MB")),
            self.loaded
                .map_or_else(|| "-".to_owned(), |value| format!("{value:.1}s")),
            self.per_token
                .map_or_else(|| "-".to_owned(), |value| format!("{value} ms")),
            self.projected.map_or_else(|| "-".to_owned(), as_duration),
        )
    }
}

/// Milliseconds as something a person reads without counting zeros.
#[allow(clippy::integer_division)]
fn as_duration(millis: u64) -> String {
    if millis < 10_000 {
        return format!("{millis} ms");
    }
    if millis < 600_000 {
        return format!("{}.{} s", millis / 1000, (millis % 1000) / 100);
    }
    format!("{} min", millis / 60_000)
}

/// The last two path components, which is what tells two quantizations apart.
fn short(path: &str) -> String {
    let parts: Vec<&str> = path.rsplit('/').take(1).collect();
    parts.join("/")
}

/// What a forward pass actually asks the partition to do, counted rather than
/// timed.
///
/// **A count is not a timing and needs no quiet machine.** How many products a
/// token costs, and of what shapes, is a fact about the model's own dimensions;
/// it is what says whether the cost of starting workers is paid once per token
/// or two hundred times. Reading it from the shape rather than instrumenting
/// the pass keeps this honest about what it is: arithmetic on numbers the file
/// states, not an observation of a run.
fn shapes(path: &str) -> std::process::ExitCode {
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
    let model = match load(&file, &bytes) {
        Ok(model) => model,
        Err(failure) => {
            eprintln!("the model would not load: {failure}");
            return std::process::ExitCode::FAILURE;
        }
    };
    let shape = &model.shape;
    let width = shape.embedding;
    let head = shape.head_dimension();
    let queries = shape.query_width();
    let keys = shape.key_value_width();

    // Per block: q, k, v, o, gate, up, down. Plus one output projection a token.
    let per_block: [(&str, usize, usize); 7] = [
        ("attn_q", queries, width),
        ("attn_k", keys, width),
        ("attn_v", keys, width),
        ("attn_output", width, queries),
        ("ffn_gate", shape.feed_forward, width),
        ("ffn_up", shape.feed_forward, width),
        ("ffn_down", width, shape.feed_forward),
    ];

    println!("model    {path}");
    println!(
        "shape    {} block(s), width {width}, head {head}, feed-forward {}, vocabulary {}",
        shape.blocks, shape.feed_forward, shape.vocabulary
    );
    println!();
    println!("{:>16}{:>12}{:>16}", "product", "rows", "rows x columns");
    let mut per_token_products = 0_usize;
    for (name, rows, columns) in per_block {
        println!("{name:>16}{rows:>12}{:>16}", rows.saturating_mul(columns));
        per_token_products = per_token_products.saturating_add(shape.blocks);
    }
    println!(
        "{:>16}{:>12}{:>16}",
        "output",
        shape.vocabulary,
        shape.vocabulary.saturating_mul(width)
    );
    per_token_products = per_token_products.saturating_add(1);
    println!();
    println!(
        "{per_token_products} product(s) a token. Every one of them starts and joins its \
         workers, so a\npartition that costs anything to start pays that cost {per_token_products} \
         times per token."
    );
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
    print!("{:>16}{:>14}", "rows x columns", "before");
    for count in &counts {
        print!("{:>14}", format!("{count}t"));
    }
    println!("{:>14}", "best");
    println!("(median, and the middle half of the repeats beside it)");

    for (rows, columns) in SHAPES {
        let mut noise = Noise::seeded(u64::try_from(rows).unwrap_or(1));
        let matrix = noise.values(rows.saturating_mul(columns));
        let vector = noise.values(columns);

        // What the serial path cost *before* B-366 rewrote it: the same
        // arithmetic in a loop that pushes into a fresh vector, rather than
        // writing into one the partition allocated. A change that made the
        // one-thread path slower to make the many-thread path possible would be
        // a cost this table has to show rather than one it can leave out.
        let mut before = Vec::new();
        for _ in 0..repeats {
            let started = Instant::now();
            let produced = as_it_was_written(&matrix, &vector, rows, columns);
            before.push(u64::try_from(started.elapsed().as_micros()).unwrap_or(u64::MAX));
            if !same_bits(
                &produced,
                &mcf_standin::ops::matmul_vec(&matrix, &vector, rows, columns),
            ) {
                eprintln!("the rewritten serial path disagrees with the one it replaced");
                return std::process::ExitCode::FAILURE;
            }
        }

        let mut medians = Vec::new();
        let mut taken_per_count: Vec<Vec<u64>> = Vec::new();
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
            taken_per_count.push(taken);
        }

        print!(
            "{rows:>10} x{columns:>4}{:>9}u",
            median(&before).unwrap_or(0)
        );
        for row in &taken_per_count {
            print!("{:>14}", cell(row));
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

/// One cell: the median in microseconds, and the middle half beside it.
///
/// A6 wants no number without its spread, and a table of medians alone cannot
/// say whether the difference between two of its columns is a difference or the
/// machine breathing.
fn cell(values: &[u64]) -> String {
    let middle = median(values).unwrap_or(0);
    match middle_half_in_tenths(values) {
        Some(tenths) => format!("{middle}u +-{}%", tenths.div_ceil(10)),
        None => format!("{middle}u"),
    }
}

/// The serial matrix-vector product exactly as it was written before B-366.
///
/// Kept here rather than in the engine because it is not a second
/// implementation MCF ships — it is the *previous* one, so that the cost of the
/// rewrite is a measured number instead of an assurance. It must agree with the
/// current serial path bit for bit, which the sweep asserts on every cell.
fn as_it_was_written(matrix: &[f32], vector: &[f32], rows: usize, columns: usize) -> Vec<f32> {
    if vector.len() != columns || matrix.len() != rows.saturating_mul(columns) {
        return Vec::new();
    }
    let mut out = Vec::with_capacity(rows);
    for row in 0..rows {
        let start = row.saturating_mul(columns);
        let slice = matrix
            .get(start..start.saturating_add(columns))
            .unwrap_or(&[]);
        let mut total = 0.0_f32;
        for (weight, value) in slice.iter().zip(vector.iter()) {
            total = weight.mul_add(*value, total);
        }
        out.push(total);
    }
    out
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
