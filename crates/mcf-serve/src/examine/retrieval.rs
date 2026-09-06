//! Retrieval by depth: whether a fact planted in a long prompt comes back
//! (B-497, D52, B-055).
//!
//! The usable-context probe says a prompt of some length is *accepted*.
//! Whether the model then uses what is in it is a different question, and
//! this puts it mechanically: a random run of digits — made fresh each
//! run, so nothing is remembered from training — is planted at a stated
//! fraction of a stated length of filler, the model is asked for it
//! through its own template, and the answer either carries those digits
//! or does not. Exact match, no rater. What comes back is a grid: depth
//! by placement, found or missed.

use mcf_record::json::Value;

use super::{Found, Site, as_integer, framed_ids, whole};
use crate::generation::Draw;
use crate::served::{Prompt, Startup};

/// The measurement's name.
pub const NAME: &str = "retrieval-by-depth";

/// The depths tried, in tokens; each is capped at the server's window
/// less room for the question and the answer.
const DEPTHS: [usize; 3] = [1024, 4096, 16384];

/// Where the fact is planted, as hundredths of the filler.
const PLACEMENTS: [usize; 3] = [10, 50, 90];

/// How many tokens the answer may take: enough for a model that thinks
/// first to reach the digits.
const BUDGET: usize = 200;

/// What the prompt is padded with, repeated: plain prose that says
/// nothing about numbers.
pub const FILLER: &str = "The path along the shore turns inland where the cliff has fallen, and \
the walkers who use it know to look for the cairn before the turn. In spring the gorse covers \
the slope and the smell of it reaches the road. The fishermen keep their boats in the cove \
below, drawn up above the tide line on rollers cut from old telegraph poles. Nobody has fished \
from here for a living in forty years, but the boats are painted every winter all the same. \
The school closed, then the shop, and the bus comes twice a day if the road is open. ";

/// The room kept for the template, the question and the answer.
const ROOM: usize = 320;

/// Runs it.
#[must_use]
pub fn measure(site: &Site<'_>) -> Found {
    // The window the deepest depth needs, where the model holds it: the
    // depth, the room around it and the answer.
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
    let said = |failure: mcf_core::Failure| failure.detail().to_owned();
    let unit = match engine.tokenize(FILLER, false, false) {
        Ok(tokens) => tokens.len().max(1),
        Err(failure) => return Found::could_not_tell(&said(failure)),
    };
    let window = usize::try_from(engine.window).unwrap_or(usize::MAX);
    let digits = fresh_digits();
    let mut lines = vec![format!(
        "  a six-digit number made for this run, planted in filler and asked back through the \
         model's own template; the window is {window} tokens"
    )];
    let mut grid = Vec::new();
    let mut found = 0_usize;
    let mut placements = 0_usize;
    let mut first_miss: Option<(usize, usize)> = None;
    for depth in DEPTHS {
        if depth.saturating_add(ROOM) > window {
            lines.push(format!("  depth {depth:>6}   not asked: past the window"));
            continue;
        }
        #[expect(
            clippy::integer_division,
            reason = "how many repeats of the filler reach the depth"
        )]
        let repeats = (depth.saturating_sub(ROOM) / unit).max(1);
        let mut row = Vec::new();
        for placement in PLACEMENTS {
            if site.asker_gone() {
                return Found::could_not_tell(crate::served::CLIENT_LEFT);
            }
            let (got, prompt_tokens, produced) =
                match asked_back(site, &engine, repeats, placement, &digits) {
                    Ok(answered) => answered,
                    Err(why) => return Found::could_not_tell(&why),
                };
            placements = placements.saturating_add(1);
            if got {
                found = found.saturating_add(1);
            } else if first_miss.is_none() {
                first_miss = Some((depth, placement));
            }
            row.push(format!(
                "at {placement}% {}",
                if got { "found" } else { "MISSED" }
            ));
            grid.push(Value::map([
                ("depth", Value::Integer(as_integer(depth))),
                ("prompt_tokens", Value::Integer(as_integer(prompt_tokens))),
                (
                    "placement_hundredths",
                    Value::Integer(as_integer(placement)),
                ),
                ("found", Value::Bool(got)),
                ("produced", Value::Integer(as_integer(produced))),
            ]));
        }
        lines.push(format!("  depth {depth:>6}   {}", row.join("   ")));
    }
    if placements == 0 {
        return Found::could_not_tell("no depth fits the window with room for the question");
    }
    lines.push(match first_miss {
        Some((depth, placement)) => format!(
            "  found in {found} of {placements}; the first miss at depth {depth}, placed at \
             {placement}%"
        ),
        None => format!("  found in every one of {placements} placement(s)"),
    });
    Found {
        lines,
        fields: vec![
            ("window", whole(engine.window)),
            ("budget", Value::Integer(as_integer(BUDGET))),
            ("placements", Value::Integer(as_integer(placements))),
            ("found", Value::Integer(as_integer(found))),
            (
                "first_miss_depth",
                first_miss.map_or(Value::Null, |(depth, _)| Value::Integer(as_integer(depth))),
            ),
            (
                "first_miss_placement",
                first_miss.map_or(Value::Null, |(_, placement)| {
                    Value::Integer(as_integer(placement))
                }),
            ),
            ("grid", Value::List(grid)),
        ],
    }
}

/// One placement asked: whether the digits came back, how long the
/// prompt was, and how many tokens the answer took.
fn asked_back(
    site: &Site<'_>,
    engine: &crate::served::Served,
    repeats: usize,
    placement: usize,
    digits: &str,
) -> Result<(bool, usize, usize), String> {
    let text = planted(repeats, placement, digits);
    let ids = framed_ids(engine, &text)?;
    let completed = engine
        .complete(
            Prompt::Identifiers(&ids),
            BUDGET,
            Draw::greedy(0),
            false,
            site.waiting,
        )
        .map_err(|failure| failure.detail().to_owned())?;
    Ok((
        completed.text.contains(digits),
        ids.len(),
        completed.predicted,
    ))
}

/// The filler repeated, the fact planted at the placement, and the
/// question after it.
pub(crate) fn planted(repeats: usize, placement: usize, digits: &str) -> String {
    #[expect(
        clippy::integer_division,
        reason = "a placement in hundredths of the repeats"
    )]
    let at = (repeats * placement / 100).min(repeats);
    let mut text = String::with_capacity(FILLER.len() * (repeats + 1) + 200);
    for step in 0..=repeats {
        if step == at {
            text.push_str("The secret number is ");
            text.push_str(digits);
            text.push_str(". ");
        }
        if step < repeats {
            text.push_str(FILLER);
        }
    }
    text.push_str("\n\nWhat is the secret number? Answer with the digits only.");
    text
}

/// Six digits nothing has seen before this run: from the clock, so that
/// no run asks for a number a model could have been trained to say.
fn fresh_digits() -> String {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |since| since.as_nanos());
    let six = u64::try_from(nanos % 900_000).unwrap_or(0) + 100_000;
    six.to_string()
}
