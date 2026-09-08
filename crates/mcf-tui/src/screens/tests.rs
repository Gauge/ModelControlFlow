use super::*;

#[test]
fn every_place_is_named_once() {
    for (index, item) in Where::ALL.iter().enumerate() {
        assert!(!item.label().is_empty());
        assert!(!item.title().is_empty());
        for other in Where::ALL.iter().skip(index + 1) {
            assert_ne!(
                item.label(),
                other.label(),
                "two items say {}",
                item.label()
            );
        }
    }
}

#[test]
fn the_menu_fits_the_smallest_terminal() {
    let width: usize = Where::ALL
        .iter()
        .map(|item| item.label().chars().count() + 3)
        .sum();
    assert!(
        width + 12 <= 80,
        "the menu needs {width} columns and leaves no room for the state"
    );
}

#[test]
fn a_size_is_exact() {
    assert_eq!(gigabytes(5_020_000_000), "5.02 GB");
    assert_eq!(gigabytes(90_000_000), "0.09 GB");
    assert_eq!(gigabytes(0), "0.00 GB");
}

#[test]
fn an_unknown_reading_is_a_dash() {
    assert_eq!(or_unknown(None::<u32>, " %"), UNKNOWN);
    assert_eq!(or_unknown(Some(38_u32), " %"), "38 %");
    assert_ne!(or_unknown(None::<u32>, " %"), "0 %");
}

#[test]
fn a_column_does_not_move_under_the_reader() {
    let mut screen = Screen::new(40, 2);
    columns(
        &mut screen,
        0,
        0,
        &[("a", 10, false, Ink::Plain), ("1", 8, true, Ink::Plain)],
    );
    columns(
        &mut screen,
        0,
        1,
        &[
            ("a much longer label", 10, false, Ink::Plain),
            ("100000", 8, true, Ink::Plain),
        ],
    );
    let ends_at = |row: usize| screen.line(row).trim_end().chars().count();
    assert_eq!(
        ends_at(0),
        ends_at(1),
        "the right-aligned figures do not end in the same column:\n  {:?}\n  {:?}",
        screen.line(0),
        screen.line(1)
    );
    assert!(
        !screen.line(1).contains("longer label"),
        "a label overran its column: {:?}",
        screen.line(1)
    );
}
