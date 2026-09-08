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
