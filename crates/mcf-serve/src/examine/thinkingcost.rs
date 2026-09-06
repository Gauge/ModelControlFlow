//! Thinking against accuracy: on a model whose template takes the
//! thinking switch, the same exact questions with thinking on and off —
//! right or not, and the tokens spent before the answer (B-554, D55,
//! B-441).
//!
//! Thinking costs tokens and time; whether it buys right answers on this
//! model is the figure. The questions are the arithmetic and reckoning
//! measurements' kind, each with one whole-number answer. Where the
//! template renders the same text with the switch on and off, the model
//! has no switch, and this says so.

use mcf_record::json::Value;

use super::paraphrase::answer_in;
use super::{Found, Reading, Site, as_integer, timed};
use crate::generation::Draw;
use crate::served::{Prompt, Startup};
use crate::turn::{Turn, frame};

/// The measurement's name.
pub const NAME: &str = "thinking-cost";

/// How many tokens a reply may take with thinking on.
const BUDGET: usize = 1200;

/// The questions and their answers.
pub const QUESTIONS: &[(&str, &str, i64)] = &[
    (
        "product",
        "What is 347 times 29? Answer with the number only.",
        10_063,
    ),
    (
        "difference",
        "What is 10000 minus 4387? Answer with the number only.",
        5613,
    ),
    (
        "apples",
        "A crate holds 24 apples. Nine crates arrive and 37 apples are sold. How many apples are left? Answer with the number only.",
        179,
    ),
    (
        "trains",
        "A train leaves at 09:15 and arrives at 13:40 the same day. How many minutes does the journey take? Answer with the number only.",
        265,
    ),
    (
        "digits",
        "How many times does the digit 7 appear when you write every number from 1 to 100? Answer with the number only.",
        20,
    ),
    (
        "squares",
        "What is the sum of the squares of 3, 4 and 12? Answer with the number only.",
        169,
    ),
];

/// The tokens a reply spent inside its thinking markers, and the text
/// after them.
#[must_use]
pub fn thought_and_answer(said: &str) -> (Option<&str>, &str) {
    if let Some((thought, answer)) = said.split_once("</think>") {
        (Some(thought.trim_start_matches("<think>")), answer)
    } else {
        (None, said)
    }
}

/// Runs it.
#[must_use]
#[allow(
    clippy::too_many_lines,
    reason = "one measurement read straight through: each question each way, a row set each"
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
    let on = match frame(
        &engine,
        &Turn {
            thinking: Some(true),
            ..Turn::default()
        },
    ) {
        Ok(frame) => frame,
        Err(failure) => return Found::could_not_tell(&said(failure)),
    };
    let off = match frame(
        &engine,
        &Turn {
            thinking: Some(false),
            ..Turn::default()
        },
    ) {
        Ok(frame) => frame,
        Err(failure) => return Found::could_not_tell(&said(failure)),
    };
    if on.before == off.before && on.after == off.after {
        return Found::could_not_tell(
            "this model's template renders the same text with thinking on and off, so it has no \
             switch to measure",
        );
    }
    let mut rows = Vec::new();
    let mut lines = vec![format!(
        "  {} question(s) each asked with thinking on and off, greedy; the tokens before the \
         answer counted from the thinking markers",
        QUESTIONS.len()
    )];
    let (mut right_on, mut right_off, mut thought_all) = (0_usize, 0_usize, 0_usize);
    for (at, (name, asks, answer)) in QUESTIONS.iter().enumerate() {
        site.progress(at, QUESTIONS.len(), name);
        let mut said_here = Vec::new();
        for (mode, held) in [("on", &on), ("off", &off)] {
            if site.asker_gone() {
                return Found::could_not_tell(crate::served::CLIENT_LEFT);
            }
            let text = format!("{}{asks}{}", held.before, held.after);
            let ids = match engine.tokenize(&text, true, true) {
                Ok(tokens) => tokens
                    .into_iter()
                    .map(|token| token.id)
                    .collect::<Vec<usize>>(),
                Err(failure) => return Found::could_not_tell(&said(failure)),
            };
            let (done, ns) = timed(|| {
                engine.complete(
                    Prompt::Identifiers(&ids),
                    BUDGET,
                    Draw::greedy(0),
                    false,
                    site.timed(),
                )
            });
            let completed = match done {
                Ok(completed) => completed,
                Err(failure) => return Found::could_not_tell(&said(failure)),
            };
            let (thought, after) = thought_and_answer(&completed.text);
            let thought_tokens = thought.map_or(0, |thought| {
                engine
                    .tokenize(thought, false, false)
                    .map_or(0, |tokens| tokens.len())
            });
            let read = answer_in(after);
            let right = read == Some(*answer);
            if mode == "on" {
                right_on = right_on.saturating_add(usize::from(right));
                thought_all = thought_all.saturating_add(thought_tokens);
            } else {
                right_off = right_off.saturating_add(usize::from(right));
            }
            let dims = [
                ("question", Value::text(*name)),
                ("thinking", Value::text(mode)),
            ];
            rows.push(Reading::new(&dims, "right", i64::from(right), "bool"));
            if let Some(read) = read {
                rows.push(Reading::new(&dims, "answer", read, "count"));
            }
            rows.push(Reading::new(
                &dims,
                "thought_tokens",
                as_integer(thought_tokens),
                "tokens",
            ));
            rows.push(Reading::new(
                &dims,
                "tokens",
                as_integer(completed.predicted),
                "tokens",
            ));
            rows.push(Reading::new(
                &dims,
                "ns",
                i64::try_from(ns).unwrap_or(i64::MAX),
                "ns",
            ));
            rows.push(Reading::new(
                &dims,
                "cut_at_budget",
                i64::from(completed.predicted >= BUDGET),
                "bool",
            ));
            said_here.push(format!(
                "{mode} {} ({thought_tokens} thinking)",
                if right { "right" } else { "WRONG" }
            ));
        }
        lines.push(format!("  {name:<12} {}", said_here.join("   ")));
    }
    lines.push(format!(
        "  right with thinking on in {right_on} of {}, off in {right_off}; {thought_all} \
         token(s) of thinking in all",
        QUESTIONS.len()
    ));
    Found {
        lines,
        fields: vec![
            ("questions", Value::Integer(as_integer(QUESTIONS.len()))),
            ("right_on", Value::Integer(as_integer(right_on))),
            ("right_off", Value::Integer(as_integer(right_off))),
            ("thought_tokens", Value::Integer(as_integer(thought_all))),
        ],
        rows,
    }
}

#[cfg(test)]
mod tests {
    use super::{QUESTIONS, thought_and_answer};

    #[test]
    fn the_thought_is_split_from_the_answer_at_its_marker() {
        assert_eq!(
            thought_and_answer("<think>a b c</think>\n42"),
            (Some("a b c"), "\n42")
        );
        assert_eq!(thought_and_answer("42"), (None, "42"));
        assert_eq!(QUESTIONS[0].2, 347 * 29);
        assert_eq!(QUESTIONS[2].2, 24 * 9 - 37);
        assert_eq!(QUESTIONS[3].2, (13 * 60 + 40) - (9 * 60 + 15));
        assert_eq!(QUESTIONS[5].2, 9 + 16 + 144);
    }
}
