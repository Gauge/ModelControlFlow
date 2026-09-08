use super::{Attributability, Scheduling, TOLERATED_DELAY_PPM, Watch, scheduling};
use crate::attested::Attested;
use crate::time::{Duration, Monotonic};

fn judge(waited_ns: u64, elapsed_ns: u64) -> Attributability {
    judge_with_faults(waited_ns, elapsed_ns, Attested::Known(0))
}

fn judge_with_faults(
    waited_ns: u64,
    elapsed_ns: u64,
    major_faults: Attested<u64>,
) -> Attributability {
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
        major_faults,
    )
}

#[test]
fn the_readings_f3_took_fall_on_the_sides_they_should() {
    let quiet = judge(4_148, 21_834_777);
    assert!(quiet.permits_assertion(), "{quiet}");
    assert_eq!(quiet.delay_ppm(), Some(189));

    let loaded = judge(15_477_083, 139_121_632);
    assert!(!loaded.permits_assertion(), "{loaded}");
    assert_eq!(loaded.delay_ppm(), Some(111_248));
}

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

#[test]
fn a_measurement_with_no_queuing_is_attributable() {
    let clean = judge(0, 1_000_000);
    assert_eq!(clean.delay_ppm(), Some(0));
    assert!(clean.permits_assertion());
    assert!(clean.to_string().contains("attributable"), "{clean}");
}

#[test]
fn unknown_is_not_permission() {
    assert!(!Attributability::Unknown.permits_assertion());
    assert_eq!(Attributability::Unknown.delay_ppm(), None);
    assert_eq!(judge(0, 0), Attributability::Unknown);
}

#[test]
fn the_verdict_carries_the_number_it_rests_on() {
    let loaded = judge(15_477_083, 139_121_632);
    let rendered = loaded.to_string();
    assert!(rendered.contains("UNATTRIBUTABLE"), "{rendered}");
    assert!(rendered.contains("11.1248 %"), "{rendered}");
    assert!(rendered.contains("tolerated below"), "{rendered}");
}

#[test]
fn the_accounting_is_read_or_reported_absent() {
    let Attested::Known(first) = scheduling() else {
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

#[test]
fn a_reading_that_went_to_a_device_is_not_a_reading_of_mcf() {
    let clean = judge_with_faults(4_148, 21_834_777, Attested::Known(0));
    assert!(clean.permits_assertion(), "{clean}");

    let faulted = judge_with_faults(4_148, 21_834_777, Attested::Known(1));
    assert!(!faulted.permits_assertion(), "{faulted}");
    assert_eq!(faulted.major_faults(), Some(1));
    assert!(faulted.to_string().contains("storage"), "{faulted}");
    assert_eq!(faulted.delay_ppm(), Some(189));
}

#[test]
fn storage_is_reported_ahead_of_queuing_when_both_hold() {
    let both = judge_with_faults(15_477_083, 139_121_632, Attested::Known(9));
    assert!(!both.permits_assertion(), "{both}");
    assert_eq!(both.major_faults(), Some(9));
}

#[test]
fn an_unreadable_fault_counter_leaves_the_other_signal_deciding() {
    let quiet = judge_with_faults(4_148, 21_834_777, Attested::Unknown);
    assert!(quiet.permits_assertion(), "{quiet}");
    assert_eq!(quiet.major_faults(), None);

    let loaded = judge_with_faults(15_477_083, 139_121_632, Attested::Unknown);
    assert!(!loaded.permits_assertion(), "{loaded}");
}
