//! The arithmetic and the readers, pinned against values a reader can
//! check by hand; nothing here starts an engine.
#![allow(clippy::panic, clippy::expect_used, clippy::unwrap_used)]

use std::path::Path;

use mcf_record::json::Value;

use super::{FAMILIES, MEASURES, as_ms, per_cent, per_second, planned, ppm};

/// Every measurement is in exactly one family, and every family's
/// measurements are ones the run makes.
#[test]
fn every_measurement_is_in_one_family() {
    let mut seen = std::collections::BTreeSet::new();
    for (_, members) in FAMILIES {
        for member in members {
            assert!(
                MEASURES.contains(member),
                "{member} is in a family and not in the run"
            );
            assert!(seen.insert(*member), "{member} is in two families");
        }
    }
    assert_eq!(seen.len(), MEASURES.len(), "a measurement is in no family");
}

/// Naming some keeps the run's order and drops what is not named.
#[test]
fn a_plan_keeps_the_runs_order() {
    let asked = vec![
        "determinism".to_owned(),
        "offload-curve".to_owned(),
        "nothing-of-the-kind".to_owned(),
    ];
    assert_eq!(planned(&asked), vec!["offload-curve", "determinism"]);
    assert_eq!(planned(&[]).len(), MEASURES.len());
}

/// The figures are whole numbers read the way a person would.
#[test]
fn the_figures_are_read_by_hand() {
    assert_eq!(as_ms(1_234_567), "1.2");
    assert_eq!(per_cent(123_456), "12.3%");
    assert_eq!(per_cent(-5_000), "0.5%");
    assert_eq!(ppm(1, 4), 250_000);
    assert_eq!(ppm(1, 0), 0);
    assert_eq!(per_second(1024, 500_000_000), 2048);
    assert_eq!(per_second(1, 0), 0);
}

/// Precision is read off a file's name, and the reference is the most
/// precise sibling above the model's own.
#[test]
fn the_reference_is_the_most_precise_sibling() {
    use super::fidelity::{precision_of, reference_among};
    let at = |name: &str| std::path::PathBuf::from(format!("/store/repo/{name}"));
    assert_eq!(precision_of(&at("Model-UD-Q4_K_XL.gguf")), Some(4));
    assert_eq!(precision_of(&at("Model-UD-IQ1_M.gguf")), Some(1));
    assert_eq!(precision_of(&at("Model-BF16.gguf")), Some(16));
    assert_eq!(precision_of(&at("Model-Q8_0.gguf")), Some(8));
    assert_eq!(precision_of(&at("Model.gguf")), None);
    let siblings = vec![
        at("Model-UD-IQ1_M.gguf"),
        at("Model-UD-Q4_K_XL.gguf"),
        at("Model-Q8_0.gguf"),
    ];
    assert_eq!(
        reference_among(&at("Model-UD-IQ1_M.gguf"), &siblings),
        Some(at("Model-Q8_0.gguf"))
    );
    assert_eq!(
        reference_among(&at("Model-Q8_0.gguf"), &siblings),
        None,
        "the most precise file has no reference above it"
    );
}

/// A divergence is the first position that differs, or the shorter
/// length where one run is a prefix of the other.
#[test]
fn a_divergence_is_where_two_runs_part() {
    use super::determinism::{distinct_of, divergence_at};
    assert_eq!(divergence_at(&[1, 2, 3], &[1, 2, 3]), None);
    assert_eq!(divergence_at(&[1, 2, 3], &[1, 9, 3]), Some(1));
    assert_eq!(divergence_at(&[1, 2, 3], &[1, 2]), Some(2));
    assert_eq!(distinct_of(&[vec![1], vec![2], vec![1]]), 2);
    assert_eq!(distinct_of(&[]), 0);
}

/// Repeats and loops are counted, not judged.
#[test]
fn repeats_and_loops_are_counted() {
    use super::degeneration::{loop_onset, repeated_by_hundred};
    let fresh: Vec<usize> = (0..200).collect();
    assert_eq!(repeated_by_hundred(&fresh), vec![0, 0]);
    assert_eq!(loop_onset(&fresh), None);
    let looped: Vec<usize> = (0..40).chain(0..40).chain(0..40).collect();
    // Every four-token run in the second hundred recurs from the first
    // forty tokens on, since the hundred is the tail of one cycle and a
    // whole one.
    let shares = repeated_by_hundred(&looped);
    assert_eq!(shares.len(), 2);
    assert!(shares[0] > 500_000, "{shares:?}");
    assert_eq!(
        loop_onset(&looped),
        Some(72),
        "the block ending at 72 repeats 40 tokens earlier"
    );
}

/// The fact is planted where it was asked for, and the question follows.
#[test]
fn the_fact_is_planted_where_asked() {
    use super::retrieval::planted;
    let text = planted(10, 50, "483921");
    assert!(text.contains("The secret number is 483921."));
    let at = text.find("483921").unwrap();
    let before = text.get(..at).unwrap().matches("The path along").count();
    assert_eq!(before, 5, "half of ten repeats come before the fact");
    assert!(text.ends_with("Answer with the digits only."));
    assert!(planted(10, 0, "1").starts_with("The secret number is 1."));
}

/// Conformance is a parser's answer.
#[test]
fn conformance_is_read_by_a_parser() {
    use super::grammar::conforms;
    assert!(conforms(r#"{"name": "Ada", "age": 36, "city": "London"}"#));
    assert!(conforms(
        "Sure:\n```json\n{\"name\": \"Ada\", \"age\": 36, \"city\": \"London\"}\n```"
    ));
    assert!(!conforms(
        r#"{"name": "Ada", "age": "36", "city": "London"}"#
    ));
    assert!(!conforms(r#"{"name": "Ada", "city": "London"}"#));
    assert!(!conforms("Ada is 36 and lives in London."));
}

/// What changed in a round trip is named.
#[test]
fn a_changed_round_trip_is_named() {
    use super::tokenizer::changed;
    assert_eq!(
        changed("the end ", "the end"),
        "whitespace at an end was lost"
    );
    assert_eq!(changed("a  b", "a b"), "whitespace inside was changed");
    assert_eq!(changed("naïve", "na?ve"), "came back as \"na?ve\"");
}

/// A recorded finding reads back as a sentence, and a measurement that
/// could not tell says so.
#[test]
fn a_recorded_finding_reads_back() {
    let body = Value::map([
        ("method", Value::text("quantization-fidelity")),
        ("reference", Value::text("Model-Q8_0.gguf")),
        ("agreed", Value::Integer(90)),
        ("positions", Value::Integer(96)),
        ("millibits_per_token", Value::Integer(81)),
    ]);
    assert_eq!(
        super::recorded_said(&body).as_deref(),
        Some("agreed with Model-Q8_0.gguf at 90 of 96 position(s); 81 millibits a token")
    );
    let could_not = Value::map([
        ("method", Value::text("cold-start")),
        ("could_not_tell", Value::text("another process holds it")),
    ]);
    assert_eq!(
        super::recorded_said(&could_not).as_deref(),
        Some("could not tell: another process holds it")
    );
    let not_ours = Value::map([("method", Value::text("chat-template"))]);
    assert_eq!(super::recorded_said(&not_ours), None);
}

/// A call is read in either form a template writes, its arguments with
/// it, and judged against what the task expects by exact match (B-517).
#[test]
fn a_call_is_read_in_either_form_and_judged_exactly() {
    use super::tooluse::call_in;
    let object = call_in(
        "Sure. <tool_call>{\"name\": \"get_weather\", \"arguments\": {\"city\": \"Paris\"}}</tool_call>",
    )
    .expect("an object call reads");
    assert_eq!(object.name, "get_weather");
    assert_eq!(object.arguments.get("city"), Some(&Value::text("Paris")));
    let block = call_in(
        "<function=set_alarm>\n<parameter=hour>\n6\n</parameter>\n<parameter=minute>\n45\n</parameter>\n</function>",
    )
    .expect("a function block reads");
    assert_eq!(block.name, "set_alarm");
    assert_eq!(block.arguments.get("hour"), Some(&Value::Integer(6)));
    assert_eq!(block.arguments.get("minute"), Some(&Value::Integer(45)));
    let strung = call_in(
        "{\"name\": \"calculate\", \"arguments\": \"{\\\"expression\\\": \\\"1234 * 5678\\\"}\"}",
    )
    .expect("arguments as a string read");
    assert_eq!(
        strung.arguments.get("expression"),
        Some(&Value::text("1234 * 5678"))
    );
    assert_eq!(call_in("The weather is fine today."), None);
    let two = super::tooluse::calls_in(
        "<tool_call>{\"name\": \"get_weather\", \"arguments\": {\"city\": \"Paris\"}}</tool_call>\n<tool_call>{\"name\": \"get_weather\", \"arguments\": {\"city\": \"Tokyo\"}}</tool_call>",
    );
    assert_eq!(two.len(), 2, "{two:?}");
    assert_eq!(two[1].arguments.get("city"), Some(&Value::text("Tokyo")));
    let thought =
        call_in("<think>I could call it.</think>{\"name\": \"get_time\", \"arguments\": {}}")
            .expect("the answer after the thought reads");
    assert_eq!(thought.name, "get_time");
}

/// The siblings of a model are the model files beside it: not a
/// projector, and of a sharded file only its first part.
#[test]
fn siblings_are_the_model_files_beside_it() {
    let dir = std::env::temp_dir().join(format!("mcf-examine-siblings-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    for name in [
        "M-Q4_K_M.gguf",
        "M-Q8_0.gguf",
        "mmproj-F16.gguf",
        "M-BF16-00001-of-00002.gguf",
        "M-BF16-00002-of-00002.gguf",
        "notes.txt",
    ] {
        std::fs::write(dir.join(name), b"x").unwrap();
    }
    let found: Vec<String> = super::fidelity::siblings_of(&dir.join("M-Q4_K_M.gguf"))
        .iter()
        .map(|held| held.file_name().unwrap().to_string_lossy().into_owned())
        .collect();
    let _gone = std::fs::remove_dir_all(&dir);
    assert_eq!(
        found,
        vec!["M-BF16-00001-of-00002.gguf", "M-Q4_K_M.gguf", "M-Q8_0.gguf"]
    );
    let _ = Path::new("/");
}
