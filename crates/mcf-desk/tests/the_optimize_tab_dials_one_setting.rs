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
fn a_number_of_takes_is_picked_rather_than_cycled_through() {
    let mut desk = desk();
    assert_eq!(desk.optimizing.sweep.repeats, 1);
    desk.act(Act::Takes(3));
    assert_eq!(
        desk.optimizing.sweep.repeats, 3,
        "asking for three takes gives three, rather than one press towards them"
    );
    desk.act(Act::Takes(1));
    assert_eq!(desk.optimizing.sweep.repeats, 1);
    desk.act(Act::Takes(99));
    assert_eq!(
        desk.optimizing.sweep.repeats, 3,
        "and nothing outside what the control offers gets through it"
    );
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
fn a_value_typed_by_hand_is_a_value_that_runs_without_being_added() {
    let mut desk = desk();
    desk.optimizing.custom.set("777");
    desk.act(Act::CustomValue(mcf_desk::ui::Touched::At(0)));
    assert!(
        desk.optimizing.custom_refused.is_none(),
        "777 is a micro-batch, so there is nothing to say about it: {:?}",
        desk.optimizing.custom_refused
    );
    assert_eq!(
        desk.optimizing.what_was_typed(),
        Ok(Some(mcf_optimize::dial::Step::Whole(777))),
        "a value that is set is a value that runs, and nothing has to be pressed for it"
    );
}

#[test]
fn a_typed_value_outside_the_dial_is_refused_as_it_is_typed_and_says_the_range() {
    let mut desk = desk();
    desk.optimizing.custom.set("999999");
    desk.act(Act::CustomValue(mcf_desk::ui::Touched::At(0)));
    let why = desk.optimizing.custom_refused.clone().unwrap_or_default();
    assert!(
        why.contains("outside"),
        "saying so while it is being typed is the point, rather than at the moment the \
         sweep is started and it is too late to have meant something else: {why}"
    );
    assert!(desk.optimizing.what_was_typed().is_err());
}

#[test]
fn a_typed_value_that_is_not_a_number_is_refused_by_name() {
    let mut desk = desk();
    desk.optimizing.custom.set("quite a lot");
    desk.act(Act::CustomValue(mcf_desk::ui::Touched::At(0)));
    let why = desk.optimizing.custom_refused.clone().unwrap_or_default();
    assert!(why.contains("quite a lot"), "the refusal quotes it: {why}");
}

#[test]
fn a_value_typed_that_is_already_offered_is_not_run_twice() {
    let mut desk = desk();
    let already = desk
        .optimizing
        .sweep
        .steps
        .first()
        .copied()
        .expect("a dial offers values of its own");
    desk.optimizing.custom.set(already.said());
    desk.act(Act::CustomValue(mcf_desk::ui::Touched::At(0)));
    assert!(
        desk.optimizing.custom_refused.is_none(),
        "typing a value that is already on the list is not a mistake to be told about"
    );
    let before = desk.optimizing.sweep.steps.len();
    desk.act(Act::Sweep);
    assert_eq!(
        desk.optimizing.sweep.steps.len(),
        before,
        "and it runs once rather than twice: {:?}",
        desk.optimizing.sweep.steps
    );
}

#[test]
fn a_dial_read_in_thousandths_takes_a_value_with_a_point_in_it() {
    let mut desk = desk();
    desk.act(Act::Dial(dial_at(&desk, Dial::Temperature)));
    desk.optimizing.custom.set("0.35");
    desk.act(Act::CustomValue(mcf_desk::ui::Touched::At(0)));
    assert_eq!(
        desk.optimizing.what_was_typed(),
        Ok(Some(mcf_optimize::dial::Step::Thousandths(350))),
        "0.35 is 350 thousandths, and never 35 or 0"
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
    desk.dials_offered()
        .iter()
        .position(|dial| *dial == wanted)
        .unwrap_or(0)
}

/// A model with no thinking to dial at all.
///
/// This used to open a thinking section — `<think>a</think>` — because a section without a
/// named level was treated as nothing to dial. It is not: the engine cuts a section short
/// itself, so such a model can be told not to think even though it names no level. What is
/// left with nothing to dial is a template that neither names a level nor marks a section.
fn reading_no_level() -> Desk {
    let mut desk = desk();
    desk.declared = Some(mcf_serve::declared::Declared {
        thinking: mcf_serve::thinking::Thinking::in_template("{{ messages }}"),
        draft_head: Some(1),
        ..mcf_serve::declared::Declared::default()
    });
    desk
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
fn every_dial_is_offered_whatever_the_model_is() {
    for held in [desk(), reading_a_level()] {
        assert_eq!(
            held.dials_offered().len(),
            Dial::ALL.len(),
            "a setting that is missing looks like a setting MCF does not have; one that is \
             there and says why it cannot run tells you something about the model"
        );
    }
}

#[test]
fn a_model_that_reads_no_level_says_why_the_dial_would_do_nothing() {
    let desk = reading_no_level();
    let why = desk
        .why_the_dial_does_nothing(Dial::ThinkingLevel)
        .unwrap_or_default();
    assert!(why.contains("neither names a thinking level"), "{why}");
}

#[test]
fn a_model_that_reads_a_level_has_nothing_to_explain() {
    let desk = reading_a_level();
    assert!(
        desk.why_the_dial_does_nothing(Dial::ThinkingLevel)
            .is_none()
    );
}

#[test]
fn a_sweep_of_a_dial_that_would_do_nothing_is_refused_before_a_model_is_held() {
    let mut desk = reading_no_level();
    desk.act(Act::Dial(dial_at(&desk, Dial::ThinkingLevel)));
    desk.act(Act::Sweep);
    assert!(!desk.optimizing.running);
    let why = desk.optimizing.refused.clone().unwrap_or_default();
    assert!(
        why.contains("nothing here to ask for or to cut short"),
        "the refusal says what is wrong with the model rather than with the request: {why}"
    );
}

#[test]
fn a_budget_is_refused_for_a_model_whose_template_marks_no_thinking() {
    let mut desk = desk();
    desk.declared = Some(mcf_serve::declared::Declared {
        thinking: mcf_serve::thinking::Thinking::in_template("{{ messages }}"),
        ..mcf_serve::declared::Declared::default()
    });
    let why = desk
        .why_the_dial_does_nothing(Dial::ThinkingBudget)
        .unwrap_or_default();
    assert!(
        why.contains("nothing to count"),
        "the engine builds no budget sampler without a thinking section: {why}"
    );
}

#[test]
fn a_budget_is_offered_for_a_model_that_does_mark_its_thinking() {
    let mut desk = desk();
    desk.declared = Some(mcf_serve::declared::Declared {
        thinking: mcf_serve::thinking::Thinking::in_template("<think>a</think>"),
        ..mcf_serve::declared::Declared::default()
    });
    assert!(
        desk.why_the_dial_does_nothing(Dial::ThinkingBudget)
            .is_none()
    );
}

#[test]
fn a_draft_depth_is_refused_for_a_file_that_carries_no_draft_head() {
    let mut desk = reading_no_level();
    desk.declared = Some(mcf_serve::declared::Declared {
        thinking: mcf_serve::thinking::Thinking::in_template("<think>a</think>"),
        ..mcf_serve::declared::Declared::default()
    });
    let why = desk
        .why_the_dial_does_nothing(Dial::DraftDepth)
        .unwrap_or_default();
    assert!(why.contains("no draft head"), "{why}");
}
#[test]
fn a_model_whose_template_reads_a_level_is_offered_it_with_off_in_front() {
    let desk = reading_a_level();
    assert!(desk.dials_offered().contains(&Dial::ThinkingLevel));
    assert_eq!(
        desk.levels_of_the_model(),
        vec![
            mcf_optimize::dial::Dial::OFF.to_owned(),
            "low".to_owned(),
            "medium".to_owned(),
            "xhigh".to_owned()
        ],
        "the levels are that model's own words, not a list MCF made up — except off, which \
         is no word at all: the engine cuts the thinking section short itself, so it holds \
         wherever a template opens one"
    );
    assert!(
        !desk.levels_of_the_model().iter().any(|held| held == "none"),
        "and the template's own word for no level is not offered beside it, because asking \
         for that word only takes the level away and leaves the template's default behind"
    );
}

#[test]
fn a_model_whose_template_opens_no_thinking_section_is_offered_no_way_to_turn_it_off() {
    let mut desk = desk();
    desk.declared = Some(mcf_serve::declared::Declared {
        thinking: mcf_serve::thinking::Thinking::in_template(
            "{{ reasoning_strength }} and no section at all",
        ),
        ..mcf_serve::declared::Declared::default()
    });
    assert!(
        !desk
            .levels_of_the_model()
            .iter()
            .any(|held| held == mcf_optimize::dial::Dial::OFF),
        "there is nothing for the engine to cut short, so off would be a value that did \
         nothing and said it did: {:?}",
        desk.levels_of_the_model()
    );
}

/// Most templates name no effort level at all — Qwen3, Nemotron-3, Laguna and Gemma among
/// the families held on the machine this was written on. Every one of them can still be
/// told not to think, and MCF used to report all of them as having no thinking to control,
/// because it offered off only alongside a level somebody could name.
#[test]
fn a_model_that_names_no_level_but_can_be_switched_off_is_offered_off() {
    let mut desk = desk();
    desk.declared = Some(mcf_serve::declared::Declared {
        thinking: mcf_serve::thinking::Thinking::in_template(
            "{%- set enable_thinking = enable_thinking if enable_thinking is defined else True %}\
             {%- if enable_thinking %}{{- '<think>' }}{%- endif %}",
        ),
        ..mcf_serve::declared::Declared::default()
    });
    assert_eq!(
        desk.levels_of_the_model(),
        vec![mcf_optimize::dial::Dial::OFF.to_owned()],
        "off and nothing else: the template offers no level to name, and thinking on is \
         what it does already"
    );
    assert!(
        desk.why_the_dial_does_nothing(Dial::ThinkingLevel)
            .is_none(),
        "there is something here to sweep — off against the model's own way of working"
    );
}

/// A template that marks a section without reading a switch can still be cut short by the
/// engine, so off holds there too.
#[test]
fn a_section_with_no_switch_can_still_be_turned_off() {
    let mut desk = desk();
    desk.declared = Some(mcf_serve::declared::Declared {
        thinking: mcf_serve::thinking::Thinking::in_template("{{- '<think>' }} and no switch"),
        ..mcf_serve::declared::Declared::default()
    });
    assert_eq!(
        desk.levels_of_the_model(),
        vec![mcf_optimize::dial::Dial::OFF.to_owned()]
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
fn a_level_is_read_as_one_of_the_model_s_own_words() {
    let mut desk = reading_a_level();
    desk.act(Act::Dial(dial_at(&desk, Dial::ThinkingLevel)));
    let named = desk.optimizing.named.clone();
    assert!(
        named.iter().any(|held| held == "xhigh"),
        "this template names xhigh: {named:?}"
    );
    desk.optimizing.custom.set("xhigh");
    assert!(
        matches!(desk.optimizing.what_was_typed(), Ok(Some(Step::Whole(_)))),
        "{named:?} {:?}",
        desk.optimizing.custom_refused
    );
}

#[test]
fn a_level_this_model_does_not_take_is_refused_even_though_another_model_would() {
    let mut desk = reading_a_level();
    desk.act(Act::Dial(dial_at(&desk, Dial::ThinkingLevel)));
    desk.optimizing.custom.set("high");
    desk.act(Act::CustomValue(mcf_desk::ui::Touched::At(0)));
    assert!(
        desk.optimizing.custom_refused.is_some(),
        "this template raises an exception for 'high', so sending it would fail the trial"
    );
}

#[test]
fn the_level_dial_stays_chosen_even_when_a_model_cannot_use_it() {
    let mut desk = reading_a_level();
    desk.act(Act::Dial(dial_at(&desk, Dial::ThinkingLevel)));
    assert_eq!(desk.optimizing.sweep.dial, Dial::ThinkingLevel);
    desk.declared = Some(mcf_serve::declared::Declared::default());
    desk.optimizing.named = desk.levels_of_the_model();
    assert_eq!(
        desk.optimizing.sweep.dial,
        Dial::ThinkingLevel,
        "moving to another model must not silently pick a different setting to measure"
    );
    assert!(desk.dials_offered().contains(&Dial::ThinkingLevel));
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
            why: None,
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

fn a_row(step: u32, set: usize, repeat: u8) -> mcf_optimize::ledger::Row {
    let at = a_spot(step, set, repeat);
    mcf_optimize::ledger::Row {
        recorded: "before".to_owned(),
        under: mcf_optimize::ledger::Under::default(),
        at,
        reading: mcf_optimize::reading::Reading {
            dial: at.dial,
            step: at.step,
            set: at.set,
            repeat: at.repeat,
            passed: 4,
            of: 8,
            produced: 100,
            milliseconds: 1000,
            ending: mcf_optimize::reading::Ending::Answered,
            why: None,
            per_task: Vec::new(),
        },
    }
}

#[test]
fn forgetting_one_row_leaves_the_others_and_starts_nothing() {
    let mut desk = desk();
    desk.optimizing.rows = vec![a_row(0, 1, 1), a_row(4096, 1, 1)];
    desk.act(Act::ForgetRow(a_spot(0, 1, 1)));
    assert!(
        !desk.optimizing.running,
        "forgetting a reading runs nothing"
    );
    assert!(
        desk.optimizing.refused.is_none(),
        "{:?}",
        desk.optimizing.refused
    );
}

#[test]
fn forgetting_a_row_that_was_picked_lets_go_of_the_pick_too() {
    let mut desk = desk();
    let one = a_spot(0, 1, 1);
    desk.act(Act::PickRow(one));
    assert_eq!(desk.optimizing.picked, vec![one]);
    desk.act(Act::ForgetRow(one));
    assert!(
        desk.optimizing.picked.is_empty(),
        "a pick of a reading that is gone would act on nothing"
    );
}

#[test]
fn running_one_row_again_leaves_the_sweep_beside_it_alone() {
    let mut desk = desk();
    let steps = desk.optimizing.sweep.steps.clone();
    let sets = desk.optimizing.sweep.sets.clone();
    let repeats = desk.optimizing.sweep.repeats;
    desk.act(Act::RerunRow(a_spot(8192, 4, 2)));
    assert_eq!(desk.optimizing.sweep.steps, steps);
    assert_eq!(desk.optimizing.sweep.sets, sets);
    assert_eq!(desk.optimizing.sweep.repeats, repeats);
}

#[test]
fn a_sweep_in_flight_is_not_interrupted_by_running_one_row_again() {
    let mut desk = desk();
    desk.optimizing.running = true;
    desk.act(Act::RerunRow(a_spot(0, 1, 1)));
    assert!(
        desk.optimizing.refused.is_none(),
        "the sweep carries on and nothing is said about it: {:?}",
        desk.optimizing.refused
    );
}

#[test]
fn forgetting_a_row_while_a_sweep_is_going_says_why_it_will_not() {
    let mut desk = desk();
    desk.optimizing.running = true;
    desk.act(Act::ForgetRow(a_spot(0, 1, 1)));
    let why = desk.optimizing.refused.clone().unwrap_or_default();
    assert!(
        why.contains("stop it"),
        "taking a reading out from under a sweep that is reading it would be a mess: {why}"
    );
}

#[test]
fn choosing_a_setting_that_only_changes_speed_ranks_by_speed() {
    let mut desk = desk();
    desk.act(Act::Dial(dial_at(&desk, Dial::MicroBatch)));
    assert_eq!(
        desk.optimizing.measure,
        mcf_optimize::reading::Measure::Speed,
        "a micro-batch cannot change an answer, so correctness has nothing to say"
    );
    assert_eq!(
        desk.optimizing.sweep.sets,
        vec![1],
        "and the tasks are not run at all, so eight sets of them is eight times nothing"
    );
    assert_eq!(
        desk.optimizing.sweep.repeats,
        mcf_optimize::trial::TIMES_TIMED,
        "a sweep tells values apart rather than settling any one of them, so a value is timed \
         once and taken again by hand when a reading looks wrong"
    );
}

#[test]
fn asking_for_correctness_on_a_speed_setting_is_refused_with_the_reason() {
    let mut desk = desk();
    desk.act(Act::Dial(dial_at(&desk, Dial::MicroBatch)));
    let correctness = mcf_optimize::reading::Measure::ALL
        .iter()
        .position(|held| *held == mcf_optimize::reading::Measure::Correctness)
        .unwrap_or(0);
    desk.act(Act::SweepMeasure(correctness));
    assert_eq!(
        desk.optimizing.measure,
        mcf_optimize::reading::Measure::Speed,
        "it stays where it was"
    );
    let why = desk.optimizing.refused.clone().unwrap_or_default();
    assert!(why.contains("only how fast"), "{why}");
}

#[test]
fn choosing_a_setting_that_changes_answers_leaves_the_ranking_alone() {
    let mut desk = desk();
    desk.act(Act::Dial(dial_at(&desk, Dial::ThinkingBudget)));
    assert_eq!(
        desk.optimizing.measure,
        mcf_optimize::reading::Measure::Correctness
    );
    assert_eq!(desk.optimizing.sweep.sets.len(), 8);
}

#[test]
fn a_timed_sweep_lays_out_one_trial_for_each_take_of_each_value() {
    let mut desk = desk();
    desk.act(Act::Dial(dial_at(&desk, Dial::MicroBatch)));
    let values = desk.optimizing.sweep.steps.len().max(1);
    assert_eq!(
        desk.optimizing.sweep.trials(),
        values * usize::from(mcf_optimize::trial::TIMES_TIMED),
        "one set, so the count is values times takes and nothing else"
    );
}

#[test]
fn moving_from_a_timed_setting_to_one_that_marks_answers_puts_the_sets_back() {
    let mut desk = desk();
    desk.act(Act::Dial(dial_at(&desk, Dial::MicroBatch)));
    assert_eq!(desk.optimizing.sweep.sets, vec![1]);
    desk.act(Act::Dial(dial_at(&desk, Dial::ThinkingBudget)));
    assert_eq!(
        desk.optimizing.sweep.sets.len(),
        8,
        "the tasks come back when the setting being dialled can change an answer"
    );
}

#[test]
fn a_finished_sweep_asks_before_it_moves_anything() {
    let mut desk = desk();
    assert!(
        desk.optimizing.settled.is_none(),
        "nothing has been measured, so there is nothing to decide about"
    );
    desk.optimizing.settled = Some(mcf_optimize::dial::Step::Whole(2048));
    desk.act(Act::KeepAsIs);
    assert!(
        desk.optimizing.settled.is_none(),
        "leaving the settings alone is an answer, and it puts the question away"
    );
    assert!(
        desk.optimizing.adopted.is_none(),
        "and it says nothing was taken up, because nothing was"
    );
}

#[test]
fn every_setting_a_sweep_can_move_lands_somewhere_when_it_is_taken_up() {
    for dial in mcf_optimize::dial::Dial::ALL {
        let mut desk = desk();
        desk.settings = Some(mcf_serve::hosting::Hosting::recommended(
            "llama.cpp-vulkan",
            "a card",
            true,
            131_072,
            Some(32),
            true,
            None,
        ));
        let before = desk.settings.clone();
        desk.optimizing.sweep = mcf_optimize::dial::Sweep::on(dial);
        desk.optimizing.named = vec!["low".to_owned(), "high".to_owned()];
        let step = dial.step_of(match dial.scale() {
            mcf_optimize::dial::Scale::Whole => 1,
            mcf_optimize::dial::Scale::Thousandths => 500,
        });
        desk.optimizing.settled = Some(step);
        desk.act(Act::AdoptBest);
        assert!(
            desk.optimizing.settled.is_none(),
            "{} left the question standing after it was answered",
            dial.label()
        );
        assert!(
            desk.optimizing.adopted.is_some(),
            "{} said nothing about what it did",
            dial.label()
        );
        assert_ne!(
            desk.settings,
            before,
            "{} was taken up and nothing in the settings moved, so the sweep's answer went \
             nowhere",
            dial.label()
        );
    }
}

/// One column for every row that names a thing on the left and shows it on the right. The
/// tab is read down the left edge, and a left edge that moves from block to block is read
/// three times instead of once.
#[test]
fn the_optimize_tab_names_things_in_one_column() {
    let source = include_str!("../src/view.rs");
    let mut offenders = Vec::new();
    for (number, line) in source.lines().enumerate() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("//") || trimmed.starts_with("///") {
            continue;
        }
        for margin in ["area.x + 110.0", "area.x + 140.0", "area.x + 160.0"] {
            if line.contains(margin) {
                offenders.push(format!("view.rs:{}: {margin}", number.saturating_add(1)));
            }
        }
    }
    assert!(
        offenders.is_empty(),
        "a labelled row picked its own left margin instead of the one column the tab uses \
         (NAMED): {offenders:#?}"
    );
}

/// The values of a setting the model names are places in a list, not points on a span. A
/// search that climbed through them would ask for the place after the last one and write
/// the number down as if it were a level — a row reading "4" among low, medium and high.
#[test]
fn a_setting_the_model_names_is_never_searched_over_a_span_of_numbers() {
    let mut desk = reading_a_level();
    desk.act(Act::Dial(dial_at(&desk, Dial::ThinkingLevel)));
    let automatic = mcf_optimize::hunt::Way::ALL
        .iter()
        .position(|way| *way == mcf_optimize::hunt::Way::Halving)
        .unwrap_or(0);
    desk.act(Act::SweepWay(automatic));
    assert_eq!(
        desk.optimizing.way,
        mcf_optimize::hunt::Way::ByHand,
        "there is no span to search over, so asking for one is refused rather than obeyed"
    );
    assert!(
        desk.optimizing.refused.is_some(),
        "and it says so rather than quietly ignoring the press"
    );

    desk.optimizing.way = mcf_optimize::hunt::Way::Halving;
    desk.act(Act::Sweep);
    assert_eq!(
        desk.optimizing.way,
        mcf_optimize::hunt::Way::ByHand,
        "however it got set, a sweep of a named setting runs the names"
    );
    let levels = desk.optimizing.named.len();
    assert!(levels > 0, "this model names levels");
    for step in &desk.optimizing.sweep.steps {
        let at = match *step {
            Step::Whole(held) | Step::Thousandths(held) => held as usize,
        };
        assert!(
            at < levels,
            "{at} is past the end of {:?}, so it would run as a number nobody named",
            desk.optimizing.named
        );
    }
}

/// A setting is worth moving because of what it does to the answers, so marking them is the
/// default nearly everywhere. The one exception is the setting that cannot touch an answer.
#[test]
fn every_setting_is_ranked_by_correctness_to_begin_with_except_the_batches() {
    for dial in Dial::ALL {
        let mut desk = desk();
        desk.act(Act::Dial(dial_at(&desk, dial)));
        // Neither batch can change which tokens come back, only how fast they do, so
        // neither opens ranked by what the answer said.
        let wanted = if matches!(dial, Dial::MicroBatch | Dial::Batch) {
            mcf_optimize::reading::Measure::Speed
        } else {
            mcf_optimize::reading::Measure::Correctness
        };
        assert_eq!(
            desk.optimizing.measure,
            wanted,
            "{} opens ranked by the wrong thing",
            dial.label()
        );
        if wanted.needs_the_answers_run() {
            assert_eq!(
                desk.optimizing.sweep.sets.len(),
                8,
                "{} marks answers, so it runs the sets",
                dial.label()
            );
        } else {
            assert_eq!(
                desk.optimizing.sweep.sets,
                vec![1],
                "{} takes a rate off one trial, so eight sets would be eight times nothing",
                dial.label()
            );
        }
    }
}

#[test]
fn a_draft_head_can_be_marked_now_that_its_answers_may_differ() {
    let mut desk = desk();
    desk.act(Act::Dial(dial_at(&desk, Dial::DraftDepth)));
    let speed = mcf_optimize::reading::Measure::ALL
        .iter()
        .position(|held| *held == mcf_optimize::reading::Measure::Speed)
        .unwrap_or(0);
    desk.act(Act::SweepMeasure(speed));
    assert_eq!(
        desk.optimizing.measure,
        mcf_optimize::reading::Measure::Speed,
        "and it can still be timed instead, which is what it was only ever able to be"
    );
    assert_eq!(
        desk.optimizing.sweep.sets,
        vec![1],
        "and choosing that lays out the run that goes with it"
    );
    assert!(desk.optimizing.refused.is_none());
}

#[test]
fn the_one_setting_that_cannot_change_an_answer_still_refuses_to_be_marked() {
    let mut desk = desk();
    desk.act(Act::Dial(dial_at(&desk, Dial::MicroBatch)));
    let correctness = mcf_optimize::reading::Measure::ALL
        .iter()
        .position(|held| *held == mcf_optimize::reading::Measure::Correctness)
        .unwrap_or(0);
    desk.act(Act::SweepMeasure(correctness));
    assert_eq!(
        desk.optimizing.measure,
        mcf_optimize::reading::Measure::Speed,
        "it stays where it was"
    );
    let why = desk.optimizing.refused.clone().unwrap_or_default();
    assert!(why.contains("only how fast"), "{why}");
}

/// The two batches are not independent: the engine will not push a pass wider than the
/// batch it was handed, so a sweep of one has to say what it did to the other or the
/// reading is taken under conditions nobody wrote down.
mod the_two_batches_hold_together {
    use super::{Act, Dial, desk, dial_at};

    fn adopted(dial: Dial, to: u32) -> Option<mcf_serve::hosting::Hosting> {
        let mut desk = desk();
        desk.settings = Some(mcf_serve::hosting::Hosting::recommended(
            "llama.cpp",
            "a card",
            true,
            32_768,
            Some(8),
            true,
            None,
        ));
        desk.act(Act::Dial(dial_at(&desk, dial)));
        desk.optimizing.settled = Some(mcf_optimize::dial::Step::Whole(to));
        desk.act(Act::AdoptBest);
        desk.settings
    }

    #[test]
    fn a_batch_under_the_pass_narrows_the_pass() {
        let Some(held) = adopted(Dial::Batch, 256) else {
            return;
        };
        assert_eq!(held.batch, 256);
        assert!(
            held.ubatch <= 256,
            "the engine would have narrowed it anyway, and a reading has to be taken under \
             what actually ran: ubatch {}",
            held.ubatch
        );
    }

    #[test]
    fn a_pass_over_the_batch_widens_the_batch() {
        let Some(held) = adopted(Dial::MicroBatch, 4096) else {
            return;
        };
        assert_eq!(held.ubatch, 4096);
        assert!(held.batch >= 4096, "batch {}", held.batch);
    }

    /// And the dial is offered at all, which is what was missing: the micro-batch could be
    /// measured on this machine and the batch beside it could not.
    #[test]
    fn the_prompt_batch_is_a_setting_a_sweep_can_measure() {
        let desk = desk();
        assert!(desk.dials_offered().contains(&Dial::Batch));
        assert_eq!(Dial::Batch.flag(), Some("--batch-size"));
        assert!(
            Dial::Batch.times_reading_the_prompt(),
            "a batch shows in how fast a prompt is read, not in how fast an answer is \
             written, and timing the wrong one reads the same number back at every value"
        );
    }
}
