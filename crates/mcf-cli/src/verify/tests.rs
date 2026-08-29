//! What verifying a bundle has to say, and what it has to refuse to say.

use mcf_record::json::Value;

use super::{per_cent, rerun, the_refusal, what_it_claims};

fn a_claim() -> Value {
    Value::map([(
        "body",
        Value::map([
            ("left", Value::map([("arm", Value::text("/models/a.gguf"))])),
            (
                "right",
                Value::map([("arm", Value::text("/models/b.gguf"))]),
            ),
            ("outcome", Value::map([("kind", Value::text("differ"))])),
            (
                "discipline",
                Value::map([("tokens_pinned", Value::Integer(128))]),
            ),
            (
                "method",
                Value::map([
                    ("prompt", Value::text("Once upon a time")),
                    ("resolving_ppm", Value::Integer(50_000)),
                    ("engine_asked", Value::text("provisioned")),
                    ("cold", Value::Bool(true)),
                    ("ceiling", Value::Integer(200)),
                ]),
            ),
        ]),
    )])
}

/// **The sentence B-212 exists for.** A bundle that disagrees names what
/// differs and stops: picking one of nine differences and calling it the cause
/// is A8's confound wearing a helpful voice.
#[test]
fn it_refuses_to_attribute_the_gap() {
    let said = the_refusal().join("\n");
    assert!(said.contains("will not say which"), "{said}");
    assert!(
        !said.contains("one of nine"),
        "the refusal must not state a count it has not counted (A7): {said}"
    );
    assert!(said.contains("attribution is yours"), "{said}");
    assert!(
        said.contains("A8"),
        "and cites the rule it is obeying: {said}"
    );
}

/// The method the bundle carries is what a re-run needs, and the command is
/// built from it rather than from anything this machine assumes.
#[test]
fn the_rerun_is_built_from_the_bundles_own_method() {
    let held = rerun(&a_claim());
    for expected in [
        "mcf bench /models/a.gguf",
        "--against /models/b.gguf",
        "--limit 128",
        "--resolving 5",
        "--engine provisioned",
        "--cold",
    ] {
        assert!(
            held.contains(expected),
            "`{expected}` is missing from: {held}"
        );
    }
    assert!(
        held.contains("Once upon a time"),
        "the prompt is part of the method and travels with it: {held}"
    );
}

/// A bundle that states nothing where the method should be does not have the
/// gap filled in for it (A7).
#[test]
fn an_unstated_method_is_not_invented() {
    let bare = Value::map([("body", Value::map([("outcome", Value::map::<String>([]))]))]);
    let held = rerun(&bare);
    assert!(held.contains("<model>"), "{held}");
    assert!(held.contains("<prompt>"), "{held}");

    let claims = what_it_claims(&bare).join("\n");
    assert!(
        claims.contains("— not stated"),
        "a method the bundle does not carry is said to be absent: {claims}"
    );
}

/// A percentage is rendered from parts per million without a float (A6).
#[test]
fn a_resolution_renders_without_a_float() {
    assert_eq!(per_cent(50_000), "5");
    assert_eq!(per_cent(25_000), "2.5");
    assert_eq!(per_cent(1_000), "0.1");
    assert_eq!(per_cent(1_000_000), "100");
}

/// A bundle that is not there is refused rather than treated as empty.
#[test]
fn a_bundle_that_is_not_there_is_refused() {
    let response = super::run("/nonexistent/mcf-verify-test.mcf-bundle");
    assert!(!response.served, "{}", response.text);
    assert!(
        response.text.contains("could not be read"),
        "{}",
        response.text
    );
}
