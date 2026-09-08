use mcf_core::measurement::Basis;

use super::{NoBand, Point, band, between};

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

#[test]
fn a_projection_is_a_band_read_between_measured_points() {
    let held = band(&history(), 150, 128).expect("150 sits between 100 and 200");
    assert_eq!(held.band().low().as_nanos(), 1_500);
    assert_eq!(held.band().high().as_nanos(), 1_800);
    assert_eq!(held.band().basis(), &Basis::LocalHistory);
}

#[test]
fn a_measured_size_projects_to_what_was_measured() {
    let held = band(&history(), 200, 128).expect("200 is measured");
    assert_eq!(held.band().low().as_nanos(), 2_000);
    assert_eq!(held.band().high().as_nanos(), 2_400);
}

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

#[test]
fn one_point_is_not_enough_to_read_between() {
    let held = band(&history()[..1], 100, 128);
    assert_eq!(
        held.map(|_| ()),
        Err(NoBand::TooLittleHistory { points: 1 })
    );
}

#[test]
fn reading_between_two_points_is_exact_at_them() {
    assert_eq!(between(100, 1_000, 200, 2_000, 100), 1_000);
    assert_eq!(between(100, 1_000, 200, 2_000, 200), 2_000);
    assert_eq!(between(100, 1_000, 200, 2_000, 150), 1_500);
    assert_eq!(between(100, 2_000, 200, 1_000, 150), 1_500);
    assert_eq!(between(100, 1_000, 100, 9_999, 100), 1_000);
}

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

#[test]
fn the_projection_is_scored_against_what_was_measured() {
    let held = super::score(&history());
    assert_eq!(held.unscorable, 2);
    assert_eq!(held.scored(), 1);
    assert_eq!(held.inside, 1, "the middle point is where the line says");
    assert_eq!(held.worst, mcf_core::measurement::PartsPerMillion(0));
    assert!(
        format!("{held}").contains("could not be scored"),
        "and the unscorable ones are said rather than dropped: {held}"
    );
}

#[test]
fn a_miss_is_counted_and_measured() {
    let mut held = history();
    if let Some(middle) = held.get_mut(1) {
        middle.fastest = 4_000;
        middle.slowest = 4_800;
    }
    let scored = super::score(&held);
    assert_eq!(scored.outside, 1);
    assert_eq!(scored.inside, 0);
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
