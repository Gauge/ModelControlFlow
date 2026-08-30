use super::*;

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

/// Every colour role the console has, the window has — so a refusal is the
/// same colour on both and neither surface has a palette the other lacks.
#[test]
fn every_role_has_a_colour() {
    for ink in [
        Ink::Plain,
        Ink::Quiet,
        Ink::Heading,
        Ink::Figure,
        Ink::Refusal,
        Ink::Held,
        Ink::Selected,
    ] {
        let (r, g, b) = colour(ink);
        assert!(
            u16::from(r) + u16::from(g) + u16::from(b) > 0,
            "{ink:?} is black on black"
        );
    }
    assert_ne!(
        colour(Ink::Refusal),
        colour(Ink::Held),
        "a refusal and a success must not look alike"
    );
}

/// The window draws the console's screens rather than screens of its own.
#[test]
fn a_daemon_that_is_not_there_is_drawn_as_one() {
    let mut desk = Desk::new(std::path::PathBuf::from("/nowhere/control.sock"));
    desk.status = Some(Err("nothing is listening on /nowhere".to_owned()));
    let mut screen = Screen::new(120, 40);
    draw(&desk, &mut screen);
    let drawn = screen.rendered();
    assert!(drawn.contains("not running"), "the refusal is not drawn");
    assert!(drawn.contains("mcf serve"), "it does not say what to do");
}

/// The window is a larger grid, not a different program: it fills whatever it
/// is given, at any size a window can be.
#[test]
fn the_screen_fills_whatever_the_window_is() {
    for (columns, rows) in [(80_u16, 24_u16), (120, 40), (160, 48), (200, 60)] {
        let mut desk = Desk::new(std::path::PathBuf::from("/nowhere"));
        desk.status = Some(Ok(Value::map([("engines", Value::List(Vec::new()))])));
        let mut screen = Screen::new(columns, rows);
        draw(&desk, &mut screen);
        for row in 0..rows as usize {
            assert_eq!(
                screen.line(row).chars().count(),
                columns as usize,
                "{columns}x{rows} row {row} is short"
            );
        }
        assert!(screen.line(0).starts_with('┌'));
        assert!(screen.line(rows as usize - 1).starts_with('└'));
    }
}
