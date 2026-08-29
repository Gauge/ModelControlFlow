//! What reuse has to get right.

use super::{Reuse, Warmth};

/// The daemon's own words, matched rather than parsed loosely.
#[test]
fn the_daemons_states_are_read_as_it_writes_them() {
    for (said, expected) in [
        ("resident", Warmth::Warm),
        ("resident_in_server", Warmth::Warm),
        ("loaded", Warmth::Cold),
        ("loaded_for_this_request", Warmth::Cold),
        ("per_request_subprocess", Warmth::Cold),
    ] {
        assert_eq!(Warmth::from_account(Some(said)), expected, "{said}");
    }
}

/// **A state MCF has not been taught is not the one it superficially
/// resembles.** A record written by a newer build could say anything, and
/// guessing which of two it meant is A7's forbidden substitution — with the
/// added sting that the wrong guess here silently makes a mixed run look
/// uniform.
#[test]
fn a_state_mcf_does_not_know_is_unstated() {
    for said in [
        None,
        Some("not_loaded"),
        Some("resident_somewhere_else"),
        Some("loaded_from_a_cache_mcf_has_not_heard_of"),
        Some(""),
    ] {
        assert_eq!(
            Warmth::from_account(said),
            Warmth::Unstated,
            "{said:?} is not a state this build knows"
        );
    }
}

/// Trials that were all alike are a condition; trials that were not are named
/// as not.
#[test]
fn a_uniform_run_is_a_condition_and_a_mixed_one_is_a_finding() {
    let cold = Reuse::over([Warmth::Cold; 8]);
    assert_eq!(cold, Reuse::Uniform(Warmth::Cold));
    assert!(cold.is_uniform());
    assert!(cold.condition().starts_with("cold:"), "{cold}");

    let mixed = Reuse::over([Warmth::Cold, Warmth::Warm, Warmth::Cold]);
    assert_eq!(
        mixed,
        Reuse::Mixed {
            cold: 2,
            warm: 1,
            unstated: 0
        }
    );
    assert!(!mixed.is_uniform());
    assert!(
        mixed.condition().contains("not one measurement"),
        "the mixed condition says what is wrong with it: {mixed}"
    );
}

/// **One warm trial among cold ones is a mixed run**, however many there were.
/// §6.13's concern is not proportion; it is that the result depends on hidden
/// history, and one trial's worth of it is enough to make the set two things.
#[test]
fn one_trial_of_the_other_kind_makes_a_run_mixed() {
    let mut trials = vec![Warmth::Cold; 199];
    trials.push(Warmth::Warm);
    assert!(
        !Reuse::over(trials).is_uniform(),
        "one warm trial in two hundred is still two measurements"
    );
}

/// An unstated trial among stated ones is mixed too: *MCF does not know what
/// this one reused* is not the same as *it reused nothing*, and treating it as
/// agreement would be A7's substitution wearing a different hat.
#[test]
fn an_unstated_trial_among_stated_ones_is_mixed() {
    assert!(!Reuse::over([Warmth::Cold, Warmth::Unstated]).is_uniform());
    assert!(
        Reuse::over([Warmth::Unstated; 4]).is_uniform(),
        "a run MCF knows nothing about is uniform in knowing nothing, and says so"
    );
    assert!(
        Reuse::over([Warmth::Unstated; 4])
            .condition()
            .contains("did not say"),
        "and says which it is"
    );
}

/// No trials is its own answer rather than a warm run with none in it.
#[test]
fn no_trials_is_not_a_uniform_run() {
    assert_eq!(Reuse::over([]), Reuse::Nothing);
    assert_eq!(Reuse::over([]).condition(), "no trials");
}
