//! Tests for the time model.
//!
//! A19: every reported quantity is tested against an independently known
//! value. The dates below are checked against dates whose weekday and
//! leap-year status are matters of public record, not against what the code
//! returns.

use super::{
    Clock, Duration, Instant, Monotonic, Simulated, SimulatedClock, SystemClock, Timestamp,
    UtcOffset,
};
use crate::attested::Attested;
use crate::build_identity::BuildIdentity;
use crate::measurement::{Conditions, Floor, Measurement, Quantity};

/// A duration is taken between two readings of one clock, and says how long.
#[test]
fn a_duration_is_the_interval_between_two_readings() {
    let clock = SimulatedClock::new();
    let start = clock.now();
    clock.advance(1_500);
    let elapsed = clock.now().saturating_duration_since(start);
    assert_eq!(elapsed.as_nanos(), 1_500);
}

/// D9: a monotonic clock does not go backwards, so an interval taken the wrong
/// way round saturates at zero rather than becoming enormous. The loud
/// invalidation D9 asks for needs the laboratory to inject the anomaly
/// (B-009); what the type guarantees now is that the wrong answer is bounded.
#[test]
fn a_backwards_interval_saturates_rather_than_wrapping() {
    let earlier = Instant::<Monotonic>::from_nanos(10);
    let later = Instant::<Monotonic>::from_nanos(1_000);
    assert_eq!(earlier.saturating_duration_since(later), Duration::ZERO);
    assert_eq!(later.saturating_duration_since(earlier).as_nanos(), 990);
}

/// A11: a duration knows which clock produced it, and carries that in the
/// value rather than leaving its caller to remember.
#[test]
fn a_duration_knows_which_clock_produced_it() {
    let simulated = SimulatedClock::new();
    let a = simulated.now();
    simulated.advance(7);
    assert!(simulated.now().saturating_duration_since(a).is_simulated());

    let real = SystemClock;
    let b = real.now();
    let elapsed = real.now().saturating_duration_since(b);
    assert!(!elapsed.is_simulated());
}

/// The rendering names the clock, so a simulated interval cannot be read as a
/// real one on any surface (A11, A6).
#[test]
fn a_duration_renders_with_its_clock() {
    assert_eq!(
        Duration::<Simulated>::from_nanos(42).to_string(),
        "42 ns (simulated)"
    );
    assert_eq!(
        Duration::<Monotonic>::from_nanos(42).to_string(),
        "42 ns (monotonic)"
    );
}

/// The two are different types. This test does not assert that — the compiler
/// does, and a mismatch would not build — but it records that a measurement
/// over one cannot hold a sample of the other, which is A11's check.
#[test]
fn a_measurement_of_durations_is_a_measurement_of_one_clocks_durations() {
    let conditions = Conditions::new(BuildIdentity::current(), Floor::nothing_known());
    let simulated: Measurement<Duration<Simulated>> = Measurement::of(
        Duration::from_nanos(10),
        Duration::from_nanos(30),
        [Duration::from_nanos(20)],
        conditions,
    );
    assert_eq!(simulated.spread().median.as_nanos(), 20);
    assert!(simulated.spread().median.is_simulated());
    assert_eq!(<Duration<Simulated> as Quantity>::UNIT, "ns");
}

/// The simulated clock moves only when a scenario moves it. B27 makes
/// determinism a feature of the laboratory rather than of the world.
#[test]
fn the_simulated_clock_moves_only_when_told() {
    let clock = SimulatedClock::new();
    assert_eq!(clock.now().as_nanos(), 0);
    assert_eq!(clock.now().as_nanos(), 0);
    clock.advance(5);
    assert_eq!(clock.now().as_nanos(), 5);
}

/// The real clock does not go backwards between two readings.
#[test]
fn the_system_clock_is_monotonic_across_readings() {
    let clock = SystemClock;
    let mut previous = clock.now();
    for _ in 0..1_000 {
        let now = clock.now();
        assert!(now >= previous, "{now} came before {previous}");
        previous = now;
    }
}

/// A19: civil dates checked against dates of public record — the epoch itself,
/// a leap day, the last day of a century that is not a leap year, and a moment
/// before the epoch.
#[test]
fn civil_dates_are_exact() {
    /// Nanoseconds since the epoch, and the year, month, day, hour, minute
    /// and second it is.
    type Case = (i128, (i64, u8, u8, u8, u8, u8));

    let cases: [Case; 5] = [
        (0, (1970, 1, 1, 0, 0, 0)),
        // 2000-02-29 was a leap day: 2000 is divisible by 400.
        (951_782_400_000_000_000, (2000, 2, 29, 0, 0, 0)),
        // 1900 was not a leap year, so 1900-03-01 follows 1900-02-28.
        (-2_203_891_200_000_000_000, (1900, 3, 1, 0, 0, 0)),
        // One second before the epoch is the last second of 1969.
        (-1_000_000_000, (1969, 12, 31, 23, 59, 59)),
        (1_756_058_651_442_000_000, (2025, 8, 24, 18, 4, 11)),
    ];
    for (nanos, (year, month, day, hour, minute, second)) in cases {
        let civil = Timestamp::from_utc_nanos(nanos, Attested::Unknown).civil_utc();
        assert_eq!(
            (
                civil.year,
                civil.month,
                civil.day,
                civil.hour,
                civil.minute,
                civil.second
            ),
            (year, month, day, hour, minute, second),
            "{nanos} ns"
        );
    }
}

/// Sub-second precision survives, and a moment before the epoch does not
/// borrow a second from the wrong side.
#[test]
fn sub_second_precision_survives_both_sides_of_the_epoch() {
    let after = Timestamp::from_utc_nanos(1_442_000_000, Attested::Unknown).civil_utc();
    assert_eq!((after.second, after.nanosecond), (1, 442_000_000));
    let before = Timestamp::from_utc_nanos(-1, Attested::Unknown).civil_utc();
    assert_eq!((before.year, before.month, before.day), (1969, 12, 31));
    assert_eq!((before.second, before.nanosecond), (59, 999_999_999));
}

/// D9: the offset is stored alongside, never folded in. The rendered moment is
/// UTC whatever the offset says.
#[test]
fn the_offset_is_stated_and_never_folded_into_the_moment() {
    let moment = Timestamp::from_utc_nanos(0, Attested::Unknown);
    let with_offset =
        moment.with_offset(UtcOffset::from_seconds_east(-5 * 3600).expect("-05:00 is an offset"));
    assert_eq!(moment.utc_nanos(), with_offset.utc_nanos());
    assert!(with_offset.to_string().starts_with("1970-01-01T00:00:00"));
    assert!(with_offset.to_string().contains("-05:00"));
}

/// A7: an offset MCF has not read renders as `unknown`, not as `+00:00`.
#[test]
fn an_unread_offset_is_unknown_and_not_utc() {
    let rendered = Timestamp::from_utc_nanos(0, Attested::Unknown).to_string();
    assert!(rendered.contains("local offset unknown"), "{rendered}");
}

/// B-352: the offset is read where the platform publishes one, and stays
/// unknown where it does not — never `+00:00`, which is a real offset most
/// machines do not have.
#[test]
fn the_offset_is_read_where_the_platform_publishes_one() {
    let now = Timestamp::now();
    match now.offset() {
        Attested::Known(offset) => {
            assert!(offset.seconds_east().abs() <= 14 * 3600);
            assert!(!now.to_string().contains("local offset unknown"), "{now}");
        }
        // A machine with no zone file. The honest answer, and the one B19
        // requires the suite to tolerate.
        Attested::Unknown => assert!(now.to_string().contains("local offset unknown")),
    }
}

/// D9: reading the offset does not move the moment. The two are stored side by
/// side, and `Timestamp` has no arithmetic that could fold one into the other.
#[test]
fn reading_the_offset_does_not_move_the_moment() {
    let now = Timestamp::now();
    let without = Timestamp::from_utc_nanos(now.utc_nanos(), Attested::Unknown);
    assert_eq!(now.utc_nanos(), without.utc_nanos());
    assert_eq!(now.civil_utc(), without.civil_utc());
}

/// An offset outside any real zone has no representation.
#[test]
fn a_value_that_is_not_an_offset_is_refused() {
    assert!(UtcOffset::from_seconds_east(27 * 3600).is_none());
    assert!(UtcOffset::from_seconds_east(-27 * 3600).is_none());
    assert!(UtcOffset::from_seconds_east(14 * 3600).is_some());
    assert_eq!(UtcOffset::UTC.to_string(), "+00:00");
}

/// The wall clock advances, which is all a timestamp promises. It is
/// deliberately not asserted to be monotonic: it is not, and B37 exists
/// because it is not.
#[test]
fn the_wall_clock_reports_a_plausible_era() {
    let now = Timestamp::now().civil_utc();
    assert!(
        now.year >= 2024,
        "the system clock reports {}, which predates this code",
        now.year
    );
}
