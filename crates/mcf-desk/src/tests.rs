use super::{ACTIONS, Caret, Desk, Model, Page, component_from, model_from};
use mcf_record::json::Value;

#[test]
fn every_action_reaches_a_request_or_asks_nothing() {
    assert!(!ACTIONS.is_empty());
    for action in ACTIONS {
        assert!(!action.key.is_empty());
        assert!(!action.does.is_empty());
        if let Some(reaches) = action.reaches {
            assert!(
                matches!(
                    reaches,
                    "Status" | "Holding" | "Stop" | "Components" | "Anatomy" | "Failures"
                ),
                "{} reaches {reaches}, which this surface cannot build",
                action.key
            );
        }
    }
    assert!(ACTIONS.iter().any(|action| action.reaches.is_some()));
}

#[test]
fn nothing_in_the_menu_leads_nowhere() {
    assert!(!Page::MENU.is_empty());
    for (page, label) in Page::MENU {
        assert!(!label.is_empty());
        assert_eq!(
            page.section(),
            *page,
            "{label} is not a section, so it cannot be lit when you are on it"
        );
    }
    assert_eq!(Page::Adding.section(), Page::Models);
    assert_eq!(Page::Hosting.section(), Page::Hosting);
    assert_eq!(Page::Anatomy.section(), Page::Models);
    assert_eq!(Page::Vocabulary.section(), Page::Models);
    assert_eq!(Page::Host.section(), Page::Models);
    assert!(
        !Page::MENU.iter().any(|(page, _)| *page == Page::Host),
        "Host is the list Models shows, not a second entry for it"
    );
}

#[test]
fn a_model_that_will_not_run_is_never_drawn_as_ready() {
    let refused = Model {
        name: "too big".to_owned(),
        refused: Some("this model needs more memory than this computer has".to_owned()),
        ..Model::default()
    };
    assert!(!refused.will_run());
    assert!(refused.in_a_sentence().contains("more memory"));
    assert!(refused.where_it_runs().contains("more memory"));
}

#[test]
fn an_unmeasured_model_never_reports_a_speed() {
    let held = Model {
        name: "never timed".to_owned(),
        engine: Some("llama.cpp".to_owned()),
        device: Some("NVIDIA".to_owned()),
        device_free: None,
        measured_body: None,
        cross_checked: Vec::new(),
        prompt_reported: false,
        applied_addressing: None,
        applied_budget: None,
        on_a_card: true,
        ..Model::default()
    };
    assert!(held.will_run());
    let said = held.in_a_sentence();
    assert!(
        said.contains("not timed it"),
        "an unmeasured model must say so: {said}"
    );
    assert!(
        !said.contains('0'),
        "an absent measurement must not become a figure: {said}"
    );
    let rows = held.technical();
    let speed = rows
        .iter()
        .find(|(name, _)| name == "Speed")
        .map(|(_, value)| value.clone())
        .unwrap_or_default();
    assert_eq!(speed, crate::words::UNMEASURED);
}

#[test]
fn nothing_is_thrown_away_on_the_way_to_a_plain_sentence() {
    let held = Model {
        name: "Assistant-8B".to_owned(),
        path: "/home/somebody/.local/share/mcf/models/Assistant-8B.gguf".to_owned(),
        bytes: Some(5_020_000_000),
        architecture: Some("an-architecture".to_owned()),
        trained: Some(40_960),
        context: Some(32_768),
        engine: Some("llama.cpp-cuda".to_owned()),
        device: Some("NVIDIA GeForce RTX 5080".to_owned()),
        device_free: None,
        measured_body: None,
        cross_checked: Vec::new(),
        prompt_reported: false,
        applied_addressing: None,
        applied_budget: None,
        on_a_card: true,
        speed: Some(155.0),
        ..Model::default()
    };
    let rows = held.technical();
    let flat = rows
        .iter()
        .map(|(name, value)| format!("{name}: {value}"))
        .collect::<Vec<_>>()
        .join("\n");

    assert!(flat.contains("155.0 tokens a second"), "{flat}");
    assert!(flat.contains("32,768 tokens"), "{flat}");
    assert!(flat.contains("40,960 tokens"), "{flat}");
    assert!(flat.contains("llama.cpp-cuda"), "{flat}");
    assert!(flat.contains("an-architecture"), "{flat}");
    assert!(flat.contains("5,020,000,000 bytes"), "{flat}");
    assert!(flat.contains(&held.path), "{flat}");
}

#[test]
fn a_daemon_answer_becomes_a_model() {
    let answered = Value::map([
        (
            "path",
            Value::text("/var/lib/mcf/models/SmolLM2-135M-Instruct.gguf"),
        ),
        ("bytes", Value::Integer(270_000_000)),
        (
            "runs",
            Value::map([
                ("architecture", Value::text("llama")),
                ("trained_context", Value::Integer(8_192)),
                (
                    "resolved",
                    Value::map([
                        ("known", Value::Bool(true)),
                        ("engine", Value::text("llama.cpp-cuda")),
                        ("device", Value::text("NVIDIA GeForce RTX 5080")),
                        ("device_kind", Value::text("gpu")),
                        ("context", Value::Integer(8_192)),
                    ]),
                ),
            ]),
        ),
    ]);
    let held = model_from(&answered);

    assert_eq!(held.name, "SmolLM2-135M-Instruct");
    assert_eq!(held.bytes, Some(270_000_000));
    assert_eq!(held.architecture.as_deref(), Some("llama"));
    assert_eq!(held.context, Some(8_192));
    assert!(held.on_a_card);
    assert!(held.will_run());
    assert_eq!(held.speed, None);
    assert_eq!(held.start_up, None);
}

#[test]
fn a_refusal_travels_from_the_daemon_word_for_word() {
    let answered = Value::map([
        ("path", Value::text("/models/enormous.gguf")),
        ("bytes", Value::Integer(400_000_000_000)),
        (
            "runs",
            Value::map([(
                "resolved",
                Value::map([
                    ("known", Value::Bool(false)),
                    ("why", Value::text("no engine here has room for this model")),
                ]),
            )]),
        ),
    ]);
    let held = model_from(&answered);
    assert!(!held.will_run());
    assert_eq!(
        held.refused.as_deref(),
        Some("no engine here has room for this model"),
        "the window must not rewrite the daemon's reason"
    );
}

#[test]
fn an_unreadable_machine_is_never_reported_as_an_empty_one() {
    let desk = Desk::new(std::path::PathBuf::from("/nowhere/control.sock"));
    assert_eq!(desk.memory_sentence(), crate::words::UNMEASURED);
    assert_eq!(desk.processor_sentence(), crate::words::UNMEASURED);
    assert_eq!(desk.card_sentence(), "None found");
    assert!(
        desk.capability_sentence().contains("not been able to read"),
        "{}",
        desk.capability_sentence()
    );
}

#[test]
fn a_daemon_that_is_not_there_is_said_in_words() {
    let mut desk = Desk::new(std::path::PathBuf::from("/nowhere/control.sock"));
    desk.refresh();
    let why = desk.refusal.clone().unwrap_or_default();
    assert!(why.contains("not answering"), "{why}");
    assert!(
        !why.contains("/nowhere"),
        "the path is shown to a person: {why}"
    );
    assert!(
        !why.contains("sock"),
        "the socket is shown to a person: {why}"
    );
    assert_eq!(desk.doing_sentence(), "MCF is not answering");
}

#[test]
fn every_menu_entry_reaches_something_built() {
    let named: Vec<&str> = Page::MENU.iter().map(|(_, label)| *label).collect();
    assert_eq!(
        named,
        ["System", "Models", "Server", "Diagnostics", "Exit"],
        "the window's places are the four D49 names, and Exit"
    );
    for (page, label) in Page::MENU {
        assert_eq!(page.section(), *page, "{label} is not a section of its own");
    }
}

#[test]
fn typing_is_only_typing_where_something_takes_it() {
    let mut desk = Desk::new(std::path::PathBuf::from("/nowhere"));
    for page in [Page::Adding, Page::Hosting, Page::Models] {
        desk.page = page;
        assert!(
            desk.takes_typing(),
            "{page:?} has a field and does not take typing"
        );
    }
    for page in [
        Page::Monitor,
        Page::Host,
        Page::Diagnostics,
        Page::Anatomy,
        Page::Vocabulary,
        Page::Settings,
    ] {
        desk.page = page;
        assert!(
            !desk.takes_typing(),
            "{page:?} has no field and takes typing"
        );
    }
}

#[test]
fn an_empty_field_asks_for_nothing() {
    let mut desk = Desk::new(std::path::PathBuf::from("/nowhere"));
    desk.page = Page::Adding;
    desk.typed = "   ".to_owned();
    desk.look_up();
    assert!(
        matches!(desk.doing, crate::Doing::Nothing),
        "a lookup was started for an empty name"
    );
    desk.models = vec![Model::default()];
    desk.chosen = Some(0);
    desk.ask(0);
    assert!(
        matches!(desk.doing, crate::Doing::Nothing),
        "a question was asked with nothing in it"
    );
}

#[test]
fn only_one_thing_runs_at_a_time() {
    let mut desk = Desk::new(std::path::PathBuf::from("/nowhere"));
    desk.typed = "owner/repository".to_owned();
    desk.look_up();
    assert!(matches!(desk.doing, crate::Doing::Listing(_)));
    desk.download("owner/repository", "a-model.gguf");
    assert!(
        matches!(desk.doing, crate::Doing::Downloading(_)),
        "starting a download did not replace what was running"
    );
}

#[test]
fn a_picker_opens_a_list_rather_than_going_somewhere() {
    let mut desk = Desk::new(std::path::PathBuf::from("/nowhere"));
    desk.page = Page::Diagnostics;
    assert_eq!(desk.open, None, "a picker is closed until it is opened");

    desk.act(crate::Act::Open(crate::Picker::Model));
    assert_eq!(desk.open, Some(crate::Picker::Model));
    assert_eq!(
        desk.page,
        Page::Diagnostics,
        "opening the model picker navigated away, which is what it used to do"
    );

    desk.act(crate::Act::Open(crate::Picker::Model));
    assert_eq!(desk.open, None, "a second click did not shut the list");

    desk.act(crate::Act::Open(crate::Picker::Model));
    desk.act(crate::Act::Open(crate::Picker::Window));
    assert_eq!(desk.open, Some(crate::Picker::Window));
}

#[test]
fn a_window_is_picked_and_never_cycled() {
    let mut desk = Desk::new(std::path::PathBuf::from("/nowhere"));
    let offered = crate::windows();
    assert!(
        offered.contains(&desk.window),
        "the window it starts on is not one of the ones offered"
    );
    for wanted in offered {
        desk.act(crate::Act::Open(crate::Picker::Window));
        desk.act(crate::Act::SetWindow(wanted));
        assert_eq!(desk.window, wanted);
        assert_eq!(desk.open, None, "picking did not shut the list");
    }
    for held in offered {
        assert!(held.is_power_of_two(), "{held} is not a power of two");
    }
}

#[test]
fn choosing_a_model_shuts_the_list_and_stays_on_the_screen() {
    let mut desk = Desk::new(std::path::PathBuf::from("/nowhere"));
    desk.page = Page::Diagnostics;
    desk.models = vec![Model::default(), Model::default()];
    desk.act(crate::Act::Open(crate::Picker::Model));
    desk.act(crate::Act::Choose(1));
    assert_eq!(desk.chosen, Some(1));
    assert_eq!(desk.open, None, "choosing did not shut the list");
    assert_eq!(
        desk.page,
        Page::Diagnostics,
        "choosing a model took the reader off the screen they were setting up"
    );
}

#[test]
fn a_test_that_never_ran_reports_neither_a_time_nor_a_result() {
    for test in crate::tests() {
        assert_eq!(test.ran, None, "{} claims a run time", test.name);
        assert_eq!(test.result, None, "{} claims a result", test.name);
        assert!(
            test.seconds.is_none_or(|seconds| seconds > 0),
            "{} has an estimate of nothing",
            test.name
        );
    }
}

#[test]
fn each_card_has_its_own_cost() {
    let desk = Desk::new(std::path::PathBuf::from("/nowhere"));
    assert_eq!(desk.estimate(false), (104, 255), "one ladder's time");
    assert_eq!(
        desk.cross_check_estimate(),
        (52, 127),
        "the cross-check's own time"
    );
    let (low, high) = desk.estimate(true);
    assert!(
        low < 104 && high < 255,
        "a quick run costs less than the ladder"
    );
}

#[test]
fn a_card_starts_its_own_run() {
    let mut desk = Desk::new(std::path::PathBuf::from("/nowhere"));
    desk.models = vec![Model::default()];
    desk.chosen = Some(0);
    desk.page = Page::Diagnostics;
    desk.act(crate::Act::Run(crate::Card::Prompt));
    assert_eq!(desk.page, Page::Prompt);
    desk.page = Page::Diagnostics;
    desk.act(crate::Act::Run(crate::Card::Comparison));
    assert!(matches!(desk.doing, crate::Doing::Nothing));
    assert!(
        crate::Card::Comparison
            .command("a-model")
            .is_some_and(|command| command.starts_with("mcf bench a-model")),
        "the comparison card names no command"
    );
    assert_eq!(crate::Card::Capabilities.command("a-model"), None);
    desk.act(crate::Act::Run(crate::Card::Capabilities));
    assert!(
        matches!(desk.doing, crate::Doing::Probing(_)),
        "the capabilities card did not start the probes"
    );
    desk.act(crate::Act::SeeStatistics);
    assert_eq!(desk.page, Page::Models);
    assert_eq!(desk.tab, crate::Tab::Statistics);
}

#[test]
fn a_measurement_in_the_record_reaches_the_model() {
    let answered = Value::map([
        ("path", Value::text("/models/a-model.gguf")),
        ("bytes", Value::Integer(420_000_000)),
        (
            "runs",
            Value::map([
                ("architecture", Value::text("an-architecture")),
                (
                    "measured",
                    Value::map([(
                        "readings",
                        Value::List(vec![
                            Value::map([
                                ("depth", Value::Integer(512)),
                                ("ms_per_token", Value::text("1.529")),
                                ("measured", Value::Bool(true)),
                            ]),
                            Value::map([
                                ("depth", Value::Integer(1024)),
                                ("measured", Value::Bool(false)),
                            ]),
                            Value::map([
                                ("depth", Value::Integer(2048)),
                                ("ms_per_token", Value::text("1.412")),
                                ("measured", Value::Bool(true)),
                            ]),
                        ]),
                    )]),
                ),
            ]),
        ),
    ]);
    let held = model_from(&answered);
    assert!(held.measured(), "a measured model reads as unmeasured");
    assert_eq!(held.fastest, Some(1.529));
    assert_eq!(held.slowest, Some(1.412));
    assert_eq!(held.speed_at_512(), "1.53 ms/token");
    assert_eq!(held.speed_at_window(), "1.41 ms/token");
    let [shallowest, deepest] = held.speed_rows();
    assert_eq!(shallowest.0, "at 512 tokens");
    assert_eq!(deepest.0, "at 2,048 tokens", "{deepest:?}");
    assert_eq!(held.start_up(), crate::view::UNKNOWN);
}

#[test]
fn a_start_up_in_the_record_is_the_daemons_figure_or_nothing() {
    let recorded = |first_token: Value| {
        Value::map([
            ("path", Value::text("/models/a-model.gguf")),
            (
                "runs",
                Value::map([(
                    "measured",
                    Value::map([
                        ("readings", Value::List(Vec::new())),
                        ("first_token", first_token),
                    ]),
                )]),
            ),
        ])
    };
    let read = model_from(&recorded(Value::map([
        ("measured", Value::Bool(true)),
        ("depth", Value::Integer(512)),
        ("ms", Value::text("412.7")),
    ])));
    assert_eq!(read.start_up.as_deref(), Some("412.7"));
    assert_eq!(read.start_up(), "412.7 ms");
    assert!(read.measured());
    let not_read = model_from(&recorded(Value::map([
        ("measured", Value::Bool(false)),
        ("why", Value::text("no rung of the ladder separated")),
    ])));
    assert_eq!(not_read.start_up, None);
    assert_eq!(not_read.start_up(), crate::view::UNKNOWN);
    assert!(!not_read.measured());
}

fn a_rung() -> Value {
    Value::map([
        ("depth", Value::Integer(512)),
        ("ms_per_token", Value::text("1.529")),
        ("measured", Value::Bool(true)),
    ])
}

fn a_last_line(prompt_reading: Value, first_token: Value) -> Value {
    Value::map([
        ("measuring", Value::text("a-model")),
        ("readings", Value::List(vec![a_rung()])),
        ("prompt_reading", prompt_reading),
        ("first_token", first_token),
        (
            "memory",
            Value::map([
                ("measured", Value::Bool(false)),
                (
                    "why",
                    Value::text("every rung ran in one window of 4,096 tokens"),
                ),
            ]),
        ),
        (
            "fall_off",
            Value::map([
                ("measured", Value::Bool(false)),
                (
                    "why",
                    Value::text("one rung measured, and a fall-off is read between two"),
                ),
            ]),
        ),
        ("done", Value::Bool(true)),
        (
            "conditions",
            Value::map([("engine_ran", Value::text("llama.cpp-cuda"))]),
        ),
    ])
}

fn a_desk_that_ran(prompt_reading: Value, first_token: Value) -> Desk {
    let mut desk = Desk::new(std::path::PathBuf::from("/nowhere/control.sock"));
    desk.doing = crate::Doing::Measuring(crate::job::Job::already(
        "measuring a-model".to_owned(),
        vec![
            Value::map([("reading", a_rung())]),
            a_last_line(prompt_reading, first_token),
        ],
    ));
    desk.keep_the_run();
    desk
}

fn row(desk: &Desk, name: &str) -> Option<Vec<String>> {
    desk.tests
        .iter()
        .find(|test| test.name == name)
        .unwrap_or_else(|| panic!("no row named {name}"))
        .result
        .clone()
}

#[test]
fn a_finished_run_fills_the_rows_the_ladder_answers() {
    let desk = a_desk_that_ran(
        Value::map([
            ("measured", Value::Bool(true)),
            ("ms_per_token", Value::text("1.953")),
            (
                "between",
                Value::List(vec![Value::Integer(512), Value::Integer(2048)]),
            ),
            ("method", Value::text("the slope between the outer rungs")),
        ]),
        Value::map([
            ("measured", Value::Bool(true)),
            ("depth", Value::Integer(512)),
            ("ms", Value::text("412.7")),
            (
                "includes",
                Value::text("warm, with the file in the page cache"),
            ),
        ]),
    );
    assert_eq!(
        row(&desk, "Generation speed against depth").as_deref(),
        Some(
            &[
                "at 512 tokens   1.529 ms a token".to_owned(),
                "measured on llama.cpp-cuda".to_owned()
            ][..]
        )
    );
    assert_eq!(
        row(&desk, "Prompt reading speed").as_deref(),
        Some(
            &[
                "1.953 ms a token of prompt".to_owned(),
                "between 512 and 2,048 tokens deep".to_owned(),
                "the slope between the outer rungs".to_owned()
            ][..]
        )
    );
    assert_eq!(
        row(&desk, "Start-up to first token").as_deref(),
        Some(
            &[
                "412.7 ms to the first token, 512 tokens deep".to_owned(),
                "warm, with the file in the page cache".to_owned()
            ][..]
        )
    );
    assert_eq!(
        row(&desk, "Memory ceiling — largest context").as_deref(),
        Some(&["not measured: every rung ran in one window of 4,096 tokens".to_owned()][..])
    );
    assert_eq!(
        row(&desk, "Fall-off with depth").as_deref(),
        Some(
            &["not measured: one rung measured, and a fall-off is read between two".to_owned()][..]
        )
    );
    assert_eq!(
        row(&desk, "MCF's engine and the provisioned one agree"),
        None
    );
}

#[test]
fn a_finished_cross_check_fills_its_row_in_the_daemons_words() {
    let mut desk = Desk::new(std::path::PathBuf::from("/nowhere/control.sock"));
    desk.doing = crate::Doing::CrossChecking(crate::job::Job::already(
        "cross-checking a-model".to_owned(),
        vec![Value::map([
            ("cross_checked", Value::text("a-model")),
            (
                "conditions",
                Value::map([("engine_ran", Value::text("llama.cpp @925e11"))]),
            ),
            (
                "said",
                Value::List(vec![
                    Value::text("MCF's own engine read 120 position(s)"),
                    Value::text("AGREE — never worse than rank 2"),
                ]),
            ),
            ("done", Value::Bool(true)),
        ])],
    ));
    desk.keep_the_cross_check();
    assert_eq!(
        row(&desk, "MCF's engine and the provisioned one agree").as_deref(),
        Some(
            &[
                "MCF's own engine read 120 position(s)".to_owned(),
                "AGREE — never worse than rank 2".to_owned(),
                "against llama.cpp @925e11".to_owned(),
            ][..]
        )
    );
    assert_eq!(row(&desk, "Generation speed against depth"), None);

    let mut refused = crate::job::Job::already("cross-checking a-model".to_owned(), Vec::new());
    refused.refused = Some("the two engines could not be compared".to_owned());
    desk.doing = crate::Doing::CrossChecking(refused);
    desk.keep_the_cross_check();
    assert_eq!(
        row(&desk, "MCF's engine and the provisioned one agree").as_deref(),
        Some(&["the two engines could not be compared".to_owned()][..])
    );
}

#[test]
fn a_figure_the_run_could_not_read_is_said_not_read() {
    let desk = a_desk_that_ran(
        Value::map([
            ("measured", Value::Bool(false)),
            ("why", Value::text("only one rung separated")),
        ]),
        Value::map([
            ("measured", Value::Bool(false)),
            ("why", Value::text("no rung of the ladder separated")),
        ]),
    );
    assert_eq!(
        row(&desk, "Prompt reading speed").as_deref(),
        Some(&["not measured: only one rung separated".to_owned()][..])
    );
    assert_eq!(
        row(&desk, "Start-up to first token").as_deref(),
        Some(&["not measured: no rung of the ladder separated".to_owned()][..])
    );
}

#[test]
fn a_model_nothing_measured_stays_unmeasured() {
    let answered = Value::map([
        ("path", Value::text("/models/b-model.gguf")),
        ("runs", Value::map([("architecture", Value::text("llama"))])),
    ]);
    let held = model_from(&answered);
    assert!(!held.measured());
    assert_eq!(held.fastest, None);
    assert_eq!(held.speed, None);
    assert_eq!(held.speed_at_512(), crate::view::UNKNOWN);
}

#[test]
fn a_ladder_that_never_separated_yields_no_speed() {
    let answered = Value::map([
        ("path", Value::text("/models/c-model.gguf")),
        (
            "runs",
            Value::map([(
                "measured",
                Value::map([(
                    "readings",
                    Value::List(vec![Value::map([
                        ("depth", Value::Integer(512)),
                        ("measured", Value::Bool(false)),
                        (
                            "why",
                            Value::text("no pair of runs at this depth separated"),
                        ),
                    ])]),
                )]),
            )]),
        ),
    ]);
    let held = model_from(&answered);
    assert!(!held.measured());
    assert_eq!(held.fastest, None);
    assert_eq!(held.slowest, None);
}

#[test]
fn a_pasted_reference_is_taken_as_a_value() {
    let mut desk = Desk::new(std::path::PathBuf::from("/nowhere/control.sock"));
    desk.page = Page::Adding;

    desk.paste("an-owner/a-repository-GGUF");
    assert_eq!(
        desk.typed, "an-owner/a-repository-GGUF",
        "a plain reference is kept as it is"
    );

    desk.typed.clear();
    desk.paste("an-owner/a-repository-GGUF\n");
    assert_eq!(desk.typed, "an-owner/a-repository-GGUF");

    desk.typed.clear();
    desk.paste("an-owner/a-repository-GGUF\nand a second line\n");
    assert_eq!(desk.typed, "an-owner/a-repository-GGUF");

    desk.typed.clear();
    desk.paste("\tan-owner/a-repository-GGUF\r");
    assert_eq!(desk.typed, "an-owner/a-repository-GGUF");

    desk.typed.clear();
    desk.typed.push_str("an-owner/");
    desk.paste("   \n  ");
    assert_eq!(desk.typed, "an-owner/", "an empty paste changes nothing");

    desk.typed.clear();
    desk.typed.push_str("an-owner/");
    desk.paste("a-repository-GGUF");
    assert_eq!(desk.typed, "an-owner/a-repository-GGUF");
}

#[test]
fn a_paste_is_bounded() {
    let mut desk = Desk::new(std::path::PathBuf::from("/nowhere/control.sock"));
    desk.page = Page::Adding;
    desk.paste(&"a".repeat(4096));
    assert_eq!(
        desk.typed.chars().count(),
        512,
        "a paste is capped rather than accepted whole"
    );
    desk.paste("bbbb");
    assert_eq!(desk.typed.chars().count(), 512);
}

#[test]
fn a_half_built_component_is_not_a_provisioned_one() {
    let read = |present: bool, provisioned: bool, engine: bool| {
        component_from(&Value::map([
            ("name", Value::text("a-component")),
            ("commit", Value::text("0123456789abcdef")),
            ("role", Value::text("what having it lets MCF claim")),
            ("image", Value::text("an-image")),
            ("present", Value::Bool(present)),
            ("provisioned", Value::Bool(provisioned)),
            ("usable_engine", Value::Bool(engine)),
            ("prefix", Value::text("/somewhere/a-component@0123456789ab")),
        ]))
    };

    let absent = read(false, false, false);
    assert!(!absent.present && !absent.provisioned);

    let partway = read(true, false, false);
    assert!(
        partway.present && !partway.provisioned,
        "a prefix without its provenance is a run that stopped partway"
    );

    let library = read(true, true, false);
    assert!(
        library.provisioned,
        "a component that is not an engine is still provisioned"
    );
    assert!(!library.usable_engine);

    let engine = read(true, true, true);
    assert!(engine.provisioned && engine.usable_engine);

    assert_eq!(engine.commit, "0123456789ab");
}

#[test]
fn what_is_hosted_carries_where_it_answers() {
    let mut desk = Desk::new(std::path::PathBuf::from("/nowhere/control.sock"));
    assert!(
        desk.hosted.is_none(),
        "nothing is hosted until the daemon says so"
    );
    desk.hosted = Some(crate::Hosted {
        model: "/a/store/an-owner/a-repository/a-file.gguf".to_owned(),
        address: "http://127.0.0.1:8080/v1".to_owned(),
        since: "2026-08-31T00:00:00Z".to_owned(),
        context: Some(8192),
        projector: None,
        takes: None,
        api_key: false,
        network_address: None,
        in_use: None,
    });
    let hosting = desk.hosted.as_ref().expect("just set");
    assert_eq!(
        hosting.model.rsplit('/').next(),
        Some("a-file.gguf"),
        "the monitor names what is answering, not where it sits"
    );
    assert_eq!(hosting.context, Some(8192));
}

#[test]
fn a_busy_daemon_is_not_a_missing_one() {
    let mut desk = Desk::new(std::path::PathBuf::from("/nowhere/control.sock"));

    desk.refresh();
    assert!(
        desk.refusal.is_some(),
        "a socket with nothing behind it is a refusal"
    );
    assert!(!desk.busy, "and not merely busy");
    let (word, _) = desk.state_line();
    assert_eq!(word, "NOT UP");

    desk.refusal = None;
    desk.busy = true;
    desk.doing = crate::Doing::Hosting(crate::job::Job::start(
        std::path::Path::new("/nowhere/control.sock"),
        mcf_serve::control::Request::Hosted,
        "holding a-model".to_owned(),
    ));
    let (word, said) = desk.state_line();
    assert_eq!(word, "HOLDING", "it is holding, not down");
    assert!(said.contains("holding a-model"), "{said}");
    assert!(
        said.contains("so far"),
        "how long it has waited, not a promise about how long it will take: {said}"
    );
}

#[test]
fn an_unanswered_poll_keeps_what_was_hosted() {
    let mut desk = Desk::new(std::path::PathBuf::from("/nowhere/control.sock"));
    desk.hosted = Some(crate::Hosted {
        model: "/a/model.gguf".to_owned(),
        address: "http://127.0.0.1:8080/v1".to_owned(),
        since: "2026-09-01T00:00:00Z".to_owned(),
        context: Some(4096),
        projector: None,
        takes: None,
        api_key: false,
        network_address: None,
        in_use: None,
    });
    desk.read_hosted();
    assert!(
        desk.hosted.is_some(),
        "an unanswered poll says nothing about what is held"
    );
    assert!(desk.busy, "and it is recorded as busy");
}

#[test]
fn an_unanswered_reading_keeps_the_models_it_had() {
    let mut desk = Desk::new(std::path::PathBuf::from("/nowhere/control.sock"));
    desk.models = vec![
        Model {
            name: "a-model".to_owned(),
            ..Model::default()
        },
        Model {
            name: "another".to_owned(),
            ..Model::default()
        },
    ];
    desk.refresh();
    assert_eq!(
        desk.models.len(),
        2,
        "a reading that did not arrive says nothing about what is held"
    );
}

#[test]
fn a_companion_file_is_never_offered_as_a_model() {
    let entry = |path: &str, companion: bool| {
        Value::map([
            ("path", Value::text(path)),
            ("bytes", Value::Integer(1_000)),
            ("companion", Value::Bool(companion)),
        ])
    };
    let answered = Value::map([(
        "models",
        Value::List(vec![
            entry("/m/a-model.gguf", false),
            entry("/m/mmproj-F16.gguf", true),
        ]),
    )]);
    let listed: Vec<Model> = answered
        .get("models")
        .and_then(Value::as_list)
        .map(|held| {
            held.iter()
                .filter(|entry| !matches!(entry.get("companion"), Some(Value::Bool(true))))
                .map(model_from)
                .collect()
        })
        .unwrap_or_default();
    assert_eq!(listed.len(), 1, "the projector is not a model");
    assert_eq!(
        listed.first().map(|held| held.name.as_str()),
        Some("a-model")
    );
}

#[test]
fn hosting_with_no_engine_builds_the_one_mcf_named() {
    let mut desk = Desk::new(std::path::PathBuf::from("/nowhere"));
    let answered = Value::map([
        ("path", Value::text("/models/small.gguf")),
        ("bytes", Value::Integer(270_000_000)),
        ("runs", Value::map([("architecture", Value::text("llama"))])),
    ]);
    desk.models = vec![model_from(&answered)];
    desk.chosen = Some(0);
    desk.settings = None;
    desk.no_settings = Some("no engine is installed yet — MCF can build one for you".to_owned());
    desk.needs_engine = Some("llama.cpp".to_owned());

    desk.host_it();

    let crate::Doing::Provisioning(job) = &desk.doing else {
        panic!("Host with no engine must build one, not {:?}", desk.doing);
    };
    assert!(job.what.contains("llama.cpp"), "{}", job.what);
    assert!(job.what.contains("small"), "for which model: {}", job.what);
    let (word, said) = {
        desk.busy = true;
        desk.state_line()
    };
    assert_eq!(word, "BUILDING");
    assert!(said.contains("so far"), "{said}");
}

#[test]
fn hosting_with_a_nameless_refusal_builds_nothing() {
    let mut desk = Desk::new(std::path::PathBuf::from("/nowhere"));
    let answered = Value::map([
        ("path", Value::text("/models/small.gguf")),
        ("bytes", Value::Integer(270_000_000)),
        ("runs", Value::map([("architecture", Value::text("llama"))])),
    ]);
    desk.models = vec![model_from(&answered)];
    desk.chosen = Some(0);
    desk.settings = None;
    desk.no_settings = Some("a model whose header does not say how long".to_owned());
    desk.needs_engine = None;

    desk.host_it();

    assert!(
        matches!(desk.doing, crate::Doing::Nothing),
        "{:?}",
        desk.doing
    );
    assert!(desk.no_settings.is_some());
}

#[test]
fn build_on_the_components_screen_builds_that_component() {
    let mut desk = Desk::new(std::path::PathBuf::from("/nowhere"));
    desk.act(super::Act::Build("llama.cpp".to_owned()));

    let super::Doing::Provisioning(job) = &desk.doing else {
        panic!("Build must build, not {:?}", desk.doing);
    };
    assert_eq!(job.what, "building llama.cpp");
    assert_eq!(desk.building.as_deref(), Some("llama.cpp"));

    let before = desk.doing.job().map(|job| job.what.clone());
    desk.act(super::Act::Build("sdl3".to_owned()));
    let after = desk.doing.job().map(|job| job.what.clone());
    assert!(
        after == before || desk.doing.job().is_some_and(|job| job.finished),
        "{before:?} then {after:?}"
    );
}

#[test]
fn a_refused_build_lands_on_its_own_card() {
    let mut desk = Desk::new(std::path::PathBuf::from("/nowhere"));
    desk.build("llama.cpp");
    let started = std::time::Instant::now();
    while !desk.doing.job().is_some_and(|job| job.finished) {
        assert!(
            started.elapsed() < std::time::Duration::from_secs(10),
            "a job at /nowhere must refuse itself promptly"
        );
        if !desk.hear() {
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
    }
    assert!(desk.building.is_none(), "{:?}", desk.building);
    let (failed, why) = desk.build_failed.clone().expect("the refusal is kept");
    assert_eq!(failed, "llama.cpp");
    assert!(why.contains("not answering"), "{why}");
    desk.build("llama.cpp");
    assert!(desk.build_failed.is_none());
}

#[test]
fn what_is_in_it_is_asked_of_the_daemon_and_never_counted_here() {
    let mut desk = Desk::new(std::path::PathBuf::from("/nowhere/control.sock"));
    desk.act(crate::Act::Go(Page::Anatomy));
    assert_eq!(desk.page, Page::Anatomy);
    assert!(desk.anatomy.is_none());
    let why = desk.no_anatomy.clone().unwrap_or_default();
    assert!(why.contains("Choose a model"), "{why}");

    let mut desk = Desk::new(std::path::PathBuf::from("/nowhere/control.sock"));
    desk.models = vec![Model {
        name: "a-model".to_owned(),
        path: "/nowhere/a-model.gguf".to_owned(),
        bytes: Some(4_000_000_000),
        ..Model::default()
    }];
    desk.chosen = Some(0);
    desk.act(crate::Act::Go(Page::Anatomy));
    assert!(desk.anatomy.is_none());
    let why = desk.no_anatomy.clone().unwrap_or_default();
    assert!(why.contains("not answering"), "{why}");
    assert!(
        !why.contains("sock"),
        "the socket is shown to a person: {why}"
    );

    desk.no_anatomy = None;
    desk.act(crate::Act::Go(Page::Vocabulary));
    assert_eq!(desk.page, Page::Vocabulary);
    assert!(desk.anatomy.is_none());
    let why = desk.no_anatomy.clone().unwrap_or_default();
    assert!(why.contains("not answering"), "{why}");
}

#[test]
fn an_anatomy_answer_is_read_as_the_daemon_wrote_it() {
    let line = r#"{"model":"m.gguf","counted":{"elements":100,"bytes":50,"unsized_tensors":0,"blocks":2,"output_tied":true,"active":null,"parts":[{"part":"embedding","tensors":1,"elements":40,"bytes":20}],"encodings":[{"encoding":"Q4_K","tensors":3,"elements":100,"bytes":50}],"block_shapes":[{"blocks":[0,1],"mixing":"attention","feed":"dense","experts":null,"shared_expert":false,"said":"attention over the context, keys and values kept per position; one feed-forward every token passes","ranged":"0–1","bits_hundredths":[400,400],"tensors":2,"elements":60,"bytes":30}],"attending":2,"recurrent":0},"agreements":[{"what":"blocks","declared":"2","observed":"2","agrees":true}],"work":{"multiply_adds":100,"head_width":8,"queries_per_key":1,"attention_at_context":null,"cache":{"sized":true,"per_token":64,"key_heads":1,"per_head":16,"latent":false,"kept":"16 for a key and a value","context":null,"at_context":null,"sliding_window":null,"attending":2,"blocks":2,"recurrent":0}},"vocabulary":{"tokens":3,"segmentation":"llama","merges":null,"kinds":[{"kind":"text","count":2},{"kind":"control","count":1}],"word_starts":1,"digit_tokens":0,"longest_digits":0,"digits":"none: every digit is spelled some other way","longest":{"token":"▁the","bytes":6},"named":[{"what":"end of text","identifier":2,"spelled":"</s>","beyond":null}],"adds_beginning":true,"beginning":"yes, the file says so","template":null,"no_template":"none in the file — a chat turn has no framing the file states"}}"#;
    let value = mcf_record::json::parse(line).expect("the line parses");
    let said = mcf_serve::anatomy::Said::from_value(&value).expect("the value reads");
    assert_eq!(said.elements, 100);
    assert_eq!(said.families.len(), 1);
    assert_eq!(
        said.agreements.first().and_then(|held| held.agrees),
        Some(true)
    );
    assert!(matches!(
        said.cache,
        mcf_serve::anatomy::SaidCache::Sized { per_token: 64, .. }
    ));
    assert_eq!(said.vocabulary.tokens, 3);
    assert_eq!(said.vocabulary.segmentation, "llama");
    assert_eq!(
        said.vocabulary
            .named
            .first()
            .and_then(|n| n.spelled.as_deref()),
        Some("</s>")
    );
    assert!(said.vocabulary.template.is_err());
}

#[test]
fn the_window_asks_the_turn_it_was_given() {
    let mut desk = Desk::new(std::path::PathBuf::from("/tmp/mcf-not-here.sock"));
    assert_eq!(desk.asked_turn(), None, "nothing typed asks for nothing");
    desk.page = Page::Hosting;
    desk.caret = Caret::System;
    desk.typing().push_str("Be terse.");
    desk.caret = Caret::Effort;
    desk.typing().push_str("low");
    desk.cycle_thinking();
    let asked = desk.asked_turn().expect("a turn was asked for");
    assert_eq!(asked.system.as_deref(), Some("Be terse."));
    assert_eq!(asked.effort.as_deref(), Some("low"));
    assert_eq!(asked.thinking, Some(true));
    assert!(desk.typed.is_empty(), "the question is still empty");
}

#[test]
fn thinking_rounds_through_unsaid() {
    let mut desk = Desk::new(std::path::PathBuf::from("/tmp/mcf-not-here.sock"));
    assert_eq!(desk.thinking, None);
    desk.cycle_thinking();
    assert_eq!(desk.thinking, Some(true));
    desk.cycle_thinking();
    assert_eq!(desk.thinking, Some(false));
    desk.cycle_thinking();
    assert_eq!(desk.thinking, None, "back to what the template does itself");
}

#[test]
fn a_stale_caret_does_not_take_another_screens_typing() {
    let mut desk = Desk::new(std::path::PathBuf::from("/tmp/mcf-not-here.sock"));
    desk.page = Page::Hosting;
    desk.caret = Caret::Picture;
    desk.typing().push_str("/tmp/a.png");
    desk.page = Page::Prompt;
    desk.typing().push_str("a document");
    assert_eq!(desk.picture, "/tmp/a.png");
    assert_eq!(desk.typed, "a document");
}

#[test]
fn a_run_under_way_is_said_on_every_page() {
    let mut desk = Desk::new(std::path::PathBuf::from("/nowhere/control.sock"));
    assert_eq!(desk.state_word(), "MCF");
    assert!(
        desk.under_way().is_none(),
        "nothing runs, nothing is under way"
    );

    let mut job = crate::job::Job::start(
        std::path::Path::new("/nowhere/control.sock"),
        mcf_serve::control::Request::Hosted,
        "measuring a-model".to_owned(),
    );
    job.answers.push(Value::map([(
        "running",
        Value::map([
            ("depth", Value::Integer(1024)),
            ("produce", Value::Integer(1)),
            ("repeat", Value::Integer(1)),
            ("of_repeats", Value::Integer(3)),
        ]),
    )]));
    desk.doing = crate::Doing::Measuring(job);
    desk.page = Page::Monitor;
    let word = desk.state_word();
    assert!(word.starts_with("working — measuring a-model"), "{word}");
    assert!(
        word.contains("so far"),
        "how long it has run, not a promise: {word}"
    );
    let step = desk
        .doing
        .job()
        .and_then(mcf_tui::screens::diagnostics::step_of)
        .unwrap_or_default();
    assert!(
        step.contains("1,024 tokens") && step.contains("loading the model"),
        "{step}"
    );

    if let crate::Doing::Measuring(job) = &mut desk.doing {
        job.finished = true;
    }
    assert_eq!(desk.state_word(), "MCF");
}

#[test]
fn a_load_says_how_far_and_about_how_long() {
    let said = crate::loading_said(6_100_000_000, false, Some(17_600_000_000), 12);
    assert!(
        said.starts_with("Loading: 6.1 GB of 17.6 GB weights · 12 s"),
        "{said}"
    );
    assert!(said.ends_with("~22 s left"), "{said}");
    let nothing_yet = crate::loading_said(100_000_000, false, Some(17_600_000_000), 3);
    assert!(
        !nothing_yet.contains("left"),
        "no estimate off the first crumbs: {nothing_yet}"
    );
    let past = crate::loading_said(21_200_000_000, true, Some(16_300_000_000), 2);
    assert!(
        past.contains("weights on") && !past.contains("left"),
        "past the weights, what follows is not the file's to size: {past}"
    );
    let without_size = crate::loading_said(6_100_000_000, true, None, 12);
    assert_eq!(without_size, "Loading to GPU: 6.1 GB · 12 s");

    let desk = Desk::new(std::path::PathBuf::from("/nowhere/control.sock"));
    assert!(desk.loading_line().is_none());
}

#[test]
fn a_typed_setting_is_taken_or_refused_with_the_word() {
    let mut desk = Desk::new(std::path::PathBuf::from("/nowhere/control.sock"));
    let recommended = mcf_serve::hosting::Hosting::recommended(
        "llama.cpp",
        "CPU",
        false,
        4096,
        Some(8),
        true,
        None,
    );
    desk.settings = Some(recommended.clone());
    desk.recommended = Some(recommended);
    desk.page = Page::Models;

    desk.edit(crate::Field::Context);
    assert!(
        desk.takes_typing(),
        "a field being typed into takes the keys"
    );
    desk.typing().clear();
    desk.typing().push_str("32,768");
    desk.apply_edit();
    assert_eq!(
        desk.settings.as_ref().map(|held| held.context),
        Some(32_768)
    );
    assert!(desk.edit_refused.is_none());

    desk.edit(crate::Field::Context);
    desk.typing().clear();
    desk.typing().push_str("lots");
    desk.apply_edit();
    assert_eq!(
        desk.settings.as_ref().map(|held| held.context),
        Some(32_768),
        "a word is not a window, and the window stays as it was"
    );
    let why = desk.edit_refused.clone().unwrap_or_default();
    assert!(
        why.contains("lots"),
        "the refusal names what was typed: {why}"
    );

    desk.edit(crate::Field::Port);
    desk.typing().clear();
    desk.typing().push_str("80");
    desk.apply_edit();
    assert!(
        desk.edit_refused
            .as_deref()
            .is_some_and(|why| why.contains("1024")),
        "a port MCF cannot bind without rights is refused, not tried"
    );

    desk.flip(crate::Switch::FlashAttention);
    assert_eq!(
        desk.settings.as_ref().map(|held| held.flash_attention),
        Some(true)
    );
    assert!(
        desk.editing.is_none(),
        "nothing is being typed into a setting now"
    );
    assert!(
        desk.takes_typing(),
        "the library's search field still takes typing (D51)"
    );
}

#[test]
fn the_challenges_card_takes_the_retries_and_the_window_and_runs_with_them() {
    let mut desk = Desk::new(std::path::PathBuf::from("/nowhere/control.sock"));
    desk.page = Page::Diagnostics;
    assert_eq!(
        desk.eval_arguments("/m.gguf", Some(0)),
        vec!["eval", "/m.gguf", "--only", "challenges", "--tier", "easy"],
        "a tier's row is the catalogue held to that tier; ten attempts is the default and is not said"
    );
    assert_eq!(
        desk.eval_arguments("/m.gguf", Some(4)),
        vec!["eval", "/m.gguf", "--only", "editing"]
    );
    desk.edit(crate::Field::Retries);
    assert!(desk.takes_typing(), "the card's field takes the keys");
    desk.typing().clear();
    desk.typing().push('3');
    desk.entered();
    assert_eq!(desk.retries, 3);
    desk.edit(crate::Field::Window);
    desk.typing().clear();
    desk.typing().push_str("1024");
    desk.apply_edit();
    assert_eq!(desk.challenge_window, None);
    let why = desk.edit_refused.clone().unwrap_or_default();
    assert!(why.contains("4096"), "the refusal names the floor: {why}");
    desk.edit(crate::Field::Window);
    desk.typing().clear();
    desk.typing().push_str("8,192");
    desk.apply_edit();
    assert_eq!(desk.challenge_window, Some(8192));
    assert!(desk.edit_refused.is_none());
    desk.edit(crate::Field::Languages);
    desk.typing().clear();
    desk.typing().push_str("go, cobol");
    desk.apply_edit();
    assert_eq!(desk.challenge_languages, None);
    assert!(
        desk.edit_refused
            .clone()
            .unwrap_or_default()
            .contains("cobol")
    );
    desk.edit(crate::Field::Languages);
    desk.typing().clear();
    desk.typing().push_str("go, python");
    desk.apply_edit();
    assert_eq!(desk.challenge_languages.as_deref(), Some("go,python"));
    assert_eq!(
        desk.eval_arguments("/m.gguf", Some(2)),
        vec![
            "eval",
            "/m.gguf",
            "--only",
            "challenges",
            "--tier",
            "hard",
            "--languages",
            "go,python",
            "--retries",
            "3",
            "--window",
            "8192"
        ]
    );
    desk.edit(crate::Field::Window);
    desk.typing().clear();
    desk.apply_edit();
    assert_eq!(
        desk.challenge_window, None,
        "emptied is sized to the turn again"
    );
    assert!(!desk.takes_typing(), "nothing is being typed once applied");
}

#[test]
fn a_run_on_the_hosted_model_is_said_to_cost_a_second_copy() {
    let mut desk = Desk::new(std::path::PathBuf::from("/nowhere/control.sock"));
    desk.models = vec![
        Model {
            name: "a".to_owned(),
            path: "/m/a.gguf".to_owned(),
            bytes: Some(17_665_334_432),
            ..Model::default()
        },
        Model {
            name: "b".to_owned(),
            path: "/m/b.gguf".to_owned(),
            bytes: None,
            ..Model::default()
        },
    ];
    desk.chosen = Some(0);
    assert_eq!(desk.second_copy(), None, "nothing hosted, nothing to say");
    desk.hosted = Some(crate::Hosted {
        model: "/m/a.gguf".to_owned(),
        address: "127.0.0.1:8080".to_owned(),
        since: "2026-09-06T13:56:00Z".to_owned(),
        context: Some(4096),
        projector: None,
        takes: None,
        api_key: false,
        network_address: None,
        in_use: None,
    });
    let said = desk.second_copy().unwrap_or_default();
    assert!(
        said.contains("lets the hold go") && said.contains("one copy"),
        "a run on the hosted model lets the hold go and takes it up again: {said}"
    );
    desk.chosen = Some(1);
    let said = desk.second_copy().unwrap_or_default();
    assert!(
        said.contains("one model runs at a time") && said.contains("a is hosted"),
        "a run on another model is the rule, said before Run: {said}"
    );
}

#[test]
fn a_running_suite_shows_its_results_as_they_come() {
    let mut desk = Desk::new(std::path::PathBuf::from("/nowhere/control.sock"));
    assert!(desk.results_so_far().is_empty());
    desk.doing = crate::Doing::Evaluating(crate::job::Job::already(
        "running the Challenges: easy suite on a-model".to_owned(),
        vec![
            Value::map([(
                "line",
                Value::text("progress: 0/28 challenges · easy · python · merge-sorted"),
            )]),
            Value::map([("line", Value::text("result: easy · arrays · merge-sorted"))]),
            Value::map([("line", Value::text("result:     Merge two sorted lists."))]),
            Value::map([(
                "line",
                Value::text(
                    "result:     python     solved at attempt 1 · 0 correction(s) · 97 token(s) · 3.7s",
                ),
            )]),
            Value::map([(
                "line",
                Value::text("progress: 1/28 challenges · easy · go · merge-sorted"),
            )]),
        ],
    ));
    assert_eq!(
        desk.results_so_far(),
        vec![
            "easy · arrays · merge-sorted",
            "    Merge two sorted lists.",
            "    python     solved at attempt 1 · 0 correction(s) · 97 token(s) · 3.7s",
        ]
    );
}

#[test]
fn a_row_whose_run_did_not_finish_resumes_it() {
    let mut desk = Desk::new(std::path::PathBuf::from("/nowhere/control.sock"));
    desk.models = vec![Model {
        name: "a".to_owned(),
        path: "/m/a.gguf".to_owned(),
        ..Model::default()
    }];
    desk.chosen = Some(0);
    let row = mcf_record::readings::Reading::new(
        &[
            ("challenge", Value::text("merge-sorted")),
            ("language", Value::text("python")),
        ],
        "solved",
        1,
        "bool",
    );
    let part = |ended: Option<&str>| {
        mcf_record::readings::part_body(
            "/m/a.gguf",
            "challenges-easy",
            "e",
            vec![],
            std::slice::from_ref(&row),
            "r1",
            1,
            ended,
        )
    };
    desk.readings = Some((
        "/m/a.gguf".to_owned(),
        vec![part(Some("stopped after 3 of 14"))],
    ));
    assert!(desk.resumable(crate::Diagnostic::Eval(0)));
    assert_eq!(
        desk.eval_arguments("/m/a.gguf", Some(0)),
        vec![
            "eval",
            "/m/a.gguf",
            "--only",
            "challenges",
            "--tier",
            "easy",
            "--resume"
        ]
    );
    desk.readings = Some(("/m/a.gguf".to_owned(), vec![part(None)]));
    assert!(
        desk.resumable(crate::Diagnostic::Eval(0)),
        "cut off is resumable too"
    );
    desk.readings = Some(("/m/a.gguf".to_owned(), vec![part(Some("finished"))]));
    assert!(!desk.resumable(crate::Diagnostic::Eval(0)));
    assert!(
        !desk
            .eval_arguments("/m/a.gguf", Some(0))
            .contains(&"--resume".to_owned())
    );
}

#[test]
fn a_run_says_about_how_long_is_left_from_its_own_pace() {
    let mut desk = Desk::new(std::path::PathBuf::from("/nowhere/control.sock"));
    let mut going = crate::job::Job::already(
        "running the Challenges: easy suite on a-model".to_owned(),
        vec![Value::map([(
            "line",
            Value::text("progress: 1/56 challenges · easy · python · x"),
        )])],
    );
    going.finished = false;
    desk.doing = crate::Doing::Evaluating(going);
    assert_eq!(desk.time_left(), None, "a fifty-sixth done is not a pace");
    if let crate::Doing::Evaluating(job) = &mut desk.doing {
        job.answers.push(Value::map([(
            "line",
            Value::text("progress: 14/56 challenges · easy · go · y"),
        )]));
    }
    assert_eq!(desk.time_left(), None);
}

#[test]
fn the_model_under_test_is_read_and_its_cost_is_summed() {
    let reading = |joules: &str, over: &str, live: &str| {
        Value::map([
            ("model", Value::text("/m/Assistant-2B-Instruct-Q4_K_M.gguf")),
            ("engine", Value::text("provisioned llama.cpp @925e")),
            ("window", Value::Integer(8192)),
            (
                "use",
                Value::map([
                    ("generated_tokens", Value::text("1500")),
                    ("generated_tokens_live", Value::text(live)),
                    ("prompted_tokens", Value::text("9000")),
                    ("requests_processing", Value::text("1")),
                    ("card_power_watts", Value::text("100.000")),
                    ("power_named", Value::text("package")),
                    ("card_energy_joules", Value::text(joules)),
                    ("card_energy_over_seconds", Value::text(over)),
                ]),
            ),
        ])
    };
    let under = crate::UnderTest::from_value(&reading("200.000", "2.000", "1500"));
    assert_eq!(under.name(), "Assistant-2B-Instruct-Q4_K_M");
    assert_eq!(under.window, Some(8192));
    assert_eq!(under.in_use.generated, Some(1500));
    assert_eq!(under.in_use.prompted, Some(9000));
    assert_eq!(under.in_use.card_power_watts, Some(100.0));
    assert_eq!(under.in_use.power_named.as_deref(), Some("package"));

    let mut desk = Desk::new(std::path::PathBuf::from("/nowhere/control.sock"));
    desk.under_test = Some(under);
    desk.tally();
    assert_eq!(desk.spent.seconds, 0, "nothing running: nothing spent");

    let mut going = crate::job::Job::already("running".to_owned(), Vec::new());
    going.finished = false;
    desk.doing = crate::Doing::Evaluating(going);
    desk.tally();
    assert_eq!(desk.spent.seconds, 2);
    assert_eq!(
        desk.spent.millijoules, 200_000,
        "two hundred joules, as the daemon measured them"
    );
    assert_eq!(desk.spent.tokens_at_start, Some(1500));

    desk.under_test = Some(crate::UnderTest::from_value(&reading(
        "300.000", "3.000", "1900",
    )));
    desk.tally();
    assert_eq!(desk.spent.seconds, 3);
    assert_eq!(desk.spent.tokens(), Some(400));
    assert_eq!(
        desk.spent.tokens_per_kilojoule(),
        Some(1333),
        "four hundred tokens over three tenths of a kilojoule"
    );
    desk.tally_afresh();
    assert_eq!(desk.spent, crate::Spent::default());
}

#[test]
fn a_run_on_a_card_that_says_nothing_counts_seconds_and_no_energy() {
    let answer = Value::map([
        ("model", Value::text("/m/Assistant-2B-Instruct-Q4_K_M.gguf")),
        ("engine", Value::text("provisioned llama.cpp @925e")),
        ("window", Value::Integer(8192)),
        (
            "use",
            Value::map([("generated_tokens", Value::text("1500"))]),
        ),
    ]);
    let mut desk = Desk::new(std::path::PathBuf::from("/nowhere/control.sock"));
    desk.under_test = Some(crate::UnderTest::from_value(&answer));
    let mut going = crate::job::Job::already("running".to_owned(), Vec::new());
    going.finished = false;
    desk.doing = crate::Doing::Evaluating(going);
    desk.tally();
    desk.tally();
    assert_eq!(desk.spent.seconds, 2);
    assert_eq!(desk.spent.millijoules, 0);
    assert_eq!(desk.spent.tokens_per_kilojoule(), None);
}

#[test]
fn start_server_takes_the_key_being_typed_and_names_the_field_it_needs() {
    let mut desk = Desk::new(std::path::PathBuf::from("/nowhere/control.sock"));
    let recommended = mcf_serve::hosting::Hosting::recommended(
        "llama.cpp",
        "CPU",
        false,
        4096,
        Some(8),
        true,
        None,
    );
    desk.settings = Some(recommended.clone());
    desk.recommended = Some(recommended);
    desk.models = vec![Model {
        name: "a".to_owned(),
        path: "/m/a.gguf".to_owned(),
        ..Model::default()
    }];
    desk.chosen = Some(0);
    desk.page = Page::Models;
    desk.flip(crate::Switch::Open);
    assert!(desk.settings.as_ref().is_some_and(|held| held.open));
    desk.host_it();
    let why = desk.host_refused.clone().unwrap_or_default();
    assert!(why.contains("API key field"), "{why}");
    assert!(
        !matches!(desk.doing, crate::Doing::Hosting(_)),
        "nothing was asked of the daemon"
    );
    desk.edit(crate::Field::ApiKey);
    desk.typing().clear();
    desk.typing().push_str("mcf-home");
    desk.host_it();
    assert_eq!(
        desk.settings
            .as_ref()
            .and_then(|held| held.api_key.clone())
            .as_deref(),
        Some("mcf-home"),
        "the key typed is the key the hold gets"
    );
    assert!(
        !desk
            .host_refused
            .as_deref()
            .is_some_and(|why| why.contains("API key field")),
        "with a key, the hold is not refused for one: {:?}",
        desk.host_refused
    );
}

#[test]
fn the_diagnostics_are_one_list_and_a_probes_finding_is_kept() {
    use crate::Diagnostic;
    let all = Diagnostic::all();
    assert_eq!(all.len(), 4 + 9 + 47 + 7, "{all:?}");
    let names: std::collections::BTreeSet<&str> = all.iter().map(|held| held.name()).collect();
    assert_eq!(names.len(), all.len(), "two rows share a name");
    assert_eq!(Diagnostic::Probe(2).name(), "stop-conditions");
    assert_eq!(Diagnostic::Measure(0).name(), "offload-curve");
    assert_eq!(Diagnostic::Probe(2).card(), crate::Card::Capabilities);
    assert_eq!(Diagnostic::Measure(0).card(), crate::Card::Performance);
    assert_eq!(Diagnostic::Measure(6).card(), crate::Card::Fidelity);
    assert!(!Diagnostic::Measure(6).answers().is_empty());
    assert_eq!(Diagnostic::Throughput.method(), None);
    assert_eq!(Diagnostic::Throughput.readings_method(), Some("throughput"));
    assert_eq!(Diagnostic::Probe(3).name(), "tool-calls");
    assert_eq!(
        Diagnostic::Probe(3).readings_method(),
        Some("tool-calling"),
        "a probe's readings are under its record name"
    );
    let families = Diagnostic::families();
    assert_eq!(families.len(), 6);
    assert_eq!(families[1].1, Some(crate::Card::Capabilities));
    assert_eq!(families[5].1, Some(crate::Card::Coding));
    assert_eq!(families[5].2.len(), crate::SUITES.len());
    assert_eq!(Diagnostic::Eval(4).name(), "Editing");
    assert_eq!(Diagnostic::Eval(5).readings_method(), Some("test-writing"));
    assert_eq!(Diagnostic::Eval(0).suite(), Some("challenges-easy"));
    assert_eq!(
        Diagnostic::Eval(2).readings_method(),
        Some("challenges-hard")
    );
    assert_eq!(Diagnostic::Eval(0).card(), crate::Card::Coding);

    let mut desk = Desk::new(std::path::PathBuf::from("/nowhere"));
    assert_eq!(desk.diagnostic, Diagnostic::Throughput);
    assert!(!desk.probes_apply);
    desk.act(crate::Act::ApplyProbes);
    assert!(desk.probes_apply);
    desk.act(crate::Act::Show(Diagnostic::Probe(2)));
    assert_eq!(desk.diagnostic, Diagnostic::Probe(2));

    desk.models = vec![Model::default()];
    desk.chosen = Some(0);
    let step = mcf_record::json::Value::map([
        ("name", mcf_record::json::Value::text("stop-conditions")),
        ("count", mcf_record::json::Value::Integer(1)),
        ("of", mcf_record::json::Value::Integer(1)),
    ]);
    let mut job = crate::job::Job::already(
        "probing".to_owned(),
        vec![
            mcf_record::json::Value::map([
                ("step", step.clone()),
                ("lines", mcf_record::json::Value::List(Vec::new())),
            ]),
            mcf_record::json::Value::map([
                ("step", step),
                (
                    "lines",
                    mcf_record::json::Value::List(vec![
                        mcf_record::json::Value::text("  stop-conditions"),
                        mcf_record::json::Value::text(" ended its own turn in 5 of 5 trials"),
                    ]),
                ),
            ]),
        ],
    );
    job.finished = false;
    desk.doing = crate::Doing::Probing(job);
    assert_eq!(
        desk.running_diagnostic(),
        Some(Diagnostic::Probe(2)),
        "the row running is the one the step names"
    );
    if let crate::Doing::Probing(job) = &mut desk.doing {
        job.finished = true;
    }
    assert_eq!(desk.running_diagnostic(), None);
    desk.keep_the_probes();
    let probed = &desk.models[0].probed;
    assert_eq!(probed.len(), 1, "{probed:?}");
    assert_eq!(probed[0].name, "stop-conditions");
    assert_eq!(probed[0].lines.len(), 2);
    assert!(probed[0].at.is_some(), "a finding just taken says when");
    let (when, _) = desk
        .last_run(Diagnostic::Probe(2))
        .unwrap_or_else(|| panic!("the row does not say when it last ran"));
    assert_eq!(crate::when_said(&when).len(), 16);
    assert_eq!(desk.last_run(Diagnostic::Probe(0)), None);
    assert_eq!(
        crate::when_said("2026-09-05T15:41:12.123456789Z (local offset +00:00)"),
        "2026-09-05 15:41"
    );
}

#[test]
fn run_all_takes_every_run_in_turn_and_the_strip_says_how_far() {
    let mut desk = Desk::new(std::path::PathBuf::from("/nowhere"));
    desk.models = vec![Model::default()];
    desk.chosen = Some(0);
    assert_eq!(desk.run_fraction(), None, "nothing running has no fraction");
    desk.act(crate::Act::RunAll);
    assert!(
        matches!(desk.doing, crate::Doing::Measuring(_)),
        "a Run all does not begin with the ladder"
    );
    assert_eq!(desk.queued.len(), Desk::EVERY_RUN.len() - 1);
    assert_eq!(desk.sequence_place(), Some((1, Desk::EVERY_RUN.len())));
    let mut going = crate::job::Job::already(
        "measuring".to_owned(),
        vec![mcf_record::json::Value::map([
            ("so_far", mcf_record::json::Value::Integer(2)),
            ("of", mcf_record::json::Value::Integer(4)),
        ])],
    );
    going.finished = false;
    desk.doing = crate::Doing::Measuring(going);
    assert!((desk.run_fraction().unwrap_or(0.0) - 0.5).abs() < 0.01);
    let whole = desk.sequence_fraction().unwrap_or(0.0);
    assert!((whole - 0.5 / 7.0).abs() < 0.01, "{whole}");
    if let crate::Doing::Measuring(job) = &mut desk.doing {
        job.finished = true;
    }
    desk.hear();
    assert!(
        matches!(desk.doing, crate::Doing::CrossChecking(_)),
        "the cross-check did not follow the ladder"
    );
    assert_eq!(desk.sequence_place(), Some((2, Desk::EVERY_RUN.len())));
    let step = |count: i64, lines: usize| {
        mcf_record::json::Value::map([
            (
                "step",
                mcf_record::json::Value::map([
                    ("name", mcf_record::json::Value::text("context")),
                    ("count", mcf_record::json::Value::Integer(count)),
                    ("of", mcf_record::json::Value::Integer(9)),
                ]),
            ),
            (
                "lines",
                mcf_record::json::Value::List(vec![mcf_record::json::Value::text("said"); lines]),
            ),
        ])
    };
    let mut probing = crate::job::Job::already("probing".to_owned(), vec![step(2, 0)]);
    probing.finished = false;
    desk.doing = crate::Doing::Probing(probing);
    assert!((desk.run_fraction().unwrap_or(0.0) - 1.0 / 9.0).abs() < 0.01);
    let mut probing = crate::job::Job::already("probing".to_owned(), vec![step(2, 1)]);
    probing.finished = false;
    desk.doing = crate::Doing::Probing(probing);
    assert!((desk.run_fraction().unwrap_or(0.0) - 2.0 / 9.0).abs() < 0.01);
    if let crate::Doing::Probing(job) = &mut desk.doing {
        job.finished = true;
        job.refused = Some("no".to_owned());
    }
    desk.hear();
    assert!(desk.queued.is_empty(), "a refusal did not end the sequence");
    assert_eq!(desk.sequence_place(), None);
    desk.act(crate::Act::RunAll);
    desk.act(crate::Act::Stop);
    assert!(desk.queued.is_empty());
}

#[test]
fn a_findings_columns_are_parted_for_the_page() {
    assert_eq!(
        crate::view::columns_said("  batch    64      405.3 ms       2525 tokens a second"),
        "batch · 64 · 405.3 ms · 2525 tokens a second"
    );
    assert_eq!(
        crate::view::columns_said("one line, one space"),
        "one line, one space"
    );
}

#[test]
fn a_family_runs_whole_and_a_row_runs_one() {
    use crate::Diagnostic;
    let mut desk = Desk::new(std::path::PathBuf::from("/nowhere"));
    assert_eq!(crate::Card::Performance.measures().len(), 6);
    assert!(crate::Card::Throughput.measures().is_empty());
    desk.models = vec![Model {
        probed: vec![crate::Finding {
            name: "determinism".to_owned(),
            at: Some("2026-09-01T10:00:00.000000000Z (local offset +00:00)".to_owned()),
            engine: Some("provisioned llama.cpp".to_owned()),
            lines: vec!["old".to_owned()],
        }],
        readings_at: std::collections::BTreeMap::new(),
        ..Model::default()
    }];
    desk.chosen = Some(0);
    let place = mcf_serve::examine::MEASURES
        .iter()
        .position(|name| *name == "determinism")
        .unwrap_or_else(|| panic!("determinism is not a measurement"));
    assert_eq!(
        desk.last_run(Diagnostic::Measure(place)),
        Some((
            "2026-09-01T10:00:00.000000000Z (local offset +00:00)".to_owned(),
            Some("provisioned llama.cpp".to_owned())
        ))
    );
    let step = |name: &str| {
        mcf_record::json::Value::map([
            ("name", mcf_record::json::Value::text(name)),
            ("count", mcf_record::json::Value::Integer(1)),
            ("of", mcf_record::json::Value::Integer(2)),
        ])
    };
    let mut job = crate::job::Job::already(
        "examining".to_owned(),
        vec![
            mcf_record::json::Value::map([
                ("step", step("determinism")),
                (
                    "lines",
                    mcf_record::json::Value::List(vec![mcf_record::json::Value::text(
                        "  5 of 5 run(s) identical",
                    )]),
                ),
            ]),
            mcf_record::json::Value::map([
                ("step", step("bits-per-byte")),
                (
                    "lines",
                    mcf_record::json::Value::List(vec![mcf_record::json::Value::text(
                        "  1.2 bits a byte",
                    )]),
                ),
            ]),
        ],
    );
    job.finished = true;
    desk.doing = crate::Doing::Examining(job);
    desk.keep_the_examination();
    let probed = &desk.models[0].probed;
    assert_eq!(probed.len(), 2, "{probed:?}");
    assert_eq!(
        probed[0].lines,
        vec!["  5 of 5 run(s) identical".to_owned()]
    );
    assert_eq!(probed[1].name, "bits-per-byte");
    desk.act(crate::Act::Run(crate::Card::Behaviour));
    assert!(
        matches!(desk.doing, crate::Doing::Examining(_)),
        "the family's Run all did not start an examination"
    );
    desk.act(crate::Act::RunOne(Diagnostic::Measure(place)));
    assert!(matches!(desk.doing, crate::Doing::Examining(_)));
    assert_eq!(desk.diagnostic, Diagnostic::Measure(place));
    desk.act(crate::Act::RunOne(Diagnostic::Probe(0)));
    assert!(matches!(desk.doing, crate::Doing::Probing(_)));
    desk.act(crate::Act::RunOne(Diagnostic::CrossCheck));
    assert!(matches!(desk.doing, crate::Doing::CrossChecking(_)));
}

#[test]
fn the_last_hold_settings_come_back_with_one_press() {
    let mut desk = Desk::new(std::path::PathBuf::from("/nowhere"));
    let recommended = mcf_serve::hosting::Hosting::recommended(
        "llama.cpp",
        "CPU",
        false,
        32_768,
        Some(8),
        false,
        None,
    );
    let mut last = recommended.clone();
    last.context = 8_192;
    desk.recommended = Some(recommended.clone());
    desk.settings = Some(recommended.clone());
    desk.last_settings = Some((last.clone(), "2026-09-05".to_owned()));
    desk.act(crate::Act::LastSettings);
    assert_eq!(desk.settings.as_ref().map(|held| held.context), Some(8_192));
    desk.act(crate::Act::Recommended);
    assert_eq!(
        desk.settings.as_ref().map(|held| held.context),
        Some(32_768)
    );
}

#[test]
fn the_search_field_narrows_the_library() {
    let mut desk = Desk::new(std::path::PathBuf::from("/nowhere"));
    let model = |name: &str, architecture: &str| Model {
        name: name.to_owned(),
        path: format!("/store/owner/{name}-GGUF/{name}.gguf"),
        architecture: Some(architecture.to_owned()),
        ..Model::default()
    };
    desk.models = vec![
        model("Assistant-8B-Q4_K_M", "llama"),
        model("Coder-30B-Q4_K_XL", "a-moe-architecture"),
        model("Tiny-135M", "llama"),
    ];
    let members = |desk: &Desk| -> Vec<usize> {
        desk.library()
            .into_iter()
            .flat_map(|group| group.members)
            .collect()
    };
    assert_eq!(members(&desk), vec![0, 1, 2]);
    desk.filter = "LLAMA".to_owned();
    assert_eq!(
        members(&desk),
        vec![0, 2],
        "the architecture counts, case aside"
    );
    desk.filter = "q4_k".to_owned();
    assert_eq!(members(&desk), vec![0, 1]);
    desk.filter = "coder xl".to_owned();
    assert_eq!(members(&desk), vec![1], "every word must match");
    desk.filter = "gemma".to_owned();
    assert!(desk.library().is_empty());
    assert!(!desk.hub_matches(), "nothing has been asked of the hub");
    desk.page = Page::Models;
    desk.entered();
    assert!(
        matches!(desk.doing, crate::Doing::Listing(_)),
        "Return on words nothing matches did not search the hub"
    );
}

#[test]
fn the_hubs_answer_is_kept_for_its_words() {
    use mcf_record::json::Value;
    let mut desk = Desk::new(std::path::PathBuf::from("/nowhere"));
    desk.filter = "gemma".to_owned();
    let mut job = crate::job::Job::already(
        "searching".to_owned(),
        vec![Value::map([
            ("query", Value::text("gemma")),
            (
                "repositories",
                Value::List(vec![Value::map([
                    ("id", Value::text("someone/gemma-GGUF")),
                    ("downloads", Value::Integer(1_295_081)),
                ])]),
            ),
            ("done", Value::Bool(true)),
        ])],
    );
    job.finished = true;
    desk.doing = crate::Doing::Listing(job);
    desk.hear_for_review();
    let hub = desk.hub.as_ref().expect("the hub's answer is kept");
    assert_eq!(hub.query, "gemma");
    assert_eq!(hub.repositories.len(), 1);
    assert_eq!(hub.repositories[0].downloads, Some(1_295_081));
    assert!(desk.hub_matches());
    desk.filter = "gemma 4".to_owned();
    assert!(!desk.hub_matches(), "other words are another question");
}

#[test]
fn a_model_is_a_repository_with_its_quantizations() {
    let mut desk = Desk::new(std::path::PathBuf::from("/nowhere"));
    let held = |file: &str, repository: Option<&str>| Model {
        name: file.trim_end_matches(".gguf").to_owned(),
        path: format!("/store/{file}"),
        file: file.to_owned(),
        repository: repository.map(str::to_owned),
        bytes: Some(1),
        ..Model::default()
    };
    desk.models = vec![
        held("Model-Q4_K_M.gguf", Some("owner/Model-GGUF")),
        held("Other.gguf", None),
        held("Model-Q8_0.gguf", Some("owner/Model-GGUF")),
    ];
    let groups = desk.library();
    assert_eq!(groups.len(), 2, "{groups:?}");
    assert_eq!(groups[0].members, vec![0, 2]);
    assert_eq!(groups[0].name(&desk.models), "Model-GGUF");
    assert_eq!(groups[1].name(&desk.models), "Other");
    desk.chosen = Some(0);
    let _replaced = desk.offered.insert(
        "owner/Model-GGUF".to_owned(),
        vec![
            crate::OfferedFile {
                file: "Model-Q8_0.gguf".to_owned(),
                bytes: Some(2),
                fits: Some(true),
            },
            crate::OfferedFile {
                file: "Model-BF16.gguf".to_owned(),
                bytes: Some(4),
                fits: Some(false),
            },
        ],
    );
    let listed = desk.quantizations();
    let files: Vec<&str> = listed.iter().map(|quant| quant.file.as_str()).collect();
    assert_eq!(
        files,
        vec!["Model-Q4_K_M.gguf", "Model-Q8_0.gguf", "Model-BF16.gguf"],
        "here first, then the hub's, and a file here is not listed twice"
    );
    assert_eq!(desk.quantization_at(), Some(0));
    desk.pick_quantization(1);
    assert_eq!(
        desk.chosen,
        Some(2),
        "a quantization here becomes the subject"
    );
    assert!(desk.pending.is_none());
    desk.pick_quantization(2);
    assert_eq!(
        desk.pending.as_ref().map(|pending| pending.file.as_str()),
        Some("Model-BF16.gguf"),
        "a quantization on the hub is the subject as not downloaded"
    );
    assert_eq!(desk.quantization_at(), Some(2));
    desk.pick_quantization(0);
    assert!(desk.pending.is_none());
    assert_eq!(desk.chosen, Some(0));
}

#[test]
fn a_pick_not_here_downloads_first_and_then_does_the_thing() {
    let mut desk = Desk::new(std::path::PathBuf::from("/nowhere"));
    desk.pending = Some(crate::Pending {
        repository: "owner/Model-GGUF".to_owned(),
        file: "Model-Q8_0.gguf".to_owned(),
        bytes: Some(2),
        fits: Some(true),
    });
    desk.act(crate::Act::DownloadThen(std::boxed::Box::new(
        crate::Act::HostIt,
    )));
    assert!(
        matches!(desk.doing, crate::Doing::Downloading(_)),
        "the download did not go first"
    );
    assert_eq!(desk.after_download, Some(crate::Act::HostIt));
    desk.models = vec![Model {
        name: "Model-Q8_0".to_owned(),
        path: "/store/owner/Model-GGUF/Model-Q8_0.gguf".to_owned(),
        file: "Model-Q8_0.gguf".to_owned(),
        repository: Some("owner/Model-GGUF".to_owned()),
        ..Model::default()
    }];
    desk.settle_download();
    assert_eq!(desk.chosen, Some(0), "the file here is the subject now");
    assert!(desk.pending.is_none());
    assert!(
        desk.after_download.is_none(),
        "the thing named was not done"
    );
    assert_eq!(desk.page, Page::Hosting, "the server was not started");
}

#[test]
fn the_filters_narrow_the_library_with_the_words() {
    let mut desk = Desk::new(std::path::PathBuf::from("/nowhere"));
    let model = |name: &str, architecture: &str, bytes: u64, refused: Option<&str>| Model {
        name: name.to_owned(),
        path: format!("/store/{name}.gguf"),
        architecture: Some(architecture.to_owned()),
        bytes: Some(bytes),
        refused: refused.map(str::to_owned),
        ..Model::default()
    };
    desk.models = vec![
        model("Small", "llama", 5_000_000_000, None),
        model("Large", "llama", 40_000_000_000, Some("too large")),
        model("Moe", "a-moe", 18_000_000_000, None),
    ];
    let members = |desk: &Desk| -> Vec<usize> {
        desk.library()
            .into_iter()
            .flat_map(|group| group.members)
            .collect()
    };
    assert_eq!(desk.architectures(), vec!["a-moe", "llama"]);
    assert!(!desk.filters.any_set());
    desk.act(crate::Act::SetArchitecture(2));
    assert_eq!(desk.filters.architecture.as_deref(), Some("llama"));
    assert_eq!(members(&desk), vec![0, 1]);
    desk.act(crate::Act::SetFits(1));
    assert_eq!(members(&desk), vec![0], "only what will run here");
    desk.act(crate::Act::SetArchitecture(0));
    assert_eq!(members(&desk), vec![0, 2], "any architecture again");
    desk.act(crate::Act::SetSize(2));
    assert_eq!(desk.filters.size, Some(20_000_000_000));
    assert_eq!(members(&desk), vec![0, 2]);
    desk.act(crate::Act::SetSize(1));
    assert_eq!(members(&desk), vec![0], "up to 8 GB");
    desk.filter = "moe".to_owned();
    assert!(
        members(&desk).is_empty(),
        "the words and the filters both apply"
    );
    desk.act(crate::Act::SetSize(0));
    desk.act(crate::Act::SetFits(0));
    assert_eq!(members(&desk), vec![2]);
    assert!(!desk.filters.any_set());
    desk.act(crate::Act::ToggleFilters);
    assert!(desk.filters.open);
}

#[test]
fn a_models_runs_are_read_from_under_runs() {
    let line = r#"{"path":"/models/a.gguf","bytes":10,"companion":false,"parts":1,"provenance":null,"runs":{"architecture":"llama","measured":null,"cross_checked":null,"prompt_reported":null,"probed":[{"method":"chat-template","at":"2026-09-06T02:26:33Z","engine":"provisioned llama.cpp","said":"im_start…im_end as assistant"}],"configured":{"addressing":"set by the chat-template probe","budget":null},"readings_at":{"coding":"2026-09-06T05:07:15Z","editing":"2026-09-06T05:06:35Z"},"trained_context":4096,"cache_bytes_per_token":null,"resolved":null}}"#;
    let held = mcf_record::json::parse(line).unwrap();
    let model = crate::model_from(&held);
    assert_eq!(model.probed.len(), 1);
    assert_eq!(model.probed[0].name, "chat-template");
    assert_eq!(
        model.applied_addressing.as_deref(),
        Some("set by the chat-template probe")
    );
    assert_eq!(
        model.readings_at.get("coding").map(String::as_str),
        Some("2026-09-06T05:07:15Z")
    );
    let mut desk = Desk::new(std::path::PathBuf::from("/nowhere"));
    desk.models.push(model);
    desk.chosen = Some(0);
    assert!(
        desk.last_run(crate::Diagnostic::Eval(4)).is_some(),
        "editing's row says when it last ran"
    );
    assert!(
        desk.last_run(crate::Diagnostic::Eval(0)).is_none(),
        "the easy tier has not run; the older coding suite's time is not its own"
    );
    assert!(
        desk.last_run(crate::Diagnostic::Eval(5)).is_none(),
        "test writing has not run"
    );
}

#[test]
#[ignore = "needs a running daemon; set MCF_LIVE_SOCKET to its control socket"]
fn a_live_daemon_says_when_each_methods_readings_were_taken() {
    let Ok(socket) = std::env::var("MCF_LIVE_SOCKET") else {
        return;
    };
    let mut desk = Desk::new(std::path::PathBuf::from(socket));
    desk.refresh();
    assert!(!desk.models.is_empty(), "the daemon listed no models");
    let with_readings = desk
        .models
        .iter()
        .filter(|held| !held.readings_at.is_empty())
        .count();
    assert!(with_readings > 0, "no model carried readings_at");
    let with_findings = desk
        .models
        .iter()
        .filter(|held| !held.probed.is_empty())
        .count();
    assert!(with_findings > 0, "no model carried its probed findings");
}

#[test]
fn a_step_says_how_far_it_is_and_a_suite_line_gives_a_fraction() {
    let mut desk = Desk::new(std::path::PathBuf::from("/nowhere"));
    let mut going = crate::job::Job::already(
        "examining".to_owned(),
        vec![mcf_record::json::Value::map([
            (
                "step",
                mcf_record::json::Value::map([
                    ("name", mcf_record::json::Value::text("seeing")),
                    ("count", mcf_record::json::Value::Integer(2)),
                    ("of", mcf_record::json::Value::Integer(4)),
                ]),
            ),
            (
                "progress",
                mcf_record::json::Value::map([
                    ("done", mcf_record::json::Value::Integer(6)),
                    ("of", mcf_record::json::Value::Integer(12)),
                    ("doing", mcf_record::json::Value::text("number-472")),
                ]),
            ),
        ])],
    );
    going.finished = false;
    desk.doing = crate::Doing::Examining(going);
    let fraction = desk.run_fraction().unwrap_or(0.0);
    assert!((fraction - 0.375).abs() < 0.01, "{fraction}");
    let mut suite = crate::job::Job::already(
        "evaluating".to_owned(),
        vec![mcf_record::json::Value::map([
            (
                "line",
                mcf_record::json::Value::text("progress: 3/12 coding · rust · glob-match"),
            ),
            ("done", mcf_record::json::Value::Bool(false)),
        ])],
    );
    suite.finished = false;
    desk.doing = crate::Doing::Evaluating(suite);
    let fraction = desk.run_fraction().unwrap_or(0.0);
    assert!((fraction - 0.25).abs() < 0.01, "{fraction}");
}

#[test]
fn every_taxonomy_category_renders_with_its_subsystem_and_context() {
    use mcf_core::failure::{Attribution, Category, Disposition, Failure, Subsystem};
    for category in Category::ALL {
        let failure = Failure::new(
            category,
            Attribution::Machine,
            Disposition::Refused,
            Subsystem::new("mcf-serve::daemon"),
            "what happened, in a sentence",
        )
        .with_context("looked_for", "/somewhere/on/the/disk");
        let entry = Value::map([
            ("body", mcf_record::encode::failure(&failure)),
            (
                "recorded_at",
                Value::text("2026-09-07T15:20:04.250382271Z (local offset -07:00)"),
            ),
        ]);
        let fault = super::fault_from(&entry);
        assert_eq!(fault.category, category.code());
        assert_eq!(fault.meaning, category.meaning());
        assert_eq!(fault.at, "2026-09-07T15:20:04");
        let lines = super::fault_lines(&fault);
        assert!(
            lines[0].contains(category.code()) && lines[0].contains(category.meaning()),
            "{}: {lines:?}",
            category.code()
        );
        assert!(
            lines
                .iter()
                .any(|line| line.contains("in mcf-serve::daemon")),
            "{}: the subsystem is not named: {lines:?}",
            category.code()
        );
        assert!(
            lines
                .iter()
                .any(|line| line == "looked_for: /somewhere/on/the/disk"),
            "{}: the context is not there: {lines:?}",
            category.code()
        );
        assert!(
            lines
                .iter()
                .any(|line| line == "what happened, in a sentence"),
            "{}: the detail is not there: {lines:?}",
            category.code()
        );
    }
}

#[test]
fn a_failures_cause_and_axes_are_said_plainly() {
    use mcf_core::failure::{Attribution, Category, Disposition, Failure, Subsystem};
    let cause = Failure::new(
        Category::EngineExitImmediate,
        Attribution::Machine,
        Disposition::Partial,
        Subsystem::new("mcf-serve::served"),
        "the engine left at once",
    );
    let failure = Failure::new(
        Category::EngineSpawnRefused,
        Attribution::User,
        Disposition::Refused,
        Subsystem::new("mcf-serve::daemon"),
        "the hold could not start",
    )
    .caused_by(cause);
    let mut fault = super::fault_from(&mcf_record::encode::failure(&failure));
    fault.asked = "host".to_owned();
    let lines = super::fault_lines(&fault);
    assert!(lines.contains(&"asked: host".to_owned()), "{lines:?}");
    assert!(
        lines[1].starts_with("the operator's doing; refused; in mcf-serve::daemon"),
        "{lines:?}"
    );
    assert_eq!(
        lines.last().map(String::as_str),
        Some("because: engine.exit.immediate — the engine left at once")
    );
    assert_eq!(
        super::fault_lines(&super::Fault::default())[1],
        "unattributed; no disposition; in "
    );
}

#[test]
fn closing_lets_go_of_what_is_held_and_of_nothing_else() {
    let mut desk = Desk::new(std::path::PathBuf::from("/nowhere"));
    assert_eq!(desk.to_let_go(), None);

    desk.hosted = Some(super::Hosted {
        model: "/models/Assistant-2B-Instruct-Q4_K_M.gguf".to_owned(),
        address: "http://127.0.0.1:17817".to_owned(),
        since: String::new(),
        context: Some(8192),
        projector: None,
        takes: None,
        api_key: false,
        network_address: None,
        in_use: None,
    });
    assert_eq!(
        desk.to_let_go().as_deref(),
        Some("Assistant-2B-Instruct-Q4_K_M")
    );

    desk.hosted = None;
    desk.under_test = Some(super::UnderTest {
        model: "/models/Assistant-2B-Instruct-Q4_K_M.gguf".to_owned(),
        engine: "provisioned llama.cpp".to_owned(),
        window: Some(4096),
        in_use: super::Use::default(),
    });
    assert_eq!(desk.to_let_go(), None);
}

#[test]
fn a_build_reads_as_its_version_and_the_first_of_its_revision() {
    assert_eq!(
        super::said_of("0.1.0-m0", "52b825a5d1f8685f0a048c99f034225405802c2a"),
        "0.1.0-m0 · 52b825a"
    );
    assert_eq!(super::said_of("0.1.0-m0", "unknown"), "0.1.0-m0");
    assert_eq!(super::said_of("0.1.0-m0", ""), "0.1.0-m0");
    let mine = Desk::build_said();
    assert!(
        mine.starts_with(env!("CARGO_PKG_VERSION")),
        "the window does not name its own version: {mine}"
    );
}

#[test]
fn a_daemon_of_another_age_is_shown_and_one_of_the_same_age_is_not() {
    let mut desk = Desk::new(std::path::PathBuf::from("/nowhere"));
    assert_eq!(desk.daemon_build, None);
    desk.daemon_build = Some(Desk::build_said());
    assert_eq!(
        desk.daemon_build
            .as_ref()
            .filter(|held| **held != Desk::build_said()),
        None,
        "a daemon of this window's own age is not called out"
    );
    desk.daemon_build = Some("0.1.0-m0 · 74ca562".to_owned());
    assert!(
        desk.daemon_build
            .as_ref()
            .is_some_and(|held| *held != Desk::build_said()),
        "a daemon of another age is not shown as one"
    );
}
