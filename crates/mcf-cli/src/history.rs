use std::path::Path;

use mcf_bench::project::Point;
use mcf_record::journal::EntryKind;
use mcf_record::journal::index::{self, Index};
use mcf_record::json::Value;

#[derive(Debug, Default)]
pub(crate) struct History {
    pub(crate) points: Vec<Point>,
    pub(crate) unreadable: usize,
}

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
        if body
            .get("reuse")
            .and_then(Value::as_text)
            .is_some_and(|held| held.starts_with("MIXED"))
        {
            continue;
        }
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
    let meta = std::fs::metadata(Path::new(named)).ok()?;
    Some(Point {
        bytes: meta.len(),
        tokens,
        fastest: seen.first().copied().unwrap_or(0),
        slowest: seen.last().copied().unwrap_or(0),
        competing,
    })
}

pub(crate) fn probed_context(model: &Path) -> Option<usize> {
    let journal = mcf_record::journal::default_path()?;
    let index = Index::over(&journal, &index::default_path(&journal)).ok()?;
    let wanted = Some(EntryKind::ModelProbed);
    for located in index.latest(wanted, index.count_matching(wanted)) {
        let Ok(entry) = index.read(&located) else {
            continue;
        };
        let body = entry.body();
        if body.get("model").and_then(Value::as_text) != Some(&model.display().to_string()) {
            continue;
        }
        if body.get("method").and_then(Value::as_text)
            != Some(mcf_serve::probes::USABLE_CONTEXT.name)
        {
            continue;
        }
        if let Some(accepted) = body
            .get("accepted_tokens")
            .and_then(Value::as_integer)
            .and_then(|held| usize::try_from(held).ok())
        {
            return Some(accepted);
        }
    }
    None
}
