//! A benchmark has no pass condition, and refuses what it cannot measure.

use super::{bench_where, is_a_stand_in, per_cent_of};
use mcf_core::measurement::PartsPerMillion;

/// A model MCF will not find, so the refusal is about the reference rather
/// than about the machine.
const NOWHERE: &str = "/nonexistent/mcf-bench-test/model.gguf";

/// A benchmark with no daemon is a refusal, not a result — and it says which
/// command starts one.
#[test]
fn without_a_daemon_it_refuses_rather_than_reporting() {
    let response = bench_where(None, NOWHERE, NOWHERE, "hello", None, 0, None, None, false);
    assert!(!response.served);
    assert!(
        response.text.contains("there is no model at"),
        "the first thing wrong is said first: {}",
        response.text
    );
}

/// And where the models exist but nothing is listening, the refusal is about
/// the daemon.
#[test]
fn a_missing_daemon_is_named() {
    let scratch = std::env::temp_dir().join(format!("mcf-bench-{}.gguf", std::process::id()));
    let _written = std::fs::write(&scratch, b"not a model, and never read");
    let named = scratch.display().to_string();
    let response = bench_where(None, &named, &named, "hello", None, 0, None, None, false);
    let _removed = std::fs::remove_file(&scratch);
    assert!(!response.served);
    assert!(
        response.text.contains("none is listening") && response.text.contains("mcf serve"),
        "{}",
        response.text
    );
}

/// **B65 by name.** MCF's own stand-in is written to be read rather than to be
/// fast, so a timing taken from it measures the stand-in — and a benchmark
/// must refuse rather than mark, because a marked number is a number somebody
/// will quote without its mark.
#[test]
fn a_stand_in_is_recognized_by_name() {
    for named in [
        "MCF's own stand-in, build 0.1.0-m0",
        "stand-in",
        "STAND-IN",
        "a stand in",
    ] {
        assert!(is_a_stand_in(named), "{named} is a stand-in");
    }
    for named in ["llama.cpp @ 925e1179947e", "provisioned", "vendored engine"] {
        assert!(!is_a_stand_in(named), "{named} is not");
    }
}

/// A percentage is read as two integers, because this crate holds no float
/// (A6) — and more precision than the unit admits is refused rather than
/// rounded away (A7).
#[test]
fn a_resolution_is_read_without_a_float() {
    assert_eq!(per_cent_of("5"), Some(PartsPerMillion(50_000)));
    assert_eq!(per_cent_of("2.5"), Some(PartsPerMillion(25_000)));
    assert_eq!(per_cent_of("0.1"), Some(PartsPerMillion(1_000)));
    assert_eq!(per_cent_of("100"), Some(PartsPerMillion(1_000_000)));
    assert_eq!(per_cent_of("2.55"), None, "one decimal place is the unit");
    assert_eq!(
        per_cent_of("0"),
        None,
        "a zero difference is not a question"
    );
    assert_eq!(per_cent_of("five"), None);
}

/// `--cold` is how an operator asks for a uniform run when the warm path
/// cannot give one (§6.13, F65) — so it must reach the runner, and the flag
/// must be the thing that decides it rather than a default nobody set.
#[test]
fn asking_for_a_cold_run_is_an_argument_and_not_a_default() {
    let source = include_str!("../bench.rs");
    assert!(
        source.contains("let (left_identifiers, right_identifiers) = if cold {"),
        "the flag must decide whether identifiers are sent, which is what decides whether the \
         request reaches the server that holds a model (B-376, F65)"
    );
    assert!(
        source.contains("(None, None)"),
        "and a cold run sends text, which is a fresh process per request and therefore uniform"
    );
}

/// A benchmark that could not produce a uniform run says what to do about it,
/// rather than leaving an operator with a refusal and no next step (§3.9).
#[test]
fn a_mixed_run_says_what_to_do_about_it() {
    let source = include_str!("../bench.rs");
    for said in [
        "`--cold` makes every trial load the model",
        "comparing a model with itself is uniform too",
        "resident at a time and a paired comparison alternates them",
    ] {
        assert!(
            source.contains(said),
            "the advice after a mixed run must name a way forward: `{said}` is gone"
        );
    }
}
