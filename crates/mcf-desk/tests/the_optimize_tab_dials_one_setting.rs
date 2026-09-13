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
    desk.act(Act::Dial(dial_at(&desk, Dial::Temperature)));
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
fn a_sweep_with_no_model_chosen_is_refused_and_says_why() {
    let mut desk = desk();
    desk.act(Act::Sweep);
    assert!(!desk.optimizing.running, "nothing was started");
    let Some(why) = &desk.optimizing.refused else {
        panic!("a refusal says why");
    };
    assert!(why.contains("choose a model"), "{why}");
    assert!(
        !why.contains("host"),
        "a sweep holds the model itself, so being asked to host one first would be wrong: \
         {why}"
    );
}

#[test]
fn correctness_is_what_a_sweep_ranks_by_unless_told_otherwise() {
    let desk = desk();
    assert_eq!(
        desk.optimizing.measure,
        mcf_optimize::reading::Measure::Correctness,
        "the point of dialling a setting in is answers that are right"
    );
}

#[test]
fn the_values_chosen_by_hand_are_out_of_the_way_while_the_search_is_automatic() {
    let mut desk = desk();
    assert_eq!(desk.optimizing.way, mcf_optimize::hunt::Way::Halving);
    desk.act(Act::SweepWay(by_hand()));
    assert_eq!(desk.optimizing.way, mcf_optimize::hunt::Way::ByHand);
    desk.act(Act::SweepWay(0));
    assert_eq!(desk.optimizing.way, mcf_optimize::hunt::Way::Halving);
}

#[test]
fn the_two_ways_of_searching_are_named_in_one_word_each() {
    for way in mcf_optimize::hunt::Way::ALL {
        assert!(
            !way.label().contains('—') && way.label().split_whitespace().count() == 1,
            "a button says what it is, not how it works: {:?}",
            way.label()
        );
    }
    for measure in mcf_optimize::reading::Measure::ALL {
        assert!(
            !measure.label().contains('—') && measure.label().split_whitespace().count() == 1,
            "{:?}",
            measure.label()
        );
    }
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
    desk.act(Act::Dial(dial_at(&desk, Dial::Temperature)));
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

fn dial_at(desk: &Desk, wanted: Dial) -> usize {
    desk.dials_worth_offering()
        .iter()
        .position(|dial| *dial == wanted)
        .unwrap_or(0)
}

fn reading_a_level() -> Desk {
    let mut desk = desk();
    desk.declared = Some(mcf_serve::declared::Declared {
        thinking: mcf_serve::thinking::Thinking::in_template(
            "{%- if reasoning_effort not in ('xhigh', 'medium', 'low') %}{{ raise_exception('no') \
             }}{%- endif %}{% if enable_thinking %}<think>{% endif %}",
        ),
        ..mcf_serve::declared::Declared::default()
    });
    desk
}

fn a_spot(step: u32, set: usize, repeat: u8) -> mcf_optimize::ledger::At {
    mcf_optimize::ledger::At {
        dial: Dial::ThinkingBudget,
        step: Step::Whole(step),
        set,
        repeat,
    }
}

#[test]
fn a_row_is_picked_by_clicking_it_and_let_go_by_clicking_again() {
    let mut desk = desk();
    let one = a_spot(4096, 1, 1);
    desk.act(Act::PickRow(one));
    assert_eq!(desk.optimizing.picked, vec![one]);
    desk.act(Act::PickRow(one));
    assert!(desk.optimizing.picked.is_empty());
}

#[test]
fn several_rows_can_be_picked_at_once() {
    let mut desk = desk();
    let held = [a_spot(0, 1, 1), a_spot(4096, 2, 3), a_spot(8192, 5, 2)];
    for spot in held {
        desk.act(Act::PickRow(spot));
    }
    assert_eq!(desk.optimizing.picked.len(), 3);
    desk.act(Act::PickNone);
    assert!(desk.optimizing.picked.is_empty(), "and let go of in one go");
}

#[test]
fn running_picked_readings_again_leaves_the_sweep_alone() {
    let mut desk = desk();
    let steps = desk.optimizing.sweep.steps.clone();
    let sets = desk.optimizing.sweep.sets.clone();
    let repeats = desk.optimizing.sweep.repeats;
    for spot in [a_spot(0, 3, 1), a_spot(8192, 1, 2)] {
        desk.act(Act::PickRow(spot));
    }
    desk.act(Act::RerunPicked);
    assert_eq!(
        desk.optimizing.sweep.steps, steps,
        "running two readings again is not a new sweep, and must not rewrite the one set up"
    );
    assert_eq!(desk.optimizing.sweep.sets, sets);
    assert_eq!(desk.optimizing.sweep.repeats, repeats);
    assert!(desk.optimizing.picked.is_empty());
}
#[test]
fn running_nothing_again_does_nothing() {
    let mut desk = desk();
    desk.act(Act::RerunPicked);
    assert!(!desk.optimizing.running);
    assert!(desk.optimizing.picked.is_empty());
}

#[test]
fn every_dial_offered_is_one_the_engine_enforces() {
    for dial in Dial::ALL {
        assert!(
            dial.flag().is_some() || dial.field().is_some(),
            "{} would only reach the model as words in a prompt, and a model is free to \
             ignore those",
            dial.label()
        );
    }
}

#[test]
fn a_model_whose_template_reads_no_level_is_not_offered_one() {
    let desk = desk();
    assert!(
        !desk.dials_worth_offering().contains(&Dial::ThinkingLevel),
        "setting a level on a model that never reads it would change nothing at all"
    );
}

#[test]
fn a_model_whose_template_reads_a_level_is_offered_it() {
    let desk = reading_a_level();
    assert!(desk.dials_worth_offering().contains(&Dial::ThinkingLevel));
    assert_eq!(
        desk.levels_of_the_model(),
        vec![
            "none".to_owned(),
            "low".to_owned(),
            "medium".to_owned(),
            "xhigh".to_owned()
        ],
        "the levels offered are that model's own words, not a list MCF made up"
    );
}

#[test]
fn choosing_the_level_puts_every_level_the_model_takes_on_the_list() {
    let mut desk = reading_a_level();
    desk.act(Act::Dial(dial_at(&desk, Dial::ThinkingLevel)));
    assert_eq!(desk.optimizing.sweep.dial, Dial::ThinkingLevel);
    assert_eq!(
        desk.optimizing.sweep.steps.len(),
        4,
        "four words means four values, and running all four is the whole search"
    );
    assert_eq!(
        desk.optimizing.way,
        mcf_optimize::hunt::Way::ByHand,
        "there is nothing to halve between four named words"
    );
}

#[test]
fn a_level_is_typed_in_as_one_of_the_model_s_own_words() {
    let mut desk = reading_a_level();
    desk.act(Act::Dial(dial_at(&desk, Dial::ThinkingLevel)));
    desk.optimizing.sweep.steps.clear();
    desk.optimizing.custom.set("xhigh");
    desk.act(Act::AddCustom);
    assert!(
        desk.optimizing.sweep.steps.contains(&Step::Whole(3)),
        "{:?} {:?}",
        desk.optimizing.sweep.steps,
        desk.optimizing.custom_refused
    );
}

#[test]
fn a_level_this_model_does_not_take_is_refused_even_though_another_model_would() {
    let mut desk = reading_a_level();
    desk.act(Act::Dial(dial_at(&desk, Dial::ThinkingLevel)));
    desk.optimizing.custom.set("high");
    desk.act(Act::AddCustom);
    assert!(
        desk.optimizing.custom_refused.is_some(),
        "this template raises an exception for 'high', so sending it would fail the trial"
    );
}

#[test]
fn moving_to_a_model_that_reads_no_level_moves_off_the_level_dial() {
    let mut desk = reading_a_level();
    desk.act(Act::Dial(dial_at(&desk, Dial::ThinkingLevel)));
    assert_eq!(desk.optimizing.sweep.dial, Dial::ThinkingLevel);
    desk.declared = Some(mcf_serve::declared::Declared::default());
    desk.optimizing.named = desk.levels_of_the_model();
    assert!(
        !desk.dials_worth_offering().contains(&Dial::ThinkingLevel),
        "the new model reads no level, so the dial is not among those offered"
    );
}

#[test]
fn the_level_a_sweep_sends_is_the_word_and_the_record_keeps_the_word_too() {
    let desk = reading_a_level();
    let named = desk.levels_of_the_model();
    assert_eq!(
        Dial::ThinkingLevel.said_among(Step::Whole(1), &named),
        "low",
        "what is drawn in the table is the word the model was actually given"
    );
    assert_eq!(
        Dial::ThinkingLevel.read_among("LOW", &named),
        Some(Step::Whole(1)),
        "and typing it back in capitals finds the same level"
    );
}

#[test]
fn opening_the_tab_reads_what_was_already_measured() {
    let mut desk = desk();
    desk.optimizing.rows.push(mcf_optimize::ledger::Row {
        recorded: "before".to_owned(),
        under: mcf_optimize::ledger::Under::default(),
        at: a_spot(4096, 1, 1),
        reading: mcf_optimize::reading::Reading {
            dial: Dial::ThinkingBudget,
            step: Step::Whole(4096),
            set: 1,
            repeat: 1,
            passed: 4,
            of: 8,
            produced: 10,
            milliseconds: 10,
            ending: mcf_optimize::reading::Ending::Answered,
            per_task: Vec::new(),
        },
    });
    desk.act(Act::Tab(mcf_desk::Tab::Optimize));
    assert!(
        desk.optimizing.rows.is_empty(),
        "opening the tab goes back to the record rather than showing whatever was left over"
    );
}

#[test]
fn running_one_picked_reading_again_asks_for_that_take_of_that_set_only() {
    let mut desk = desk();
    desk.act(Act::PickRow(a_spot(8192, 5, 3)));
    desk.act(Act::RerunPicked);
    assert!(
        desk.optimizing.picked.is_empty(),
        "the pick is spent once it has been acted on"
    );
    assert_eq!(
        desk.optimizing.sweep.repeats, 1,
        "picking take three of set five must not quietly ask for takes one and two as well"
    );
    assert_eq!(
        desk.optimizing.sweep.sets,
        (1..=8).collect::<Vec<usize>>(),
        "an exact rerun does not narrow the sweep to the set that was picked"
    );
}
