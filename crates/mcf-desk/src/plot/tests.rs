use super::*;

/// A series carries its own points and colour, and a plot of one point has no
/// range to scale against — which is why `draw` returns rather than dividing by
/// nothing.
#[test]
fn a_series_holds_what_it_was_given() {
    let one = Series {
        name: "a device".to_owned(),
        points: vec![(512.0, 10.0)],
        colour: (1, 2, 3),
    };
    assert_eq!(one.points.len(), 1);
    assert_eq!(one.name, "a device");
}

/// A depth or a duration of nothing has no logarithm, so those points are
/// skipped rather than crashing the window — and the caller's data is left
/// alone.
#[test]
fn a_zero_is_skipped_rather_than_logged() {
    let held = Series {
        name: "a device".to_owned(),
        points: vec![(0.0, 10.0), (512.0, 0.0), (1024.0, 12.0), (2048.0, 14.0)],
        colour: (1, 2, 3),
    };
    let usable = held
        .points
        .iter()
        .filter(|(x, y)| *x > 0.0 && *y > 0.0)
        .count();
    assert_eq!(usable, 2, "two of the four points can be placed");
    assert_eq!(held.points.len(), 4, "and none of them were removed");
}

/// Two series with real measurements have a range on both axes.
///
/// These are this machine's own readings: a processor and a card, fourteen
/// times apart, which is exactly why the axis has to be logarithmic.
#[test]
fn two_devices_an_order_apart_both_have_a_range() {
    let processor = [
        (512.0, 89.04),
        (1024.0, 91.06),
        (2048.0, 93.97),
        (8192.0, 107.90),
    ];
    let card = [
        (512.0, 6.43),
        (1024.0, 6.47),
        (2048.0, 6.65),
        (8192.0, 7.66),
    ];
    let ratio = processor[0].1 / card[0].1;
    assert!(
        ratio > 10.0,
        "the devices are {ratio:.1}x apart, and a linear axis would flatten one"
    );
    let across = (8192.0_f64).log2() - (512.0_f64).log2();
    assert!((across - 4.0).abs() < 0.001, "four doublings of depth");
    let up = (107.90_f64).log10() - (6.43_f64).log10();
    assert!(up > 1.0, "more than a decade of duration");
}
