use std::hint::black_box;
use std::time::Instant;

use mcf_record::json::Value;

const READ_FOR_NS: u64 = 150_000_000;

const REPEATS: usize = 3;

const SMALLEST_SLICE: u64 = 1 << 20;

pub const LARGEST_WORKING_SET: u64 = 1 << 30;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Reading {
    pub at_depth: Option<u64>,
    pub working_set_bytes: u64,
    pub threads: u64,
    pub bytes_per_second: u64,
    pub passes: u64,
}

#[must_use]
pub fn read_bandwidth(working_set_bytes: u64) -> Option<Reading> {
    let working_set = working_set_bytes.clamp(SMALLEST_SLICE, LARGEST_WORKING_SET);
    let offered = std::thread::available_parallelism().map_or(1, std::num::NonZero::get);
    let threads = u64::try_from(offered)
        .unwrap_or(1)
        .min(working_set.checked_div(SMALLEST_SLICE).unwrap_or(1))
        .max(1);
    let words = usize::try_from(working_set.checked_div(threads)?.checked_div(8)?).ok()?;
    let slice_bytes = u64::try_from(words).ok()?.saturating_mul(8);
    let mut best: Option<(u64, u64)> = None;
    for _ in 0..REPEATS {
        let started = Instant::now();
        let passes: Vec<u64> = std::thread::scope(|scope| {
            let readers: Vec<_> = (0..threads)
                .map(|thread| scope.spawn(move || read_a_slice(words, thread)))
                .collect();
            readers
                .into_iter()
                .map(|reader| reader.join().unwrap_or(0))
                .collect()
        });
        let took = u64::try_from(started.elapsed().as_nanos()).unwrap_or(u64::MAX);
        let moved = passes
            .iter()
            .fold(0_u64, |sum, passes| sum.saturating_add(*passes))
            .saturating_mul(slice_bytes);
        let rate = u128::from(moved)
            .saturating_mul(1_000_000_000)
            .checked_div(u128::from(took))
            .and_then(|rate| u64::try_from(rate).ok())
            .unwrap_or(0);
        let fewest = passes.iter().copied().min().unwrap_or(0);
        if rate > 0 && best.is_none_or(|(held, _)| rate > held) {
            best = Some((rate, fewest));
        }
    }
    let (bytes_per_second, passes) = best?;
    Some(Reading {
        at_depth: None,
        working_set_bytes: slice_bytes.saturating_mul(threads),
        threads,
        bytes_per_second,
        passes,
    })
}

fn read_a_slice(words: usize, seed: u64) -> u64 {
    let slice: Vec<u64> = (0..words)
        .map(|index| {
            u64::try_from(index)
                .unwrap_or(0)
                .wrapping_mul(seed.wrapping_add(1))
        })
        .collect();
    let started = Instant::now();
    let mut passes = 0_u64;
    while u64::try_from(started.elapsed().as_nanos()).unwrap_or(u64::MAX) < READ_FOR_NS {
        let sum = slice
            .iter()
            .fold(0_u64, |sum, word| sum.wrapping_add(*word));
        black_box(sum);
        passes = passes.saturating_add(1);
    }
    passes
}

impl Reading {
    #[must_use]
    pub fn as_value(&self) -> Value {
        let count = |held: u64| Value::Integer(i64::try_from(held).unwrap_or(i64::MAX));
        Value::map([
            ("at_depth", self.at_depth.map_or(Value::Null, count)),
            ("working_set_bytes", count(self.working_set_bytes)),
            ("threads", count(self.threads)),
            ("bytes_per_second", count(self.bytes_per_second)),
            ("passes", count(self.passes)),
        ])
    }

    #[must_use]
    pub fn from_value(held: &Value) -> Option<Self> {
        let number = |key: &str| {
            held.get(key)?
                .as_integer()
                .and_then(|value| u64::try_from(value).ok())
        };
        Some(Self {
            at_depth: number("at_depth"),
            working_set_bytes: number("working_set_bytes")?,
            threads: number("threads")?,
            bytes_per_second: number("bytes_per_second")?,
            passes: number("passes")?,
        })
    }
}

#[must_use]
pub fn along_a_ladder(depths: &[u64], cache_per_token: Option<u64>) -> Vec<Reading> {
    match cache_per_token {
        Some(per_token) if per_token > 0 => depths
            .iter()
            .filter_map(|depth| {
                let mut reading = read_bandwidth(depth.saturating_mul(per_token))?;
                reading.at_depth = Some(*depth);
                Some(reading)
            })
            .collect(),
        _ => read_bandwidth(LARGEST_WORKING_SET).into_iter().collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_working_set_reads_at_a_rate() {
        let reading = read_bandwidth(8 << 20).unwrap_or_else(|| panic!("nothing read"));
        assert!(reading.bytes_per_second > 100_000_000, "{reading:?}");
        assert!(reading.passes > 0, "{reading:?}");
        assert!(reading.threads >= 1 && reading.threads <= 8, "{reading:?}");
        assert!(
            reading.working_set_bytes >= reading.threads * SMALLEST_SLICE,
            "{reading:?}"
        );
    }

    #[test]
    fn the_working_set_is_bounded_and_said() {
        let small = read_bandwidth(1).unwrap_or_else(|| panic!("nothing read"));
        assert_eq!(small.threads, 1);
        assert_eq!(small.working_set_bytes, SMALLEST_SLICE);
        let reading = Reading {
            at_depth: Some(512),
            ..small
        };
        assert_eq!(Reading::from_value(&reading.as_value()), Some(reading));
        assert_eq!(Reading::from_value(&Value::map::<&str>([])), None);
    }
}
