use super::*;

/// Every action either reaches a control-plane request or asks MCF nothing.
///
/// The table is what the parity check reads (A22, B-072). An action naming a
/// request the control plane does not have would be a capability only this
/// surface has, which is the shape the rule forbids.
#[test]
fn every_action_reaches_a_request_or_asks_nothing() {
    assert!(!ACTIONS.is_empty(), "a surface with no actions is not one");
    for action in ACTIONS {
        assert!(
            !action.key.is_empty(),
            "an action with no key cannot be taken"
        );
        assert!(
            !action.does.is_empty(),
            "an action that does not say what it does"
        );
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

/// The keys shown are the keys handled, and no key is offered twice.
#[test]
fn the_legend_does_not_promise_twice() {
    for (index, action) in ACTIONS.iter().enumerate() {
        for other in &ACTIONS[index + 1..] {
            assert_ne!(
                action.key, other.key,
                "{} is offered twice and would do whichever came first",
                action.key
            );
        }
    }
}

/// A size MCF does not know renders as unknown, never as zero (A7).
#[test]
fn an_unknown_size_is_not_a_zero() {
    let nothing = Value::map([("path", Value::text("/models/x.gguf"))]);
    assert_eq!(gigabytes(&nothing), "unknown");
    let known = Value::map([("bytes", Value::Integer(5_030_000_000))]);
    assert_eq!(gigabytes(&known), "5.03 GB");
}

/// A model is named by its file, not by where it happens to sit.
#[test]
fn a_model_is_named_by_its_file() {
    // A name no model here has, so the check stays neutral about which
    // model is the reference (B28, B-018).
    let model = Value::map([("path", Value::text("/a/deep/place/example-7b-q4.gguf"))]);
    assert_eq!(name_of(&model), "example-7b-q4.gguf");
    let nameless = Value::map([("bytes", Value::Integer(1))]);
    assert_eq!(name_of(&nameless), "unknown");
}

/// A refusal keeps its category, so the screen says the same thing the record
/// would (A2, C1).
#[test]
fn a_refusal_keeps_its_category() {
    let body = Value::map([
        ("category", Value::text("platform.mechanism.unavailable")),
        ("what", Value::text("nothing is listening")),
    ]);
    let said = why(&body);
    assert!(said.contains("platform.mechanism.unavailable"), "{said}");
    assert!(said.contains("nothing is listening"), "{said}");

    // And a body that says neither still says something.
    let bare = Value::map([("unrelated", Value::Integer(1))]);
    assert!(!why(&bare).is_empty());
}

/// The screen a refused daemon draws says so rather than showing an empty
/// table, which would report success (A2).
#[test]
fn a_refused_daemon_is_drawn_as_a_refusal() {
    let mut app = App::new(std::path::PathBuf::from("/nowhere/control.sock"));
    app.status = Some(Err("nothing is listening on /nowhere".to_owned()));
    app.models = Some(Err("nothing is listening on /nowhere".to_owned()));
    let mut screen = Screen::new(80, 24);
    draw(&app, &mut screen);
    let drawn = screen.rendered();
    assert!(
        drawn.contains("nothing is listening"),
        "the refusal is not on the screen"
    );
}

/// A screen narrower than its content is clipped, never wrapped.
///
/// A wrapped row stops a table being one, and the operator reads a value under
/// the wrong heading.
#[test]
fn a_narrow_screen_clips_rather_than_wraps() {
    let mut screen = Screen::new(20, 3);
    screen.put(0, 0, "a name far longer than twenty columns", Ink::Plain);
    assert!(
        !screen.line(0).contains("columns"),
        "the text wrapped instead of being clipped: {:?}",
        screen.line(0)
    );
    assert_eq!(
        screen.line(1).trim(),
        "",
        "the overflow landed on the next row"
    );
}

/// Drawing off the bottom of the screen is nothing, not a panic.
#[test]
fn drawing_past_the_edge_is_harmless() {
    let mut screen = Screen::new(10, 2);
    screen.put(0, 99, "far below", Ink::Plain);
    screen.put(99, 0, "far right", Ink::Plain);
    screen.select_row(99);
    screen.rule(0, 99, 100, Ink::Quiet);
    let _drawn = screen.rendered();
}
