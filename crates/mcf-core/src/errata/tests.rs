use super::{KNOWN, affecting};

#[test]
fn everything_recorded_before_a_correction_is_affected() {
    let contention = KNOWN.first().expect("the list is not empty");
    let before = contention.corrected_at_utc_nanos.saturating_sub(1);
    assert!(
        affecting(before).iter().any(|held| held.finding == "F90"),
        "a measurement taken a nanosecond before the fix was taken by the broken instrument"
    );
    assert!(
        !affecting(contention.corrected_at_utc_nanos)
            .iter()
            .any(|held| held.finding == "F90"),
        "and one taken at the moment of correction was not"
    );
}

#[test]
fn a_recent_measurement_is_affected_by_nothing() {
    let latest = KNOWN
        .iter()
        .map(|held| held.corrected_at_utc_nanos)
        .max()
        .expect("the list is not empty");
    assert!(affecting(latest).is_empty());
}

#[test]
fn an_old_measurement_is_affected_by_all_of_them() {
    assert_eq!(affecting(0).len(), KNOWN.len());
}

#[test]
fn every_erratum_says_what_it_did_rather_than_only_that_it_was_wrong() {
    for held in KNOWN {
        assert!(!held.effect.is_empty(), "{} states no effect", held.finding);
        assert!(
            !held.defect.is_empty() && !held.instrument.is_empty(),
            "{} names no defect or no instrument",
            held.finding
        );
        assert!(
            held.finding.starts_with('F'),
            "an erratum cites the finding that establishes it: {held}"
        );
        assert!(
            held.effect.len() > 40,
            "{} describes its effect too thinly to act on",
            held.finding
        );
    }
}

#[test]
fn the_readable_moment_and_the_nanoseconds_agree() {
    for held in KNOWN {
        let rendered = crate::time::Timestamp::from_utc_nanos(
            i128::from(held.corrected_at_utc_nanos),
            crate::attested::Attested::Unknown,
        )
        .to_string();
        let (moment, _) = held
            .corrected_at
            .split_once('Z')
            .expect("a corrected_at is an RFC 3339 moment in UTC");
        assert!(
            rendered.starts_with(moment),
            "{} says {} and its nanoseconds are {rendered}",
            held.finding,
            held.corrected_at
        );
    }
}

#[test]
fn the_corrections_are_ordered_and_distinct() {
    let mut moments: Vec<i64> = KNOWN
        .iter()
        .map(|held| held.corrected_at_utc_nanos)
        .collect();
    let held = moments.len();
    moments.sort_unstable();
    moments.dedup();
    assert_eq!(held, moments.len(), "two errata claim the same moment");
    assert!(
        KNOWN.windows(2).all(|pair| match pair {
            [one, other] => one.corrected_at_utc_nanos < other.corrected_at_utc_nanos,
            _ => true,
        }),
        "the list reads oldest first, which is how a reader follows it"
    );
}
