//! Nothing manufactures a measurement.
//!
//! A6's check is `compiler`: `Measurement<T>` cannot be constructed without
//! conditions, `n` and spread (B-005). The type is written that way — two
//! samples are positional arguments and the condition set is the last one —
//! but *staying* written that way is the part a type cannot check about
//! itself. A `Default`, a `From<Q>` or a second constructor added later would
//! reopen the hole without any existing test noticing.
//!
//! This is a source check rather than a compile-fail test because a
//! compile-fail harness is a third-party dependency, and B15 admits weight only
//! against a stated cost. Reading forty lines of one module costs nothing and
//! answers the same question.

// Every item in this file is test code; see the note in `taxonomy_agreement.rs`.
#![allow(clippy::panic)]

/// The only trait implementations `Measurement` carries are the derived ones
/// and `Display`. Each of the forbidden ones is a way to obtain a measurement
/// from something that is not a set of samples taken under conditions.
#[test]
fn no_trait_implementation_manufactures_a_measurement() {
    let source = measurement_source();
    for forbidden in [
        "impl<Q: Quantity> Default for Measurement",
        "impl Default for Measurement",
        "From<Q> for Measurement",
        "FromIterator",
    ] {
        assert!(
            !source.contains(forbidden),
            "`{forbidden}` would let a measurement exist without its conditions (A6)"
        );
    }
}

/// Every associated function that returns a `Measurement` takes a `Conditions`.
///
/// The list of constructors is read from the source rather than declared here,
/// so a constructor added without conditions fails this test rather than
/// slipping past a list nobody updated.
///
/// **A constructor may take a `&Measurement<Q>` instead**, which is the same
/// guarantee reached one step along: a thing built out of a measurement cannot
/// be built out of a bare number, and the conditions came with the measurement.
/// `Stated` is that shape — a statistic that renders with its sample count and
/// spread, added because a surface could take a percentile out of a measurement
/// and print it alone (B-073).
#[test]
fn every_constructor_takes_conditions() {
    let source = measurement_source();
    let mut constructors = Vec::new();
    let mut lines = source.lines().peekable();
    while let Some(line) = lines.next() {
        let trimmed = line.trim();
        if !trimmed.starts_with("pub fn ") && !trimmed.starts_with("pub const fn ") {
            continue;
        }
        // Gather the signature up to the opening brace, since it spans lines.
        let mut signature = trimmed.to_owned();
        while !signature.contains('{') {
            match lines.next() {
                Some(continuation) => {
                    signature.push(' ');
                    signature.push_str(continuation.trim());
                }
                None => break,
            }
        }
        if signature.contains("-> Self") || signature.contains("-> Option<Self>") {
            constructors.push(signature);
        }
    }

    assert!(
        !constructors.is_empty(),
        "no constructor was found, so this check is reading the wrong file"
    );
    for constructor in &constructors {
        // Either the conditions themselves, or the measurement that carries
        // them. `Stated` is the second kind (B-073): a statistic taken *out of*
        // a measurement, which cannot exist without one and therefore cannot
        // exist without the conditions that measurement was built with. What
        // the rule forbids is a constructor reachable from a bare value, and
        // both spellings are closed to one.
        assert!(
            constructor.contains("conditions: Conditions")
                || constructor.contains("&Measurement<Q>"),
            "a measurement can be built without its conditions: {constructor}"
        );
    }
}

/// Nothing converts between an estimate and a measurement, in either
/// direction.
///
/// A20 is absolute: an estimate can never be promoted into a measurement,
/// never compared with one, and can only be *replaced* by one. "Replaced"
/// needs no mechanism, so the enforcement is that no mechanism exists — and
/// the failure mode is a convenience added later for one call site.
#[test]
fn nothing_converts_between_an_estimate_and_a_measurement() {
    for (file, forbidden) in [
        (
            "estimate.rs",
            [
                "for Measurement",
                "-> Measurement",
                "fn measure",
                "fn promote",
            ],
        ),
        (
            "mod.rs",
            [
                "for Estimate",
                "-> Estimate",
                "fn estimate",
                "fn as_estimate",
            ],
        ),
    ] {
        let source = code_only(&source_file(file));
        for pattern in forbidden {
            assert!(
                !source.contains(pattern),
                "`{pattern}` in {file} would let an estimate and a measurement meet (A20)"
            );
        }
    }
}

/// A file with its documentation removed.
///
/// The documentation names the forbidden constructs in order to say they are
/// absent, so a check that grepped the whole file would fail on the sentence
/// explaining why it passes.
fn code_only(source: &str) -> String {
    source
        .lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n")
}

fn source_file(name: &str) -> String {
    let path = mcf_checks::workspace::root()
        .join("crates/mcf-core/src/measurement")
        .join(name);
    std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("{} is readable: {error}", path.display()))
}

/// The condition floor has no `Default`.
///
/// `Attested<T>` gained one — its default is `Unknown`, which is A7 rather than
/// an exception to it — and that made `#[derive(Default)]` viable on any struct
/// of `Attested` fields. `Floor` must not take it: the floor is a struct
/// literal precisely so that adding a condition breaks every construction site,
/// and a `Default` would let a caller silently omit the new one (§3.16, B16).
#[test]
fn the_condition_floor_has_no_default() {
    let source = code_only(&source_file("conditions.rs"));
    for forbidden in ["Default for Floor", "Default)]", "Default,"] {
        assert!(
            !source.contains(forbidden),
            "`{forbidden}` would let a condition be omitted silently (§3.3, §3.16)"
        );
    }
    assert!(
        source.contains("pub struct Floor {"),
        "this check is reading the wrong file"
    );
}

fn measurement_source() -> String {
    let path = mcf_checks::workspace::root().join("crates/mcf-core/src/measurement/mod.rs");
    std::fs::read_to_string(&path).unwrap_or_else(|error| {
        panic!("{} is readable: {error}", path.display());
    })
}
