use mcf_desk::{Act, Desk, Tab};
use mcf_optimize::dial::{Dial, Step};

fn desk() -> Desk {
    Desk::new(std::path::PathBuf::from("/tmp/mcf-optimize-test.sock"))
}

#[test]
fn optimize_sits_between_configure_and_statistics() {
    assert_eq!(Tab::ALL.len(), 4);
    assert_eq!(Tab::ALL.get(1).map(|tab| tab.label()), Some("Optimize"));
}

#[test]
fn picking_a_dial_replaces_the_values_with_that_dials_own() {
    let mut desk = desk();
    desk.act(Act::Dial(1));
    assert_eq!(desk.optimizing.sweep.dial, Dial::Temperature);
    assert!(
        desk.optimizing
            .sweep
            .steps
            .iter()
            .all(|step| matches!(step, Step::Thousandths(_))),
        "temperature is swept over decimals, not whole tokens"
    );
}

#[test]
fn a_value_is_turned_off_and_on_again_without_disturbing_the_others() {
    let mut desk = desk();
    let all = desk.optimizing.sweep.steps.len();
    desk.act(Act::SweepValue(0));
    assert_eq!(desk.optimizing.sweep.steps.len(), all - 1);
    desk.act(Act::SweepValue(0));
    assert_eq!(desk.optimizing.sweep.steps.len(), all);
}

#[test]
fn a_test_set_is_turned_off_and_on_again() {
    let mut desk = desk();
    assert_eq!(desk.optimizing.sweep.sets.len(), 8);
    desk.act(Act::TestSet(3));
    assert!(!desk.optimizing.sweep.sets.contains(&3));
    desk.act(Act::TestSet(3));
    assert!(desk.optimizing.sweep.sets.contains(&3));
    assert!(
        desk.optimizing.sweep.sets.windows(2).all(|two| two
            .first()
            .zip(two.get(1))
            .is_none_or(|(one, next)| one < next)),
        "the sets stay in order"
    );
}

#[test]
fn repeats_cycle_one_two_three_and_back() {
    let mut desk = desk();
    assert_eq!(desk.optimizing.sweep.repeats, 1);
    desk.act(Act::Repeats);
    assert_eq!(desk.optimizing.sweep.repeats, 2);
    desk.act(Act::Repeats);
    assert_eq!(desk.optimizing.sweep.repeats, 3);
    desk.act(Act::Repeats);
    assert_eq!(desk.optimizing.sweep.repeats, 1);
}

#[test]
fn a_sweep_with_nothing_held_is_refused_and_says_why() {
    let mut desk = desk();
    desk.act(Act::Sweep);
    assert!(!desk.optimizing.running, "nothing was started");
    let Some(why) = &desk.optimizing.refused else {
        panic!("a refusal says why");
    };
    assert!(why.contains("host one first"), "{why}");
}

#[test]
fn a_sweep_with_no_values_chosen_is_refused_before_it_looks_for_a_model() {
    let mut desk = desk();
    for _ in 0..desk.optimizing.sweep.dial.suggested().len() {
        desk.act(Act::SweepValue(0));
    }
    desk.optimizing.sweep.steps.clear();
    desk.act(Act::Sweep);
    let Some(why) = &desk.optimizing.refused else {
        panic!("a refusal says why");
    };
    assert!(why.contains("at least one value"), "{why}");
}

#[test]
fn the_corpus_the_tab_offers_is_the_whole_sixty_four() {
    assert_eq!(mcf_optimize::corpus::task_count(), 64);
    assert_eq!(mcf_optimize::corpus::Set::all().len(), 8);
}
