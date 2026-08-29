//! Tests for trials and series.
//!
//! What is being checked is that everything a later question needs is present
//! in the trials themselves — §3.27's pairing above all — and that a thinned
//! series cannot be mistaken for a full one.

use super::{Arm, Draw, Position, STANDARD, SeedSet, Series, SessionId, Thinning, Trial, Trials};
use crate::build_identity::BuildIdentity;
use crate::measurement::{Conditions, Count, Floor};

fn conditions() -> Conditions {
    Conditions::new(BuildIdentity::current(), Floor::nothing_known())
}

/// What trial `index` drew, from MCF's published set (B61, D19).
fn drew(index: usize) -> Draw {
    Draw::Seeded {
        seed: SeedSet::Standard.seed_for(index).unwrap_or(0),
        from: STANDARD.to_owned(),
    }
}

fn session() -> SessionId {
    SessionId::new("2026-08-25T10-00-00Z")
}

/// Six trials, interleaved A, B, A, B, A, B — which is what B53 requires and
/// what the position field exists to make legible.
fn interleaved() -> Trials<Count> {
    let a = Arm::new("Q4_K_M");
    let b = Arm::new("Q5_K_M");
    Trials::from((0..6).map(|position| {
        let arm = if position % 2 == 0 {
            a.clone()
        } else {
            b.clone()
        };
        // The A arm reads lower every time; the pairing is what should show it.
        let value = if position % 2 == 0 {
            10 + position
        } else {
            20 + position
        };
        Trial::new(
            Count(value),
            arm,
            Position(u32::try_from(position).unwrap_or(0)),
            session(),
            drew(usize::try_from(position).unwrap_or(0)),
        )
    }))
}

/// Every trial carries its arm, its position and its session, and there is no
/// way to build one without them.
#[test]
fn a_trial_carries_everything_a_later_question_needs() {
    let trials = interleaved();
    let first = trials.all().first().expect("six trials");
    assert_eq!(first.arm(), &Arm::new("Q4_K_M"));
    assert_eq!(first.position(), Position(0));
    assert_eq!(first.session(), &session());
    assert_eq!(first.value(), Count(10));
}

/// §3.27: paired analysis from the trials alone. The k-th trial of one arm
/// with the k-th of the other — the two that saw the same afternoon.
#[test]
fn arms_pair_by_their_place_in_the_interleaving() {
    let trials = interleaved();
    let paired = trials.paired_with(&Arm::new("Q4_K_M"), &Arm::new("Q5_K_M"));
    assert_eq!(paired.len(), 3);
    assert_eq!(paired.unpaired, 0);
    assert_eq!(paired.left_was_smaller(), [true, true, true]);
    // The pairs are in interleaving order, so drift over the session is
    // visible rather than averaged away.
    let positions: Vec<(u32, u32)> = paired
        .pairs
        .iter()
        .map(|(l, r)| (l.position().0, r.position().0))
        .collect();
    assert_eq!(positions, [(0, 1), (2, 3), (4, 5)]);
}

/// A1: a trial with no partner is not silently dropped. It is excluded from the
/// pairing and counted, because the fact that it happened is information.
#[test]
fn an_unpaired_trial_is_counted_rather_than_forgotten() {
    let a = Arm::new("A");
    let b = Arm::new("B");
    let trials = Trials::from([
        Trial::new(Count(1), a.clone(), Position(0), session(), drew(0)),
        Trial::new(Count(2), b.clone(), Position(1), session(), drew(1)),
        Trial::new(Count(3), a.clone(), Position(2), session(), drew(2)),
    ]);
    let paired = trials.paired_with(&a, &b);
    assert_eq!(paired.len(), 1);
    assert_eq!(paired.unpaired, 1);
    assert!(!trials.is_balanced());
}

/// A balanced set is what an interleaved session produces, and an unbalanced
/// one is the first sign something stopped early.
#[test]
fn balance_is_reported_rather_than_evened_up() {
    assert!(interleaved().is_balanced());
}

/// B56: a summary is projected from trials, never stored in place of them. The
/// measurement comes out of the trials and the trials are still there.
#[test]
fn a_measurement_is_projected_from_the_trials_that_remain() {
    let trials = interleaved();
    let measured = trials
        .measure(&Arm::new("Q4_K_M"), conditions())
        .expect("three trials");
    assert_eq!(measured.n(), 3);
    assert_eq!(measured.spread().median, Count(12));
    assert_eq!(trials.all().len(), 6, "projecting consumed the trials");
}

/// §3.4: an arm with one trial has no measurement, and the honest answer is
/// that there is not one rather than a measurement of one.
#[test]
fn an_arm_with_one_trial_has_no_measurement() {
    let a = Arm::new("A");
    let trials = Trials::from([Trial::new(
        Count(1),
        a.clone(),
        Position(0),
        session(),
        drew(0),
    )]);
    assert!(trials.measure(&a, conditions()).is_none());
    assert_eq!(trials.of_arm(&a).len(), 1, "the trial is still kept");
}

/// The arms present come from the trials, so a set of trials describes its own
/// comparison rather than needing to be told what it is.
#[test]
fn the_arms_are_read_from_the_trials() {
    assert_eq!(
        interleaved().arms(),
        [Arm::new("Q4_K_M"), Arm::new("Q5_K_M")]
    );
}

// ---------------------------------------------------------------------------
// B-271 — a thinned series cannot be read as a full one.
// ---------------------------------------------------------------------------

/// The points and the resolution come back together, always. A caller holding
/// a bare slice could not tell a thinned series from a full one.
#[test]
fn points_are_never_returned_without_their_resolution() {
    let series = Series::new((0..10).map(Count), Thinning::FULL);
    let (points, thinning) = series.points();
    assert_eq!(points.len(), 10);
    assert!(thinning.is_full_resolution());
    assert!(series.is_full_resolution());
}

/// Thinning keeps every nth point and says so.
#[test]
fn a_thinned_series_says_what_was_done_to_it() {
    let series = Series::new((0..10).map(Count), Thinning::FULL);
    let thinned = series.thinned(2).expect("two is a factor");
    assert_eq!(thinned.kept(), 5);
    assert_eq!(thinned.thinning().factor(), 2);
    assert!(!thinned.is_full_resolution());
    assert!(thinned.to_string().contains("every 2th point"), "{thinned}");
}

/// Factors compose. A series thinned by three and then by two is thinned by
/// six, and reporting only the last factor would claim a resolution it does not
/// have.
#[test]
fn thinning_a_thinned_series_composes_the_factors() {
    let series = Series::new((0..30).map(Count), Thinning::FULL);
    let twice = series
        .thinned(3)
        .and_then(|once| once.thinned(2))
        .expect("both are factors");
    assert_eq!(twice.thinning().factor(), 6);
    assert!(!twice.is_full_resolution());
}

/// A factor of zero is not a factor: it would describe a series built from no
/// points at all.
#[test]
fn zero_is_not_a_thinning_factor() {
    assert!(Thinning::every(0).is_none());
    assert_eq!(Thinning::every(1), Some(Thinning::FULL));
    let series = Series::new([Count(1)], Thinning::FULL);
    assert!(series.thinned(0).is_none());
}

/// The rendering says the count and the resolution and never one alone.
#[test]
fn the_rendering_carries_the_resolution() {
    let full = Series::new((0..3).map(Count), Thinning::FULL);
    assert_eq!(full.to_string(), "3 points, full resolution");
    assert_eq!(Thinning::FULL.to_string(), "full resolution");
}
