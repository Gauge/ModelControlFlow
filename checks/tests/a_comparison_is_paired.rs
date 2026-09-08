#![allow(clippy::panic)]

#[test]
fn the_statistics_are_not_reachable_without_a_comparison() {
    let source = code_only(&bench_source("enough.rs"));
    for entry in ["over_paired_differences", "over_separate_arms"] {
        let declared = format!("pub(super) fn {entry}");
        assert!(
            source.contains(&declared),
            "`{entry}` must stay crate-internal: a public function taking bare timings is \
             block-then-subtract with extra steps (B53, B-250)"
        );
        assert!(
            !source.contains(&format!("pub fn {entry}")),
            "`{entry}` has been made public, which reopens B-250"
        );
    }
    assert!(
        !source.contains("pub fn verdict("),
        "the pre-B-250 entry point took two slices and has been removed; a function of that \
         shape cannot tell an interleaved comparison from two blocks"
    );
}

#[test]
fn every_constructor_establishes_or_disclaims_the_pairing() {
    const ADMITTED: [&str; 3] = ["Interleaving", "from_trials", "from_separate_sessions"];

    let source = code_only(&bench_source("compare.rs"));
    let mut found = Vec::new();
    let mut inside = false;
    let mut lines = source.lines().peekable();
    while let Some(line) = lines.next() {
        let trimmed = line.trim();
        if trimmed.starts_with("impl ") || trimmed.starts_with("impl<") {
            inside = trimmed.contains("Comparison<") || trimmed.contains("Interleaving<");
            continue;
        }
        if !inside || (!trimmed.starts_with("pub fn ") && !trimmed.starts_with("pub const fn ")) {
            continue;
        }
        let mut signature = trimmed.to_owned();
        while !signature.contains('{') {
            match lines.next() {
                Some(more) => {
                    signature.push(' ');
                    signature.push_str(more.trim());
                }
                None => break,
            }
        }
        let signature = signature.replace("( ", "(");
        let takes_self = signature.contains("(self")
            || signature.contains("(&self")
            || signature.contains("(mut self")
            || signature.contains("(&mut self");
        let returns_one = signature.contains("-> Self")
            || signature.contains("-> Result<Self, NotComparable>")
            || signature.contains("-> Comparison");
        if returns_one && !takes_self {
            found.push(signature);
        }
    }

    assert!(
        found.len() >= 3,
        "fewer than three ways in were found, so this check is reading the wrong file: {found:#?}"
    );
    for constructor in &found {
        assert!(
            ADMITTED.iter().any(|name| constructor.contains(name))
                || constructor.contains("fn new("),
            "a way to build a comparison that neither establishes nor disclaims the pairing: \
             {constructor} (B53, B-250)"
        );
    }
}

#[test]
fn no_trait_implementation_manufactures_a_comparison() {
    let source = code_only(&bench_source("compare.rs"));
    for forbidden in [
        "Default for Comparison",
        "From<(",
        "FromIterator",
        "Extend for Comparison",
    ] {
        assert!(
            !source.contains(forbidden),
            "`{forbidden}` would let a comparison exist without a pairing (B53, B-250)"
        );
    }
}

#[test]
fn a_cross_session_comparison_is_weaker_in_the_type() {
    let source = code_only(&bench_source("compare.rs"));
    assert!(
        source.contains("pub fn from_separate_sessions("),
        "§3.27's *it may be all that exists* must remain constructible"
    );
    assert!(
        source.contains("pub fn paired_differences(&self) -> Option<Vec<Difference>>"),
        "a comparison with no pairing must have no paired difference to report"
    );
    assert!(
        source.contains("Assembled {"),
        "the weaker construction must be labelled by a distinct strength"
    );
}

#[test]
fn blocked_trials_are_refused_by_name() {
    let source = code_only(&bench_source("compare.rs"));
    assert!(
        source.contains("RanInBlocks {"),
        "the refusal B-250 is about must exist by name"
    );
    assert!(
        source.contains("return Err(NotComparable::RanInBlocks {"),
        "the refusal must be reached, not merely declared"
    );
}

fn bench_source(file: &str) -> String {
    let path = mcf_checks::workspace::root()
        .join("crates/mcf-bench/src")
        .join(file);
    std::fs::read_to_string(&path).unwrap_or_else(|error| {
        panic!("{} is readable: {error}", path.display());
    })
}

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
