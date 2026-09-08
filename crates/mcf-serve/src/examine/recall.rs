use mcf_record::json::Value;

use super::paraphrase::answer_in;
use super::tooluse::as_read;
use super::{Found, Reading, Site, as_integer, timed};
use crate::generation::Draw;
use crate::served::{Prompt, Startup};

pub const NAME: &str = "recall";

pub const DISTANCES: [usize; 3] = [2, 5, 10];

const BUDGET: usize = 60;

const DISTRACTORS: [&str; 10] = [
    "What colour is the sky on a clear day? One word.",
    "How many legs does a spider have? Number only.",
    "Name a fruit that is yellow. One word.",
    "What is 6 times 7? Number only.",
    "Which is heavier, a kilogram of feathers or a kilogram of iron? One word: same, feathers or iron.",
    "What is the capital of France? One word.",
    "How many minutes are in an hour? Number only.",
    "Name a musical instrument with strings. One word.",
    "What is 100 divided by 4? Number only.",
    "Is water a solid, a liquid or a gas at room temperature? One word.",
];

pub const FACTS: &[(&str, &str, &str, i64)] = &[
    (
        "locker",
        "My locker number is 4127. Please remember it. Reply with just OK.",
        "What is my locker number? Answer with the number only.",
        4127,
    ),
    (
        "floor",
        "I work on the 23rd floor. Please remember that. Reply with just OK.",
        "Which floor do I work on? Answer with the number only.",
        23,
    ),
    (
        "birthday",
        "My birthday is on the 19th of the month. Please remember it. Reply with just OK.",
        "What day of the month is my birthday? Answer with the number only.",
        19,
    ),
];

pub const CORRECTION: (&str, &str, &str, i64, i64) = (
    "The meeting is in room 302. Reply with just OK.",
    "Correction: the meeting has moved to room 415. Reply with just OK.",
    "Which room is the meeting in? Answer with the number only.",
    302,
    415,
);

fn turn(role: &str, content: &str) -> Value {
    Value::map([
        ("role", Value::text(role)),
        ("content", Value::text(content)),
    ])
}

#[must_use]
#[allow(
    clippy::too_many_lines,
    reason = "one measurement read straight through: each fact at each distance, the correction, a row set each"
)]
pub fn measure(site: &Site<'_>) -> Found {
    let engine = match site.server(&Startup {
        projector: None,
        ..site.startup()
    }) {
        Ok(engine) => engine,
        Err(why) => return Found::could_not_tell(&why),
    };
    let said = |failure: mcf_core::Failure| failure.detail().to_owned();
    let reply = |messages: &[Value]| -> Result<(String, usize, u64), String> {
        let rendered = engine
            .render_with_tools(Value::List(messages.to_vec()), Value::Null)
            .map_err(said)?;
        let ids = as_read(&engine, &rendered)?;
        let (done, ns) = timed(|| {
            engine.complete(
                Prompt::Identifiers(&ids),
                BUDGET,
                Draw::greedy(0),
                false,
                site.timed(),
            )
        });
        let completed = done.map_err(said)?;
        Ok((completed.text, completed.predicted, ns))
    };
    let mut rows = Vec::new();
    let mut lines = vec![format!(
        "  {} fact(s) each asked back after {} distractor turn(s), and one correction, through \
         the model's own template with its own replies kept; greedy",
        FACTS.len(),
        DISTANCES
            .iter()
            .map(usize::to_string)
            .collect::<Vec<String>>()
            .join(", ")
    )];
    let mut recalled_by_distance: Vec<usize> = vec![0; DISTANCES.len()];
    for (at, (name, tell, ask, answer)) in FACTS.iter().enumerate() {
        site.progress(at, FACTS.len(), name);
        let mut said_at = Vec::with_capacity(DISTANCES.len());
        for (which, distance) in DISTANCES.iter().enumerate() {
            if site.asker_gone() {
                return Found::could_not_tell(crate::served::CLIENT_LEFT);
            }
            let mut messages = vec![turn("user", tell)];
            let (first, _, _) = match reply(&messages) {
                Ok(held) => held,
                Err(why) => return Found::could_not_tell(&why),
            };
            messages.push(turn("assistant", first.trim()));
            let mut prompt_tokens = 0_usize;
            for distractor in DISTRACTORS.iter().take(*distance) {
                messages.push(turn("user", distractor));
                let (answered, _, _) = match reply(&messages) {
                    Ok(held) => held,
                    Err(why) => return Found::could_not_tell(&why),
                };
                messages.push(turn("assistant", answered.trim()));
            }
            messages.push(turn("user", ask));
            let (final_reply, tokens, ns) = match reply(&messages) {
                Ok(held) => held,
                Err(why) => return Found::could_not_tell(&why),
            };
            if let Ok(rendered) =
                engine.render_with_tools(Value::List(messages.clone()), Value::Null)
                && let Ok(ids) = as_read(&engine, &rendered)
            {
                prompt_tokens = ids.len();
            }
            let read = answer_in(&final_reply);
            let recalled = read == Some(*answer);
            if let Some(count) = recalled_by_distance.get_mut(which) {
                *count = count.saturating_add(usize::from(recalled));
            }
            let dims = [
                ("fact", Value::text(*name)),
                ("distance", Value::Integer(as_integer(*distance))),
            ];
            rows.push(Reading::new(&dims, "recalled", i64::from(recalled), "bool"));
            if let Some(read) = read {
                rows.push(Reading::new(&dims, "answer", read, "count"));
            }
            rows.push(Reading::new(
                &dims,
                "prompt_tokens",
                as_integer(prompt_tokens),
                "tokens",
            ));
            rows.push(Reading::new(&dims, "tokens", as_integer(tokens), "tokens"));
            rows.push(Reading::new(
                &dims,
                "ns",
                i64::try_from(ns).unwrap_or(i64::MAX),
                "ns",
            ));
            said_at.push(format!(
                "after {distance} {}",
                if recalled { "recalled" } else { "LOST" }
            ));
        }
        lines.push(format!("  {name:<10} {}", said_at.join("   ")));
    }
    if site.asker_gone() {
        return Found::could_not_tell(crate::served::CLIENT_LEFT);
    }
    let (told, corrected, asked, old, new) = CORRECTION;
    let mut messages = vec![turn("user", told)];
    let corrected_held = (|| -> Result<(bool, bool, usize), String> {
        let (first, _, _) = reply(&messages)?;
        messages.push(turn("assistant", first.trim()));
        messages.push(turn("user", corrected));
        let (second, _, _) = reply(&messages)?;
        messages.push(turn("assistant", second.trim()));
        for distractor in DISTRACTORS.iter().take(3) {
            messages.push(turn("user", distractor));
            let (answered, _, _) = reply(&messages)?;
            messages.push(turn("assistant", answered.trim()));
        }
        messages.push(turn("user", asked));
        let (final_reply, tokens, _) = reply(&messages)?;
        let read = answer_in(&final_reply);
        Ok((read == Some(new), read == Some(old), tokens))
    })();
    let (honoured, stale) = match corrected_held {
        Ok((honoured, stale, tokens)) => {
            let dims = [("fact", Value::text("correction"))];
            rows.push(Reading::new(
                &dims,
                "corrected",
                i64::from(honoured),
                "bool",
            ));
            rows.push(Reading::new(
                &dims,
                "gave_the_old",
                i64::from(stale),
                "bool",
            ));
            rows.push(Reading::new(&dims, "tokens", as_integer(tokens), "tokens"));
            (honoured, stale)
        }
        Err(why) => return Found::could_not_tell(&why),
    };
    lines.push(format!(
        "  correction {}",
        if honoured {
            "honoured after three turns"
        } else if stale {
            "NOT honoured: the old value came back"
        } else {
            "NOT honoured: neither value came back"
        }
    ));
    for (distance, count) in DISTANCES.iter().zip(&recalled_by_distance) {
        rows.push(Reading::new(
            &[("distance", Value::Integer(as_integer(*distance)))],
            "recalled",
            as_integer(*count),
            "count",
        ));
    }
    let recalled_all: usize = recalled_by_distance.iter().sum();
    lines.push(format!(
        "  recalled in {recalled_all} of {} ask(s)",
        FACTS.len().saturating_mul(DISTANCES.len())
    ));
    Found {
        lines,
        fields: vec![
            ("facts", Value::Integer(as_integer(FACTS.len()))),
            (
                "asked",
                Value::Integer(as_integer(FACTS.len().saturating_mul(DISTANCES.len()))),
            ),
            ("recalled", Value::Integer(as_integer(recalled_all))),
            (
                "recalled_at_ten",
                Value::Integer(as_integer(
                    recalled_by_distance.last().copied().unwrap_or(0),
                )),
            ),
            ("corrected", Value::Bool(honoured)),
        ],
        rows,
    }
}
