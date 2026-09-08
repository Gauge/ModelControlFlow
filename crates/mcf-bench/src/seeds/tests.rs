use mcf_core::measurement::PartsPerMillion;

use super::{Representative, representative};

const FIVE: PartsPerMillion = PartsPerMillion(50_000);

fn around(centre: u64, spread_ppm: u64, count: usize) -> Vec<u64> {
    let steps: [i64; 10] = [0, 10, -10, 5, -5, 7, -7, 2, -2, 9];
    (0..count)
        .map(|at| {
            let step = steps.get(at % steps.len()).copied().unwrap_or(0);
            let offset = i128::from(centre)
                .saturating_mul(i128::from(spread_ppm))
                .saturating_mul(i128::from(step))
                .wrapping_div(10_000_000);
            u64::try_from(i128::from(centre).saturating_add(offset)).unwrap_or(centre)
        })
        .collect()
}

#[test]
fn a_prefix_that_matches_the_stream_clears_the_set() {
    let held = representative(
        &around(1_000, 20_000, 32),
        &around(1_000, 20_000, 320),
        FIVE,
    );
    match held {
        Representative::Yes {
            resolving,
            standard,
            larger,
        } => {
            assert_eq!(resolving, FIVE);
            assert_eq!((standard, larger), (32, 320));
        }
        other => panic!("a matching prefix must clear the set: {other}"),
    }
    assert!(
        representative(
            &around(1_000, 20_000, 32),
            &around(1_000, 20_000, 320),
            FIVE
        )
        .clears_the_set()
    );
}

#[test]
fn a_prefix_that_differs_is_unrepresentative() {
    let held = representative(
        &around(1_500, 20_000, 32),
        &around(1_000, 20_000, 320),
        FIVE,
    );
    let Representative::No { by, .. } = &held else {
        panic!("a prefix half again as large must be caught: {held}")
    };
    assert!(by.0 > 300_000, "about fifty percent: {by:?}");
    assert!(!held.clears_the_set());
    let text = format!("{held}");
    assert!(
        text.contains("UNREPRESENTATIVE") && text.contains("break in comparability"),
        "the finding says what D19 does about it: {text}"
    );
}

#[test]
fn not_decided_does_not_clear_the_set() {
    let held = representative(
        &around(1_000, 400_000, 4),
        &around(1_010, 400_000, 12),
        PartsPerMillion(2_000),
    );
    assert!(
        matches!(held, Representative::NotYet { .. }),
        "noise wider than the question decides nothing: {held}"
    );
    assert!(!held.clears_the_set());
}

#[test]
fn a_draw_no_larger_than_the_set_is_not_the_comparison_d19_asks_for() {
    let held = representative(&around(1_000, 20_000, 32), &around(2_000, 20_000, 32), FIVE);
    assert!(
        matches!(held, Representative::NotYet { after: 32 }),
        "an equal draw is refused even where it plainly differs: {held}"
    );
    assert!(!held.clears_the_set());
}

#[test]
fn a_clearance_never_reads_as_a_discovery() {
    let cleared = representative(
        &around(1_000, 20_000, 32),
        &around(1_000, 20_000, 320),
        FIVE,
    );
    let found = representative(
        &around(2_000, 20_000, 32),
        &around(1_000, 20_000, 320),
        FIVE,
    );
    let (cleared, found) = (format!("{cleared}"), format!("{found}"));
    assert!(cleared.contains("indistinguishable"), "{cleared}");
    assert!(!cleared.contains("UNREPRESENTATIVE"), "{cleared}");
    assert!(found.contains("UNREPRESENTATIVE"), "{found}");
    assert!(!found.contains("indistinguishable"), "{found}");
}
