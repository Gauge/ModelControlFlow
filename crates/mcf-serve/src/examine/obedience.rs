use mcf_record::json::Value;

use super::instructions::Constraint;
use super::tooluse::as_read;
use super::{Found, Reading, Site, as_integer};
use crate::generation::Draw;
use crate::served::{Prompt, Startup};

pub const NAME: &str = "obedience";

const BUDGET: usize = 80;

const TURNS: [&str; 5] = [
    "What is the capital of Italy?",
    "Name a colour of the rainbow.",
    "What is 9 times 8?",
    "Tell me one thing about the sea.",
    "What day comes after Monday?",
];

pub const RULES: &[(&str, &str, Constraint)] = &[
    (
        "capitals",
        "Answer every question using capital letters only. Never use a lowercase letter.",
        Constraint::Uppercase,
    ),
    (
        "closing-word",
        "End every answer with the exact word BANANA.",
        Constraint::EndsWith("BANANA"),
    ),
    (
        "no-digits",
        "Never write a digit. Write every number out in words.",
        Constraint::NoDigits,
    ),
    (
        "one-sentence",
        "Answer in exactly one sentence, always.",
        Constraint::Sentences(1),
    ),
];

fn turn(role: &str, content: &str) -> Value {
    Value::map([
        ("role", Value::text(role)),
        ("content", Value::text(content)),
    ])
}

#[must_use]
pub fn measure(site: &Site<'_>) -> Found {
    let engine = match site.server(&Startup {
        projector: None,
        ..site.startup()
    }) {
        Ok(engine) => engine,
        Err(why) => return Found::could_not_tell(&why),
    };
    let said = |failure: mcf_core::Failure| failure.detail().to_owned();
    let reply = |messages: &[Value]| -> Result<(String, usize), String> {
        let rendered = engine
            .render_with_tools(Value::List(messages.to_vec()), Value::Null)
            .map_err(said)?;
        let ids = as_read(&engine, &rendered)?;
        let completed = engine
            .complete(
                Prompt::Identifiers(&ids),
                BUDGET,
                Draw::greedy(0),
                false,
                site.timed(),
            )
            .map_err(said)?;
        Ok((completed.text, completed.predicted))
    };
    let mut rows = Vec::new();
    let mut lines = vec![format!(
        "  {} rule(s) in the system turn, each over {} user turn(s) with the model's replies \
         kept; every reply checked by a parser; greedy",
        RULES.len(),
        TURNS.len()
    )];
    let (mut held_all, mut turns_all, mut whole_rules) = (0_usize, 0_usize, 0_usize);
    for (at, (name, system, constraint)) in RULES.iter().enumerate() {
        site.progress(at, RULES.len(), name);
        if site.asker_gone() {
            return Found::could_not_tell(crate::served::CLIENT_LEFT);
        }
        let mut messages = vec![turn("system", system)];
        let mut held_here = 0_usize;
        let mut said_turns = Vec::with_capacity(TURNS.len());
        for (at, asks) in TURNS.iter().enumerate() {
            messages.push(turn("user", asks));
            let (answered, tokens) = match reply(&messages) {
                Ok(held) => held,
                Err(why) => return Found::could_not_tell(&why),
            };
            let holds = constraint.holds(&answered);
            held_here = held_here.saturating_add(usize::from(holds));
            let dims = [
                ("rule", Value::text(*name)),
                ("turn", Value::Integer(as_integer(at.saturating_add(1)))),
            ];
            rows.push(Reading::new(&dims, "held", i64::from(holds), "bool"));
            rows.push(Reading::new(&dims, "tokens", as_integer(tokens), "tokens"));
            said_turns.push(if holds { "held" } else { "BROKE" });
            messages.push(turn("assistant", answered.trim()));
        }
        held_all = held_all.saturating_add(held_here);
        turns_all = turns_all.saturating_add(TURNS.len());
        whole_rules = whole_rules.saturating_add(usize::from(held_here == TURNS.len()));
        rows.push(Reading::new(
            &[("rule", Value::text(*name))],
            "held",
            as_integer(held_here),
            "count",
        ));
        lines.push(format!("  {name:<14} {}", said_turns.join("  ")));
    }
    lines.push(format!(
        "  held in {held_all} of {turns_all} turn(s); {whole_rules} of {} rule(s) held every turn",
        RULES.len()
    ));
    Found {
        lines,
        fields: vec![
            ("rules", Value::Integer(as_integer(RULES.len()))),
            ("turns", Value::Integer(as_integer(turns_all))),
            ("held", Value::Integer(as_integer(held_all))),
            ("whole", Value::Integer(as_integer(whole_rules))),
        ],
        rows,
    }
}

#[cfg(test)]
mod tests {
    use super::RULES;

    #[test]
    fn every_rule_is_checked_by_its_own_constraint() {
        let breaks = [
            "Rome, of course.",
            "Rome.",
            "It is 72.",
            "Rome. It is old. It rains.",
        ];
        for ((name, system, constraint), broken) in RULES.iter().zip(breaks) {
            assert!(!system.is_empty(), "{name}");
            assert!(
                !constraint.holds(broken),
                "{name} should not hold {broken:?}"
            );
        }
    }
}
