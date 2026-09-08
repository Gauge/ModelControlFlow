#![allow(clippy::panic)]

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

fn public_functions(source: &str) -> Vec<String> {
    source
        .lines()
        .map(str::trim)
        .filter(|line| line.starts_with("pub fn ") || line.starts_with("pub const fn "))
        .map(str::to_owned)
        .collect()
}

#[test]
fn an_instant_is_where_an_interval_comes_from() {
    let source = time_source("instant.rs");
    assert!(
        source.contains("pub const fn saturating_duration_since"),
        "the monotonic reading no longer produces intervals, so the check above \
         is passing vacuously"
    );
}

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
