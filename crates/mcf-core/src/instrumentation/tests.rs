use crate::measurement::PartsPerMillion;

use super::{Profile, Timed};

#[test]
fn an_unwatched_run_makes_a_timing() {
    let held = Timed::new(380_u64, Profile::NothingBeyondTheClock).expect("nothing was watching");
    assert_eq!(*held.value(), 380);
    assert_eq!(held.under(), &Profile::NothingBeyondTheClock);
}

#[test]
fn a_light_profile_is_admitted_because_it_carries_its_cost() {
    let under = Profile::Light {
        watcher: "a counter read either side".to_owned(),
        residual: PartsPerMillion(3_000),
    };
    assert!(under.suits_timing());
    let held = Timed::new(380_u64, under).expect("a characterized perturbation is a condition");
    assert!(
        held.under().to_string().contains("0.3%"),
        "and the cost must be in the sentence, since a condition nobody sees is not one: {}",
        held.under()
    );
}

#[test]
fn a_deep_profile_cannot_produce_a_timing() {
    let under = Profile::Deep {
        watcher: "a sampling profiler".to_owned(),
    };
    assert!(!under.suits_timing());
    let Err(why) = Timed::new(380_u64, under) else {
        panic!("a timing under deep instrumentation is a timing of the instrument (B-164)");
    };
    assert!(why.to_string().contains("no single residual"), "{why}");
    assert!(
        why.to_string().contains("debugging observation"),
        "the refusal must say what the run *is*, not only what it is not — a deep run is \
         useful, it is just not a timing: {why}"
    );
}

#[test]
fn nothing_here_declares_an_overhead_negligible() {
    let source: String = include_str!("../instrumentation.rs")
        .lines()
        .filter(|line| {
            let trimmed = line.trim_start();
            !trimmed.starts_with("//!") && !trimmed.starts_with("///") && !trimmed.starts_with("//")
        })
        .collect::<Vec<&str>>()
        .join("\n");
    for threshold in ["negligible", "acceptable", "< 1%", "small enough"] {
        assert!(
            !source.contains(threshold),
            "`{threshold}` would be a figure MCF chose about somebody else's measurement"
        );
    }
}
