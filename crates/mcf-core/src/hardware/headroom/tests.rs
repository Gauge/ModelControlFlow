use super::{Band, Headroom, MEASURED};

fn on(competing: u64, capacity: u64) -> Headroom {
    Headroom {
        competing,
        capacity,
        band: MEASURED,
    }
}

#[test]
fn the_same_busy_thread_count_is_different_on_two_machines() {
    let small = on(16_000, 32_000);
    let large = on(16_000, 256_000);
    assert!(
        !small.within_band(),
        "half a machine is outside the band: {small}"
    );
    assert!(
        large.within_band(),
        "a sixteenth of one is inside it: {large}"
    );
}

#[test]
fn the_band_is_where_f95_measured_it() {
    assert_eq!(
        MEASURED.fraction, 300_000,
        "thirty percent of capacity (F95)"
    );
    assert!(
        on(9_000, 32_000).within_band(),
        "28% was indistinguishable from baseline in the sweep"
    );
    assert!(
        !on(17_000, 32_000).within_band(),
        "53% was six to thirty times wider"
    );
}

#[test]
fn an_idle_machine_has_room() {
    assert!(on(1_260, 32_000).within_band());
    assert_eq!(on(0, 32_000).fraction(), 0);
}

#[test]
fn a_run_outside_the_band_is_a_measurement_and_not_a_refusal() {
    let shown = on(27_000, 32_000).to_string();
    assert!(shown.contains("OUTSIDE the band"), "{shown}");
    assert!(
        shown.contains("real measurement that is not fit to contribute"),
        "the operator's decision was to run and mark rather than to refuse: {shown}"
    );
    for refusing in ["refused", "will not run", "aborted"] {
        assert!(!shown.contains(refusing), "{refusing} in {shown}");
    }
}

#[test]
fn the_band_says_it_is_somebody_elses_measurement() {
    let shown = on(27_000, 32_000).to_string();
    assert!(
        shown.contains("DECLARED"),
        "A21: a figure measured on one machine is declared here, not verified: {shown}"
    );
    assert!(
        shown.contains("prototypes/contention-band"),
        "and the route to replacing it must be named, or the marking is a disclaimer: {shown}"
    );
    assert!(
        MEASURED.measured_on.contains("F95"),
        "the band cites the finding that established it"
    );
}

#[test]
fn a_machine_with_no_capacity_reported_does_not_divide_by_it() {
    let held = Headroom {
        competing: 5_000,
        capacity: 0,
        band: Band {
            fraction: 300_000,
            measured_on: "a test",
        },
    };
    assert!(held.fraction() > 0);
}

#[test]
fn this_machine_reads_its_own_capacity() {
    let held = Headroom::taken(0);
    assert!(held.capacity >= 1_000, "at least one thread: {held:?}");
    assert_eq!(held.band, MEASURED);
}
