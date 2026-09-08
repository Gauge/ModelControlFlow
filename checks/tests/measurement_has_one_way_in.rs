#![allow(clippy::panic)]

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
            constructor.contains("conditions: Conditions")
                || constructor.contains("&Measurement<Q>"),
            "a measurement can be built without its conditions: {constructor}"
        );
    }
}

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
