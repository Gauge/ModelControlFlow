//! Every scenario produces what it claims, identically, a hundred times over.
//!
//! B-009's condition, and §3.17's requirement: *a failure found once reproduces
//! exactly, forever.* B27 holds the laboratory to every rule that governs the
//! rest of the workspace, because everything else is believed on its authority
//! — a sloppy simulator produces confident wrong results.
//!
//! B19 keeps this hermetic: no network, no accelerator, no model. Everything a
//! scenario builds, it builds in a temporary directory it removes.

use mcf_lab::{CATALOGUE, Outcome, find, repeat, run};

/// There is something to check. A catalogue that had emptied would make every
/// test below pass by having nothing to disagree with (A19).
#[test]
fn the_catalogue_is_not_empty() {
    assert!(!CATALOGUE.is_empty(), "the laboratory has no scenarios");
}

/// Every scenario produces the category it declares. A scenario that quietly
/// started producing something else would still pass a test that only asked
/// whether *a* failure came out (A21's shape: declared and observed are
/// different things, and the difference is the finding).
#[test]
fn every_scenario_produces_what_it_declares() {
    for scenario in CATALOGUE {
        let outcome = run(scenario);
        assert!(
            outcome.matches(scenario.produces),
            "{} claims {} and produced {outcome}",
            scenario.id,
            scenario.produces,
        );
    }
}

/// §3.17: a hundred runs, a hundred identical outcomes. This is the property
/// the whole laboratory rests on — B-009's stated condition, and the reason
/// determinism is a feature of the lab rather than of the world (B27).
#[test]
fn every_scenario_reproduces_identically_across_a_hundred_runs() {
    for scenario in CATALOGUE {
        let repetition = repeat(scenario, 100);
        assert!(
            repetition.is_deterministic(),
            "{}: {}",
            scenario.id,
            repetition.statement()
        );
        assert_eq!(repetition.runs, 100);
    }
}

/// B21: the laboratory rebuilds a failure from its record, so every failure a
/// scenario produces carries enough context to be worth recording. A failure
/// with nothing attached is one nobody can act on.
#[test]
fn every_produced_failure_carries_its_context() {
    for scenario in CATALOGUE {
        let Outcome::Produced(failure) = run(scenario) else {
            continue;
        };
        assert!(
            !failure.context().is_empty(),
            "{} produced a failure with no context",
            scenario.id
        );
        assert!(!failure.detail().is_empty(), "{}", scenario.id);
        assert!(!failure.subsystem().as_str().is_empty(), "{}", scenario.id);
    }
}

/// Identifiers are unique and stable-looking. C5 makes them stable for life,
/// because a failure record names the scenario that rebuilds it.
#[test]
fn scenario_identifiers_are_unique_and_well_formed() {
    let mut ids: Vec<&str> = CATALOGUE.iter().map(|s| s.id).collect();
    let count = ids.len();
    ids.sort_unstable();
    ids.dedup();
    assert_eq!(ids.len(), count, "two scenarios share an identifier");
    for scenario in CATALOGUE {
        assert!(
            scenario.id.contains('/'),
            "{} is not named <area>/<what>",
            scenario.id
        );
        assert!(!scenario.summary.is_empty(), "{} says nothing", scenario.id);
    }
}

/// A category with more than one scenario returns all of them. Two different
/// failures wearing one code are two things a reader may need to tell apart.
#[test]
fn a_category_returns_every_scenario_that_produces_it() {
    for scenario in CATALOGUE {
        let found = find(scenario.produces);
        assert!(
            found.iter().any(|s| s.id == scenario.id),
            "{} is not found under its own category",
            scenario.id
        );
        for other in &found {
            assert_eq!(other.produces, scenario.produces);
        }
    }
}

/// B58, A27: a scenario leaves nothing behind. Running the whole catalogue
/// twice must leave no residue for the second pass to trip over — and the
/// second pass agreeing with the first is how that is observable.
#[test]
fn running_the_catalogue_twice_leaves_no_residue() {
    let first: Vec<Outcome> = CATALOGUE.iter().map(run).collect();
    let second: Vec<Outcome> = CATALOGUE.iter().map(run).collect();
    assert_eq!(first, second, "a scenario contaminated the next run");
}

/// A11: nothing here can become a performance number. The laboratory's clock
/// is a different type from the monotonic one, so this is a compiler check —
/// what is recorded here is that a scenario's clock is the simulated one and
/// says so.
#[test]
fn the_laboratory_clock_is_simulated() {
    use mcf_core::time::{Clock as _, Duration, Simulated};
    let clock = mcf_lab::clock();
    let start = clock.now();
    clock.advance(1_000_000_000);
    let elapsed: Duration<Simulated> = clock.now().saturating_duration_since(start);
    assert_eq!(elapsed.as_nanos(), 1_000_000_000);
    assert!(elapsed.is_simulated());
}
