//! Saying it is not there: questions whose answer is absent from a
//! passage given — whether a figure is invented or the absence stated
//! (B-549, D55, B-517).
//!
//! A model handed a passage and asked for something it does not say
//! either says so or makes something up, and the second is the worse
//! habit by far. Each passage carries some figures and not others; the
//! present ones are asked for, as the control, and the absent ones with
//! them. A reply to an absent question that holds a number invented one;
//! one that says it is not stated, said so.

use mcf_record::json::Value;

use super::paraphrase::{answer_in, numbers_in};
use super::{Found, Reading, Site, as_integer, framed_ids};
use crate::generation::Draw;
use crate::served::{Prompt, Startup};

/// The measurement's name.
pub const NAME: &str = "absent";

/// How many tokens a reply may take.
const BUDGET: usize = 60;

/// One passage: its name, its text, the questions it answers with their
/// answers, and the questions it does not.
pub type Passage = (
    &'static str,
    &'static str,
    &'static [(&'static str, i64)],
    &'static [&'static str],
);

/// The passages, each with present questions (answer known) and absent
/// ones (nothing in the text answers them).
pub const PASSAGES: &[Passage] = &[
    (
        "ferry",
        "The morning ferry leaves the harbour at 07:40 and takes 55 minutes to cross. It \
         carries up to 120 passengers and 18 cars. Tickets are 6 euros for adults.",
        &[
            ("How many minutes does the crossing take?", 55),
            ("How many cars can it carry?", 18),
        ],
        &[
            "How many crossings does the ferry make each day?",
            "How much is a child's ticket in euros?",
            "How many crew members work on the ferry?",
        ],
    ),
    (
        "orchard",
        "The orchard has 240 apple trees planted in 12 rows. Last year it produced 9 tonnes of \
         fruit, picked by a team of 15 over three weeks.",
        &[
            ("How many rows of trees are there?", 12),
            ("How many tonnes were picked last year?", 9),
        ],
        &[
            "How many pear trees does the orchard have?",
            "How many tonnes were picked the year before last?",
            "How old are the trees in years?",
        ],
    ),
    (
        "library",
        "The town library holds 48,000 books on three floors and is open 6 days a week. It lent \
         2,300 books in March and has 14 staff.",
        &[
            ("How many staff does the library have?", 14),
            ("How many books did it lend in March?", 2300),
        ],
        &[
            "How many books did the library lend in April?",
            "How many reading rooms does it have?",
            "How many members does the library have?",
        ],
    ),
];

/// The words a reply says the absence with, any one of them.
const ABSENCE: [&str; 8] = [
    "not stated",
    "does not say",
    "doesn't say",
    "not mentioned",
    "not given",
    "no information",
    "not specified",
    "cannot be determined",
];

/// What is asked.
#[must_use]
pub fn ask_for(passage: &str, question: &str) -> String {
    format!(
        "Read this text:\n\n{passage}\n\n{question} If the text does not say, answer exactly: \
         not stated. Otherwise answer with the number only."
    )
}

/// How a reply to an absent question reads: invented a figure, stated the
/// absence, or neither.
#[must_use]
pub fn read_absent(said: &str) -> (bool, bool) {
    let lowered = said.to_lowercase();
    let stated = ABSENCE.iter().any(|words| lowered.contains(words));
    let invented = !stated && !numbers_in(said).is_empty();
    (invented, stated)
}

/// Runs it.
#[must_use]
#[allow(
    clippy::too_many_lines,
    reason = "one measurement read straight through: each passage's present and absent questions, a row set each"
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
        "  {} passage(s), each with questions the text answers and questions it does not; \
         greedy; a number in a reply to an absent question is an invention",
        PASSAGES.len()
    )];
    let (mut present_right, mut present_all, mut invented_all, mut stated_all, mut absent_all) =
        (0_usize, 0_usize, 0_usize, 0_usize, 0_usize);
    for (name, passage, present, absent) in PASSAGES {
        let mut said_here = Vec::new();
        for (kind, question, answer) in present
            .iter()
            .map(|(q, a)| ("present", *q, Some(*a)))
            .chain(absent.iter().map(|q| ("absent", *q, None)))
        {
            if site.asker_gone() {
                return Found::could_not_tell(crate::served::CLIENT_LEFT);
            }
            let ids = match framed_ids(&engine, &ask_for(passage, question)) {
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
            let dims = [
                ("passage", Value::text(*name)),
                ("kind", Value::text(kind)),
                ("question", Value::text(question)),
            ];
            if let Some(answer) = answer {
                let right = answer_in(&completed.text) == Some(answer);
                present_all = present_all.saturating_add(1);
                present_right = present_right.saturating_add(usize::from(right));
                rows.push(Reading::new(&dims, "right", i64::from(right), "bool"));
                said_here.push(if right { "right" } else { "WRONG" });
            } else {
                {
                    let (invented, stated) = read_absent(&completed.text);
                    absent_all = absent_all.saturating_add(1);
                    invented_all = invented_all.saturating_add(usize::from(invented));
                    stated_all = stated_all.saturating_add(usize::from(stated));
                    rows.push(Reading::new(&dims, "invented", i64::from(invented), "bool"));
                    rows.push(Reading::new(
                        &dims,
                        "stated_absent",
                        i64::from(stated),
                        "bool",
                    ));
                    said_here.push(if stated {
                        "absent"
                    } else if invented {
                        "INVENTED"
                    } else {
                        "neither"
                    });
                }
            }
            rows.push(Reading::new(
                &dims,
                "tokens",
                as_integer(completed.predicted),
                "tokens",
            ));
        }
        lines.push(format!("  {name:<10} {}", said_here.join("  ")));
    }
    lines.push(format!(
        "  present: {present_right} of {present_all} right; absent: {stated_all} of {absent_all} \
         stated absent, {invented_all} invented"
    ));
    Found {
        lines,
        fields: vec![
            ("present", Value::Integer(as_integer(present_all))),
            ("present_right", Value::Integer(as_integer(present_right))),
            ("absent", Value::Integer(as_integer(absent_all))),
            ("stated_absent", Value::Integer(as_integer(stated_all))),
            ("invented", Value::Integer(as_integer(invented_all))),
        ],
        rows,
    }
}

#[cfg(test)]
mod tests {
    use super::{PASSAGES, read_absent};

    #[test]
    fn an_invention_is_a_number_where_the_text_had_none() {
        assert_eq!(read_absent("Not stated."), (false, true));
        assert_eq!(
            read_absent("The text does not say how many."),
            (false, true)
        );
        assert_eq!(read_absent("About 4 crossings a day."), (true, false));
        assert_eq!(read_absent("I am not sure."), (false, false));
        for (name, passage, present, absent) in PASSAGES {
            assert!(
                !passage.is_empty() && !present.is_empty() && !absent.is_empty(),
                "{name}"
            );
        }
    }
}
