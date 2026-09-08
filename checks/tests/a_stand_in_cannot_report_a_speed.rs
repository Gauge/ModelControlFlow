#![allow(clippy::panic)]

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

    let stand_in = block(&source, "impl Run<StandIn> {");
    for forbidden in ["fn timing", "Timing<", "-> Timing"] {
        assert!(
            !stand_in.contains(forbidden),
            "`{forbidden}` on Run<StandIn> would let a naive kernel report a speed (B65, D31)"
        );
    }

    let generic = block(&source, "impl<E: Engine> Run<E> {");
    for forbidden in ["fn timing", "-> Timing"] {
        assert!(
            !generic.contains(forbidden),
            "`{forbidden}` on the generic run gives every engine a timing (B65)"
        );
    }
}

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
