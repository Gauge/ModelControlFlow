//! Tests for the attributability signal.
//!
//! The judgement is separated from the reading so that it can be tested with
//! supplied numbers: D26 says the laboratory simulates what MCF observes rather
//! than what causes it, and producing genuine contention inside a test suite
//! would be producing the cause.

use super::{Attributability, Scheduling, TOLERATED_DELAY_PPM, Watch, scheduling};
use crate::attested::Attested;
use crate::time::{Duration, Monotonic};

fn judge(waited_ns: u64, elapsed_ns: u64) -> Attributability {
    Watch::judge(
        Scheduling {
            on_cpu: 0,
            waiting: 0,
        },
        Scheduling {
            on_cpu: 0,
            waiting: waited_ns,
        },
        Duration::<Monotonic>::from_nanos(elapsed_ns),
    )
}

/// F3's two readings, as numbers. A quiet machine measured 190 parts per
/// million of queuing and a loaded one 111 000 over the same work; the
/// threshold sits between them with two orders of margin on one side and one on
/// the other.
#[test]
fn the_readings_f3_took_fall_on_the_sides_they_should() {
    // 4 148 ns of queuing across 21 834 777 ns of measurement.
    let quiet = judge(4_148, 21_834_777);
    assert!(quiet.permits_assertion(), "{quiet}");
    assert_eq!(quiet.delay_ppm(), Some(189));

    // 15 477 083 ns across 139 121 632 ns.
    let loaded = judge(15_477_083, 139_121_632);
    assert!(!loaded.permits_assertion(), "{loaded}");
    assert_eq!(loaded.delay_ppm(), Some(111_248));
}

/// The threshold is a boundary and both sides of it are checked.
#[test]
fn the_threshold_separates_queuing_from_working() {
    let at = judge(TOLERATED_DELAY_PPM, 1_000_000);
    assert!(
        at.permits_assertion(),
        "the threshold itself was refused: {at}"
    );

    let past = judge(TOLERATED_DELAY_PPM + 1, 1_000_000);
    assert!(
        !past.permits_assertion(),
        "one part past it was allowed: {past}"
    );
}

/// No queuing at all is attributable, which is the ordinary case on a machine
/// nobody else is using.
#[test]
fn a_measurement_with_no_queuing_is_attributable() {
    let clean = judge(0, 1_000_000);
    assert_eq!(clean.delay_ppm(), Some(0));
    assert!(clean.permits_assertion());
    assert!(clean.to_string().contains("attributable"), "{clean}");
}

/// A7: what the platform does not account for is unknown, and unknown is not
/// permission — reading an absent value as a favourable one is the substitution
/// A7 forbids.
#[test]
fn unknown_is_not_permission() {
    assert!(!Attributability::Unknown.permits_assertion());
    assert_eq!(Attributability::Unknown.delay_ppm(), None);
    // An interval in which nothing measurably happened cannot be attributed
    // either: there is no denominator.
    assert_eq!(judge(0, 0), Attributability::Unknown);
}

/// The rendering says how much of the measurement was queuing, because a
/// verdict without its number is a verdict nobody can disagree with.
#[test]
fn the_verdict_carries_the_number_it_rests_on() {
    let loaded = judge(15_477_083, 139_121_632);
    let rendered = loaded.to_string();
    assert!(rendered.contains("UNATTRIBUTABLE"), "{rendered}");
    assert!(rendered.contains("11.1248 %"), "{rendered}");
    assert!(rendered.contains("tolerated below"), "{rendered}");
}

/// Reading the kernel's accounting works or says it does not (A7). B19 keeps
/// this true on a platform that does not publish it, which is the case D29
/// calls *attempted, uncharacterized*.
///
/// What is asserted of a successful reading is that the counters are
/// **cumulative** — the property the whole signal rests on, since a delay is a
/// difference between two of them. Not that they are non-zero: a thread that
/// has not yet been descheduled has legitimately spent no measured time
/// anywhere, and asserting otherwise made this test fail on a fast machine for
/// a reason that had nothing to do with MCF.
#[test]
fn the_accounting_is_read_or_reported_absent() {
    let Attested::Known(first) = scheduling() else {
        // Correct on a platform that does not account for it, and the
        // consequence is stated rather than silent: nothing can be asserted.
        assert!(!Watch::start().finish().permits_assertion());
        return;
    };

    let mut total = 0_u64;
    for value in 0..500_000_u64 {
        total = total.wrapping_add(value);
    }
    assert!(total > 0);

    let Attested::Known(second) = scheduling() else {
        panic!("the accounting was readable and then was not");
    };
    assert!(
        second.on_cpu >= first.on_cpu,
        "processor time went backwards"
    );
    assert!(
        second.waiting >= first.waiting,
        "queuing time went backwards"
    );
}

/// A watch over work that actually happened produces a verdict, whichever way
/// it goes. What is asserted is that it produces one — the value depends on
/// what else the machine is doing, and asserting *that* would make the suite
/// fail on a busy build server for the right reason at the wrong time.
#[test]
fn a_watch_over_real_work_reaches_a_verdict() {
    let watch = Watch::start();
    let mut total = 0_u64;
    for value in 0..200_000_u64 {
        total = total.wrapping_add(value);
    }
    assert!(total > 0);
    let verdict = watch.finish();
    assert!(!verdict.to_string().is_empty());
}
