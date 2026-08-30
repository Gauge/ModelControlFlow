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
                matches!(reaches, "Status" | "Holding" | "Stop"),
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
