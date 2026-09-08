use mcf_core::attested::Attested;
use mcf_core::configuration::Thousandths;

use mcf_record::json::parse;

use super::{Recommendation, WHERE, from_json};

fn json(text: &str) -> mcf_record::json::Value {
    parse(text).expect("the fixture is JSON")
}

#[test]
fn a_real_generation_config_is_read() {
    let held = from_json(&json(
        r#"{"bos_token_id":1,"eos_token_id":2,"temperature":"0.7","top_p":"0.95","top_k":50}"#,
    ));
    let Recommendation::Declared { sampling, fields } = &held else {
        panic!("a stated temperature is a recommendation: {held:?}")
    };
    assert_eq!(sampling.temperature, Attested::Known(Thousandths(700)));
    assert_eq!(sampling.top_p, Attested::Known(Thousandths(950)));
    assert_eq!(sampling.top_k, Attested::Known(50));
    assert_eq!(sampling.repetition_penalty, Attested::Unknown);
    assert_eq!(fields, &["temperature", "top_p", "top_k"]);
    assert!(
        held.describe().contains("unverified"),
        "{}",
        held.describe()
    );
}

#[test]
fn a_config_with_no_sampler_parameter_is_not_the_same_as_no_config() {
    let stated = from_json(&json(r#"{"bos_token_id":1,"eos_token_id":2}"#));
    assert_eq!(stated, Recommendation::NothingStated);
    assert_ne!(stated, Recommendation::NoneDeclared);
    assert!(stated.sampling().is_none());
    assert!(
        stated.describe().contains("looked and said nothing"),
        "{}",
        stated.describe()
    );
    assert!(
        Recommendation::NoneDeclared.describe().contains(WHERE),
        "an absence says where MCF looked, not merely that it found nothing"
    );
}

#[test]
fn an_absent_config_says_where_the_recommendation_would_be() {
    let text = Recommendation::NoneDeclared.describe();
    assert!(text.contains("base repository"), "{text}");
    assert!(text.contains("conversion repository"), "{text}");
}

#[test]
fn a_decimal_is_read_exactly() {
    for (written, expected) in [
        ("0.7", 700),
        ("0.95", 950),
        ("1", 1_000),
        ("1.0", 1_000),
        ("0.9500", 950),
        ("1.05", 1_050),
        ("0.001", 1),
    ] {
        let held = from_json(&json(&format!(r#"{{"temperature":"{written}"}}"#)));
        assert_eq!(
            held.sampling().map(|s| s.temperature),
            Some(Attested::Known(Thousandths(expected))),
            "{written} is {expected} thousandths"
        );
    }
}

#[test]
fn more_precision_than_the_unit_holds_is_refused() {
    for hostile in ["0.7001", "-0.5", "abc", "1e9", "0.7.7", ""] {
        let held = from_json(&json(&format!(r#"{{"temperature":"{hostile}"}}"#)));
        assert_eq!(
            held,
            Recommendation::NothingStated,
            "{hostile} is not a temperature, and is refused rather than repaired"
        );
    }
}

#[test]
fn a_top_k_that_is_not_a_count_is_not_read() {
    for hostile in [r#""fifty""#, "-3", "9999999999999"] {
        let held = from_json(&json(&format!(r#"{{"top_k":{hostile}}}"#)));
        assert_eq!(held, Recommendation::NothingStated, "{hostile}");
    }
}

#[test]
fn the_generation_budget_is_read_under_the_publishers_name() {
    let held = from_json(&json(r#"{"max_new_tokens":2048}"#));
    assert_eq!(
        held.sampling().map(|s| s.max_output_tokens),
        Some(Attested::Known(2_048))
    );
}
