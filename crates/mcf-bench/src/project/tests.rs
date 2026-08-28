//! What a projection has to get right, and what it has to refuse.

use mcf_core::measurement::Basis;

use super::{NoBand, Point, band, between};

/// What this machine has measured, in the shape F67's frontier produced: one
/// budget, several sizes, each with the fastest and slowest trial seen.
fn history() -> Vec<Point> {
    vec![
        Point {
            bytes: 100,
            tokens: 128,
            fastest: 1_000,
            slowest: 1_200,
            competing: None,
        },
        Point {
            bytes: 200,
            tokens: 128,
            fastest: 2_000,
            slowest: 2_400,
            competing: None,
        },
        Point {
            bytes: 400,
            tokens: 128,
            fastest: 4_000,
            slowest: 4_800,
            competing: None,
        },
    ]
}

/// A projection is a **band** and it is read between the two measured points
/// the file sits between (B46, B-214).
#[test]
fn a_projection_is_a_band_read_between_measured_points() {
    let held = band(&history(), 150, 128).expect("150 sits between 100 and 200");
    assert_eq!(held.band().low().as_nanos(), 1_500);
    assert_eq!(held.band().high().as_nanos(), 1_800);
    assert_eq!(held.band().basis(), &Basis::LocalHistory);
}

/// It lands exactly on a measured point where the file *is* one.
#[test]
fn a_measured_size_projects_to_what_was_measured() {
    let held = band(&history(), 200, 128).expect("200 is measured");
    assert_eq!(held.band().low().as_nanos(), 2_000);
    assert_eq!(held.band().high().as_nanos(), 2_400);
}

/// **Absent where there is no history** — and absent where the history is all
/// on one side. Projecting past the data is extrapolation, and a straight line
/// beyond it is the confident wrong number B46 names; F67 measured that the
/// relationship it would rest on is not straight at the ends.
#[test]
fn outside_what_was_measured_there_is_no_band() {
    for asked in [50_u64, 401, 10_000] {
        let held = band(&history(), asked, 128);
        assert_eq!(
            held.map(|_| ()),
            Err(NoBand::OutsideWhatWasMeasured {
                smallest: 100,
                largest: 400
            }),
            "{asked} bytes is outside 100–400 and is not extrapolated to"
        );
    }
    let text = format!(
        "{}",
        NoBand::OutsideWhatWasMeasured {
            smallest: 100,
            largest: 400
        }
    );
    assert!(text.contains("confident wrong number"), "{text}");
}

/// **Two requests of different lengths are two different things.** A budget
/// this machine has no history at gets no band, and the refusal says which
/// budgets it does have.
#[test]
fn a_budget_with_no_history_gets_no_band() {
    let held = band(&history(), 150, 512);
    assert_eq!(
        held.map(|_| ()),
        Err(NoBand::NoHistoryAtThatBudget {
            tokens: 512,
            instead: vec![128]
        })
    );
    let text = format!(
        "{}",
        NoBand::NoHistoryAtThatBudget {
            tokens: 512,
            instead: vec![128]
        }
    );
    assert!(
        text.contains("128"),
        "the refusal names what is there: {text}"
    );
    assert!(text.contains("two different things"), "{text}");
}

/// And a machine that has measured nothing says so, rather than saying it has
/// no history *at that budget* — which would imply another budget would work.
#[test]
fn a_machine_that_measured_nothing_says_so() {
    let held = band(&[], 150, 128);
    assert_eq!(
        held.map(|_| ()),
        Err(NoBand::NoHistoryAtThatBudget {
            tokens: 128,
            instead: Vec::new()
        })
    );
    let text = format!(
        "{}",
        NoBand::NoHistoryAtThatBudget {
            tokens: 128,
            instead: Vec::new()
        }
    );
    assert!(text.contains("nothing to project from"), "{text}");
}

/// One point cannot bracket anything.
#[test]
fn one_point_is_not_enough_to_read_between() {
    let held = band(&history()[..1], 100, 128);
    assert_eq!(
        held.map(|_| ()),
        Err(NoBand::TooLittleHistory { points: 1 })
    );
}

/// The interpolation is integer arithmetic, exact at the ends and monotone
/// between them (A6: this crate holds no float).
#[test]
fn reading_between_two_points_is_exact_at_them() {
    assert_eq!(between(100, 1_000, 200, 2_000, 100), 1_000);
    assert_eq!(between(100, 1_000, 200, 2_000, 200), 2_000);
    assert_eq!(between(100, 1_000, 200, 2_000, 150), 1_500);
    // Downward too: nothing here assumes the later point is the larger one.
    assert_eq!(between(100, 2_000, 200, 1_000, 150), 1_500);
    // Two points at one size have nothing to read between.
    assert_eq!(between(100, 1_000, 100, 9_999, 100), 1_000);
}

/// **A20's wall, restated where it is easy to forget.** A projection is an
/// `Estimate` and a measurement is a `Measurement`; there is no conversion in
/// either direction, so this test is about the shape rather than the values —
/// if it ever stops compiling for the right reason, that is the finding.
#[test]
fn a_projection_is_an_estimate_and_carries_its_basis() {
    let held = band(&history(), 300, 128).expect("300 sits between 200 and 400");
    assert_eq!(
        held.band().basis(),
        &Basis::LocalHistory,
        "and says so, because A20's *clearly-labelled* is not satisfied by a type name nobody sees"
    );
    assert!(held.band().low() <= held.band().high(), "a band is ordered");
    assert!(
        format!("{}", held.band()).contains("estimate"),
        "and renders as one: {}",
        held.band()
    );
}

/// **B-385.** A band says what it was read between, and *nothing recorded* is
/// not *nothing competing*.
#[test]
fn a_band_carries_the_conditions_of_what_it_rests_on() {
    let held = band(&history(), 300, 128).expect("300 sits between 200 and 400");
    assert_eq!(
        held.rested_on().busiest(),
        None,
        "history that recorded no machine reading yields no figure, not a zero"
    );
    assert!(
        held.rested_on()
            .to_string()
            .contains("unknown and not quiet"),
        "and says so: {}",
        held.rested_on()
    );

    let busy: Vec<Point> = history()
        .iter()
        .enumerate()
        .map(|(at, point)| Point {
            competing: (at == 0).then_some(33_050).or(Some(150)),
            ..*point
        })
        .collect();
    let held = band(&busy, 300, 128).expect("300 sits between 200 and 400");
    let shown = held.rested_on().to_string();
    assert!(shown.contains("0.15 core(s)"), "{shown}");
    assert_eq!(held.rested_on().busiest(), Some(150));
}

/// **§6.16 turned on the projection** (B-215). Every measurement is checked
/// against the band that would have been projected for it from the others,
/// which needs no stored predictions and tracks as the history grows.
#[test]
fn the_projection_is_scored_against_what_was_measured() {
    let held = super::score(&history());
    // Three points: the two at the ends have nothing on one side of them and
    // cannot be scored, which is not a miss — counting them would be scoring
    // the refusal to extrapolate, which is the thing the model gets right.
    assert_eq!(held.unscorable, 2);
    assert_eq!(held.scored(), 1);
    assert_eq!(held.inside, 1, "the middle point is where the line says");
    assert_eq!(held.worst, mcf_core::measurement::PartsPerMillion(0));
    assert!(
        format!("{held}").contains("could not be scored"),
        "and the unscorable ones are said rather than dropped: {held}"
    );
}

/// A point the model gets wrong is counted, and the size of the miss is
/// reported — a score that only counted hits would be a score nobody could act
/// on (§3.4).
#[test]
fn a_miss_is_counted_and_measured() {
    let mut held = history();
    // The middle point is twice what a straight line between its neighbours
    // predicts.
    if let Some(middle) = held.get_mut(1) {
        middle.fastest = 4_000;
        middle.slowest = 4_800;
    }
    let scored = super::score(&held);
    assert_eq!(scored.outside, 1);
    assert_eq!(scored.inside, 0);
    // The band its neighbours would have projected is 2000..2400; it measured
    // 4000..4800, so it overshot the band's upper edge by two thirds of it.
    assert_eq!(
        scored.worst,
        mcf_core::measurement::PartsPerMillion(666_666),
        "the miss is measured against the edge it missed, not against the value"
    );
    assert!(
        format!("{scored}").contains("worst miss"),
        "and the size of it is reported: {scored}"
    );
}

/// A history nothing can be read between is *not scored*, which does not read
/// as *scored and perfect*.
#[test]
fn a_history_with_nothing_to_read_between_is_not_scored() {
    let held = super::score(&history()[..1]);
    assert_eq!(held.scored(), 0);
    assert_eq!(held.unscorable, 1);
    let text = format!("{held}");
    assert!(text.starts_with("not scored:"), "{text}");
    assert!(
        !text.contains("fall inside"),
        "nothing scored must not read as everything passing: {text}"
    );
}
