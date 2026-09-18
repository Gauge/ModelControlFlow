//! How fast MCF hashes what it holds.
//!
//! Ignored: these are measurements, not claims. `MCF_DIGEST_FILE` points at one file,
//! `MCF_DIGEST_DIR` at a directory of them. Whole numbers throughout, because this crate
//! holds no floats: microseconds and bytes a microsecond, which is megabytes a second.

use std::path::{Path, PathBuf};

/// Bytes a microsecond is megabytes a second, near enough to state a rate in.
#[expect(
    clippy::integer_division,
    reason = "a whole rate; the remainder is noise"
)]
fn rate(bytes: u64, micros: u128) -> u64 {
    u64::try_from(u128::from(bytes) / micros.max(1)).unwrap_or(u64::MAX)
}

/// Microseconds, as seconds and thousandths.
#[expect(clippy::integer_division, reason = "seconds and thousandths, exactly")]
fn said(micros: u128) -> String {
    format!("{}.{:03}s", micros / 1_000_000, (micros / 1_000) % 1_000)
}

/// One span as a percentage of another.
#[expect(clippy::integer_division, reason = "a whole percentage")]
fn share(held: u128, of: u128) -> u128 {
    held.saturating_mul(100) / of.max(1)
}

fn size_of(path: &Path) -> u64 {
    std::fs::metadata(path).map_or(0, |held| held.len())
}

#[test]
#[ignore = "a measurement; needs MCF_DIGEST_FILE"]
fn how_fast_the_digest_is() {
    let Ok(path) = std::env::var("MCF_DIGEST_FILE") else {
        println!("set MCF_DIGEST_FILE to a file to hash");
        return;
    };
    let path = PathBuf::from(path);
    let bytes = size_of(&path);
    for pass in 0..3 {
        let at = std::time::Instant::now();
        let digest = mcf_core::integrity::digest_of(&path).expect("hashed");
        let micros = at.elapsed().as_micros();
        println!(
            "pass {pass}: {} for {bytes} bytes = {} MB/s ({})",
            said(micros),
            rate(bytes, micros),
            digest.hex().get(..16).unwrap_or_default()
        );
    }
}

/// One artifact at a time against several at a time, over a directory of them: what a
/// check of a whole library costs either way.
#[test]
#[ignore = "a measurement; needs MCF_DIGEST_DIR"]
fn how_fast_a_library_is() {
    let Ok(root) = std::env::var("MCF_DIGEST_DIR") else {
        println!("set MCF_DIGEST_DIR to a directory of artifacts");
        return;
    };
    let held = artifacts_under(&PathBuf::from(&root));
    let bytes: u64 = held.iter().map(|path| size_of(path)).sum();
    println!("{} artifact(s), {bytes} bytes", held.len());

    let at = std::time::Instant::now();
    for path in &held {
        let _digest = mcf_core::integrity::digest_of(path).expect("hashed");
    }
    let one_at_a_time = at.elapsed().as_micros();
    println!(
        "one at a time: {} = {} MB/s",
        said(one_at_a_time),
        rate(bytes, one_at_a_time)
    );

    let threads = std::thread::available_parallelism()
        .map_or(1, std::num::NonZero::get)
        .min(8)
        .min(held.len())
        .max(1);
    let next = std::sync::atomic::AtomicUsize::new(0);
    let at = std::time::Instant::now();
    std::thread::scope(|scope| {
        for _ in 0..threads {
            let _hashing = scope.spawn(|| {
                loop {
                    let mine = next.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    let Some(path) = held.get(mine) else {
                        return;
                    };
                    let _digest = mcf_core::integrity::digest_of(path).expect("hashed");
                }
            });
        }
    });
    let together = at.elapsed().as_micros();
    println!(
        "{threads} at a time: {} = {} MB/s, {}% of the time one at a time took",
        said(together),
        rate(bytes, together),
        share(together, one_at_a_time)
    );
}

fn artifacts_under(root: &Path) -> Vec<PathBuf> {
    let mut held = Vec::new();
    let mut looking = vec![root.to_path_buf()];
    while let Some(at) = looking.pop() {
        let Ok(entries) = std::fs::read_dir(&at) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                looking.push(path);
            } else if path.extension().is_some_and(|held| held == "gguf") {
                held.push(path);
            }
        }
    }
    held.sort();
    held
}
