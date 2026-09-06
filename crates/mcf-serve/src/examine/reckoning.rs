//! Dates, ordering and counting: the weekday of a date, the days between
//! two, ten numbers sorted, the letters in a word — each exact (B-545,
//! D55, A19).
//!
//! Four small skills a person leans on without noticing, each with one
//! right answer a parser can check: a weekday word, a day count, a
//! sorted list compared item by item, a letter count. Fixed questions,
//! greedy, every answer a row.

use mcf_record::json::Value;

use super::paraphrase::{answer_in, numbers_in};
use super::{Found, Reading, Site, as_integer, framed_ids};
use crate::generation::Draw;
use crate::served::{Prompt, Startup};

/// The measurement's name.
pub const NAME: &str = "reckoning";

/// How many tokens an answer may take.
const BUDGET: usize = 200;

/// What a question wants back.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Wants {
    /// A whole number.
    Number(i64),
    /// A word, case aside.
    Word(&'static str),
    /// These numbers in this order.
    Sorted(&'static [i64]),
}

/// The questions: kind, name, ask, answer.
pub const QUESTIONS: &[(&str, &str, &str, Wants)] = &[
    (
        "weekday",
        "weekday-2024-07-04",
        "What day of the week was 4 July 2024? Answer with the day's name only.",
        Wants::Word("thursday"),
    ),
    (
        "weekday",
        "weekday-2000-01-01",
        "What day of the week was 1 January 2000? Answer with the day's name only.",
        Wants::Word("saturday"),
    ),
    (
        "weekday",
        "weekday-2026-09-06",
        "What day of the week is 6 September 2026? Answer with the day's name only.",
        Wants::Word("sunday"),
    ),
    (
        "days-between",
        "between-leap-february",
        "How many days are there from 28 February 2024 to 1 March 2024? Answer with the number only.",
        Wants::Number(2),
    ),
    (
        "days-between",
        "between-a-year",
        "How many days are there from 15 March 2023 to 15 March 2024? Answer with the number only.",
        Wants::Number(366),
    ),
    (
        "days-between",
        "between-months",
        "How many days are there from 10 January 2025 to 10 April 2025? Answer with the number only.",
        Wants::Number(90),
    ),
    (
        "sort",
        "sort-ten",
        "Sort these numbers from smallest to largest: 42, 7, 19, 88, 3, 56, 21, 64, 11, 35. Answer with the sorted numbers separated by commas and nothing else.",
        Wants::Sorted(&[3, 7, 11, 19, 21, 35, 42, 56, 64, 88]),
    ),
    (
        "sort",
        "sort-with-negatives",
        "Sort these numbers from smallest to largest: 5, -12, 0, 33, -3, 8. Answer with the sorted numbers separated by commas and nothing else.",
        Wants::Sorted(&[-12, -3, 0, 5, 8, 33]),
    ),
    (
        "sort",
        "sort-descending",
        "Sort these numbers from largest to smallest: 14, 92, 6, 47, 28. Answer with the sorted numbers separated by commas and nothing else.",
        Wants::Sorted(&[92, 47, 28, 14, 6]),
    ),
    (
        "count",
        "letters-in-strawberry",
        "How many times does the letter r appear in the word strawberry? Answer with the number only.",
        Wants::Number(3),
    ),
    (
        "count",
        "letters-in-mississippi",
        "How many times does the letter s appear in the word Mississippi? Answer with the number only.",
        Wants::Number(4),
    ),
    (
        "count",
        "words-in-sentence",
        "How many words are in this sentence: \"The quick brown fox jumps over the lazy dog\"? Answer with the number only.",
        Wants::Number(9),
    ),
];

/// Whether an answer is the one wanted.
#[must_use]
pub fn right(said: &str, wants: &Wants) -> bool {
    match wants {
        Wants::Number(number) => answer_in(said) == Some(*number),
        Wants::Word(word) => said.to_lowercase().contains(word),
        Wants::Sorted(order) => {
            let signed: Vec<i64> = signed_numbers_in(said);
            signed == *order
        }
    }
}

/// Every whole number in a reply with its sign, in order: the reply
/// split on commas and whitespace, each piece that is a signed integer
/// kept. A sorted list is written that way; thousands separators are not
/// expected in one.
fn signed_numbers_in(said: &str) -> Vec<i64> {
    said.split(|c: char| c == ',' || c.is_whitespace())
        .filter_map(|piece| {
            // The piece's leading signed digits: a marker the engine writes
            // after the last number, `88<|im_end|>`, is not part of it.
            let end = piece
                .char_indices()
                .find(|(at, c)| !(c.is_ascii_digit() || (*at == 0 && *c == '-')))
                .map_or(piece.len(), |(at, _)| at);
            piece.get(..end)?.parse::<i64>().ok()
        })
        .collect()
}

/// Runs it.
#[must_use]
pub fn measure(site: &Site<'_>) -> Found {
    let engine = match site.server(&Startup {
        projector: None,
        ..site.startup()
    }) {
        Ok(engine) => engine,
        Err(why) => return Found::could_not_tell(&why),
    };
    let mut rows = Vec::new();
    let mut lines = vec![format!(
        "  {} question(s) — weekdays, days between dates, sorting, counting — greedy, each read \
         by a parser",
        QUESTIONS.len()
    )];
    let mut by_kind: std::collections::BTreeMap<&str, (usize, usize)> =
        std::collections::BTreeMap::new();
    for (at, (kind, name, asks, wants)) in QUESTIONS.iter().enumerate() {
        site.progress(at, QUESTIONS.len(), name);
        if site.asker_gone() {
            return Found::could_not_tell(crate::served::CLIENT_LEFT);
        }
        let ids = match framed_ids(&engine, asks) {
            Ok(ids) => ids,
            Err(why) => return Found::could_not_tell(&why),
        };
        let completed = match engine.complete(
            Prompt::Identifiers(&ids),
            BUDGET,
            Draw::greedy(0),
            false,
            site.timed(),
        ) {
            Ok(completed) => completed,
            Err(failure) => return Found::could_not_tell(failure.detail()),
        };
        let was_right = right(&completed.text, wants);
        let tally = by_kind.entry(kind).or_insert((0, 0));
        tally.0 = tally.0.saturating_add(usize::from(was_right));
        tally.1 = tally.1.saturating_add(1);
        let dims = [
            ("kind", Value::text(*kind)),
            ("question", Value::text(*name)),
        ];
        rows.push(Reading::new(&dims, "right", i64::from(was_right), "bool"));
        if let Wants::Number(_) = wants
            && let Some(answer) = answer_in(&completed.text)
        {
            rows.push(Reading::new(&dims, "answer", answer, "count"));
        }
        rows.push(Reading::new(
            &dims,
            "numbers",
            as_integer(numbers_in(&completed.text).len()),
            "count",
        ));
        rows.push(Reading::new(
            &dims,
            "tokens",
            as_integer(completed.predicted),
            "tokens",
        ));
        lines.push(format!(
            "  {name:<26} {}   said: {}",
            if was_right { "right" } else { "WRONG" },
            completed.text.trim().chars().take(40).collect::<String>()
        ));
    }
    let mut fields = vec![("questions", Value::Integer(as_integer(QUESTIONS.len())))];
    let mut right_all = 0_usize;
    let mut summary = Vec::new();
    for (kind, (right, of)) in &by_kind {
        right_all = right_all.saturating_add(*right);
        summary.push(format!("{kind} {right}/{of}"));
        rows.push(Reading::new(
            &[("kind", Value::text(*kind))],
            "right",
            as_integer(*right),
            "count",
        ));
    }
    fields.push(("right", Value::Integer(as_integer(right_all))));
    lines.push(format!(
        "  {right_all} of {} right: {}",
        QUESTIONS.len(),
        summary.join(", ")
    ));
    Found {
        lines,
        fields,
        rows,
    }
}

#[cfg(test)]
mod tests {
    use super::{QUESTIONS, Wants, right, signed_numbers_in};

    #[test]
    fn each_kind_is_read_exactly() {
        assert!(right("It was a Thursday.", &Wants::Word("thursday")));
        assert!(right("3", &Wants::Number(3)));
        assert!(right(
            "3, 7, 11, 19, 21, 35, 42, 56, 64, 88",
            &Wants::Sorted(&[3, 7, 11, 19, 21, 35, 42, 56, 64, 88])
        ));
        assert!(!right(
            "3, 7, 11, 19, 21, 35, 42, 56, 88, 64",
            &Wants::Sorted(&[3, 7, 11, 19, 21, 35, 42, 56, 64, 88])
        ));
        assert_eq!(
            signed_numbers_in("-12, -3, 0, 5, 8, 33"),
            vec![-12, -3, 0, 5, 8, 33]
        );
        assert_eq!(
            signed_numbers_in("Sorted: -12,-3, 0, 5."),
            vec![-12, -3, 0, 5]
        );
        assert_eq!(signed_numbers_in("92, 47, 28<|im_end|>"), vec![92, 47, 28]);
        assert!(!QUESTIONS.is_empty());
    }
}
