//! What this machine reads memory at, measured — the figure a predicted
//! fall-off divides by (B-427, F121).
//!
//! **Every generated token re-reads the cache**, and how big the cache is per
//! token of depth is arithmetic on the header. How fast it can be read is a
//! property of this machine, and not one number: a working set that fits a
//! cache level reads faster than one that streams from memory. The
//! prototype's sweep found a 36% and a 53% step at this machine's two L3
//! boundaries. So the read bandwidth is measured *at the working set the cache
//! takes* at each depth the ladder runs, rather than once at some size, and
//! each figure says what size it was taken at.
//!
//! **Taken inside the run, for the run.** B4 refuses ambient sampling; a
//! ladder is an experiment the operator started, and what the machine reads at
//! while it runs is a condition of that experiment (A6). It is taken before
//! the engine is up, so the two are not contending for the same memory bus,
//! and it is written down beside the rungs it belongs to rather than kept as
//! a machine-wide constant from some other day (A21).
//!
//! **What the loop does.** Every thread owns a slice of the working set, fills
//! it once outside the timing, and then sums it end to end as many times as
//! fit in the time allowed; the bytes every thread moved, over the wall time
//! they took together, is the aggregate rate. The sum is kept from being
//! optimised away, not from being vectorised — a vectorised read is what an
//! engine does. Best of three, because the figure asked for is what the
//! machine *can* read at, and a repeat that was interrupted is slower for a
//! reason that is not the machine's.

use std::hint::black_box;
use std::time::Instant;

use mcf_record::json::Value;

/// How long each repeat reads for, at least.
const READ_FOR_NS: u64 = 150_000_000;

/// How many repeats, of which the fastest stands.
const REPEATS: usize = 3;

/// The smallest slice one thread reads. Below it the loop is bound by its
/// own overhead rather than by memory, and the cell measures the harness.
const SMALLEST_SLICE: u64 = 1 << 20;

/// The largest working set measured. A cache larger than this streams from
/// memory as surely as one this size does, and the allocation is bounded.
pub const LARGEST_WORKING_SET: u64 = 1 << 30;

/// One measurement: what the machine read at, over what.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Reading {
    /// The depth whose cache this working set is the size of, where it was
    /// taken for a rung of the ladder.
    pub at_depth: Option<u64>,
    /// The working set read, in bytes — across every thread.
    pub working_set_bytes: u64,
    /// How many threads read at once.
    pub threads: u64,
    /// Bytes read a second, aggregate, on the fastest repeat.
    pub bytes_per_second: u64,
    /// How many times over the fastest repeat read its working set.
    pub passes: u64,
}

/// Reads a working set of about `working_set_bytes` and reports the rate.
///
/// The set is shared out among the threads the machine offers, no thread
/// taking under a mebibyte, and no set larger than [`LARGEST_WORKING_SET`].
/// `None` where nothing could be read in the time — a machine so contended
/// that not one pass finished.
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
        // In a wider word: tens of gigabytes moved, in nanoseconds, overflow
        // sixty-four bits before they are divided back down.
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

/// One thread's work: fill a slice, then sum it until the time is up.
/// Returns how many times over it was read.
fn read_a_slice(words: usize, seed: u64) -> u64 {
    // The fill is outside the timing and gives every page a first touch, so
    // the timed loop reads memory rather than faulting it in. Filling with a
    // pattern the sum depends on keeps the loop from being reduced to a
    // multiplication.
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
    /// The reading, as the wire and the record carry it.
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

    /// A reading back off the wire, where every figure is there.
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

/// The read bandwidth at the working set the cache takes at each depth of a
/// ladder — one reading a depth, each saying which.
///
/// Where the header does not size the cache there is no working set to take
/// it at, and the machine's figure is taken once at the largest set instead,
/// so that the machine is still described.
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

    /// A working set of a few mebibytes reads at a rate that is a rate:
    /// bounded below by what any machine that runs MCF reads at, and every
    /// thread took at least a mebibyte.
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

    /// The set asked for is bounded on both sides, and said as taken — and
    /// a reading survives the wire.
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
