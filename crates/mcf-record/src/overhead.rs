//! What MCF's own observation costs.
//!
//! §3.8: *MCF is part of the apparatus it measures. Its own resource
//! consumption contaminates its own benchmarks.* B3 draws the consequence —
//! *MCF's own overhead is measured and travels as a condition; an
//! uncharacterized instrument is not a scientific one* — and B-012 is the item.
//!
//! **At M0, MCF's observation is the record write.** There is no serving path
//! to instrument and no laboratory telemetry to reduce, so the whole of what
//! MCF does in order to observe is append a line and wait for it to reach the
//! medium. That is the cost measured here, and D24 budgets it at two
//! milliseconds per event.
//!
//! **The reduced-instrumentation path is not writing at all**, which is why
//! `mcf doctor --no-record` exists as more than a convenience: it is the other
//! arm of B3's comparison, and the delta between them is the number this
//! module produces. When there is more instrumentation to reduce — a lab's
//! per-token timings (B31), a serving path's counters — this is the shape that
//! measures those too.
//!
//! **It is measured where the record actually lives**, not in a temporary
//! directory. A durability barrier on a fast local disk and one on a network
//! filesystem are different costs, and the second is the one that would
//! surprise somebody.

use std::path::Path;

use mcf_core::failure::Result;
use mcf_core::measurement::{Conditions, Measurement};
use mcf_core::time::{Clock as _, Duration, Monotonic, SystemClock, Timestamp};

use crate::journal::{Entry, EntryKind, Journal};
use crate::json::Value;

/// How many appends are timed.
///
/// D27 makes an event-class figure a 99th percentile over at least a hundred
/// trials, and the record write is an event-class figure. A hundred appends of
/// a small line is a fraction of a second even with a barrier on each.
pub const TRIALS: usize = 100;

/// Measures what one recorded event costs, beside the real record.
///
/// The journal written is a sibling of the operator's, in the same directory,
/// so the measurement sees the same filesystem — and it is removed afterwards
/// (A27). Nothing is written to the record itself: measuring the cost of
/// observation must not itself be an observation, or the number would include
/// its own effect.
///
/// # Errors
///
/// Whatever opening or appending to a journal fails with, unchanged: this is a
/// measurement of MCF's cost, and a machine that cannot take the measurement
/// says why rather than reporting a number it did not take.
pub fn record_write_cost(
    beside: &Path,
    conditions: Conditions,
) -> Result<Option<Measurement<Duration<Monotonic>>>> {
    let path = beside.with_file_name("record.overhead-probe.jsonl");
    let clock = SystemClock;
    let mut samples = Vec::with_capacity(TRIALS);

    {
        let mut journal = Journal::open(&path)?;
        for sequence in 0..u64::try_from(TRIALS).unwrap_or(0) {
            let entry = Entry::new(
                EntryKind::SelfCost,
                Timestamp::from_utc_nanos(0, mcf_core::attested::Attested::Unknown),
                Value::map([(
                    "probe",
                    Value::Integer(i64::try_from(sequence).unwrap_or(i64::MAX)),
                )]),
            );
            let started = clock.now();
            journal.append(&entry)?;
            samples.push(clock.now().saturating_duration_since(started));
        }
    }

    // A27: what this created, it removes. A failure to remove is not a failure
    // of the measurement — the number is taken — so it is not propagated, and
    // the file it leaves is named for what it is.
    let _removed = std::fs::remove_file(&path);

    Ok(Measurement::from_samples(samples, conditions))
}
