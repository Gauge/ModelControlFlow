//! Nine of ten trials completing is nine data points (A4, B-087, §3.1).
//!
//! A4 is **absolute**, and its violation is named: *an all-or-nothing return
//! type on anything that can partially succeed.* That is a shape, and a shape
//! is checkable — which matters here more than usual, because the all-or-
//! nothing return is the *natural* thing to write. A loop that gives up on an
//! error and returns it has thrown away every trial that came before, and
//! nothing about it looks wrong.
//!
//! MCF wrote exactly that once: `mcf bench`'s runner returned
//! `Result<Comparison, String>` and discarded a hundred completed pairs when
//! the hundred-and-first request failed. This is what keeps it from coming
//! back.
//!
//! A source check rather than a compile-fail harness, for the reason given in
//! `measurement_has_one_way_in.rs`.

// Every item in this file is test code; see the note in `taxonomy_agreement.rs`.
#![allow(clippy::panic)]

/// The benchmark runner has no all-or-nothing return.
///
/// It always yields the comparison it built, however far it got, and the
/// reason it stopped travels *with* the trials rather than instead of them.
#[test]
fn the_benchmark_runner_returns_what_it_produced() {
    let source = code_only(&read("crates/mcf-cli/src/bench.rs"));
    assert!(
        source.contains(") -> Comparison<Monotonic> {"),
        "the runner must return the comparison it built, not a `Result` that discards it on the \
         first failed trial (A4)"
    );
    assert!(
        !source.contains(") -> Result<Comparison<Monotonic>, String> {"),
        "an all-or-nothing return type on something that can partially succeed is A4's own \
         description of its violation"
    );
    assert!(
        source.contains("running.stopped_short("),
        "and what stopped it must be recorded rather than returned in place of the trials"
    );
}

/// A comparison carries why it stopped, and a run that finished carries
/// nothing there.
///
/// *It finished* and *it was interrupted and nobody recorded why* are different
/// facts, and an `Option` is what keeps them apart (A7).
#[test]
fn a_comparison_says_why_it_stopped_or_says_nothing() {
    let source = code_only(&read("crates/mcf-bench/src/compare.rs"));
    assert!(
        source.contains("cut_short: Option<String>,"),
        "a comparison must be able to say why it stopped early (A4, B-087)"
    );
    assert!(
        source.contains("pub fn stopped_short("),
        "and the runner must be able to record it"
    );
    for forbidden in ["cut_short: bool", "was_interrupted: bool"] {
        assert!(
            !source.contains(forbidden),
            "`{forbidden}` loses the reason, and a partial result without its reason is a smaller \
             number nobody can explain (A2)"
        );
    }
}

/// The interruption reaches the record.
///
/// A partial result that is not written down is a partial result nobody can
/// tell from a short one that decided quickly, six weeks later (A4, B-086).
#[test]
fn the_interruption_is_written_down() {
    let source = code_only(&read("crates/mcf-bench/src/record.rs"));
    assert!(
        source.contains("\"cut_short\","),
        "the record must carry why a run stopped (A4, B-086)"
    );
    assert!(
        source.contains("map_or(Value::Null, Value::text)"),
        "and `null` must mean *it finished* rather than *nobody recorded it*"
    );
}

/// Everything that can end mid-stream keeps what arrived.
///
/// The generation path already does — eleven tokens before a runtime died are
/// eleven tokens, marked truncated — and this asserts it stays that way, since
/// it is the other half of A4's own example.
#[test]
fn a_generation_that_ended_early_keeps_its_tokens() {
    let source = code_only(&read("crates/mcf-cli/src/run.rs"));
    assert!(
        source.contains("token(s) were received from the daemon at"),
        "tokens that arrived before a stream ended must be kept and reported (A4)"
    );
    assert!(
        source.contains("produced"),
        "and counted, so a reader knows how much of the answer they have"
    );
}

/// The source with its documentation comments removed, so that a sentence
/// quoting a forbidden shape is not read as the shape itself.
fn code_only(source: &str) -> String {
    source
        .lines()
        .filter(|line| {
            let trimmed = line.trim_start();
            !trimmed.starts_with("//!") && !trimmed.starts_with("///") && !trimmed.starts_with("//")
        })
        .collect::<Vec<&str>>()
        .join("\n")
}

fn read(relative: &str) -> String {
    let path = mcf_checks::workspace::root().join(relative);
    std::fs::read_to_string(&path).unwrap_or_else(|error| {
        panic!("{} is readable: {error}", path.display());
    })
}

/// The verdict the operator was shown is the verdict the record keeps.
///
/// Not this file's rule, and found while checking it: the record was written
/// from a finding recomputed at the *default* resolution, so a caller who asked
/// about half a percent was shown one verdict and the record kept another. Two
/// answers to one question is what A6 exists to prevent, and a record that
/// disagrees with the screen is the worst place to have them.
#[test]
fn the_record_keeps_the_verdict_the_operator_was_shown() {
    let source = code_only(&read("crates/mcf-cli/src/bench.rs"));
    assert!(
        source.contains("let body = record::comparison(held, finding, method);"),
        "the record must be written from the finding the operator saw (A6)"
    );
    assert!(
        !source.contains("record::comparison(held, &held.finding(RESOLVING))"),
        "recomputing the finding here takes the default resolution rather than the one asked \
         about, which puts a different verdict in the record from the one on the screen"
    );
}
