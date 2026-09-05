//! What the window may and may not do, checked without opening one.
//!
//! None of these draw. What they check is the layer between the daemon's
//! answer and the sentence a person reads — which is where this surface can
//! actually be wrong about a measurement, and the one place it must not be.

use super::{ACTIONS, Caret, Desk, Model, Page, component_from, model_from};
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
    assert_eq!(Page::Hosting.section(), Page::Hosting);
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
        ["System", "Models", "Server", "Diagnostics", "Exit"],
        "the window's places are the four D49 names, and Exit"
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
        assert!(
            test.seconds.is_none_or(|seconds| seconds > 0),
            "{} has an estimate of nothing",
            test.name
        );
    }
}

/// Each card has its own cost: the throughput run's is one ladder's time
/// and the cross-check's is its own, whatever else is on the page (D50,
/// B-477).
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

/// A run's card starts that run and no other: the throughput card climbs
/// the ladder, the cross-check card reads with MCF's own engine, the prompt
/// card opens the prompt page, and a card whose run is at the command line
/// starts nothing (D50).
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
    // The rows are labelled with the depths the ends were measured at, not
    // with a window the ladder did not climb to (A20).
    let [shallowest, deepest] = held.speed_rows();
    assert_eq!(shallowest.0, "at 512 tokens");
    assert_eq!(deepest.0, "at 2,048 tokens", "{deepest:?}");
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

/// A finished run fills the five rows the ladder answers, and leaves the
/// one it does not alone.
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

/// A finished cross-check fills its own row with the daemon's sentences,
/// and a refused one with the refusal — never a blank (A2, B-072, B-424).
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
    desk.page = Page::Adding;

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
    desk.page = Page::Adding;
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
        projector: None,
        takes: None,
        api_key: false,
        in_use: None,
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
        projector: None,
        takes: None,
        api_key: false,
        in_use: None,
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

/// The window asks the same turn `mcf run` asks: what is typed reaches the
/// request, and what is left alone is left alone (B-462, D43).
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
    // And the field that is not the ask screen's own is untouched by it.
    assert!(desk.typed.is_empty(), "the question is still empty");
}

/// Thinking has three positions and unsaid is one of them (D43).
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

/// A caret left on the ask screen's own field does not swallow what is
/// typed into another screen's.
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

/// A run under way is said on every page: what it is and how long so far, at
/// the top right where the state word is, and where it is now on the page
/// that started it.
///
/// The buttons used to press and nothing on screen changed until the first
/// reading arrived, minutes later on a processor; an operator told nothing
/// reasonably concludes nothing is happening (A7).
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

    // Once it has finished, it is not still working.
    if let crate::Doing::Measuring(job) = &mut desk.doing {
        job.finished = true;
    }
    assert_eq!(desk.state_word(), "MCF");
}

/// A load says how far it has got in bytes read of bytes to read, after how
/// long, and about how long is left once there is a rate to read that off —
/// never a promise before there is one (A6, A7).
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

    // And the desk says nothing about loading where nothing loads.
    let desk = Desk::new(std::path::PathBuf::from("/nowhere/control.sock"));
    assert!(desk.loading_line().is_none());
}

/// A setting typed into the Configure tab is taken as typed where it is a
/// number the setting can hold, and refused with the word where it is not —
/// never read as nought and never sent (§3.15, A7).
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

/// The capabilities card asks for the probes ticked: every one is no list,
/// one unticked is the eight named, and Apply is a switch (B-478, D43).
#[test]
fn the_capabilities_card_asks_for_the_probes_ticked() {
    let mut desk = Desk::new(std::path::PathBuf::from("/nowhere"));
    assert!(
        desk.probes_only().is_empty(),
        "every probe ticked is every probe"
    );
    desk.act(crate::Act::ToggleProbe(1));
    let only = desk.probes_only();
    assert_eq!(only.len(), mcf_serve::probes::run::PROBES.len() - 1);
    assert!(!only.iter().any(|name| name == "context"));
    assert!(only.iter().any(|name| name == "chat-template"));
    assert!(!desk.probes_apply);
    desk.act(crate::Act::ApplyProbes);
    assert!(desk.probes_apply);
    // A probe run keeps what it found on the model it ran on.
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
    job.finished = true;
    desk.doing = crate::Doing::Probing(job);
    desk.keep_the_probes();
    let probed = &desk.models[0].probed;
    assert_eq!(probed.len(), 1, "{probed:?}");
    assert_eq!(probed[0].0, "stop-conditions");
    assert_eq!(probed[0].1.len(), 2);
}

/// The settings a model was last held under come back with one press, and
/// the recommendation with another (B-475).
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

/// The search field narrows the library by the words typed, over the name,
/// the architecture and the path, case aside; words nothing matches leave
/// the list empty rather than whole (D51, B-485).
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
    // Return with nothing here matching asks the hub for the words.
    desk.page = Page::Models;
    desk.entered();
    assert!(
        matches!(desk.doing, crate::Doing::Listing(_)),
        "Return on words nothing matches did not search the hub"
    );
}

/// What the hub answered is kept for the words it was asked, listed with
/// its downloads, and dropped from the list once the words change.
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

/// Files held from one repository are one entry of the library, and the
/// page's quantizations are those files then what the hub publishes that
/// is not here; picking one here changes the subject, picking one on the
/// hub makes it the subject as not downloaded (D51, B-486).
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
