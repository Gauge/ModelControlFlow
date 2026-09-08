use std::time::Instant;

use mcf_core::attested::Attested;
use mcf_core::build_identity::BuildIdentity;
use mcf_core::hardware::{Attributability, Machine, Watch, storage_of};
use mcf_core::measurement::{ConditionValue, Conditions, Floor, Measurement};
use mcf_core::time::{Duration, Monotonic};

const N: usize = 512;

const TRIALS: usize = 5;

const BLOCK: usize = 64;

fn main() -> std::process::ExitCode {
    let machine = Machine::read();
    println!("{}", BuildIdentity::current());
    println!("{machine}\n");

    if BuildIdentity::current().profile != "release" {
        println!(
            "NOTHING HERE IS A NUMBER ABOUT ANYTHING in this profile: a debug build is a\n\
             different artifact (§3.4), and the point of this prototype is a ratio between\n\
             optimized variants. Run it with --release.\n"
        );
    }

    let left = fill(0x51A7_1234_5678_9ABC);
    let right = fill(0x1234_5678_9ABC_DEF0);

    let reference = naive(&left, &right);

    let mut results: Vec<(&str, Measurement<Duration<Monotonic>>)> = Vec::new();
    for (name, kernel) in [
        (
            "naive (the definition)",
            naive as fn(&[f32], &[f32]) -> Vec<f32>,
        ),
        ("reordered (one line)", reordered),
        ("blocked (careful, safe)", blocked),
        ("blocked and threaded", threaded),
    ] {
        match measure(name, kernel, &left, &right, &reference, &machine) {
            Some(measured) => results.push((name, measured)),
            None => println!("  {name}: refused — see above"),
        }
    }

    report(&results);
    reference_kernel();
    std::process::ExitCode::SUCCESS
}

fn reference_kernel() {
    let program = format!(
        "import numpy, time\n\
         a = numpy.random.rand({N}, {N}).astype(numpy.float32)\n\
         b = numpy.random.rand({N}, {N}).astype(numpy.float32)\n\
         for _ in range(3): a @ b\n\
         t = []\n\
         for _ in range(9):\n\
         \x20   s = time.perf_counter_ns(); c = a @ b; t.append(time.perf_counter_ns() - s)\n\
         t.sort()\n\
         print(t[4], t[0])\n"
    );
    let outcome = std::process::Command::new("python3")
        .arg("-c")
        .arg(&program)
        .env("OPENBLAS_NUM_THREADS", "1")
        .env("OMP_NUM_THREADS", "1")
        .env("MKL_NUM_THREADS", "1")
        .output();

    let Ok(output) = outcome else {
        println!("\n  no python3 here, so there is no specialist's kernel to compare with");
        return;
    };
    if !output.status.success() {
        println!(
            "\n  this machine has no tuned BLAS installed to compare with, so the other end\n\
             \x20 of the slope is not measured here (A7: unknown, not assumed)"
        );
        return;
    }
    let text = String::from_utf8_lossy(&output.stdout);
    let mut figures = text
        .split_whitespace()
        .filter_map(|word| word.parse::<u64>().ok());
    let (Some(median), Some(minimum)) = (figures.next(), figures.next()) else {
        println!("\n  the reference kernel produced no reading this could read");
        return;
    };
    println!(
        "\n  a specialist's kernel, on this machine, single-threaded (OpenBLAS through\n\
         \x20 numpy): median {median} ns   min {minimum} ns"
    );
    println!("\x20 That is the distance §7.4 is about, and it is measured rather than assumed.");
}

fn measure(
    name: &str,
    kernel: fn(&[f32], &[f32]) -> Vec<f32>,
    left: &[f32],
    right: &[f32],
    reference: &[f32],
    machine: &Machine,
) -> Option<Measurement<Duration<Monotonic>>> {
    let watch = Watch::start();
    let mut samples = Vec::with_capacity(TRIALS);
    for _ in 0..TRIALS {
        let started = Instant::now();
        let product = kernel(left, right);
        let elapsed = started.elapsed();
        if !agrees(&product, reference) {
            println!("  {name}: DISAGREES with the definition, so its speed means nothing");
            return None;
        }
        samples.push(Duration::<Monotonic>::from_nanos(
            u64::try_from(elapsed.as_nanos()).unwrap_or(u64::MAX),
        ));
    }
    let attributability = watch.finish();
    println!("  {name}: {attributability}");
    Measurement::from_samples(samples, conditions(machine, &attributability))
}

fn conditions(machine: &Machine, attributability: &Attributability) -> Conditions {
    let binary = std::env::current_exe().ok();
    Conditions::new(
        BuildIdentity::current(),
        Floor {
            hardware_state: Attested::Known(ConditionValue::text(format!(
                "{} · {attributability}",
                machine.processor.model
            ))),
            mcf_configuration: Attested::Known(ConditionValue::text(format!(
                "the §7.4 kernel-slope prototype, {N}×{N}, block {BLOCK}, {TRIALS} trials"
            ))),
            artifact_storage: binary
                .as_deref()
                .map_or(Attested::Unknown, |path| match storage_of(path) {
                    Attested::Known(storage) => {
                        Attested::Known(ConditionValue::text(storage.to_string()))
                    }
                    Attested::Unknown => Attested::Unknown,
                }),
            ..Floor::nothing_known()
        },
    )
}

fn report(results: &[(&str, Measurement<Duration<Monotonic>>)]) {
    println!("\n{N}×{N} single-precision matrix multiply, {TRIALS} trials each\n");
    let Some((_, slowest)) = results.first() else {
        println!("nothing ran");
        return;
    };
    let baseline = slowest.spread().median.as_nanos();
    for (name, measured) in results {
        let spread = measured.spread();
        let median = spread.median.as_nanos();
        let times = ratio(baseline, median);
        println!(
            "  {name:<26} median {:>12} ns   min {:>12} ns   ×{}.{} against the definition",
            median,
            spread.minimum.as_nanos(),
            times.checked_div(10).unwrap_or(0),
            times % 10
        );
    }
    println!(
        "\n  Every variant above is safe, portable Rust MCF could maintain. What is not\n\
         \x20 above — SIMD microkernels per instruction set, packing, prefetch, the\n\
         \x20 accelerator path — needs per-architecture unsafe code and moves with every\n\
         \x20 new processor. That distance is what §7.4 is asking whether MCF should own."
    );
}

fn ratio(baseline: u64, other: u64) -> u64 {
    baseline
        .saturating_mul(10)
        .checked_div(other.max(1))
        .unwrap_or(0)
}

fn fill(seed: u64) -> Vec<f32> {
    let mut state = seed;
    (0..N * N)
        .map(|_| {
            state = state
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1);
            let bits = i32::try_from((state >> 40) & 0x1FF).unwrap_or(0);
            f32::from(i16::try_from(bits - 256).unwrap_or(0)) / 256.0
        })
        .collect()
}

fn agrees(product: &[f32], reference: &[f32]) -> bool {
    product.len() == reference.len()
        && product
            .iter()
            .zip(reference.iter())
            .all(|(one, other)| one.to_bits() == other.to_bits())
}

fn naive(left: &[f32], right: &[f32]) -> Vec<f32> {
    let mut out = vec![0.0_f32; N * N];
    for row in 0..N {
        for column in 0..N {
            let mut total = 0.0_f32;
            for inner in 0..N {
                total += get(left, row, inner) * get(right, inner, column);
            }
            set(&mut out, row, column, total);
        }
    }
    out
}

fn reordered(left: &[f32], right: &[f32]) -> Vec<f32> {
    let mut out = vec![0.0_f32; N * N];
    for row in 0..N {
        for inner in 0..N {
            let scale = get(left, row, inner);
            for column in 0..N {
                let addend = scale * get(right, inner, column);
                let at = row * N + column;
                if let Some(slot) = out.get_mut(at) {
                    *slot += addend;
                }
            }
        }
    }
    out
}

fn blocked(left: &[f32], right: &[f32]) -> Vec<f32> {
    let mut out = vec![0.0_f32; N * N];
    block_range(&mut out, left, right, 0, N);
    out
}

fn block_range(out: &mut [f32], left: &[f32], right: &[f32], from: usize, to: usize) {
    let mut row_block = from;
    while row_block < to {
        let row_end = (row_block + BLOCK).min(to);
        let mut inner_block = 0;
        while inner_block < N {
            let inner_end = (inner_block + BLOCK).min(N);
            let mut column_block = 0;
            while column_block < N {
                let column_end = (column_block + BLOCK).min(N);
                for row in row_block..row_end {
                    for inner in inner_block..inner_end {
                        let scale = get(left, row, inner);
                        for column in column_block..column_end {
                            let addend = scale * get(right, inner, column);
                            let at = (row - from) * N + column;
                            if let Some(slot) = out.get_mut(at) {
                                *slot += addend;
                            }
                        }
                    }
                }
                column_block += BLOCK;
            }
            inner_block += BLOCK;
        }
        row_block += BLOCK;
    }
}

fn threaded(left: &[f32], right: &[f32]) -> Vec<f32> {
    let workers = std::thread::available_parallelism().map_or(4, std::num::NonZero::get);
    let stripe = N.div_ceil(workers);
    let mut out = vec![0.0_f32; N * N];

    std::thread::scope(|scope| {
        let mut remaining: &mut [f32] = &mut out;
        let mut from = 0;
        while from < N {
            let to = (from + stripe).min(N);
            let rows = (to - from) * N;
            let (mine, rest) = remaining.split_at_mut(rows.min(remaining.len()));
            remaining = rest;
            let _worker = scope.spawn(move || block_range(mine, left, right, from, to));
            from = to;
        }
    });
    out
}

fn get(matrix: &[f32], row: usize, column: usize) -> f32 {
    matrix.get(row * N + column).copied().unwrap_or(0.0)
}

fn set(matrix: &mut [f32], row: usize, column: usize, value: f32) {
    if let Some(slot) = matrix.get_mut(row * N + column) {
        *slot = value;
    }
}
