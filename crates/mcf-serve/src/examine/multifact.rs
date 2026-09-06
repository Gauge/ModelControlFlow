//! Multi-fact retrieval: several facts planted through a long prompt,
//! then asked to list, to order and to sum — found, in the right order,
//! summed rightly, at each depth (B-529, D55, B-497).
//!
//! Retrieval by depth plants one number and asks for it back. A person
//! who hands a model a long document asks for more than one thing from
//! it, and asks for them together: everything of a kind, the order they
//! came in, a total. Four prices for four named items are planted at
//! four places through the filler, fresh each run; three questions are
//! put at each depth; each answer is read by a parser — the values found
//! in it, the names in their order, the one number it gives as a total.

use mcf_record::json::Value;

use super::retrieval::FILLER;
use super::{Found, Reading, Site, as_integer, framed_ids, whole};
use crate::generation::Draw;
use crate::served::{Prompt, Startup};

/// The measurement's name.
pub const NAME: &str = "multi-fact-retrieval";

/// The depths tried, in tokens; each is capped at the server's window.
const DEPTHS: [usize; 3] = [1024, 4096, 16384];

/// Where the facts are planted, as hundredths of the filler, in order.
const PLACEMENTS: [usize; 4] = [15, 40, 65, 90];

/// The items the facts are about, in the order they are planted.
pub const ITEMS: [&str; 4] = ["kestrel", "marlin", "osprey", "heron"];

/// How many tokens an answer may take.
const BUDGET: usize = 160;

/// The room kept for the template, the questions and the answer.
const ROOM: usize = 360;

/// The three questions, by name.
const ASKS: [(&str, &str); 3] = [
    (
        "list",
        "List the price of every item mentioned in the text, one per line as `item: price`. \
         Answer with the list only.",
    ),
    (
        "order",
        "List the names of the items mentioned in the text in the order they were mentioned, \
         separated by commas. Answer with the names only.",
    ),
    (
        "sum",
        "What is the total of all the prices mentioned in the text? Answer with the number \
         only.",
    ),
];

/// Runs it.
#[must_use]
#[allow(
    clippy::too_many_lines,
    reason = "one measurement read straight through: each depth, each question, each reading a row"
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
    let prices = fresh_prices();
    let total: i64 = prices.iter().sum();
    let mut lines = vec![format!(
        "  {} prices made for this run, planted at {} places through the filler and asked for \
         three ways through the model's own template; the window is {window} tokens",
        ITEMS.len(),
        PLACEMENTS.len()
    )];
    let mut rows = Vec::new();
    let (mut depths_asked, mut listed_all, mut ordered_all, mut summed_all) =
        (0_usize, 0_usize, 0_usize, 0_usize);
    for (at, depth) in DEPTHS.into_iter().enumerate() {
        site.progress(at, DEPTHS.len(), &format!("depth {depth}"));
        if depth.saturating_add(ROOM) > window {
            lines.push(format!("  depth {depth:>6}   not asked: past the window"));
            continue;
        }
        #[expect(
            clippy::integer_division,
            reason = "how many repeats of the filler reach the depth"
        )]
        let repeats = (depth.saturating_sub(ROOM) / unit).max(1);
        depths_asked = depths_asked.saturating_add(1);
        let mut said = Vec::with_capacity(ASKS.len());
        for (name, question) in ASKS {
            if site.asker_gone() {
                return Found::could_not_tell(crate::served::CLIENT_LEFT);
            }
            let text = planted(repeats, &prices, question);
            let ids = match framed_ids(&engine, &text) {
                Ok(ids) => ids,
                Err(why) => return Found::could_not_tell(&why),
            };
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
            let dims = [
                ("depth", Value::Integer(as_integer(depth))),
                ("ask", Value::text(name)),
            ];
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
            match name {
                "list" => {
                    let found = prices
                        .iter()
                        .filter(|price| completed.text.contains(&price.to_string()))
                        .count();
                    let all = found == prices.len();
                    listed_all = listed_all.saturating_add(usize::from(all));
                    rows.push(Reading::new(&dims, "found", as_integer(found), "count"));
                    rows.push(Reading::new(
                        &dims,
                        "facts",
                        as_integer(prices.len()),
                        "count",
                    ));
                    rows.push(Reading::new(&dims, "listed_all", i64::from(all), "bool"));
                    said.push(format!("listed {found}/{}", prices.len()));
                }
                "order" => {
                    let (named, ordered) = order_in(&completed.text);
                    ordered_all = ordered_all.saturating_add(usize::from(ordered));
                    rows.push(Reading::new(&dims, "named", as_integer(named), "count"));
                    rows.push(Reading::new(&dims, "ordered", i64::from(ordered), "bool"));
                    said.push(format!("order {}", if ordered { "right" } else { "WRONG" }));
                }
                _ => {
                    let answer = super::paraphrase::answer_in(&completed.text);
                    let summed = answer == Some(total);
                    summed_all = summed_all.saturating_add(usize::from(summed));
                    if let Some(answer) = answer {
                        rows.push(Reading::new(&dims, "answer", answer, "count"));
                    }
                    rows.push(Reading::new(&dims, "summed", i64::from(summed), "bool"));
                    said.push(format!("sum {}", if summed { "right" } else { "WRONG" }));
                }
            }
        }
        lines.push(format!("  depth {depth:>6}   {}", said.join("   ")));
    }
    if depths_asked == 0 {
        return Found::could_not_tell("no depth fits the window with room for the questions");
    }
    lines.push(format!(
        "  over {depths_asked} depth(s): every price listed in {listed_all}, the order right in \
         {ordered_all}, the sum right in {summed_all}"
    ));
    Found {
        lines,
        fields: vec![
            ("window", whole(engine.window)),
            ("facts", Value::Integer(as_integer(ITEMS.len()))),
            ("depths", Value::Integer(as_integer(depths_asked))),
            ("listed_all", Value::Integer(as_integer(listed_all))),
            ("ordered", Value::Integer(as_integer(ordered_all))),
            ("summed", Value::Integer(as_integer(summed_all))),
        ],
        rows,
    }
}

/// The filler repeated, one fact planted at each placement, and the
/// question after it.
#[must_use]
pub fn planted(repeats: usize, prices: &[i64], question: &str) -> String {
    let mut text = String::with_capacity(FILLER.len() * (repeats + 1) + 400);
    for step in 0..=repeats {
        for ((placement, item), price) in PLACEMENTS.iter().zip(ITEMS).zip(prices) {
            #[expect(
                clippy::integer_division,
                reason = "a placement in hundredths of the repeats"
            )]
            let at = (repeats * placement / 100).min(repeats);
            if step == at {
                use std::fmt::Write as _;
                let _wrote = write!(text, "The {item} is priced at {price} units. ");
            }
        }
        if step < repeats {
            text.push_str(FILLER);
        }
    }
    text.push_str("\n\n");
    text.push_str(question);
    text
}

/// How many item names an answer holds, and whether every one is there
/// in the planted order.
#[must_use]
pub fn order_in(said: &str) -> (usize, bool) {
    let lowered = said.to_lowercase();
    let positions: Vec<Option<usize>> = ITEMS.iter().map(|item| lowered.find(item)).collect();
    let named = positions.iter().flatten().count();
    let ordered = named == ITEMS.len()
        && positions
            .windows(2)
            .all(|pair| matches!(pair, [Some(before), Some(after)] if before < after));
    (named, ordered)
}

/// Four three-digit prices nothing has seen before this run, all
/// different, from the clock.
fn fresh_prices() -> Vec<i64> {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |since| since.as_nanos());
    let mut seed = u64::try_from(nanos % 1_000_000_007).unwrap_or(7);
    let mut prices = Vec::with_capacity(ITEMS.len());
    while prices.len() < ITEMS.len() {
        seed = seed
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        let price = i64::try_from((seed >> 33) % 900).unwrap_or(0) + 100;
        if !prices.contains(&price) {
            prices.push(price);
        }
    }
    prices
}

#[cfg(test)]
mod tests {
    use super::{ITEMS, order_in, planted};

    #[test]
    fn every_fact_is_planted_once_in_order_before_the_question() {
        let text = planted(20, &[111, 222, 333, 444], "What?");
        for (item, price) in ITEMS.iter().zip([111, 222, 333, 444]) {
            let fact = format!("The {item} is priced at {price} units.");
            assert_eq!(text.matches(&fact).count(), 1, "{fact}");
        }
        let (named, ordered) = order_in(&text);
        assert_eq!((named, ordered), (4, true));
        assert!(text.ends_with("What?"));
    }

    #[test]
    fn the_order_is_read_from_where_the_names_fall() {
        assert_eq!(order_in("Kestrel, marlin, osprey, heron"), (4, true));
        assert_eq!(order_in("marlin, kestrel, osprey, heron"), (4, false));
        assert_eq!(order_in("kestrel and heron"), (2, false));
    }
}
