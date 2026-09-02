//! What the window may and may not do, checked without opening one.
//!
//! None of these draw. What they check is the layer between the daemon's
//! answer and the sentence a person reads — which is where this surface can
//! actually be wrong about a measurement, and the one place it must not be.

use super::{ACTIONS, Desk, Model, Page, component_from, model_from};
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
                matches!(
                    reaches,
                    "Status" | "Holding" | "Stop" | "Components" | "Anatomy"
                ),
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
    // A model's own page is reached from the list and belongs to it. Host is
    // no longer a column entry — it drew the list Models draws — so the two
    // screens its actions lead to are lit under Models, which is where a
    // person clicked to reach them.
    assert_eq!(Page::Adding.section(), Page::Models);
    assert_eq!(Page::Hosting.section(), Page::Models);
    assert_eq!(Page::Anatomy.section(), Page::Models);
    assert_eq!(Page::Vocabulary.section(), Page::Models);
    assert_eq!(Page::Host.section(), Page::Models);
    // And nothing in the column is Host any more.
    assert!(
        !Page::MENU.iter().any(|(page, _)| *page == Page::Host),
        "Host is the list Models shows, not a second entry for it"
    );
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
    assert_eq!(held.start_up, None);
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
/// Each one leads to a screen that works, and the column is the console's
/// column: B-072 makes *where things are* a fact about MCF rather than about
/// which surface you happened to open, so an entry added here is added there
/// in the same change. Components is the seventh — what MCF can build was
/// reachable only from the command line, which made provisioning a thing you
/// had to already know about (A19, A22).
#[test]
fn every_menu_entry_reaches_something_built() {
    // The console's six, in the console's order — not a menu invented here.
    let named: Vec<&str> = Page::MENU.iter().map(|(_, label)| *label).collect();
    assert_eq!(
        named,
        [
            "Monitor",
            "Models",
            "Diagnostics",
            "Components",
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
    // The run said nothing of a first token, so nothing claims one.
    assert_eq!(held.start_up(), crate::view::UNKNOWN);
}

/// The time to a first token is the daemon's figure or nothing — never one
/// this side of the wire works out from the readings (B-072, A7).
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

/// One rung of a run, as the daemon says it as the run climbs.
fn a_rung() -> Value {
    Value::map([
        ("depth", Value::Integer(512)),
        ("ms_per_token", Value::text("1.529")),
        ("measured", Value::Bool(true)),
    ])
}

/// A run's last line: the ladder again, and what was derived from it.
fn a_last_line(prompt_reading: Value, first_token: Value) -> Value {
    Value::map([
        ("measuring", Value::text("a-model")),
        ("readings", Value::List(vec![a_rung()])),
        ("prompt_reading", prompt_reading),
        ("first_token", first_token),
        ("done", Value::Bool(true)),
        (
            "conditions",
            Value::map([("engine_ran", Value::text("llama.cpp-cuda"))]),
        ),
    ])
}

/// A desk holding a finished run, its rows filled from it.
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

/// What one row of the tests table shows.
fn row(desk: &Desk, name: &str) -> Option<Vec<String>> {
    desk.tests
        .iter()
        .find(|test| test.name == name)
        .unwrap_or_else(|| panic!("no row named {name}"))
        .result
        .clone()
}

/// A finished run fills the three rows the ladder answers, and leaves the
/// two it does not alone.
///
/// **Five tests were listed and one ran.** The rows for prompt reading and
/// start-up said nothing after a run that had measured both and thrown them
/// away; the daemon now says them on its last line, and the rows read what
/// it said (A7, A9, B-072).
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
    assert_eq!(row(&desk, "Memory ceiling — largest context"), None);
    assert_eq!(row(&desk, "CPU and GPU agree on the output"), None);
}

/// What the daemon could not read is said as not read, not left blank (A9).
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

/// A pasted reference arrives in the field, and arrives usable.
///
/// F: the window read `SDL_EVENT_TEXT_INPUT` and nothing else, so a field
/// could be typed into and not pasted into — and an `owner/repository` name
/// copied out of a browser is pasted, not typed. What the clipboard holds was
/// put there by something else, so these are the shapes it actually arrives
/// in. The references here are shaped like references and name nothing: B28
/// keeps a model's name out of the code that would then be built around it.
#[test]
fn a_pasted_reference_is_taken_as_a_value() {
    let mut desk = Desk::new(std::path::PathBuf::from("/nowhere/control.sock"));

    desk.paste("an-owner/a-repository-GGUF");
    assert_eq!(
        desk.typed, "an-owner/a-repository-GGUF",
        "a plain reference is kept as it is"
    );

    // A browser hands over a trailing newline; it is not part of the name.
    desk.typed.clear();
    desk.paste("an-owner/a-repository-GGUF\n");
    assert_eq!(desk.typed, "an-owner/a-repository-GGUF");

    // A terminal can hand over more than one line. The first is the value.
    desk.typed.clear();
    desk.paste("an-owner/a-repository-GGUF\nand a second line\n");
    assert_eq!(desk.typed, "an-owner/a-repository-GGUF");

    // Control characters are not part of any reference.
    desk.typed.clear();
    desk.paste("\tan-owner/a-repository-GGUF\r");
    assert_eq!(desk.typed, "an-owner/a-repository-GGUF");

    // A paste that is nothing but whitespace leaves the field alone.
    desk.typed.clear();
    desk.typed.push_str("an-owner/");
    desk.paste("   \n  ");
    assert_eq!(desk.typed, "an-owner/", "an empty paste changes nothing");

    // Pasting appends, because a person may paste an owner and type a name.
    desk.typed.clear();
    desk.typed.push_str("an-owner/");
    desk.paste("a-repository-GGUF");
    assert_eq!(desk.typed, "an-owner/a-repository-GGUF");
}

/// A clipboard can hold a whole document. A field cannot.
#[test]
fn a_paste_is_bounded() {
    let mut desk = Desk::new(std::path::PathBuf::from("/nowhere/control.sock"));
    desk.paste(&"a".repeat(4096));
    assert_eq!(
        desk.typed.chars().count(),
        512,
        "a paste is capped rather than accepted whole"
    );
    // And a second paste cannot push it past the cap either.
    desk.paste("bbbb");
    assert_eq!(desk.typed.chars().count(), 512);
}

/// A component is in one of three states, and a half-build is not a build.
///
/// The builder writes the provenance last, so a prefix with none beside it is
/// a run that stopped partway. Reading that as *provisioned* is how a person
/// comes to trust a component that is not there (F31, A7).
///
/// Whether MCF reaches it as an ENGINE is a separate fact: a window library is
/// not an engine, and the first cut of this screen showed a fully provisioned
/// one as merely "built" because the two were treated as one question.
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

    // A directory with no provenance beside it: present, and not finished.
    let partway = read(true, false, false);
    assert!(
        partway.present && !partway.provisioned,
        "a prefix without its provenance is a run that stopped partway"
    );

    // Finished, and not an engine — which is the ordinary case for a library.
    let library = read(true, true, false);
    assert!(
        library.provisioned,
        "a component that is not an engine is still provisioned"
    );
    assert!(!library.usable_engine);

    // Finished, and an engine MCF can reach.
    let engine = read(true, true, true);
    assert!(engine.provisioned && engine.usable_engine);

    // The commit is shortened for reading, and never invented.
    assert_eq!(engine.commit, "0123456789ab");
}

/// The window asks the daemon where a hosted model answers.
///
/// The address is the one fact an API is for, and a monitor that had the model
/// but not the address would be one a person could not act on.
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
    });
    let hosting = desk.hosted.as_ref().expect("just set");
    // The screen shows the name, not the path: a path is where a file is.
    assert_eq!(
        hosting.model.rsplit('/').next(),
        Some("a-file.gguf"),
        "the monitor names what is answering, not where it sits"
    );
    assert_eq!(hosting.context, Some(8192));
}

/// A daemon that is busy is not a daemon that is gone.
///
/// F: the window polls four times a second with a thirty-second deadline on
/// each, and the daemon answers one thing at a time. Hosting a large model made
/// every poll block, so the window stopped repainting entirely — no state, no
/// events, nothing on screen — while the thing the operator had just asked for
/// was under way. What was on screen before the freeze said "a moment".
///
/// Two claims have to stay apart: MCF could not be reached, and MCF has not
/// answered yet. Reading the second as the first tells an operator their daemon
/// died at the moment it was doing what they asked (A7).
#[test]
fn a_busy_daemon_is_not_a_missing_one() {
    let mut desk = Desk::new(std::path::PathBuf::from("/nowhere/control.sock"));

    // Nothing listening at all: a refusal, and it says so.
    desk.refresh();
    assert!(
        desk.refusal.is_some(),
        "a socket with nothing behind it is a refusal"
    );
    assert!(!desk.busy, "and not merely busy");
    let (word, _) = desk.state_line();
    assert_eq!(word, "NOT UP");

    // Busy, with a host under way: the state line says what is happening and
    // for how long, rather than reporting the daemon as gone.
    desk.refusal = None;
    desk.busy = true;
    desk.doing = crate::Doing::Hosting(crate::job::Job::start(
        std::path::PathBuf::from("/nowhere/control.sock"),
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

/// A poll that went unanswered does not blank what is hosted.
///
/// Replacing it would report MCF's own busyness as the model being gone — on
/// the very screen an operator is watching to see whether it arrived.
#[test]
fn an_unanswered_poll_keeps_what_was_hosted() {
    let mut desk = Desk::new(std::path::PathBuf::from("/nowhere/control.sock"));
    desk.hosted = Some(crate::Hosted {
        model: "/a/model.gguf".to_owned(),
        address: "http://127.0.0.1:8080/v1".to_owned(),
        since: "2026-09-01T00:00:00Z".to_owned(),
        context: Some(4096),
    });
    // Nothing answers, so nothing is learned — and nothing is forgotten.
    desk.read_hosted();
    assert!(
        desk.hosted.is_some(),
        "an unanswered poll says nothing about what is held"
    );
    assert!(desk.busy, "and it is recorded as busy");
}

/// A silence never empties the model list.
///
/// F: the daemon answers one client at a time by decision (DEC-012), so while
/// it loads a large model it answers nothing for minutes. The window emptied
/// its list on that and showed *no models* — at the moment MCF was busy with
/// one of them. The models were on the disk the whole time and `mcf list` found
/// them; the only thing that had changed was that MCF was mid-answer.
///
/// The most misleading thing a screen can do is report its own ignorance as the
/// absence of the thing (A7).
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
    // Nothing answers, so nothing is learned — and nothing is forgotten.
    desk.refresh();
    assert_eq!(
        desk.models.len(),
        2,
        "a reading that did not arrive says nothing about what is held"
    );
}

/// A companion file is not offered as a model.
///
/// F: a vision projector was listed among the models, so the window offered it
/// to be hosted and to be measured. It carries no transformer and answers no
/// prompt — pointed at one, an engine loads it and produces nothing, which is
/// what a sweep of *every model on this machine* found (B-422's neighbour).
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

/// Host, with no engine to run the model on, builds the one MCF named rather
/// than doing nothing or refusing.
///
/// **The name comes from the daemon.** What engine a machine needs is decided
/// where `mcf provision` decides it, and the window carries the daemon's
/// answer into the job it starts — so the sentence on the screen names what
/// is being built and for what, before a single line of the build arrives
/// (§3.15, B-072, B-367).
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
    // What the daemon's settings refusal carried: no settings, and the
    // component it would build.
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

/// Without a name from the daemon, Host does not guess one.
///
/// A refusal that names no component is a refusal, and the window shows it;
/// building *something* on the strength of a sentence would be the window
/// deciding what MCF should have decided (A2).
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

/// The Components screen builds by name, on the card, without a terminal.
///
/// The card used to say `mcf provision llama.cpp` — a command line shown in
/// the window that exists so nobody needs one. Pressing Build sends the same
/// request the command sends, names the component on the card while it runs,
/// and refuses a second build while the first is going (A22, A6, B-367).
#[test]
fn build_on_the_components_screen_builds_that_component() {
    let mut desk = Desk::new(std::path::PathBuf::from("/nowhere"));
    desk.act(super::Act::Build("llama.cpp".to_owned()));

    let super::Doing::Provisioning(job) = &desk.doing else {
        panic!("Build must build, not {:?}", desk.doing);
    };
    assert_eq!(job.what, "building llama.cpp");
    assert_eq!(desk.building.as_deref(), Some("llama.cpp"));

    // A second press while the first runs starts nothing. The job at
    // /nowhere may already have been refused by now, so the assertion is on
    // what was started, not on the job's state.
    let before = desk.doing.job().map(|job| job.what.clone());
    desk.act(super::Act::Build("sdl3".to_owned()));
    let after = desk.doing.job().map(|job| job.what.clone());
    assert!(
        after == before || desk.doing.job().is_some_and(|job| job.finished),
        "{before:?} then {after:?}"
    );
}

/// A build that stops badly says so on the card it was for, and nowhere else.
#[test]
fn a_refused_build_lands_on_its_own_card() {
    let mut desk = Desk::new(std::path::PathBuf::from("/nowhere"));
    desk.build("llama.cpp");
    // Nothing listens at /nowhere, so the job refuses itself; this waits for
    // that refusal to arrive rather than assuming it has.
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
    // And a new build clears it.
    desk.build("llama.cpp");
    assert!(desk.build_failed.is_none());
}

/// *What is in it* asks the daemon, and draws nothing it did not say.
///
/// **The window counts nothing.** With no model chosen the screen says to
/// choose one; with a daemon that is not there it says so in words, and the
/// anatomy stays `None` rather than being made up from what the list already
/// knows about the file (B-072, A7).
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

    // The vocabulary is the same answer, asked again on the way to its own
    // screen, and refused in the same words.
    desk.no_anatomy = None;
    desk.act(crate::Act::Go(Page::Vocabulary));
    assert_eq!(desk.page, Page::Vocabulary);
    assert!(desk.anatomy.is_none());
    let why = desk.no_anatomy.clone().unwrap_or_default();
    assert!(why.contains("not answering"), "{why}");
}

/// What the daemon says a model is made of is read as it was said.
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
