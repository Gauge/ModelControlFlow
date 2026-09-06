//! Paraphrase consistency: one question with an exact answer put six
//! ways — how many of the answers agree, and how many are right
//! (B-527, D55, A19).
//!
//! A model that answers a question one way and its paraphrase another
//! is a model whose answers depend on wording, which a person choosing
//! one wants to know before they wonder which wording they will use.
//! Each question has a whole-number answer, so a parser reads the
//! answer: the first whole number in the reply. Greedy throughout, so
//! that what varies is the phrasing and nothing else.

use mcf_record::json::Value;

use super::{Found, Reading, Site, as_integer, framed_ids, timed};
use crate::generation::Draw;
use crate::served::{Prompt, Startup};

/// The measurement's name.
pub const NAME: &str = "paraphrase";

/// How many tokens an answer may take.
const BUDGET: usize = 80;

/// One question, its answer, and its phrasings.
#[derive(Debug)]
pub struct Question {
    /// Its name.
    pub name: &'static str,
    /// The answer, a whole number.
    pub answer: i64,
    /// The same question, six ways.
    pub phrasings: [&'static str; 6],
}

/// The questions.
pub const QUESTIONS: &[Question] = &[
    Question {
        name: "minutes-in-three-hours",
        answer: 180,
        phrasings: [
            "How many minutes are there in three hours? Answer with the number only.",
            "Three hours is how many minutes? Answer with the number only.",
            "Convert 3 hours to minutes. Answer with the number only.",
            "If something lasts three hours, how many minutes is that? Answer with the number only.",
            "Tell me the number of minutes in 3 hours. Answer with the number only.",
            "Minutes in three hours: how many? Answer with the number only.",
        ],
    },
    Question {
        name: "days-in-a-leap-year",
        answer: 366,
        phrasings: [
            "How many days does a leap year have? Answer with the number only.",
            "A leap year has how many days? Answer with the number only.",
            "What is the number of days in a leap year? Answer with the number only.",
            "In a leap year, how many days are there? Answer with the number only.",
            "Count the days in a leap year. Answer with the number only.",
            "Days in a leap year: how many? Answer with the number only.",
        ],
    },
    Question {
        name: "sides-of-a-hexagon",
        answer: 6,
        phrasings: [
            "How many sides does a hexagon have? Answer with the number only.",
            "A hexagon has how many sides? Answer with the number only.",
            "What is the number of sides of a hexagon? Answer with the number only.",
            "Count the sides of a hexagon. Answer with the number only.",
            "Tell me how many sides there are on a hexagon. Answer with the number only.",
            "Sides on a hexagon: how many? Answer with the number only.",
        ],
    },
    Question {
        name: "seconds-in-a-quarter-hour",
        answer: 900,
        phrasings: [
            "How many seconds are there in a quarter of an hour? Answer with the number only.",
            "A quarter of an hour is how many seconds? Answer with the number only.",
            "Convert fifteen minutes to seconds. Answer with the number only.",
            "If something lasts 15 minutes, how many seconds is that? Answer with the number only.",
            "Tell me the number of seconds in 15 minutes. Answer with the number only.",
            "Seconds in a quarter hour: how many? Answer with the number only.",
        ],
    },
    Question {
        name: "fifteen-per-cent-of-two-hundred",
        answer: 30,
        phrasings: [
            "What is 15% of 200? Answer with the number only.",
            "Fifteen per cent of two hundred is what? Answer with the number only.",
            "Calculate 15 percent of 200. Answer with the number only.",
            "If you take 15% of 200, what do you get? Answer with the number only.",
            "Tell me fifteen percent of 200. Answer with the number only.",
            "15% of 200: what is it? Answer with the number only.",
        ],
    },
    Question {
        name: "legs-on-four-spiders",
        answer: 32,
        phrasings: [
            "How many legs do four spiders have altogether? Answer with the number only.",
            "Four spiders have how many legs in total? Answer with the number only.",
            "Count the legs on 4 spiders. Answer with the number only.",
            "If there are four spiders, how many legs is that in all? Answer with the number only.",
            "Tell me the total number of legs on four spiders. Answer with the number only.",
            "Legs on 4 spiders, all together: how many? Answer with the number only.",
        ],
    },
];

/// Every whole number in a reply, in order, thousands separators allowed.
#[must_use]
pub fn numbers_in(said: &str) -> Vec<i64> {
    let mut found = Vec::new();
    let mut digits = String::new();
    for character in said.chars().chain(std::iter::once(' ')) {
        if character.is_ascii_digit() {
            digits.push(character);
        } else if !digits.is_empty() && character != ',' {
            if let Ok(number) = digits.parse() {
                found.push(number);
            }
            digits.clear();
        }
    }
    found
}

/// The number a reply gives as its answer: the only number where there
/// is one; the number after the last `=` where the reply works it out;
/// otherwise the last number in it. A reply told to answer with the
/// number only and holding several has already said something else, and
/// `numbers` counts them beside the reading.
#[must_use]
pub fn answer_in(said: &str) -> Option<i64> {
    let numbers = numbers_in(said);
    if let [only] = numbers.as_slice() {
        return Some(*only);
    }
    if let Some((_, after)) = said.rsplit_once('=')
        && let Some(first) = numbers_in(after).first()
    {
        return Some(*first);
    }
    numbers.last().copied()
}

/// How many of the answers agree with the most common one.
#[must_use]
pub fn agreement(answers: &[Option<i64>]) -> usize {
    let mut counts: std::collections::BTreeMap<i64, usize> = std::collections::BTreeMap::new();
    for answer in answers.iter().flatten() {
        *counts.entry(*answer).or_insert(0) += 1;
    }
    counts.values().copied().max().unwrap_or(0)
}

/// Runs it.
#[must_use]
#[allow(
    clippy::too_many_lines,
    reason = "one measurement read straight through: each question, each phrasing a row, the agreement beside them"
)]
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
        "  {} question(s), each put {} ways, greedy; the reply's one number, or the one after \
         its last =, or its last, is the answer",
        QUESTIONS.len(),
        QUESTIONS
            .first()
            .map_or(0, |question| question.phrasings.len())
    )];
    let (mut right_all, mut agree_all, mut asked) = (0_usize, 0_usize, 0_usize);
    for question in QUESTIONS {
        let mut answers = Vec::with_capacity(question.phrasings.len());
        for (phrasing, asks) in question.phrasings.iter().enumerate() {
            if site.asker_gone() {
                return Found::could_not_tell(crate::served::CLIENT_LEFT);
            }
            let ids = match framed_ids(&engine, asks) {
                Ok(ids) => ids,
                Err(why) => return Found::could_not_tell(&why),
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
                Err(failure) => return Found::could_not_tell(failure.detail()),
            };
            let numbers = numbers_in(&completed.text);
            let answer = answer_in(&completed.text);
            let right = answer == Some(question.answer);
            asked = asked.saturating_add(1);
            right_all = right_all.saturating_add(usize::from(right));
            let dims = [
                ("question", Value::text(question.name)),
                ("phrasing", Value::Integer(as_integer(phrasing))),
            ];
            rows.push(Reading::new(
                &dims,
                "read",
                i64::from(answer.is_some()),
                "bool",
            ));
            if let Some(answer) = answer {
                rows.push(Reading::new(&dims, "answer", answer, "count"));
            }
            rows.push(Reading::new(&dims, "right", i64::from(right), "bool"));
            rows.push(Reading::new(
                &dims,
                "numbers",
                as_integer(numbers.len()),
                "count",
            ));
            rows.push(Reading::new(
                &dims,
                "stated",
                i64::from(numbers.contains(&question.answer)),
                "bool",
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
            answers.push(answer);
        }
        let agree = agreement(&answers);
        let right = answers
            .iter()
            .filter(|answer| **answer == Some(question.answer))
            .count();
        agree_all = agree_all.saturating_add(agree);
        let dims = [("question", Value::text(question.name))];
        rows.push(Reading::new(&dims, "agree", as_integer(agree), "count"));
        rows.push(Reading::new(&dims, "right", as_integer(right), "count"));
        rows.push(Reading::new(
            &dims,
            "phrasings",
            as_integer(question.phrasings.len()),
            "count",
        ));
        lines.push(format!(
            "  {:<34} {} of {} agree, {} right   {}",
            question.name,
            agree,
            question.phrasings.len(),
            right,
            answers
                .iter()
                .map(|answer| answer.map_or_else(|| "?".to_owned(), |n| n.to_string()))
                .collect::<Vec<String>>()
                .join(" ")
        ));
    }
    lines.push(format!(
        "  {right_all} of {asked} answer(s) right; {agree_all} agree with their question's most \
         common answer"
    ));
    Found {
        lines,
        fields: vec![
            ("questions", Value::Integer(as_integer(QUESTIONS.len()))),
            ("asked", Value::Integer(as_integer(asked))),
            ("right", Value::Integer(as_integer(right_all))),
            ("agree", Value::Integer(as_integer(agree_all))),
        ],
        rows,
    }
}

#[cfg(test)]
mod tests {
    use super::{QUESTIONS, agreement, answer_in, numbers_in};

    #[test]
    fn the_answer_is_the_only_number_or_the_one_worked_out_or_the_last() {
        assert_eq!(answer_in("There are 1,440 minutes in a day."), Some(1440));
        assert_eq!(answer_in("180"), Some(180));
        assert_eq!(answer_in("3 hours × 60 = 180 minutes"), Some(180));
        assert_eq!(answer_in("24 - 6 = 18"), Some(18));
        assert_eq!(answer_in("about 3 hours, so 180"), Some(180));
        assert_eq!(answer_in("none"), None);
        assert_eq!(numbers_in("17 x 23 = 391."), vec![17, 23, 391]);
    }

    #[test]
    fn agreement_is_the_size_of_the_largest_camp() {
        assert_eq!(agreement(&[Some(1), Some(1), Some(2), None]), 2);
        assert_eq!(agreement(&[None, None]), 0);
        assert_eq!(agreement(&[Some(5); 6]), 6);
    }

    #[test]
    fn every_question_has_six_phrasings_that_ask_for_the_number_only() {
        for question in QUESTIONS {
            for phrasing in question.phrasings {
                assert!(
                    phrasing.ends_with("Answer with the number only."),
                    "{phrasing}"
                );
            }
        }
    }
}
