use mcf_lab::{CATALOGUE, Outcome, find, repeat, run};

#[test]
fn the_catalogue_is_not_empty() {
    assert!(!CATALOGUE.is_empty(), "the laboratory has no scenarios");
}

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

#[test]
fn running_the_catalogue_twice_leaves_no_residue() {
    let first: Vec<Outcome> = CATALOGUE.iter().map(run).collect();
    let second: Vec<Outcome> = CATALOGUE.iter().map(run).collect();
    let differed: Vec<String> = CATALOGUE
        .iter()
        .zip(first.iter().zip(second.iter()))
        .filter(|(_, (before, after))| before != after)
        .map(|(scenario, (before, after))| {
            format!(
                "{}\n     first pass: {before:?}\n     second:    {after:?}",
                scenario.id
            )
        })
        .collect();
    assert!(
        differed.is_empty(),
        "a scenario did not reproduce across two passes of the catalogue:\n  {}",
        differed.join("\n  ")
    );
}

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
