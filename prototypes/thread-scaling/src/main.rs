use std::time::Instant;

use mcf_core::hardware::load_average;
use mcf_standin::llama::{Loaded, load};
use mcf_standin::sample::Settings;
use mcf_standin::session::{Request, generate};
use mcf_standin::threads::Threads;
use mcf_standin::{gguf, tokenizer::Vocabulary};

const COUNTS: [usize; 7] = [1, 2, 4, 8, 16, 24, 32];

const PROMPT: &str = "The capital of France is";

fn main() -> std::process::ExitCode {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    if arguments.first().is_some_and(|first| first == "ladder") {
        let rest = arguments.get(1..).unwrap_or_default();
        if rest.first().is_some_and(|first| first == "--threads") {
            let count = rest
                .get(1)
                .and_then(|value| value.parse().ok())
                .unwrap_or(1);
            return ladder(rest.get(2..).unwrap_or_default(), Threads::stated(count));
        }
        return ladder(rest, Threads::what_the_machine_reports());
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

fn ladder(paths: &[String], threads: Threads) -> std::process::ExitCode {
    const TOKENS: usize = 4;
    const CROSS_CHECK_POSITIONS: u64 = 120;

    if paths.is_empty() {
        eprintln!("usage: mcf-prototype-thread-scaling ladder <model.gguf>...");
        return std::process::ExitCode::FAILURE;
    }
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

fn one_rung(path: &str, threads: Threads, tokens: usize, positions: u64) -> String {
    let mut row = Rung::new(short(path));
    let Ok(bytes) = std::fs::read(path) else {
        return row.refused("the file could not be read");
    };
    let file = match gguf::parse(&bytes) {
        Ok(file) => file,
        Err(failure) => return row.refused(&failure.to_string()),
    };
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

struct Rung {
    name: String,
    elements: Option<u64>,
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

    fn refused(&self, why: &str) -> String {
        format!("{}\n{:>44}  {why}", self.render(), "")
    }

    fn render(&self) -> String {
        format!(
            "{:>44}{:>11}{:>12}{:>12}{:>8}{:>11}{:>12}{:>12}",
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
            match (self.per_token, self.multiplied) {
                (Some(each), Some(work)) => each
                    .saturating_mul(1000)
                    .checked_div(work.checked_div(1_000_000).unwrap_or(1).max(1))
                    .map_or_else(|| "-".to_owned(), |rate| format!("{rate}us")),
                _ => "-".to_owned(),
            },
        )
    }
}

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

fn short(path: &str) -> String {
    let parts: Vec<&str> = path.rsplit('/').take(1).collect();
    parts.join("/")
}

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

#[allow(clippy::integer_division)]
fn products(repeats: usize) -> std::process::ExitCode {
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

fn cell(values: &[u64]) -> String {
    let middle = median(values).unwrap_or(0);
    match middle_half_in_tenths(values) {
        Some(tenths) => format!("{middle}u +-{}%", tenths.div_ceil(10)),
        None => format!("{middle}u"),
    }
}

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

fn same_bits(left: &[f32], right: &[f32]) -> bool {
    left.len() == right.len()
        && left
            .iter()
            .zip(right.iter())
            .all(|(one, other)| one.to_bits() == other.to_bits())
}

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

fn said_load() -> String {
    load_average().known().map_or_else(
        || "unknown — this machine does not publish one".to_owned(),
        |average| format!("{} (thousandths of a core)", average.0),
    )
}

struct Measured {
    durations: Vec<Vec<u64>>,
    answers: Vec<Option<Vec<usize>>>,
}

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

fn median(values: &[u64]) -> Option<u64> {
    if values.is_empty() {
        return None;
    }
    let mut sorted = values.to_vec();
    sorted.sort_unstable();
    sorted.get(sorted.len().checked_div(2)?).copied()
}

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
