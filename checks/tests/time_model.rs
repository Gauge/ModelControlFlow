//! Wall-clock arithmetic has no spelling.
//!
//! B37's violation is `end_wall - start_wall`, which silently reports an NTP
//! correction as latency, and B-184's condition is that it does not compile.
//! The type is written so that it cannot — `Timestamp` implements no
//! arithmetic at all — but *staying* written that way is what a type cannot
//! check about itself. An `impl Sub for Timestamp` added later for the
//! convenience of one call site would reopen it silently.
//!
//! A source check rather than a compile-fail harness, for the reason given in
//! `measurement_has_one_way_in.rs`: a harness is a third-party dependency and
//! B15 admits weight only against a stated cost.

// Every item in this file is test code; see the note in `taxonomy_agreement.rs`.
#![allow(clippy::panic)]

/// `Timestamp` implements no arithmetic operator. Subtracting two moments is
/// the mistake; adding, negating and the assigning forms are the same mistake
/// wearing different punctuation.
#[test]
fn a_timestamp_has_no_arithmetic() {
    let source = time_source("timestamp.rs");
    for forbidden in [
        "Sub for Timestamp",
        "Sub<",
        "Add for Timestamp",
        "Add<",
        "SubAssign",
        "AddAssign",
        "Neg for Timestamp",
    ] {
        assert!(
            !source.contains(forbidden),
            "`{forbidden}` would let two wall-clock readings be subtracted (B37)"
        );
    }
}

/// Nor does it offer the same thing under a method name. `duration_since` on a
/// wall-clock reading is the operator with the punctuation filed off.
///
/// Only *public* functions are examined. `Timestamp::now` reads the system
/// clock through `SystemTime::duration_since(UNIX_EPOCH)`, which is how a
/// moment is obtained rather than how two moments are subtracted — the
/// difference the check has to make, and the reason it reads signatures rather
/// than grepping the file.
#[test]
fn a_timestamp_offers_no_interval_method() {
    for signature in public_functions(&time_source("timestamp.rs")) {
        for forbidden in ["duration_since", "elapsed", "difference", "interval"] {
            assert!(
                !signature.contains(forbidden),
                "`{signature}` is wall-clock arithmetic under another name (B37)"
            );
        }
    }
}

/// Every `pub fn` and `pub const fn` signature in a source file.
fn public_functions(source: &str) -> Vec<String> {
    source
        .lines()
        .map(str::trim)
        .filter(|line| line.starts_with("pub fn ") || line.starts_with("pub const fn "))
        .map(str::to_owned)
        .collect()
}

/// An interval comes from a monotonic reading, and that is where the method
/// lives. Asserted so that the check above cannot be satisfied by deleting the
/// capability altogether.
#[test]
fn an_instant_is_where_an_interval_comes_from() {
    let source = time_source("instant.rs");
    assert!(
        source.contains("pub const fn saturating_duration_since"),
        "the monotonic reading no longer produces intervals, so the check above \
         is passing vacuously"
    );
}

/// A duration's clock is a type parameter, not a field. If it were a field, a
/// simulated interval and a real one would be the same type and A11 would be a
/// review comment again.
#[test]
fn the_clock_is_in_the_type_and_not_in_a_field() {
    let source = time_source("duration.rs");
    assert!(
        source.contains("pub struct Duration<K: ClockKind>"),
        "the clock is no longer part of the duration's type (A11)"
    );
    for forbidden in ["kind: ClockKind", "kind: Kind", "simulated: bool"] {
        assert!(
            !source.contains(forbidden),
            "`{forbidden}` makes the clock a value, which lets a simulated \
             interval be assigned where a real one belongs (A11)"
        );
    }
}

fn time_source(file: &str) -> String {
    let path = mcf_checks::workspace::root()
        .join("crates/mcf-core/src/time")
        .join(file);
    std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("{} is readable: {error}", path.display()))
}
