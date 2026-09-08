use mcf_record::json::Value;

use super::retrieval::{FILLER, planted};
use super::{Found, Reading, Site, as_integer, framed_ids, whole};
use crate::generation::Draw;
use crate::served::{Prompt, Startup};

pub const NAME: &str = "retrieval-deep";

pub const DEPTHS: [usize; 3] = [32_768, 65_536, 131_072];

const PLACEMENTS: [usize; 2] = [10, 90];

const ROOM: usize = 320;

const BUDGET: usize = 120;

#[must_use]
#[allow(
    clippy::too_many_lines,
    reason = "one measurement read straight through: each depth, each placement a row set, or why not"
)]
pub fn measure(site: &Site<'_>) -> Found {
    let deepest = DEPTHS.iter().copied().max().unwrap_or(0);
    let wanted =
        u64::try_from(deepest.saturating_add(ROOM).saturating_add(BUDGET)).unwrap_or(u64::MAX);
    let engine = match site.server(&Startup {
        projector: None,
        context: site.context.min(wanted),
        ..site.startup()
    }) {
        Ok(engine) => engine,
        Err(why) => return Found::could_not_tell(&why),
    };
    let unit = match engine.tokenize(FILLER, false, false) {
        Ok(tokens) => tokens.len().max(1),
        Err(failure) => return Found::could_not_tell(failure.detail()),
    };
    let window = usize::try_from(engine.window).unwrap_or(usize::MAX);
    let digits = {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |since| since.as_nanos());
        (u64::try_from(nanos % 900_000).unwrap_or(0) + 100_000).to_string()
    };
    let mut rows = Vec::new();
    let mut lines = vec![format!(
        "  a six-digit number made for this run, planted at {}% and {}% of the filler at each \
         depth; the window is {window} tokens",
        PLACEMENTS[0], PLACEMENTS[1]
    )];
    let (mut found, mut asked, mut past) = (0_usize, 0_usize, 0_usize);
    for (at, depth) in DEPTHS.into_iter().enumerate() {
        site.progress(at, DEPTHS.len(), &format!("depth {depth}"));
        if depth.saturating_add(ROOM) > window {
            past = past.saturating_add(1);
            rows.push(Reading::new(
                &[("depth", Value::Integer(as_integer(depth)))],
                "past_window",
                1,
                "bool",
            ));
            lines.push(format!("  depth {depth:>7}   not asked: past the window"));
            continue;
        }
        #[expect(
            clippy::integer_division,
            reason = "how many repeats of the filler reach the depth"
        )]
        let repeats = (depth.saturating_sub(ROOM) / unit).max(1);
        let mut said = Vec::new();
        for placement in PLACEMENTS {
            if site.asker_gone() {
                return Found::could_not_tell(crate::served::CLIENT_LEFT);
            }
            let text = planted(repeats, placement, &digits);
            let ids = match framed_ids(&engine, &text) {
                Ok(ids) => ids,
                Err(why) => return Found::could_not_tell(&why),
            };
            let began = std::time::Instant::now();
            let completed = match engine.complete(
                Prompt::Identifiers(&ids),
                BUDGET,
                Draw::greedy(0),
                false,
                site.waiting,
            ) {
                Ok(completed) => completed,
                Err(failure) => return Found::could_not_tell(failure.detail()),
            };
            let ns = u64::try_from(began.elapsed().as_nanos()).unwrap_or(u64::MAX);
            let got = completed.text.contains(&digits);
            asked = asked.saturating_add(1);
            found = found.saturating_add(usize::from(got));
            let dims = [
                ("depth", Value::Integer(as_integer(depth))),
                ("placement", Value::Integer(as_integer(placement))),
            ];
            rows.push(Reading::new(&dims, "found", i64::from(got), "bool"));
            rows.push(Reading::new(
                &dims,
                "prompt_tokens",
                as_integer(ids.len()),
                "tokens",
            ));
            rows.push(Reading::new(
                &dims,
                "produced",
                as_integer(completed.predicted),
                "tokens",
            ));
            rows.push(Reading::new(
                &dims,
                "ns",
                i64::try_from(ns).unwrap_or(i64::MAX),
                "ns",
            ));
            said.push(format!(
                "at {placement}% {}",
                if got { "found" } else { "MISSED" }
            ));
        }
        lines.push(format!("  depth {depth:>7}   {}", said.join("   ")));
    }
    if asked == 0 {
        lines.push(
            "  no depth fits the window; the retrieval measurement covers what does".to_owned(),
        );
    } else {
        lines.push(format!("  found in {found} of {asked} placement(s)"));
    }
    Found {
        lines,
        fields: vec![
            ("window", whole(engine.window)),
            ("asked", Value::Integer(as_integer(asked))),
            ("found", Value::Integer(as_integer(found))),
            ("past_window", Value::Integer(as_integer(past))),
        ],
        rows,
    }
}
