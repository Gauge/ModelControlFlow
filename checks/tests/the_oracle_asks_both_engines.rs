//! The oracle's generation verdict is not decided by MCF's own ranking alone
//! (B-393, B-368, F107, F27, A19, A5).
//!
//! **What went wrong.** The generation comparison decides whether a divergence
//! is a defect by looking at MCF's *own* margin — the gap between the token MCF
//! chose and its runner-up. F27 measured that threshold on models up to 1.7B
//! and it held four times. On a 4B model it produced a false positive: at the
//! step where the two engines parted, MCF's margin was 0.71 and the
//! **reference's** own top-two gap was 0.0199. The decision was a coin flip;
//! MCF's logits, shifted by about a third of a logit in each direction — well
//! inside the arithmetic the distribution instrument tolerates — turned that tie
//! into a clear win *in MCF's ranking*, and the margin measured the win.
//!
//! **The better instrument was running beside it and disagreed.** The
//! distributions section compared the same model, prompt and step and reported
//! KL 0.0520 against a floor of 0.20 — the two engines agreeing. Two
//! instruments, one question, and the weaker one decided the verdict.
//!
//! So a generation divergence is recorded and resolved by the distribution
//! comparison at that step. One that nothing resolves is still a disagreement,
//! and the output says which instrument it rests on — A5's rule about a
//! degraded result, applied to a check's own confidence.

// Every item in this file is test code; see the note in `taxonomy_agreement.rs`.
#![allow(clippy::panic, clippy::expect_used)]

fn oracle() -> String {
    let path = mcf_checks::workspace::root().join("scripts/check-oracle.sh");
    std::fs::read_to_string(&path).unwrap_or_else(|error| {
        panic!("{} is readable: {error}", path.display());
    })
}

/// Lines that are not comments, which is where a rule can be broken.
fn instructions(source: &str) -> impl Iterator<Item = &str> {
    source
        .lines()
        .map(str::trim_end)
        .filter(|line| !line.trim_start().starts_with('#'))
}

/// The generation section records a divergence rather than counting one.
#[test]
fn a_generation_divergence_waits_for_the_other_instrument() {
    let source = oracle();
    let (before, _) = source
        .split_once("# ── the distributions")
        .expect("the distributions section is where a generation divergence is resolved");
    // Everything above the distributions section is the tokenizer and
    // generation halves. The tokenizer's disagreements are exact — identifiers
    // agree or they do not — and are counted where they are found.
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

/// And the distributions section resolves what is waiting.
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

/// A divergence nothing resolved is still a disagreement, and says so.
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
