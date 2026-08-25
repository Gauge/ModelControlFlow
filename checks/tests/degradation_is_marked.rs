//! A degraded value cannot become an undegraded one.
//!
//! A5's check is `compiler`: a degraded result is a distinct type that cannot
//! be rendered or exported as an undegraded one (B-008). The compiler holds
//! that as long as the type offers no way back — no `Deref`, no
//! `From<Degraded<T>> for T`, no `unwrap`, and no rendering that omits the
//! mark. This reads the module to check that it still offers none.
//!
//! A source check rather than a compile-fail harness, for the reason given in
//! `measurement_has_one_way_in.rs`.

// Every item in this file is test code; see the note in `taxonomy_agreement.rs`.
#![allow(clippy::panic)]

/// Nothing converts a marked value back into an unmarked one implicitly.
///
/// `Deref` is the dangerous one: it would make every method of the inner type
/// reachable through the wrapper, including its `Display`, and the mark would
/// then be one autoderef away from vanishing.
#[test]
fn nothing_converts_a_degraded_value_back() {
    let source = degradation_source();
    for forbidden in [
        "impl<T> Deref for Degraded",
        "Deref for Degraded",
        "DerefMut",
        "AsRef<T> for Degraded",
        "From<Degraded<T>> for T",
        "pub fn into_inner",
        "pub fn unwrap",
        "pub fn assume_full",
    ] {
        assert!(
            !source.contains(forbidden),
            "`{forbidden}` would let a degraded value be used as an undegraded one (A5)"
        );
    }
}

/// Taking the value hands the mark back with it.
///
/// Asserted so that the check above cannot be satisfied by removing every way
/// to reach the value, which would make the type useless rather than safe.
#[test]
fn the_only_way_out_returns_the_mark_as_well() {
    let source = degradation_source();
    assert!(
        source.contains("pub fn into_parts(self) -> (T, Degradation)"),
        "the escape hatch no longer returns the mark, so this check is passing \
         against a type that has changed shape"
    );
}

/// Every combining operation returns a degraded value. If one returned a plain
/// `T`, degradation would be recorded rather than contagious, and a CPU-derived
/// input could produce an unmarked output.
#[test]
fn every_combinator_stays_degraded() {
    let source = degradation_source();
    let inside = block(&source, "impl<T> Degraded<T> {");
    for signature in public_functions(&inside) {
        let returns_plain_value = signature.contains("-> T")
            && !signature.contains("-> Degraded")
            && !signature.contains("(T, Degradation)");
        assert!(
            !returns_plain_value,
            "`{signature}` returns an unmarked value from a marked one (A5)"
        );
    }
}

/// The mark appears in the rendering. A5's violation is a CPU-derived timing
/// displayed beside accelerator-derived ones with no distinction.
#[test]
fn the_rendering_names_the_degradation() {
    let source = degradation_source();
    let display = block(
        &source,
        "impl<T: fmt::Display> fmt::Display for Degraded<T> {",
    );
    assert!(
        display.contains("self.degradation"),
        "the rendering of a degraded value no longer names its mark (A5)"
    );
}

/// The module with its documentation removed.
///
/// The documentation *names* the forbidden constructs in order to say they are
/// absent, so a check that grepped the whole file would fail on the sentence
/// explaining why it passes.
fn degradation_source() -> String {
    let path = mcf_checks::workspace::root().join("crates/mcf-core/src/degradation/mod.rs");
    let source = std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("{} is readable: {error}", path.display()));
    source
        .lines()
        .filter(|line| {
            let trimmed = line.trim_start();
            !trimmed.starts_with("//")
        })
        .collect::<Vec<_>>()
        .join("\n")
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

fn public_functions(source: &str) -> Vec<String> {
    let mut found = Vec::new();
    let mut lines = source.lines().map(str::trim).peekable();
    while let Some(line) = lines.next() {
        if !line.starts_with("pub fn ") && !line.starts_with("pub const fn ") {
            continue;
        }
        let mut signature = line.to_owned();
        while !signature.contains('{') {
            match lines.next() {
                Some(continuation) => {
                    signature.push(' ');
                    signature.push_str(continuation);
                }
                None => break,
            }
        }
        found.push(signature);
    }
    found
}
