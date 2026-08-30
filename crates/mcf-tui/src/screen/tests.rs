use super::*;

/// Every style is closed before the next begins.
///
/// Reverse video that leaked past its cell would highlight the rest of the row,
/// and the operator would read a selection that is not there.
#[test]
fn a_style_does_not_leak_into_the_next_cell() {
    let mut screen = Screen::new(8, 1);
    screen.put(0, 0, "ab", Ink::Selected);
    screen.put(2, 0, "cd", Ink::Plain);
    let drawn = screen.rendered();
    let selected = drawn.find("\x1b[7m").expect("the selection is styled");
    let after = drawn.get(selected..).unwrap_or_default();
    assert!(
        after.contains("\x1b[0m"),
        "the selection is never turned off"
    );
}

/// A figure is placed by its last character, so a column of them lines up.
#[test]
fn figures_line_up_on_the_right() {
    let mut screen = Screen::new(12, 2);
    screen.put_right(10, 0, "5.03 GB", Ink::Figure);
    screen.put_right(10, 1, "0.09 GB", Ink::Figure);
    assert_eq!(
        screen.line(0).find("5.03"),
        screen.line(1).find("0.09"),
        "the two figures do not start at the same column:\n  {:?}\n  {:?}",
        screen.line(0),
        screen.line(1)
    );
}

/// Right-aligning something wider than the space starts at the edge rather
/// than underflowing.
#[test]
fn a_figure_wider_than_its_column_does_not_underflow() {
    let mut screen = Screen::new(6, 1);
    screen.put_right(3, 0, "a very long figure", Ink::Figure);
    let _drawn = screen.rendered();
}
