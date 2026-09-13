use super::{Hunt, Way};
use crate::dial::{Dial, Step};

fn values(steps: &[Step]) -> Vec<u32> {
    steps
        .iter()
        .map(|step| match *step {
            Step::Whole(held) | Step::Thousandths(held) => held,
        })
        .collect()
}

#[test]
fn a_hunt_opens_on_the_coarse_ladder_and_nothing_finer() {
    let hunt = Hunt::started(Dial::MicroBatch);
    assert_eq!(values(&hunt.asked()), values(&Dial::MicroBatch.coarse()));
    assert_eq!(hunt.round(), 1);
    assert!(!hunt.settled());
}

#[test]
fn the_opening_gap_is_the_widest_the_coarse_ladder_leaves() {
    let hunt = Hunt::started(Dial::MicroBatch);
    assert_eq!(
        hunt.gap(),
        2048,
        "2048 to 4096 is the widest gap in that ladder, so that is what halving starts from"
    );
}

#[test]
fn closing_in_asks_either_side_of_the_best_at_half_the_gap() {
    let mut hunt = Hunt::started(Dial::MicroBatch);
    let next = hunt.closed_in_on(Step::Whole(1024), &Dial::MicroBatch.coarse());
    assert_eq!(
        values(&next),
        vec![1536],
        "1024 either side lands on values the coarse ladder already ran, so it halves \
         again and asks 1536; 512 below is the ladder's own"
    );
    assert_eq!(
        hunt.gap(),
        512,
        "it halved twice to find somewhere new to look"
    );
}

#[test]
fn a_round_that_lands_only_on_values_already_run_halves_again_rather_than_giving_up() {
    let mut hunt = Hunt::started(Dial::MicroBatch);
    let gap = hunt.gap();
    let next = hunt.closed_in_on(Step::Whole(1024), &Dial::MicroBatch.coarse());
    assert!(
        !next.is_empty(),
        "a value with nothing new immediately beside it is not a settled hunt"
    );
    assert!(hunt.gap() < gap);
    assert!(!hunt.settled());
}

#[test]
fn a_value_already_run_is_never_asked_for_twice() {
    let mut hunt = Hunt::started(Dial::MicroBatch);
    let first = hunt.closed_in_on(Step::Whole(1024), &Dial::MicroBatch.coarse());
    for step in &first {
        assert!(
            !Dial::MicroBatch.coarse().contains(step),
            "the coarse ladder is already run and must not come back"
        );
    }
    let again = hunt.closed_in_on(Step::Whole(1024), &Dial::MicroBatch.coarse());
    for step in &again {
        assert!(!first.contains(step), "a round never repeats the last one");
    }
}

#[test]
fn a_hunt_settles_once_the_gap_can_no_longer_be_halved() {
    let mut hunt = Hunt::started(Dial::MicroBatch);
    let mut run: Vec<Step> = Dial::MicroBatch.coarse();
    for _ in 0..40 {
        let next = hunt.closed_in_on(Step::Whole(1024), &run);
        if next.is_empty() {
            break;
        }
        run.extend(next);
    }
    assert!(hunt.settled(), "halving cannot go on forever");
    assert!(
        hunt.gap() >= Dial::MicroBatch.span().finest,
        "it never proposes a gap finer than the dial can actually be set to"
    );
}

#[test]
fn a_settled_hunt_asks_for_nothing_more() {
    let mut hunt = Hunt::started(Dial::DraftDepth);
    let mut run: Vec<Step> = Dial::DraftDepth.coarse();
    loop {
        let next = hunt.closed_in_on(Step::Whole(4), &run);
        if next.is_empty() {
            break;
        }
        run.extend(next);
    }
    assert!(hunt.settled());
    assert!(hunt.closed_in_on(Step::Whole(4), &run).is_empty());
}

#[test]
fn closing_in_on_the_floor_never_proposes_below_it() {
    let mut hunt = Hunt::started(Dial::MicroBatch);
    let floor = Dial::MicroBatch.span().floor;
    let mut run: Vec<Step> = Dial::MicroBatch.coarse();
    for _ in 0..40 {
        let next = hunt.closed_in_on(Step::Whole(floor), &run);
        if next.is_empty() {
            break;
        }
        for step in &next {
            let value = match *step {
                Step::Whole(held) | Step::Thousandths(held) => held,
            };
            assert!(value >= floor, "{value} is below the floor {floor}");
        }
        run.extend(next);
    }
}

#[test]
fn closing_in_on_the_ceiling_never_proposes_above_it() {
    let mut hunt = Hunt::started(Dial::Temperature);
    let ceiling = Dial::Temperature.span().ceiling;
    let mut run: Vec<Step> = Dial::Temperature.coarse();
    for _ in 0..40 {
        let next = hunt.closed_in_on(Step::Thousandths(ceiling), &run);
        if next.is_empty() {
            break;
        }
        for step in &next {
            let value = match *step {
                Step::Whole(held) | Step::Thousandths(held) => held,
            };
            assert!(value <= ceiling, "{value} is above the ceiling {ceiling}");
        }
        run.extend(next);
    }
}

#[test]
fn every_dial_can_be_hunted_to_a_settlement() {
    for dial in Dial::ALL {
        let mut hunt = Hunt::started(dial);
        let mut run: Vec<Step> = dial.coarse();
        let middle = dial.coarse().get(1).copied().unwrap_or(dial.step_of(0));
        let mut rounds = 0;
        loop {
            let next = hunt.closed_in_on(middle, &run);
            if next.is_empty() {
                break;
            }
            run.extend(next);
            rounds += 1;
            assert!(rounds < 64, "{} never settles", dial.label());
        }
        assert!(hunt.settled(), "{} does not settle", dial.label());
    }
}

#[test]
fn the_automatic_way_is_the_one_offered_first() {
    assert_eq!(
        Way::ALL.first(),
        Some(&Way::Halving),
        "closing in automatically is the usual way to dial something in"
    );
}
