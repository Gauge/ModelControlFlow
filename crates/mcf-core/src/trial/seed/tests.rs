use std::collections::BTreeSet;

use super::{Draw, NotASeedSet, STANDARD, SeedSet, published};

#[test]
fn the_published_set_repeats_no_seed_over_a_long_prefix() {
    let drawn: BTreeSet<u64> = (0..100_000_u64).map(published).collect();
    assert_eq!(
        drawn.len(),
        100_000,
        "the published stream collided within its first hundred thousand trials, which means the \
         mixer is no longer a bijection (B61)"
    );
}

#[test]
fn the_published_set_is_fixed_for_life() {
    let first: Vec<u64> = (0..4).map(published).collect();
    assert_eq!(
        first,
        vec![
            16_294_208_416_658_607_535,
            10_451_216_379_200_822_465,
            10_905_525_725_756_348_110,
            2_092_789_425_003_139_053,
        ],
        "the published seed set has changed, which breaks comparability with every measurement \
         already recorded against {STANDARD} (C5, D19)"
    );
}

#[test]
fn the_published_set_is_unbounded() {
    assert_eq!(SeedSet::Standard.supply(), None);
    assert!(SeedSet::Standard.seed_for(0).is_some());
    assert!(SeedSet::Standard.seed_for(1_000_000).is_some());
}

#[test]
fn trial_i_draws_seed_i() {
    let held = SeedSet::declared("three", vec![11, 22, 33]).expect("a set");
    assert_eq!(held.seed_for(0), Some(11));
    assert_eq!(held.seed_for(1), Some(22));
    assert_eq!(held.seed_for(2), Some(33));
}

#[test]
fn a_repeated_seed_is_not_a_set() {
    let held = SeedSet::declared("repeats", vec![7, 9, 7]);
    assert_eq!(
        held,
        Err(NotASeedSet::Repeated { seed: 7, at: 2 }),
        "a repeat is named, with where it recurs"
    );
    let text = format!("{}", NotASeedSet::Repeated { seed: 7, at: 2 });
    assert!(
        text.contains("spread of zero"),
        "the refusal says what the artefact looks like: {text}"
    );
}

#[test]
fn a_single_seed_is_not_a_set() {
    assert_eq!(
        SeedSet::declared("one", vec![42]),
        Err(NotASeedSet::TooFew { given: 1 })
    );
    assert_eq!(
        SeedSet::declared("none", vec![]),
        Err(NotASeedSet::TooFew { given: 0 })
    );
}

#[test]
fn an_unnamed_set_is_refused() {
    assert_eq!(SeedSet::declared("", vec![1, 2]), Err(NotASeedSet::Unnamed));
    assert_eq!(
        SeedSet::declared("   ", vec![1, 2]),
        Err(NotASeedSet::Unnamed)
    );
}

#[test]
fn a_declared_set_runs_out_rather_than_repeating() {
    let held = SeedSet::declared("two", vec![1, 2]).expect("a set");
    assert_eq!(held.seed_for(1), Some(2));
    assert_eq!(
        held.seed_for(2),
        None,
        "the third trial has no seed, which is a fact to report"
    );
    assert_eq!(held.supply(), Some(2));
}

#[test]
fn every_set_names_itself() {
    assert_eq!(SeedSet::Standard.identifier(), STANDARD);
    assert_eq!(
        SeedSet::declared("mine", vec![1, 2])
            .expect("a set")
            .identifier(),
        "declared:mine"
    );
    assert_ne!(
        SeedSet::Standard.identifier(),
        SeedSet::declared("mine", vec![1, 2])
            .expect("a set")
            .identifier()
    );
}

#[test]
fn a_timing_draw_is_not_a_seeded_one() {
    let timing = Draw::LengthPinned {
        seed: 0,
        tokens: 256,
    };
    let behaviour = Draw::Seeded {
        seed: 0,
        from: STANDARD.to_owned(),
    };
    assert!(!timing.is_seeded());
    assert!(behaviour.is_seeded());
    assert_eq!(timing.seed(), behaviour.seed());
    assert_ne!(timing, behaviour, "the same seed under two disciplines");
    assert!(
        format!("{timing}").contains("pinned"),
        "a timing trial says what it pinned instead: {timing}"
    );
}
