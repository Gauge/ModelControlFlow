use super::*;

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

#[test]
fn the_legend_does_not_promise_twice() {
    for (index, action) in ACTIONS.iter().enumerate() {
        for other in ACTIONS.iter().skip(index + 1) {
            assert_ne!(action.key, other.key, "{} is offered twice", action.key);
        }
    }
}

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
        assert!(screen.line(0).starts_with('┌'), "{at:?} has no top-left");
        assert!(
            screen.line(23).starts_with('└'),
            "{at:?} has no bottom-left"
        );
    }
}

#[test]
fn the_menu_offers_a_way_out() {
    assert!(Where::ALL.contains(&Where::Exit));
    let mut console = Console::new(std::path::PathBuf::from("/nowhere"));
    console.status = Some(Ok(Value::map::<&str>([])));
    let mut screen = Screen::new(80, 24);
    draw(&console, &mut screen);
    assert!(screen.line(1).contains("Exit"), "{}", screen.line(1));
}

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

#[test]
fn the_buttons_are_pressed_rather_than_drawn() {
    let mut console = Console::new(std::path::PathBuf::from("/nowhere/control.sock"));
    console.status = Some(Ok(Value::map::<&str>([])));
    console.models = vec![
        a_runnable("first.gguf", 8192),
        a_runnable("second.gguf", 4096),
    ];
    console.at = Where::Models;

    act(&mut console, Key::Down);
    act(&mut console, Key::Tab);
    act(&mut console, Key::Down);
    assert_eq!(console.button, 1, "the second button is Run diagnostics");
    let kept = console.models.clone();
    act(&mut console, Key::Enter);
    console.models = kept;
    console.status = Some(Ok(Value::map::<&str>([])));
    assert_eq!(console.at, Where::Diagnostics);
    assert_eq!(console.model, 1, "the run is set up for the model chosen");
    assert!(
        console.on_buttons,
        "the runs are buttons and there are no rows, so the cursor lands on them (D50)"
    );

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
    assert!(drawn.contains("21 s – 51 s"), "{drawn}");

    assert!(drawn.contains("Capabilities"), "{drawn}");
    assert!(drawn.contains("Prompt analysis"), "{drawn}");
    assert!(!drawn.contains("[x]") && !drawn.contains("[ ]"), "{drawn}");
    for _ in 0..3 {
        act(&mut console, Key::Down);
    }
    assert_eq!(console.button, 3);
    act(&mut console, Key::Enter);
    assert!(
        console
            .running
            .as_ref()
            .is_some_and(|held| held.run == Run::Probes),
        "the Capabilities button did not start the probes"
    );
    console.running = None;

    act(&mut console, Key::Down);
    act(&mut console, Key::Enter);
    assert_eq!(console.at, Where::Models);
    assert_eq!(console.model, 1, "the choice survives the round trip");
}

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
    console.button = 0;
    act(&mut console, Key::Enter);
    assert!(console.running.is_some());
    assert_eq!(screens::diagnostics::QUICK_DEPTH, 1024);
}

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
    });
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
        kept: false,
    });
    let mut screen = Screen::new(80, 30);
    draw(&console, &mut screen);
    let drawn = screen.rendered();
    assert!(drawn.contains("Measuring"), "{drawn}");
    assert!(!drawn.contains("Idle"), "{drawn}");
}

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
    assert!(console.on_buttons, "there are no rows for Tab to cross to");
    for _ in 0..3 {
        act(&mut console, Key::Down);
    }
    assert_eq!(console.button, 4);
    act(&mut console, Key::Enter);
    assert_eq!(console.at, Where::Models);
}

#[test]
fn a_run_under_way_says_which_step_it_is_on() {
    let mut job = crate::job::Job::start(
        std::path::Path::new("/nowhere/control.sock"),
        mcf_serve::control::Request::Hosted,
        "measuring a-model".to_owned(),
    );
    job.answers.push(Value::map([
        ("estimate_low_seconds", Value::Integer(52)),
        ("estimate_high_seconds", Value::Integer(127)),
    ]));
    job.answers.push(Value::map([(
        "starting",
        Value::map([
            ("depth", Value::Integer(512)),
            ("step", Value::Integer(1)),
            ("of", Value::Integer(2)),
        ]),
    )]));
    assert_eq!(
        screens::diagnostics::step_of(&job).as_deref(),
        Some("step 1 of 2: measuring at 512 tokens")
    );
    job.answers.push(Value::map([(
        "running",
        Value::map([
            ("depth", Value::Integer(512)),
            ("produce", Value::Integer(1)),
            ("repeat", Value::Integer(1)),
            ("of_repeats", Value::Integer(3)),
        ]),
    )]));
    assert_eq!(
        screens::diagnostics::step_of(&job).as_deref(),
        Some("at 512 tokens, repeat 1 of 3: loading the model and asking for 1 token(s)"),
        "each generation loads the model, and the line says so"
    );
    job.answers.push(Value::map([(
        "running",
        Value::map([
            ("depth", Value::Integer(512)),
            ("produce", Value::Integer(17)),
            ("repeat", Value::Integer(2)),
            ("of_repeats", Value::Integer(3)),
        ]),
    )]));
    let lines = screens::diagnostics::progress_of(&job);
    let said: Vec<&str> = lines.iter().map(|(line, _)| line.as_str()).collect();
    assert_eq!(
        said,
        vec![
            "somewhere between 52 and 127 seconds, MCF estimates",
            "at 512 tokens, repeat 2 of 3: loading the model and asking for 17 token(s)",
        ],
        "the estimate once, then the latest step and not the ones before it"
    );

    job.finished = true;
    let lines = screens::diagnostics::progress_of(&job);
    assert!(
        lines.iter().all(|(line, _)| !line.contains("repeat")),
        "a finished run is not on a step: {lines:?}"
    );
}

#[test]
fn a_figure_names_the_device_it_was_taken_on() {
    let on_the_card = Value::map([
        ("device", Value::text("Radeon 8060S")),
        ("gpu_layers", Value::Integer(999)),
    ]);
    assert_eq!(
        screens::diagnostics::on_device(&on_the_card),
        ", Radeon 8060S with 999 layers on the card"
    );
    let on_the_processor = Value::map([
        ("device", Value::text("CPU")),
        ("gpu_layers", Value::Integer(0)),
    ]);
    assert_eq!(screens::diagnostics::on_device(&on_the_processor), ", CPU");
    assert_eq!(
        screens::diagnostics::on_device(&Value::map([("engine_ran", Value::text("x"))])),
        "",
        "a run that did not say is not given a device"
    );
}

#[test]
fn a_job_cut_short_says_so_and_keeps_what_it_heard() {
    let mut job = crate::job::Job::start(
        std::path::Path::new("/nowhere/control.sock"),
        mcf_serve::control::Request::Hosted,
        "measuring a-model".to_owned(),
    );
    job.answers.push(Value::map([(
        "reading",
        Value::map([("depth", Value::Integer(512))]),
    )]));
    job.stop();
    assert!(job.finished && job.stopped);
    let why = job.refused.clone().unwrap_or_default();
    assert!(why.contains("stopped at your asking"), "{why}");
    assert!(
        why.contains("nothing was recorded"),
        "a run cut short is not the measurement it was asked for: {why}"
    );
    assert_eq!(job.answers.len(), 1, "what was heard stays");

    let mut over = crate::job::Job::already("done".to_owned(), Vec::new());
    over.stop();
    assert!(
        !over.stopped && over.refused.is_none(),
        "a finished job is not stopped"
    );
}

#[test]
fn a_rung_says_how_many_pairs_it_was_read_off() {
    let one_of_three = Value::map([(
        "pairs",
        Value::map([
            ("of", Value::Integer(3)),
            ("separated", Value::Integer(1)),
            ("did_not_separate", Value::Integer(2)),
            ("missed_the_pin", Value::Integer(0)),
            ("refused", Value::Integer(0)),
        ]),
    )]);
    assert_eq!(
        screens::diagnostics::pairs_note(&one_of_three),
        " — over 1 of 3 pairs: 2 did not separate"
    );
    let all = Value::map([(
        "pairs",
        Value::map([("of", Value::Integer(3)), ("separated", Value::Integer(3))]),
    )]);
    assert_eq!(screens::diagnostics::pairs_note(&all), "");
    assert_eq!(
        screens::diagnostics::pairs_note(&Value::map([("depth", Value::Integer(512))])),
        "",
        "a reading from before the count is not given one"
    );
}

#[test]
fn a_spawned_command_is_read_line_by_line_to_its_exit() {
    let mut command = std::process::Command::new("sh");
    command.arg("-c").arg("echo one; echo two; exit 3");
    let mut job = crate::job::Job::spawned(command, "saying two lines".to_owned());
    let began = std::time::Instant::now();
    while !job.finished && began.elapsed().as_secs() < 10 {
        let _heard = job.drain();
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    assert!(job.finished, "the job did not finish");
    let lines: Vec<&str> = job
        .answers
        .iter()
        .filter_map(|answer| {
            answer
                .get("line")
                .and_then(mcf_record::json::Value::as_text)
        })
        .collect();
    assert_eq!(lines, ["one", "two"]);
    let end = job.conclusion().expect("the exit is the conclusion");
    assert_eq!(
        end.get("exit")
            .and_then(mcf_record::json::Value::as_integer),
        Some(3)
    );
    assert!(job.refused.is_none());
}
