#![allow(clippy::panic, clippy::expect_used)]

fn oracle() -> String {
    let path = mcf_checks::workspace::root().join("scripts/check-oracle.sh");
    std::fs::read_to_string(&path).unwrap_or_else(|error| {
        panic!("{} is readable: {error}", path.display());
    })
}

fn instructions(source: &str) -> impl Iterator<Item = &str> {
    source
        .lines()
        .map(str::trim_end)
        .filter(|line| !line.trim_start().starts_with('#'))
}

#[test]
fn a_generation_divergence_waits_for_the_other_instrument() {
    let source = oracle();
    let (before, _) = source
        .split_once("# ── the distributions")
        .expect("the distributions section is where a generation divergence is resolved");
    let (_, generation) = before
        .split_once("# ── the forward pass")
        .expect("the generation section is where the margin decides");
    for line in instructions(generation) {
        assert!(
            !line.contains("disagreements=$((disagreements + 1))"),
            "the generation section counts a disagreement from MCF's own margin, which \
             measured its own ranking rather than the disagreement on a 4B model (F107): \
             {line}"
        );
    }
    assert!(
        generation.contains(">>\"$pending\""),
        "a generation divergence must be recorded for the distribution comparison to resolve \
         (B-393, F107)"
    );
}

#[test]
fn the_distributions_resolve_what_the_margin_flagged() {
    let source = oracle();
    let (_, distributions) = source
        .split_once("# ── the distributions")
        .expect("the distributions section exists");
    assert!(
        distributions.contains("\"$pending\""),
        "the distribution comparison must look for a generation divergence waiting on its step"
    );
    assert!(
        distributions.contains(">>\"$resolved\""),
        "and must record that it resolved it, or an unresolved divergence cannot be told from \
         a resolved one (A1)"
    );
    assert!(
        distributions.contains("theirmargin"),
        "the reference's own top-two gap is the number that says whether the decision was a \
         coin flip, and MCF's margin cannot say it (F107)"
    );
}

#[test]
fn an_unresolved_divergence_is_counted_and_says_what_it_rests_on() {
    let source = oracle();
    let (_, after) = source
        .split_once("# ── the divergences nothing resolved")
        .expect("unresolved divergences are accounted for");
    assert!(
        after.contains("disagreements=$((disagreements + 1))"),
        "a divergence the better instrument never reached must still fail the check: silently \
         dropping it would make the oracle pass by having stopped testing (F103)"
    );
    assert!(
        after.contains("rests on MCF") && after.contains("rather than on both engines"),
        "and must say which instrument the verdict rests on (A5)"
    );
}
