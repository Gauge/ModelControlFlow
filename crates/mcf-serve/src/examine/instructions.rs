use mcf_record::json::Value;

use super::{Found, Reading, Site, as_integer, framed_ids, timed};
use crate::generation::{Draw, Truncation};
use crate::served::{Prompt, Startup};
use mcf_core::configuration::Thousandths;

pub const NAME: &str = "instructions";

pub const TRIALS: usize = 3;

const TEMPERATURE: u32 = 700;

const BUDGET: usize = 120;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Constraint {
    Words(usize),
    NoDigits,
    Uppercase,
    Lines(usize),
    LinesBegin(&'static str),
    LinesNumbered,
    EndsWith(&'static str),
    Sentences(usize),
}

impl Constraint {
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Words(_) => "words",
            Self::NoDigits => "no_digits",
            Self::Uppercase => "uppercase",
            Self::Lines(_) => "lines",
            Self::LinesBegin(_) => "lines_begin",
            Self::LinesNumbered => "lines_numbered",
            Self::EndsWith(_) => "ends_with",
            Self::Sentences(_) => "sentences",
        }
    }

    #[must_use]
    pub fn holds(self, said: &str) -> bool {
        let said = said.trim();
        let lines: Vec<&str> = said
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .collect();
        match self {
            Self::Words(want) => said.split_whitespace().count() == want,
            Self::NoDigits => !said.chars().any(|c| c.is_ascii_digit()),
            Self::Uppercase => !said.chars().any(char::is_lowercase),
            Self::Lines(want) => lines.len() == want,
            Self::LinesBegin(start) => {
                !lines.is_empty() && lines.iter().all(|line| line.starts_with(start))
            }
            Self::LinesNumbered => {
                !lines.is_empty()
                    && lines
                        .iter()
                        .all(|line| line.chars().next().is_some_and(|c| c.is_ascii_digit()))
            }
            Self::EndsWith(ending) => said.ends_with(ending),
            Self::Sentences(want) => {
                said.chars()
                    .filter(|c| matches!(c, '.' | '!' | '?'))
                    .count()
                    == want
            }
        }
    }
}

#[derive(Debug)]
pub struct Ask {
    pub name: &'static str,
    pub asks: &'static str,
    pub constraints: &'static [Constraint],
}

pub const ASKS: &[Ask] = &[
    Ask {
        name: "twelve-words",
        asks: "Describe a bicycle in exactly 12 words. Answer with those words only.",
        constraints: &[Constraint::Words(12)],
    },
    Ask {
        name: "three-numbered",
        asks: "Name three colours as a numbered list: exactly three lines, each starting with \
               its number, and nothing else.",
        constraints: &[Constraint::Lines(3), Constraint::LinesNumbered],
    },
    Ask {
        name: "no-digits",
        asks: "In one sentence, say how many days a week has, without using any digit. Answer \
               with the sentence only.",
        constraints: &[Constraint::NoDigits, Constraint::Sentences(1)],
    },
    Ask {
        name: "uppercase",
        asks: "Write one sentence about rain using capital letters only. Answer with the \
               sentence only.",
        constraints: &[Constraint::Uppercase, Constraint::Sentences(1)],
    },
    Ask {
        name: "closing-phrase",
        asks: "Say in one sentence what a kettle does, then end your answer with the exact \
               words: That is all.",
        constraints: &[Constraint::EndsWith("That is all.")],
    },
    Ask {
        name: "five-upper-no-digits",
        asks: "Give exactly five words about winter, all in capital letters, with no digits. \
               Answer with those words only.",
        constraints: &[
            Constraint::Words(5),
            Constraint::Uppercase,
            Constraint::NoDigits,
        ],
    },
    Ask {
        name: "four-dashed",
        asks: "List exactly four fruits, one per line, each line starting with a dash, and \
               nothing else.",
        constraints: &[Constraint::Lines(4), Constraint::LinesBegin("-")],
    },
    Ask {
        name: "two-sentences",
        asks: "Write exactly two sentences about the moon. Answer with the sentences only.",
        constraints: &[Constraint::Sentences(2)],
    },
];

#[must_use]
#[allow(
    clippy::too_many_lines,
    reason = "one measurement read straight through: each ask, each trial, each constraint a row"
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
        "  {} ask(s), {TRIALS} trial(s) each (the first greedy, the rest at temperature {}); \
         every constraint a parser's check",
        ASKS.len(),
        TEMPERATURE
    )];
    let (mut held_all, mut trials, mut constraints_held, mut constraints_asked) =
        (0_usize, 0_usize, 0_usize, 0_usize);
    for (at, ask) in ASKS.iter().enumerate() {
        site.progress(at, ASKS.len(), ask.name);
        let ids = match framed_ids(&engine, ask.asks) {
            Ok(ids) => ids,
            Err(why) => return Found::could_not_tell(&why),
        };
        let mut said = Vec::with_capacity(TRIALS);
        for trial in 0..TRIALS {
            if site.asker_gone() {
                return Found::could_not_tell(crate::served::CLIENT_LEFT);
            }
            let draw = if trial == 0 {
                Draw::greedy(0)
            } else {
                Draw {
                    seed: u64::try_from(trial).unwrap_or(0),
                    temperature: Thousandths(TEMPERATURE),
                    truncation: Truncation::OFF,
                }
            };
            let (done, ns) = timed(|| {
                engine.complete(Prompt::Identifiers(&ids), BUDGET, draw, false, site.timed())
            });
            let completed = match done {
                Ok(completed) => completed,
                Err(failure) => return Found::could_not_tell(failure.detail()),
            };
            let dims = [
                ("ask", Value::text(ask.name)),
                ("trial", Value::Integer(as_integer(trial))),
            ];
            let mut held = 0_usize;
            for constraint in ask.constraints {
                let holds = constraint.holds(&completed.text);
                held = held.saturating_add(usize::from(holds));
                rows.push(Reading::new(
                    &[
                        ("ask", Value::text(ask.name)),
                        ("trial", Value::Integer(as_integer(trial))),
                        ("constraint", Value::text(constraint.name())),
                    ],
                    "held",
                    i64::from(holds),
                    "bool",
                ));
            }
            let all = held == ask.constraints.len();
            trials = trials.saturating_add(1);
            held_all = held_all.saturating_add(usize::from(all));
            constraints_held = constraints_held.saturating_add(held);
            constraints_asked = constraints_asked.saturating_add(ask.constraints.len());
            rows.push(Reading::new(
                &dims,
                "constraints_held",
                as_integer(held),
                "count",
            ));
            rows.push(Reading::new(
                &dims,
                "constraints",
                as_integer(ask.constraints.len()),
                "count",
            ));
            rows.push(Reading::new(&dims, "all_held", i64::from(all), "bool"));
            rows.push(Reading::new(
                &dims,
                "words",
                as_integer(completed.text.split_whitespace().count()),
                "count",
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
            said.push(format!("{held}/{}", ask.constraints.len()));
        }
        lines.push(format!("  {:<22} {}", ask.name, said.join("   ")));
    }
    lines.push(format!(
        "  every constraint held in {held_all} of {trials} trial(s); {constraints_held} of \
         {constraints_asked} constraint(s) held in all"
    ));
    Found {
        lines,
        fields: vec![
            ("asks", Value::Integer(as_integer(ASKS.len()))),
            ("trials", Value::Integer(as_integer(trials))),
            ("all_held", Value::Integer(as_integer(held_all))),
            (
                "constraints_held",
                Value::Integer(as_integer(constraints_held)),
            ),
            (
                "constraints_asked",
                Value::Integer(as_integer(constraints_asked)),
            ),
        ],
        rows,
    }
}

#[cfg(test)]
mod tests {
    use super::{ASKS, Constraint};

    #[test]
    fn each_constraint_is_a_parsers_check() {
        assert!(Constraint::Words(3).holds("one  two\nthree"));
        assert!(!Constraint::Words(3).holds("one two"));
        assert!(Constraint::NoDigits.holds("seven days"));
        assert!(!Constraint::NoDigits.holds("7 days"));
        assert!(Constraint::Uppercase.holds("IT RAINS, 2 DAYS."));
        assert!(!Constraint::Uppercase.holds("It RAINS"));
        assert!(Constraint::Lines(3).holds("1. red\n\n2. green\n3. blue\n"));
        assert!(!Constraint::Lines(3).holds("1. red\n2. green"));
        assert!(Constraint::LinesNumbered.holds("1. red\n2. green"));
        assert!(!Constraint::LinesNumbered.holds("- red\n2. green"));
        assert!(Constraint::LinesBegin("-").holds("- apple\n- pear"));
        assert!(!Constraint::LinesBegin("-").holds(""));
        assert!(Constraint::EndsWith("That is all.").holds("A kettle boils water. That is all.\n"));
        assert!(!Constraint::EndsWith("That is all.").holds("That is all. Thanks."));
        assert!(Constraint::Sentences(2).holds("The moon is far. It is bright!"));
        assert!(!Constraint::Sentences(2).holds("The moon is far."));
    }

    #[test]
    fn every_ask_names_its_own_constraints() {
        for ask in ASKS {
            assert!(!ask.constraints.is_empty(), "{}", ask.name);
            for constraint in ask.constraints {
                let named = match constraint {
                    Constraint::Words(n) | Constraint::Lines(n) | Constraint::Sentences(n) => {
                        let words = [
                            "", "one", "two", "three", "four", "five", "six", "seven", "eight",
                            "nine", "ten", "eleven", "twelve",
                        ];
                        ask.asks.contains(&n.to_string())
                            || words.get(*n).is_some_and(|w| ask.asks.contains(w))
                    }
                    Constraint::NoDigits => ask.asks.contains("digit"),
                    Constraint::Uppercase => ask.asks.contains("capital"),
                    Constraint::LinesBegin(start) => ask.asks.contains("dash") && *start == "-",
                    Constraint::LinesNumbered => ask.asks.contains("numbered"),
                    Constraint::EndsWith(ending) => ask.asks.contains(ending),
                };
                assert!(named, "{}: {constraint:?}", ask.name);
            }
        }
    }
}
