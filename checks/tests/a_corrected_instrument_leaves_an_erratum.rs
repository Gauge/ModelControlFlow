#![allow(clippy::panic, clippy::expect_used, clippy::unwrap_used)]

use mcf_core::errata::{KNOWN, affecting};

fn read(relative: &str) -> String {
    let path = mcf_checks::workspace::root().join(relative);
    std::fs::read_to_string(&path).unwrap_or_else(|error| {
        panic!("{} is readable: {error}", path.display());
    })
}

#[test]
fn a_measurement_carries_the_digest_of_its_instrument() {
    let encoded = read("crates/mcf-record/src/encode.rs");
    assert!(
        encoded.contains("\"instrument\"") && encoded.contains("build_identity::instrument()"),
        "the condition set must carry what took the measurement, and a version string does not \
         change when an instrument does (F93, §3.4)"
    );
    assert!(
        encoded.contains("Attested::Unknown => Value::Null"),
        "and a platform that will not let MCF read its own executable yields nothing rather \
         than a placeholder (A7)"
    );

    let identity = read("crates/mcf-core/src/build_identity.rs");
    assert!(
        identity.contains("current_exe()"),
        "the digest must be of the running binary itself: no build cooperation, no git, and \
         it works for a binary shipped in a tarball"
    );
    assert!(
        identity.contains("OnceLock"),
        "and computed once, because hashing the executable per record write would make MCF \
         pay for its own honesty on every line"
    );
}

#[test]
fn an_erratum_applies_to_everything_before_it_and_nothing_after() {
    for held in KNOWN {
        assert!(
            affecting(held.corrected_at_utc_nanos.saturating_sub(1))
                .iter()
                .any(|found| found.finding == held.finding),
            "{} must apply to a measurement taken before it was corrected",
            held.finding
        );
        assert!(
            !affecting(held.corrected_at_utc_nanos)
                .iter()
                .any(|found| found.finding == held.finding),
            "{} must not apply to one taken after",
            held.finding
        );
    }
}

#[test]
fn nothing_edits_the_measurements_it_corrects() {
    let errata = read("crates/mcf-core/src/errata.rs");
    for rewriting in ["fn amend", "fn correct_entry", "fn rewrite", "fn overwrite"] {
        assert!(
            !errata.contains(rewriting),
            "`{rewriting}` would edit history to look better, and a record that does that is \
             not a record (A1)"
        );
    }
    let log = read("crates/mcf-cli/src/log.rs");
    assert!(
        log.contains("errata_for(entry.recorded_at())"),
        "an erratum must be rendered beside the entry it concerns: a reader meeting a \
         measurement is the person who needs to know, and they will not go looking (A2)"
    );
}
