//! A stand-in engine can never produce a timing.
//!
//! B65's check is `compiler`, and the compiler holds it by there being no
//! `timing` method on `Run<StandIn>`. What a type cannot check about itself is
//! that it stays that way — and the failure mode here is specific and
//! foreseeable: somebody wants one number from the stand-in, adds a method or a
//! conversion, and B65 becomes a convention.
//!
//! D31 explains why that number would be worthless: a throughput figure from a
//! naive kernel measures the naive kernel. But "worthless" is an argument, and
//! this is the mechanism.

// Every item in this file is test code; see the note in `taxonomy_agreement.rs`.
#![allow(clippy::panic)]

/// `timing` is defined on the vendored run alone.
#[test]
fn only_the_vendored_run_can_produce_a_timing() {
    let source = code_only(&engine_source());

    assert!(
        source.contains("impl Run<Vendored> {"),
        "the vendored run's own impl block is gone, so this check reads nothing"
    );
    let vendored = block(&source, "impl Run<Vendored> {");
    assert!(
        vendored.contains("pub fn timing"),
        "the timing constructor is no longer on the vendored run"
    );

    // The stand-in's own impl block must not have acquired one.
    let stand_in = block(&source, "impl Run<StandIn> {");
    for forbidden in ["fn timing", "Timing<", "-> Timing"] {
        assert!(
            !stand_in.contains(forbidden),
            "`{forbidden}` on Run<StandIn> would let a naive kernel report a speed (B65, D31)"
        );
    }

    // Nor may the generic block, which would give it to both.
    let generic = block(&source, "impl<E: Engine> Run<E> {");
    for forbidden in ["fn timing", "-> Timing"] {
        assert!(
            !generic.contains(forbidden),
            "`{forbidden}` on the generic run gives every engine a timing (B65)"
        );
    }
}

/// Nothing converts a stand-in run into a vendored one.
///
/// A stand-in run does not *become* vendored by being checked against one, in
/// exactly the way an estimate does not become a measurement (A20). Replacement
/// needs no mechanism, so the absence of one is the enforcement.
#[test]
fn nothing_converts_a_stand_in_run_into_a_vendored_one() {
    let source = code_only(&engine_source());
    for forbidden in [
        "From<Run<StandIn>>",
        "From<StandIn>",
        "Into<Run<Vendored>>",
        "fn promote",
        "fn as_vendored",
        "fn assume_vendored",
    ] {
        assert!(
            !source.contains(forbidden),
            "`{forbidden}` would let a stand-in result be read as a vendored one (B65)"
        );
    }
}

/// `Timing` has no constructor of its own, so the only way to one is through
/// the vendored run.
#[test]
fn a_timing_cannot_be_built_from_nothing() {
    let source = code_only(&engine_source());
    let timing = block(&source, "impl<Q: Quantity> Timing<Q> {");
    for forbidden in ["pub fn new", "pub const fn new", "-> Self"] {
        assert!(
            !timing.contains(forbidden),
            "`{forbidden}` on Timing is a way to one that does not pass through an engine (B65)"
        );
    }
}

/// The two engines are different types rather than one type with a flag.
///
/// A flag is a filter and a filter can be misconfigured; B9's violation, in a
/// third place.
#[test]
fn the_engines_are_types_and_not_a_flag() {
    let source = code_only(&engine_source());
    assert!(
        source.contains("pub struct Vendored;"),
        "the vendored engine is not its own type"
    );
    assert!(
        source.contains("pub struct StandIn;"),
        "the stand-in is not its own type"
    );
    for forbidden in ["is_stand_in", "enum EngineKind", "engine: Engine"] {
        assert!(
            !source.contains(forbidden),
            "`{forbidden}` turns the distinction back into a field (B65)"
        );
    }
}

/// A file with its documentation removed; the documentation names the
/// forbidden constructs in order to say they are absent.
fn code_only(source: &str) -> String {
    source
        .lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n")
}

fn engine_source() -> String {
    let path = mcf_checks::workspace::root().join("crates/mcf-core/src/engine.rs");
    std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("{} is readable: {error}", path.display()))
}

fn block(source: &str, opening: &str) -> String {
    let (_, rest) = source
        .split_once(opening)
        .unwrap_or_else(|| panic!("`{opening}` is no longer where this check looks"));
    let (body, _) = rest
        .split_once("\n}")
        .unwrap_or_else(|| panic!("`{opening}` is never closed"));
    body.to_owned()
}
