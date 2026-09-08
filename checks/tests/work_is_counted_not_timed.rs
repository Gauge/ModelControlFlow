#![allow(clippy::panic, clippy::expect_used, clippy::unwrap_used)]

#[test]
fn the_declaration_carries_no_duration() {
    let source = code_only(&read("crates/mcf-bench/src/planned.rs"));
    let (_, after) = source
        .split_once("pub struct Work {")
        .expect("`Work` is the declaration type, and it is declared in planned.rs");
    let (declaration, _) = after.split_once('}').expect("and its fields end somewhere");
    for forbidden in [
        "Duration",
        "Instant",
        "Timestamp",
        "from_secs",
        "from_millis",
        "seconds",
        "minutes",
        "deadline",
        "timeout",
    ] {
        assert!(
            !declaration.contains(forbidden),
            "`Work` mentions `{forbidden}`: a laboratory declares countable units, and a field \
             that is a duration is a declaration in minutes however it is spelled (B-224)"
        );
    }
    for counted in ["pub trials: usize", "pub arms: usize", "pub tokens: u32"] {
        assert!(
            declaration.contains(counted),
            "the declaration is missing `{counted}`: trials, arms and tokens are what a run \
             can be counted in (B-224)"
        );
    }
}

#[test]
fn the_duration_is_derived_and_banded() {
    let source = code_only(&read("crates/mcf-bench/src/planned.rs"));
    assert!(
        source.contains("fn expected(") && source.contains("Estimate<Duration<Monotonic>>"),
        "the duration must arrive as an `Estimate`, which is the type A20's wall is built from"
    );
    assert!(
        source.contains("Estimate::band("),
        "and banded: a duration predicted from a rate is a range, and one number is the \
         smallest possible version of a confident wrong number (B46)"
    );
    assert!(
        source.contains("each.basis().clone()"),
        "carrying the basis of the rate it was derived from, rather than asserting a stronger \
         one — a reader who is told the wrong basis cannot tell how much to believe it"
    );
    assert!(
        !source.contains("Estimate::point("),
        "a point estimate of a duration is exactly what B46 forbids"
    );
}

#[test]
fn an_impossible_amount_of_work_does_not_wrap() {
    let source = code_only(&read("crates/mcf-bench/src/planned.rs"));
    assert_eq!(
        source.matches("_mul(").count(),
        source.matches("saturating_mul(").count(),
        "every multiplication in the derivation must saturate: a wrapped one turns a century \
         of work into a microsecond, which is the one wrong answer that reads as good news"
    );
    assert!(
        source.matches("saturating_mul(").count() >= 4,
        "the counts and the derivation each multiply, and this check is worth nothing if \
         they stop"
    );
}

#[test]
fn the_operator_is_shown_counts_and_a_marked_estimate() {
    let rendering = code_only(&read("crates/mcf-bench/src/planned.rs"));
    let (_, after) = rendering
        .split_once("impl fmt::Display for Work")
        .expect("`Work` renders itself");
    let (shown, _) = after
        .split_once("\n}")
        .expect("and that rendering ends somewhere");
    for unit in ["second", "minute", "hour", " ms", "Duration"] {
        assert!(
            !shown.contains(unit),
            "the declaration renders `{unit}`: what a lab says it will do is countable, and \
             the minutes are a separate derived line (B-224)"
        );
    }

    let bench = code_only(&read("crates/mcf-cli/src/bench.rs"));
    assert!(
        bench.contains("an ESTIMATE from") && bench.contains("never a declaration"),
        "the derived duration must be marked as an estimate in the sentence a reader meets, \
         not only in the type (A20)"
    );
    assert!(
        bench.contains("no expected duration: {why}"),
        "and absent by name where there is no history to derive it from: a figure MCF chose \
         would be indistinguishable on the page from one it measured (A7, B46)"
    );
}

#[test]
fn a_behaviour_bound_has_nowhere_to_put_a_wall_clock() {
    let source = code_only(&read("crates/mcf-bench/src/planned.rs"));
    let (_, bound) = source
        .split_once("pub enum Bound {")
        .expect("`Bound` is what a run is bounded by");
    let (bound, _) = bound.split_once('}').expect("and it ends somewhere");
    assert!(
        bound.contains("Tokens(u64)"),
        "a behaviour laboratory's deadline is a token budget, which counts the same on a busy \
         machine and a quiet one (B-230)"
    );
    assert_eq!(
        bound.matches("Duration").count(),
        1,
        "exactly one variant may carry a duration — the timing one, whose whole subject is \
         elapsed time (§3.8). A second would be the wall clock B-230 forbids, wearing a \
         different name"
    );
    assert!(
        source.contains("if behaviour && !bound.suits_behaviour()"),
        "and the refusal must be in the constructor: a rule enforced anywhere else is a rule \
         somebody skips at four in the afternoon"
    );
}

#[test]
fn a_run_cannot_be_built_without_a_calibration() {
    let source = code_only(&read("crates/mcf-bench/src/planned.rs"));
    assert!(
        source.contains("calibrated: mcf_core::configuration::Calibrated"),
        "an evaluation on a configuration nobody calibrated is a measurement of an arbitrary \
         sampling setting wearing a model's name (B45, D13)"
    );
    for escape in [
        "impl Default for Planned",
        "fn without_calibration",
        "calibrated: Option<",
    ] {
        assert!(
            !source.contains(escape),
            "`{escape}` would make the tier ordering a convention again, and a convention is \
             what B-223 exists to replace"
        );
    }
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

fn read(relative: &str) -> String {
    let path = mcf_checks::workspace::root().join(relative);
    std::fs::read_to_string(&path).unwrap_or_else(|error| {
        panic!("{} is readable: {error}", path.display());
    })
}
