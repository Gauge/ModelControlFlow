#![allow(clippy::panic, clippy::expect_used, clippy::unwrap_used)]

use mcf_standin::languages::{DECLARED, SOURCE};

fn explain() -> String {
    let raw = std::fs::read_to_string(
        mcf_checks::workspace::root().join("crates/mcf-cli/src/explain.rs"),
    )
    .expect("explain.rs is readable");
    let mut joined = String::new();
    let mut continuing = false;
    for line in raw.lines() {
        let text = if continuing { line.trim_start() } else { line };
        continuing = text.ends_with('\\');
        joined.push_str(text.strip_suffix('\\').unwrap_or(text));
        if !continuing {
            joined.push('\n');
        }
    }
    joined
}

#[test]
fn the_sentence_set_is_declared_and_cited() {
    assert!(
        DECLARED.len() >= 10,
        "a set of a handful of European languages would answer a question nobody has"
    );
    assert!(
        SOURCE.contains("Universal Declaration of Human Rights"),
        "the sentences must cite where they came from: a ratio with an unattributed text \
         behind it is not reproducible (§II, §3.4)"
    );
    for sample in DECLARED {
        assert!(
            !sample.text.is_empty(),
            "{} has no sentence",
            sample.language
        );
        assert!(
            !sample.language.is_empty(),
            "a sentence with no language named is a number with no label"
        );
    }
    let mut named: Vec<&str> = DECLARED.iter().map(|sample| sample.language).collect();
    named.sort_unstable();
    let held = named.len();
    named.dedup();
    assert_eq!(held, named.len(), "a language appears twice");
}

#[test]
fn the_wording_keeps_the_vocabulary_and_the_model_apart() {
    let source = explain();
    assert!(
        source.contains("not a claim about the model's fluency"),
        "a vocabulary that spells a script expensively says nothing about how well the model \
         handles it, and the sentence must say so (§3.15)"
    );
    assert!(
        source.contains("a property of the file"),
        "and must name what it *is* a property of, since a reader who is told only what it is \
         not will supply the rest themselves"
    );
    assert!(
        source.contains("nothing here rates it"),
        "§3.15: MCF shows the meaningful value and does not grade it (DEC-002)"
    );
}

#[test]
fn no_word_in_the_answer_grades_a_language() {
    let source = explain();
    let (_, answer) = source
        .split_once("fn language_cost(")
        .expect("`language_cost` is what produces the table");
    let (answer, _) = answer
        .split_once("\n}\n")
        .expect("and it ends before the next item");
    for judgement in [
        "inefficient",
        "wasteful",
        "poorly",
        "badly",
        "worse",
        "better",
        "penalis",
        "penaliz",
        "should use",
        "avoid",
    ] {
        assert!(
            !answer.contains(judgement),
            "`{judgement}` grades what MCF is only supposed to show: what a vocabulary spends \
             is a fact, and the reader draws the conclusion (§3.15, DEC-002)"
        );
    }
}

#[test]
fn the_ratio_is_over_the_same_meaning() {
    let source = explain();
    assert!(
        source.contains("so every line above is the same meaning"),
        "the sentences are parallel translations and the ratio is only meaningful because of \
         it; a reader who does not know that cannot tell this from tokens-per-character"
    );
    assert!(
        source.contains("character(s)"),
        "and the character count stays on the line, so a reader who wants the per-character \
         reading can have it (A1)"
    );
}
