use std::path::Path;

use mcf_core::failure::Result;
use mcf_core::measurement::{Conditions, Measurement};
use mcf_core::time::{Clock as _, Duration, Monotonic, SystemClock, Timestamp};

use crate::journal::{Entry, EntryKind, Journal};
use crate::json::Value;

pub const TRIALS: usize = 100;

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

    let _removed = std::fs::remove_file(&path);

    Ok(Measurement::from_samples(samples, conditions))
}
