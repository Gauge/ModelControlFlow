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
fn a_sweep_by_hand_with_no_values_chosen_is_refused_before_it_looks_for_a_model() {
    let mut desk = desk();
    desk.act(Act::SweepWay(by_hand()));
    desk.optimizing.sweep.steps.clear();
    desk.act(Act::Sweep);
    let Some(why) = &desk.optimizing.refused else {
        panic!("a refusal says why");
    };
    assert!(why.contains("at least one value"), "{why}");
}

fn by_hand() -> usize {
    mcf_optimize::hunt::Way::ALL
        .iter()
        .position(|way| *way == mcf_optimize::hunt::Way::ByHand)
        .unwrap_or(1)
}

#[test]
fn the_automatic_search_is_what_a_sweep_does_unless_told_otherwise() {
    let desk = desk();
    assert_eq!(
        desk.optimizing.way,
        mcf_optimize::hunt::Way::Halving,
        "closing in automatically is the usual way to dial something in"
    );
}

#[test]
fn an_automatic_sweep_needs_no_values_chosen_because_it_chooses_them() {
    let mut desk = desk();
    desk.optimizing.sweep.steps.clear();
    desk.act(Act::Sweep);
    let why = desk.optimizing.refused.clone().unwrap_or_default();
    assert!(
        !why.contains("at least one value"),
        "the automatic search picks its own values: {why}"
    );
}

#[test]
fn a_sweep_with_no_test_set_is_refused_whichever_way_it_searches() {
    for way in [0, by_hand()] {
        let mut desk = desk();
        desk.act(Act::SweepWay(way));
        desk.optimizing.sweep.sets.clear();
        desk.act(Act::Sweep);
        let why = desk.optimizing.refused.clone().unwrap_or_default();
        assert!(why.contains("at least one test set"), "{why}");
    }
}

#[test]
fn a_value_typed_by_hand_joins_the_list_and_switches_to_choosing_by_hand() {
    let mut desk = desk();
    desk.optimizing.sweep.steps.clear();
    desk.optimizing.custom.set("777");
    desk.act(Act::AddCustom);
    assert!(
        desk.optimizing
            .sweep
            .steps
            .contains(&mcf_optimize::dial::Step::Whole(777)),
        "a value typed in is a value to run"
    );
    assert_eq!(
        desk.optimizing.way,
        mcf_optimize::hunt::Way::ByHand,
        "naming a value is choosing the values"
    );
    assert!(
        desk.optimizing.custom.said().is_empty(),
        "the box is cleared"
    );
    assert!(desk.optimizing.custom_refused.is_none());
}

#[test]
fn a_typed_value_outside_the_dial_is_refused_and_says_the_range() {
    let mut desk = desk();
    desk.optimizing.custom.set("999999");
    desk.act(Act::AddCustom);
    let why = desk.optimizing.custom_refused.clone().unwrap_or_default();
    assert!(why.contains("outside"), "{why}");
    assert!(
        !desk
            .optimizing
            .sweep
            .steps
            .contains(&mcf_optimize::dial::Step::Whole(999_999))
    );
}

#[test]
fn a_typed_value_that_is_not_a_number_is_refused_by_name() {
    let mut desk = desk();
    desk.optimizing.custom.set("quite a lot");
    desk.act(Act::AddCustom);
    let why = desk.optimizing.custom_refused.clone().unwrap_or_default();
    assert!(why.contains("quite a lot"), "the refusal quotes it: {why}");
}

#[test]
fn a_value_already_in_the_list_is_not_added_twice() {
    let mut desk = desk();
    desk.optimizing.sweep.steps.clear();
    desk.optimizing.custom.set("512");
    desk.act(Act::AddCustom);
    desk.optimizing.custom.set("512");
    desk.act(Act::AddCustom);
    assert_eq!(desk.optimizing.sweep.steps.len(), 1);
    let why = desk.optimizing.custom_refused.clone().unwrap_or_default();
    assert!(why.contains("already"), "{why}");
}

#[test]
fn a_dial_read_in_thousandths_takes_a_value_with_a_point_in_it() {
    let mut desk = desk();
    let at = mcf_optimize::dial::Dial::ALL
        .iter()
        .position(|dial| *dial == mcf_optimize::dial::Dial::Temperature)
        .unwrap_or(0);
    desk.act(Act::Dial(at));
    desk.optimizing.sweep.steps.clear();
    desk.optimizing.custom.set("0.35");
    desk.act(Act::AddCustom);
    assert!(
        desk.optimizing
            .sweep
            .steps
            .contains(&mcf_optimize::dial::Step::Thousandths(350)),
        "0.35 is 350 thousandths, and never 35 or 0: {:?}",
        desk.optimizing.sweep.steps
    );
}

#[test]
fn typing_a_value_takes_the_keys_while_the_optimize_tab_is_open() {
    let mut desk = desk();
    desk.tab = mcf_desk::Tab::Optimize;
    assert!(!desk.typing_into_a_value());
    desk.act(Act::CustomValue(mcf_desk::ui::Touched::At(0)));
    assert!(desk.typing_into_a_value());
    desk.typing().put("128", 16);
    assert_eq!(desk.optimizing.custom.said(), "128");
    desk.stopped_typing();
    assert!(
        !desk.typing_into_a_value(),
        "escape gives the keys back and clears the box"
    );
    assert!(desk.optimizing.custom.said().is_empty());
}

#[test]
fn the_corpus_the_tab_offers_is_the_whole_sixty_four() {
    assert_eq!(mcf_optimize::corpus::task_count(), 64);
    assert_eq!(mcf_optimize::corpus::Set::all().len(), 8);
}
