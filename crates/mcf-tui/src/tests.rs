use super::*;

/// Every action either reaches a control-plane request or asks MCF nothing.
///
/// The parity check reads this table (A22, B-072). An action naming a request
/// the control plane does not have would be a capability only this surface has.
#[test]
fn every_action_reaches_a_request_or_asks_nothing() {
    assert!(!ACTIONS.is_empty(), "a surface with no actions is not one");
    for action in ACTIONS {
        assert!(!action.key.is_empty());
        assert!(!action.does.is_empty());
        if let Some(reaches) = action.reaches {
            assert!(
                matches!(
                    reaches,
                    "Status" | "Holding" | "Stop" | "Measure" | "CrossCheck"
                ),
                "{} reaches {reaches}, which this surface cannot build",
                action.key
            );
        }
    }
    assert!(
        ACTIONS.iter().any(|action| action.reaches.is_some()),
        "a surface that asks MCF nothing is not a client of anything"
    );
}

/// No key is offered twice, or the second would never be reached.
#[test]
fn the_legend_does_not_promise_twice() {
    for (index, action) in ACTIONS.iter().enumerate() {
        for other in ACTIONS.iter().skip(index + 1) {
            assert_ne!(action.key, other.key, "{} is offered twice", action.key);
        }
    }
}

/// A model the daemon could not work out says so, and says what MCF said —
/// never a zero and never a guess (A7).
#[test]
fn a_model_that_cannot_run_carries_the_reason() {
    let model = Value::map([
        ("path", Value::text("/models/example.gguf")),
        (
            "runs",
            Value::map([
                ("architecture", Value::Null),
                (
                    "resolved",
                    Value::map([
                        ("known", Value::Bool(false)),
                        ("why", Value::text("no engine is installed yet")),
                    ]),
                ),
            ]),
        ),
    ]);
    let held = Console::describe(&model);
    assert_eq!(held.name, "example.gguf");
    assert_eq!(held.architecture, None, "unknown must not become a word");
    assert_eq!(held.bytes, None, "a missing size is not zero");
    match held.engine {
        Err(why) => assert!(why.contains("no engine"), "{why}"),
        Ok(_) => panic!("a model with no engine must not resolve"),
    }
}

/// A model the daemon did work out carries the engine, device and window.
#[test]
fn a_model_that_runs_carries_where() {
    let model = Value::map([
        ("path", Value::text("/deep/place/example.gguf")),
        ("bytes", Value::Integer(5_020_000_000)),
        (
            "runs",
            Value::map([
                ("architecture", Value::text("an-architecture")),
                ("trained_context", Value::Integer(40_960)),
                ("cache_bytes_per_token", Value::Integer(114_688)),
                (
                    "resolved",
                    Value::map([
                        ("known", Value::Bool(true)),
                        ("engine", Value::text("an-engine")),
                        ("device", Value::text("a card")),
                        ("context", Value::Integer(32_768)),
                    ]),
                ),
            ]),
        ),
    ]);
    let held = Console::describe(&model);
    assert_eq!(held.name, "example.gguf", "named by its file, not its path");
    assert_eq!(held.trained, Some(40_960));
    assert_eq!(held.cache_per_token, Some(114_688));
    let resolved = held.engine.expect("it runs");
    assert_eq!(resolved.engine, "an-engine");
    assert_eq!(resolved.context, 32_768);
    assert_eq!(
        held.measured,
        screens::host::Measured::default(),
        "nothing measured is nothing, not a zero"
    );
}

/// What the ladder measured reaches the card in the daemon's own figures,
/// and a rung that did not separate is not the deepest reading (A7, A9).
#[test]
fn a_measured_model_carries_the_ends_of_its_ladder() {
    let reading = |depth: i64, ms: Option<&str>| {
        Value::map([
            ("depth", Value::Integer(depth)),
            ("measured", Value::Bool(ms.is_some())),
            (
                "ms_per_token",
                ms.map_or(Value::Null, |ms| Value::text(ms.to_owned())),
            ),
        ])
    };
    let model = Value::map([
        ("path", Value::text("/models/example.gguf")),
        (
            "runs",
            Value::map([(
                "measured",
                Value::map([
                    (
                        "readings",
                        Value::List(vec![
                            reading(512, Some("15.430")),
                            reading(4096, Some("66.433")),
                            reading(8192, None),
                        ]),
                    ),
                    (
                        "first_token",
                        Value::map([
                            ("measured", Value::Bool(true)),
                            ("ms", Value::text("1258.213")),
                        ]),
                    ),
                ]),
            )]),
        ),
    ]);
    let held = Console::describe(&model);
    assert_eq!(held.measured.shallowest, Some((512, "15.430".to_owned())));
    assert_eq!(
        held.measured.deepest,
        Some((4096, "66.433".to_owned())),
        "the rung that did not separate is not the deepest reading"
    );
    assert_eq!(held.measured.start_up.as_deref(), Some("1258.213"));

    // And the card says them, with their units, rather than Unknown.
    let mut console = Console::new(std::path::PathBuf::from("/nowhere/control.sock"));
    console.models = vec![held];
    console.at = Where::Models;
    let mut screen = Screen::new(100, 30);
    draw(&console, &mut screen);
    let drawn = screen.rendered();
    assert!(drawn.contains("15.430 ms/token"), "{drawn}");
    assert!(drawn.contains("1258.213 ms"), "{drawn}");
    assert!(
        !drawn.contains("run diagnostics to fill these in"),
        "{drawn}"
    );
}

/// A daemon that is not there is drawn as a refusal, not as an empty screen —
/// which would report success (A2).
#[test]
fn a_daemon_that_is_not_there_is_drawn_as_one() {
    let mut console = Console::new(std::path::PathBuf::from("/nowhere/control.sock"));
    console.status = Some(Err("nothing is listening on /nowhere".to_owned()));
    let mut screen = Screen::new(80, 24);
    draw(&console, &mut screen);
    let drawn = screen.rendered();
    assert!(
        drawn.contains("not running"),
        "the refusal is not on the screen"
    );
    assert!(drawn.contains("mcf serve"), "it does not say what to do");
}

/// Every screen fills exactly the terminal it was given, and no row is short.
///
/// The layouts in `../layout/` are drawn at eighty by twenty-four and checked
/// line by line; a screen that came out a different shape would not be the
/// screen that was agreed.
#[test]
fn every_screen_fills_the_terminal() {
    for at in Where::ALL {
        let mut console = Console::new(std::path::PathBuf::from("/nowhere"));
        console.at = at;
        console.status = Some(Ok(Value::map([("engines", Value::List(Vec::new()))])));
        let mut screen = Screen::new(80, 24);
        draw(&console, &mut screen);
        for row in 0..24 {
            let line = screen.line(row);
            assert_eq!(
                line.chars().count(),
                80,
                "{at:?} row {row} is {} columns",
                line.chars().count()
            );
        }
        // The frame closes on every side.
        assert!(screen.line(0).starts_with('┌'), "{at:?} has no top-left");
        assert!(
            screen.line(23).starts_with('└'),
            "{at:?} has no bottom-left"
        );
    }
}

/// The menu offers Exit, so leaving is something a person can see rather than
/// a key they have to know.
#[test]
fn the_menu_offers_a_way_out() {
    assert!(Where::ALL.contains(&Where::Exit));
    let mut console = Console::new(std::path::PathBuf::from("/nowhere"));
    console.status = Some(Ok(Value::map::<&str>([])));
    let mut screen = Screen::new(80, 24);
    draw(&console, &mut screen);
    assert!(screen.line(1).contains("Exit"), "{}", screen.line(1));
}

/// A model as the daemon lists one that runs, for a console to be set up on.
fn a_runnable(name: &str, context: i64) -> Held {
    Console::describe(&Value::map([
        ("path", Value::text(format!("/models/{name}"))),
        (
            "runs",
            Value::map([(
                "resolved",
                Value::map([
                    ("known", Value::Bool(true)),
                    ("engine", Value::text("an-engine")),
                    ("device", Value::text("a card")),
                    ("context", Value::Integer(context)),
                ]),
            )]),
        ),
    ]))
}

/// The buttons do what they say, from the keyboard: Tab reaches them, Enter
/// presses the one under the cursor, and a press that cannot run says why in
/// words rather than doing nothing (A2).
#[test]
fn the_buttons_are_pressed_rather_than_drawn() {
    let mut console = Console::new(std::path::PathBuf::from("/nowhere/control.sock"));
    console.status = Some(Ok(Value::map::<&str>([])));
    console.models = vec![
        a_runnable("first.gguf", 8192),
        a_runnable("second.gguf", 4096),
    ];
    console.at = Where::Models;

    // Down chooses the second model; Tab, Down, Enter presses *Run
    // diagnostics*, which opens the screen set up for that model.
    act(&mut console, Key::Down);
    act(&mut console, Key::Tab);
    act(&mut console, Key::Down);
    assert_eq!(console.button, 1, "the second button is Run diagnostics");
    // Opening a screen asks the daemon, which is not there; the models it
    // has are what it keeps until it is.
    let kept = console.models.clone();
    act(&mut console, Key::Enter);
    console.models = kept;
    console.status = Some(Ok(Value::map::<&str>([])));
    assert_eq!(console.at, Where::Diagnostics);
    assert_eq!(console.model, 1, "the run is set up for the model chosen");
    assert!(!console.on_buttons, "the cursor lands in the table");

    // The diagnostics screen names that model and its ladder.
    let mut screen = Screen::new(80, 24);
    draw(&console, &mut screen);
    let drawn = screen.rendered();
    assert!(drawn.contains("second.gguf"), "{drawn}");
    assert!(drawn.contains("512 · 1024 · 2048"), "{drawn}");
    assert!(
        !drawn.contains("4096 ·"),
        "half the window is the deepest rung"
    );
    assert_eq!(console.deepest(), Some(2048));
    // The estimate under Quick Run is a sixth of the ladder's, the way the
    // window prints it — 180 s / 6, at 0.58× and 1.42× (B-072).
    assert!(drawn.contains("21 s – 51 s"), "{drawn}");

    // Untick everything, then Run Selected: nothing runs, and it says so.
    for at in 0..console.tests.len() {
        if console.tests.get(at).is_some_and(|test| test.chosen) {
            screens::diagnostics::toggle(
                console
                    .tests
                    .iter_mut()
                    .map(|test| (test.run, &mut test.chosen)),
                at,
            );
        }
    }
    assert!(console.tests.iter().all(|test| !test.chosen));
    act(&mut console, Key::Tab);
    act(&mut console, Key::Down);
    act(&mut console, Key::Enter);
    assert!(console.running.is_none());
    assert_eq!(
        console.said.as_ref().map(|(said, _)| said.as_str()),
        Some("nothing chosen runs here")
    );

    // Back, from the buttons, returns to the Models screen on the same model.
    act(&mut console, Key::Down);
    act(&mut console, Key::Enter);
    assert_eq!(console.at, Where::Models);
    assert_eq!(console.model, 1, "the choice survives the round trip");
}

/// A model MCF has not resolved a window for has no ladder to climb, and the
/// button says so rather than measuring a window somebody guessed (§3.15).
#[test]
fn a_model_without_a_window_is_not_measured_against_one() {
    let mut console = Console::new(std::path::PathBuf::from("/nowhere/control.sock"));
    console.models = vec![Console::describe(&Value::map([(
        "path",
        Value::text("/models/unresolved.gguf"),
    )]))];
    console.at = Where::Diagnostics;
    console.on_buttons = true;
    console.button = 1;
    act(&mut console, Key::Enter);
    assert!(console.running.is_none());
    let said = console
        .said
        .clone()
        .map(|(said, _)| said)
        .unwrap_or_default();
    assert!(said.contains("no ladder to climb"), "{said}");
    // A Quick Run climbs to a depth of its own, not the window's, so it
    // needs no window — the same two rungs the window's Quick Run measures
    // (B-072), which the estimate under the button assumes.
    console.button = 0;
    act(&mut console, Key::Enter);
    assert!(console.running.is_some());
    assert_eq!(screens::diagnostics::QUICK_DEPTH, 1024);
}

/// A finished run fills the rows in the daemon's words, and the screen shows
/// what the highlighted row found under the table — the console's own
/// filling, which the window uses too (B-072).
#[test]
fn a_finished_run_fills_the_rows_and_the_screen_shows_the_one_under_the_cursor() {
    let mut console = Console::new(std::path::PathBuf::from("/nowhere/control.sock"));
    console.status = Some(Ok(Value::map::<&str>([])));
    console.models = vec![a_runnable("first.gguf", 8192)];
    console.at = Where::Diagnostics;
    let reading = |depth: i64, ms: &str| {
        Value::map([
            ("measuring", Value::text("first.gguf")),
            (
                "reading",
                Value::map([
                    ("depth", Value::Integer(depth)),
                    ("measured", Value::Bool(true)),
                    ("ms_per_token", Value::text(ms.to_owned())),
                ]),
            ),
            ("done", Value::Bool(false)),
        ])
    };
    let last = Value::map([
        ("measuring", Value::text("first.gguf")),
        ("done", Value::Bool(true)),
        (
            "conditions",
            Value::map([("engine_ran", Value::text("an-engine @abc"))]),
        ),
        (
            "first_token",
            Value::map([
                ("measured", Value::Bool(true)),
                ("ms", Value::text("1258.213")),
                ("depth", Value::Integer(512)),
            ]),
        ),
    ]);
    console.running = Some(Running {
        kept: false,
        run: Run::Ladder,
        job: job::Job::already(
            "measuring first.gguf".to_owned(),
            vec![reading(512, "15.430"), reading(1024, "17.276"), last],
        ),
        then_cross_check: false,
    });
    // Hearing a finished job keeps it and refreshes from a daemon that is
    // not there; the models are what it had.
    let kept = console.models.clone();
    console.hear();
    console.models = kept;
    console.status = Some(Ok(Value::map::<&str>([])));

    let ladder = console
        .tests
        .iter()
        .find(|test| test.name == "Generation speed against depth")
        .expect("the ladder's row");
    let found = ladder.result.clone().expect("the row is filled");
    let ladder_ran = ladder.ran;
    assert_eq!(
        found.first().map(String::as_str),
        Some("at 512 tokens   15.430 ms a token")
    );
    assert_eq!(
        found.last().map(String::as_str),
        Some("measured on an-engine @abc")
    );
    let start_up = console
        .tests
        .iter()
        .find(|test| test.name == "Start-up to first token")
        .expect("the start-up row");
    assert!(start_up.result.is_some(), "the same run answered it");
    let cross_check = console
        .tests
        .iter()
        .find(|test| test.run == Run::CrossCheck)
        .expect("the cross-check's row");
    assert!(
        cross_check.result.is_none(),
        "a run that did not compare engines does not fill their row"
    );

    console.row = 0;
    let mut screen = Screen::new(80, 30);
    draw(&console, &mut screen);
    let drawn = screen.rendered();
    assert!(
        drawn.contains("Generation speed against depth — ran"),
        "{drawn}"
    );
    assert!(
        drawn.contains("at 1,024 tokens   17.276 ms a token"),
        "{drawn}"
    );
    assert!(drawn.contains("Idle"), "{drawn}");
    after_the_run(console, ladder_ran);
}

/// A run the console started is the daemon's whole attention — it answers
/// one connection at a time — so the last status it gave is not what it is
/// doing now, and the header says what the console knows instead (A7).
/// Driving the console showed *Idle* through a whole ladder (F153).
#[test]
fn the_header_says_what_the_console_is_running() {
    let mut console = Console::new(std::path::PathBuf::from("/nowhere/control.sock"));
    console.status = Some(Ok(Value::map::<&str>([])));
    console.models = vec![a_runnable("first.gguf", 8192)];
    console.at = Where::Diagnostics;
    let mut going = job::Job::already("measuring first.gguf".to_owned(), vec![]);
    going.finished = false;
    console.running = Some(Running {
        run: Run::Ladder,
        job: going,
        then_cross_check: false,
        kept: false,
    });
    let mut screen = Screen::new(80, 30);
    draw(&console, &mut screen);
    let drawn = screen.rendered();
    assert!(drawn.contains("Measuring"), "{drawn}");
    assert!(!drawn.contains("Idle"), "{drawn}");
}

/// A finished run is heard once — a second pass must not fill the rows
/// again with a longer time, nor ask the daemon again — and the keys work
/// as before it: the cursor stays where it was, Tab crosses to the rows,
/// Space on a row ticks it, and Back leaves. Driving the console in a
/// terminal showed a run's time growing with every key pressed (F153).
fn after_the_run(mut console: Console, ladder_ran: Option<u64>) {
    console.at = Where::Diagnostics;
    console.on_buttons = true;
    console.button = 0;
    console.hear();
    let ladder = console
        .tests
        .iter()
        .find(|test| test.run == Run::Ladder)
        .expect("the ladder's row");
    assert_eq!(ladder.ran, ladder_ran, "heard once");
    assert!(console.running.as_ref().is_some_and(|held| held.kept));
    assert!(!console.busy());
    act(&mut console, Key::Down);
    assert!(console.on_buttons && console.button == 1);
    act(&mut console, Key::Tab);
    assert!(!console.on_buttons);
    act(&mut console, Key::Down);
    act(&mut console, Key::Down);
    assert_eq!(console.row, 2);
    act(&mut console, Key::Character(' '));
    assert!(
        console
            .tests
            .iter()
            .filter(|test| test.run == Run::Ladder)
            .all(|test| !test.chosen),
        "Space on a row ticks the rows one run answers"
    );
    act(&mut console, Key::Tab);
    act(&mut console, Key::Down);
    act(&mut console, Key::Down);
    act(&mut console, Key::Enter);
    assert_eq!(console.at, Where::Models);
}
