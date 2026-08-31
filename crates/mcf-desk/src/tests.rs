//! What the window may and may not do, checked without opening one.
//!
//! None of these draw. What they check is the layer between the daemon's
//! answer and the sentence a person reads — which is where this surface can
//! actually be wrong about a measurement, and the one place it must not be.

use super::{ACTIONS, Desk, Model, Page, model_from};
use mcf_record::json::Value;

/// Every action reaches a control-plane request or asks MCF nothing (A22).
#[test]
fn every_action_reaches_a_request_or_asks_nothing() {
    assert!(!ACTIONS.is_empty());
    for action in ACTIONS {
        assert!(!action.key.is_empty());
        assert!(!action.does.is_empty());
        if let Some(reaches) = action.reaches {
            assert!(
                matches!(reaches, "Status" | "Holding" | "Stop"),
                "{} reaches {reaches}, which this surface cannot build",
                action.key
            );
        }
    }
    assert!(ACTIONS.iter().any(|action| action.reaches.is_some()));
}

/// The navigation column offers only screens that exist.
///
/// A19: a column entry is an advertisement. One that leads nowhere tells
/// somebody something false about the whole application, and the person who
/// clicks it has no way to know which of the other entries to trust.
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
    // A model's own page is reached from the list and belongs to it.
    // The two screens Host's actions lead to belong to Host.
    assert_eq!(Page::Adding.section(), Page::Host);
    assert_eq!(Page::Hosting.section(), Page::Host);
}

/// A model MCF cannot run says so, and does not also claim to be ready.
#[test]
fn a_model_that_will_not_run_is_never_drawn_as_ready() {
    let refused = Model {
        name: "too big".to_owned(),
        refused: Some("this model needs more memory than this computer has".to_owned()),
        ..Model::default()
    };
    assert!(!refused.will_run());
    // The card's sentence is the refusal itself — not a cheerful line with a
    // warning somewhere else that has to be noticed.
    assert!(refused.in_a_sentence().contains("more memory"));
    assert!(refused.where_it_runs().contains("more memory"));
}

/// An unmeasured model says it is unmeasured everywhere it says anything.
#[test]
fn an_unmeasured_model_never_reports_a_speed() {
    let held = Model {
        name: "never timed".to_owned(),
        engine: Some("llama.cpp".to_owned()),
        device: Some("NVIDIA".to_owned()),
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
    // And in the technical rows, where a zero would be read as a measurement.
    let rows = held.technical();
    let speed = rows
        .iter()
        .find(|(name, _)| name == "Speed")
        .map(|(_, value)| value.clone())
        .unwrap_or_default();
    assert_eq!(speed, crate::words::UNMEASURED);
}

/// Everything the plain sentences left out is still somewhere (A1).
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

    // The card said "116 words a second"; the record's own unit is still here.
    assert!(flat.contains("155.0 tokens a second"), "{flat}");
    // It said "about 25,000 words"; the token count is still here.
    assert!(flat.contains("32,768 tokens"), "{flat}");
    assert!(flat.contains("40,960 tokens"), "{flat}");
    // It never said which engine; a person who wants to know can find out.
    assert!(flat.contains("llama.cpp-cuda"), "{flat}");
    assert!(flat.contains("an-architecture"), "{flat}");
    assert!(flat.contains("5,020,000,000 bytes"), "{flat}");
    assert!(flat.contains(&held.path), "{flat}");
}

/// The daemon's answer becomes a model without inventing anything.
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
    // Nothing measured it, so nothing claims to have.
    assert_eq!(held.speed, None);
    assert_eq!(held.wakes, None);
}

/// A model the daemon could not resolve carries the daemon's own reason.
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

/// A machine nothing could be read from says so rather than reporting zero.
#[test]
fn an_unreadable_machine_is_never_reported_as_an_empty_one() {
    let desk = Desk::new(std::path::PathBuf::from("/nowhere/control.sock"));
    // A default `Reading` is what a machine looks like before anything has
    // been sampled: every field absent.
    assert_eq!(desk.memory_sentence(), crate::words::UNMEASURED);
    assert_eq!(desk.processor_sentence(), crate::words::UNMEASURED);
    assert_eq!(desk.card_sentence(), "None found");
    assert!(
        desk.capability_sentence().contains("not been able to read"),
        "{}",
        desk.capability_sentence()
    );
}

/// When MCF cannot be reached the window says so in words, and does not put a
/// socket path in front of somebody who has never heard of one.
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

/// A menu entry now exists for each of the things the window can do.
///
/// Five, and each one leads to a screen that works. The three added — adding a
/// model, asking one something, timing one — are the three the operator asked
/// for, and each reaches the daemon rather than doing the work in the window
/// (A22, B-412).
#[test]
fn every_menu_entry_reaches_something_built() {
    // The console's six, in the console's order — not a menu invented here.
    let named: Vec<&str> = Page::MENU.iter().map(|(_, label)| *label).collect();
    assert_eq!(
        named,
        [
            "Monitor",
            "Host",
            "Diagnostics",
            "Models",
            "Settings",
            "Exit"
        ],
        "the window's menu has drifted from the console's"
    );
    for (page, label) in Page::MENU {
        assert_eq!(page.section(), *page, "{label} is not a section of its own");
    }
}

/// Typing goes to a field only where there is one.
///
/// The letter `q` closes the window, and a field that ate the application when
/// somebody typed a model name with a q in it would be a field nobody could
/// use.
#[test]
fn typing_is_only_typing_where_something_takes_it() {
    let mut desk = Desk::new(std::path::PathBuf::from("/nowhere"));
    for page in [Page::Adding, Page::Hosting] {
        desk.page = page;
        assert!(
            desk.takes_typing(),
            "{page:?} has a field and does not take typing"
        );
    }
    for page in [
        Page::Monitor,
        Page::Host,
        Page::Models,
        Page::Diagnostics,
        Page::Settings,
    ] {
        desk.page = page;
        assert!(
            !desk.takes_typing(),
            "{page:?} has no field and takes typing"
        );
    }
}

/// Nothing is asked for on an empty field.
///
/// A lookup of nothing is a request to a hub for a repository nobody named,
/// and an empty question is a generation nobody asked for. Both cost
/// something, so neither happens.
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

/// One long-running thing at a time.
///
/// Two measurements at once would be two measurements of a machine that was
/// running a measurement, and the second would be a reading of the first (A6).
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

/// A control that wears a chevron opens a list, and does not do something else.
///
/// **Both pickers on the diagnostics screen used to lie.** The model one ran
/// `Act::Go(Page::Host)` — it navigated away from the screen the reader was
/// setting up — and the window one ran a `NextWindow` that cycled to the next
/// power of two. Each drew a chevron, which is the promise that a list will
/// appear. A reader who wanted the third window of seven had to click six
/// times and count, and a reader who wanted to see the models had the page
/// taken away from them.
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

    // The same click again shuts it, which is what every dropdown does.
    desk.act(crate::Act::Open(crate::Picker::Model));
    assert_eq!(desk.open, None, "a second click did not shut the list");

    // Opening the other one replaces it: two open lists would overlap.
    desk.act(crate::Act::Open(crate::Picker::Model));
    desk.act(crate::Act::Open(crate::Picker::Window));
    assert_eq!(desk.open, Some(crate::Picker::Window));
}

/// A window is picked from the list, not counted up to.
#[test]
fn a_window_is_picked_and_never_cycled() {
    let mut desk = Desk::new(std::path::PathBuf::from("/nowhere"));
    let offered = crate::windows();
    assert!(
        offered.contains(&desk.window),
        "the window it starts on is not one of the ones offered"
    );
    // Every option is reachable in one click, which is the whole difference.
    for wanted in offered {
        desk.act(crate::Act::Open(crate::Picker::Window));
        desk.act(crate::Act::SetWindow(wanted));
        assert_eq!(desk.window, wanted);
        assert_eq!(desk.open, None, "picking did not shut the list");
    }
    // And they are powers of two, because a context window is asked for in them.
    for held in offered {
        assert!(held.is_power_of_two(), "{held} is not a power of two");
    }
}

/// Choosing a model from the list shuts it and leaves the reader where they were.
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

/// A test nobody has run has no run time and no result (A7).
///
/// **Not a zero, and not an empty result panel.** A zero in the run-time
/// column reads as *instant*, which is the one thing it is not, and a results
/// button over nothing is a button that does nothing when pressed.
#[test]
fn a_test_that_never_ran_reports_neither_a_time_nor_a_result() {
    for test in crate::tests() {
        assert_eq!(test.ran, None, "{} claims a run time", test.name);
        assert_eq!(test.result, None, "{} claims a result", test.name);
        assert!(test.seconds > 0, "{} has no estimate", test.name);
    }
}

/// The results button opens what a run found, and the same button closes it.
#[test]
fn a_result_is_opened_and_closed_by_the_one_button() {
    let mut desk = Desk::new(std::path::PathBuf::from("/nowhere"));
    assert_eq!(desk.showing, None);
    desk.act(crate::Act::Result(0));
    assert_eq!(desk.showing, Some(0));
    desk.act(crate::Act::Result(0));
    assert_eq!(desk.showing, None, "the same button did not close it");
    // A different row replaces it rather than opening a second panel.
    desk.act(crate::Act::Result(0));
    desk.act(crate::Act::Result(3));
    assert_eq!(desk.showing, Some(3));
}

/// A measurement in the record reaches the model's page.
///
/// **Until this, a run's readings went to the screen and nowhere else**, so a
/// model said `Unknown` about its speed the moment a run finished and a second
/// run could not be compared with a first (B-414, A1).
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
                            // A rung that would not separate: a result, and
                            // not a slow one. It must not become the deepest
                            // reading (A7, A9).
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
    // Nothing measured the cold start, so nothing claims to have.
    assert_eq!(held.cold_start(), crate::view::UNKNOWN);
}

/// A model with no measurement in the record says so, and does not say zero.
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

/// A ladder where nothing separated is a ladder with no reading.
///
/// Every rung is a result (A9) and none of them is a speed, so the model's
/// page shows what it showed before: nothing measured.
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
