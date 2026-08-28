//! What this machine has already measured, read back out of the record
//! (B-214, PR3, B34, D20).
//!
//! **Every point is a comparison MCF took and wrote down.** B34 lets the corpus
//! advise and never decide, and there is no corpus; a projection from local
//! history means *from this record*, and this is the one place that reads it.
//!
//! **A point per arm, not per comparison.** A comparison is two arms, each of
//! which was run and timed, and each is an independent thing this machine knows
//! about a file of that size at that budget. Reading only one would throw away
//! half the history for no reason.
//!
//! **What is skipped is counted.** A comparison whose file is no longer on the
//! disk has no size, and a size read from somewhere else would be a size about
//! another file (A7). Those points are dropped and the count comes back with
//! the history, because *fourteen points* and *fourteen points and eleven I
//! could not use* are different claims about the same record (A1).

use std::path::Path;

use mcf_bench::project::Point;
use mcf_record::journal::EntryKind;
use mcf_record::journal::index::{self, Index};
use mcf_record::json::Value;

/// Everything this machine has measured, and what could not be read.
#[derive(Debug, Default)]
pub(crate) struct History {
    /// The points, in no particular order.
    pub(crate) points: Vec<Point>,
    /// How many arms were skipped because their file is not here now.
    pub(crate) unreadable: usize,
}

/// Reads the record for what this machine has measured.
///
/// An empty history is the honest answer on a machine that has measured
/// nothing, and is not distinguished from a record that could not be opened —
/// which it should be. That distinction is B-217's, when a surface needs it;
/// what matters here is that no point is invented.
pub(crate) fn read() -> History {
    let mut held = History::default();
    let Some(journal) = mcf_record::journal::default_path() else {
        return held;
    };
    let Ok(index) = Index::over(&journal, &index::default_path(&journal)) else {
        return held;
    };
    let wanted = Some(EntryKind::Comparison);
    for located in index.latest(wanted, index.count_matching(wanted)) {
        let Ok(entry) = index.read(&located) else {
            continue;
        };
        let body = entry.body();
        let Some(tokens) = body
            .get("discipline")
            .and_then(|held| held.get("tokens_pinned"))
            .and_then(Value::as_integer)
            .and_then(|held| u32::try_from(held).ok())
        else {
            continue;
        };
        // A run whose trials were not alike is not one measurement (§6.13), so
        // it is not a point either: projecting from it would carry the mixture
        // into the projection.
        if body
            .get("reuse")
            .and_then(Value::as_text)
            .is_some_and(|held| held.starts_with("MIXED"))
        {
            continue;
        }
        // B-385: what else the machine was doing while this was measured, so
        // that a band read between two points can say what it rested on. The
        // larger of the two readings, because a projection should inherit the
        // worse of the conditions rather than the flattering one.
        let competing = [
            "competing_before_thousandths",
            "competing_after_thousandths",
        ]
        .iter()
        .filter_map(|named| {
            body.get("machine")
                .and_then(|held| held.get(named))
                .and_then(Value::as_integer)
                .and_then(|held| u64::try_from(held).ok())
        })
        .max();
        for (side, take) in [("left", "left_ns"), ("right", "right_ns")] {
            match point(body, side, take, tokens, competing) {
                Some(one) => held.points.push(one),
                None => held.unreadable = held.unreadable.saturating_add(1),
            }
        }
    }
    held
}

/// One arm of one comparison, as a point.
fn point(
    body: &Value,
    side: &str,
    take: &str,
    tokens: u32,
    competing: mcf_bench::project::Competing,
) -> Option<Point> {
    let named = body
        .get(side)
        .and_then(|arm| arm.get("arm"))
        .and_then(Value::as_text)?;
    let pairs = body.get("pairs").and_then(Value::as_list)?;
    let mut seen: Vec<u64> = pairs
        .iter()
        .filter_map(|pair| pair.get(take).and_then(Value::as_integer))
        .filter_map(|held| u64::try_from(held).ok())
        .collect();
    if seen.is_empty() {
        return None;
    }
    seen.sort_unstable();
    // The size is read from the file as it is now. A size remembered from the
    // record would be a size about the file as it was, and the two are the same
    // question only until somebody replaces it (A21, §3.6).
    let meta = std::fs::metadata(Path::new(named)).ok()?;
    Some(Point {
        bytes: meta.len(),
        tokens,
        fastest: seen.first().copied().unwrap_or(0),
        slowest: seen.last().copied().unwrap_or(0),
        competing,
    })
}
