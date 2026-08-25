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
        assert!(
            constructor.contains("conditions: Conditions"),
            "a measurement can be built without its conditions: {constructor}"
        );
    }
}

fn measurement_source() -> String {
    let path = mcf_checks::workspace::root().join("crates/mcf-core/src/measurement/mod.rs");
    std::fs::read_to_string(&path).unwrap_or_else(|error| {
        panic!("{} is readable: {error}", path.display());
    })
}
