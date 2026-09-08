use super::{Spent, release, spent, watch};

#[test]
fn what_was_spent_between_two_readings_is_their_difference() {
    let before = Spent {
        microjoules: 1_000,
        covered_ns: 2_000,
    };
    let now = Spent {
        microjoules: 3_500,
        covered_ns: 6_000,
    };
    assert_eq!(
        now.since(before),
        Spent {
            microjoules: 2_500,
            covered_ns: 4_000
        }
    );
    assert_eq!(before.since(now), Spent::default());
    assert_eq!(Spent::default().since(Spent::default()), Spent::default());
}

#[test]
fn watching_is_counted_so_the_last_engine_stops_it() {
    let before = spent();
    watch();
    watch();
    release();
    let during = spent();
    release();
    let after = spent();
    assert!(during.microjoules >= before.microjoules);
    assert!(after.microjoules >= during.microjoules);
    assert!(after.covered_ns >= before.covered_ns);
}
