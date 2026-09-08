use super::{
    Attested, Basis, Bytes, ConditionValue, Conditions, Count, Estimate, Floor, Measurement,
    PartsPerMillion, Percentile,
};
use crate::build_identity::BuildIdentity;

fn conditions() -> Conditions {
    Conditions::new(BuildIdentity::current(), Floor::nothing_known())
}

fn ten() -> Measurement<Count> {
    Measurement::of(
        Count(10),
        Count(20),
        (3..=10).map(|i| Count(i * 10)),
        conditions(),
    )
}

#[test]
fn the_sample_count_is_the_number_of_samples() {
    assert_eq!(ten().n(), 10);
    assert_eq!(Measurement::of(Count(1), Count(2), [], conditions()).n(), 2);
}

#[test]
fn one_sample_is_not_a_measurement() {
    assert!(Measurement::from_samples(vec![Count(1)], conditions()).is_none());
    assert!(Measurement::<Count>::from_samples(vec![], conditions()).is_none());
    assert!(Measurement::from_samples(vec![Count(1), Count(2)], conditions()).is_some());
}

#[test]
fn the_samples_are_kept_in_the_order_they_were_taken() {
    let measurement = Measurement::of(Count(30), Count(10), [Count(20)], conditions());
    let taken: Vec<Count> = measurement.samples().collect();
    assert_eq!(taken, [Count(30), Count(10), Count(20)]);
    assert_eq!(measurement.sorted(), [Count(10), Count(20), Count(30)]);
}

#[test]
fn the_percentiles_are_nearest_rank() {
    let measurement = ten();
    assert_eq!(measurement.at(Percentile::P5), Count(10));
    assert_eq!(measurement.at(Percentile::MEDIAN), Count(50));
    assert_eq!(measurement.at(Percentile::P95), Count(100));
}

#[test]
fn every_percentile_is_an_observed_sample() {
    let measurement = ten();
    let observed = measurement.sorted();
    for rank in 0..=100 {
        let percentile = Percentile::new(rank).expect("0..=100 are all percentiles");
        assert!(
            observed.contains(&measurement.at(percentile)),
            "p{rank} returned a value nothing observed"
        );
    }
}

#[test]
fn the_extremes_do_not_depend_on_arrival_order() {
    let ascending = Measurement::of(Count(1), Count(2), [Count(3)], conditions());
    let descending = Measurement::of(Count(3), Count(2), [Count(1)], conditions());
    for measurement in [&ascending, &descending] {
        assert_eq!(measurement.minimum(), Count(1));
        assert_eq!(measurement.maximum(), Count(3));
    }
}

#[test]
fn the_spread_is_ordered() {
    for length in 2..40_u64 {
        let samples: Vec<Count> = (0..length).map(|i| Count((i * 7) % 13)).collect();
        let measurement =
            Measurement::from_samples(samples, conditions()).expect("at least two samples");
        let spread = measurement.spread();
        assert!(spread.minimum <= spread.p5, "n={length}");
        assert!(spread.p5 <= spread.median, "n={length}");
        assert!(spread.median <= spread.p95, "n={length}");
        assert!(spread.p95 <= spread.maximum, "n={length}");
    }
}

#[test]
fn two_samples_give_a_two_point_spread() {
    let measurement = Measurement::of(Bytes(100), Bytes(200), [], conditions());
    let spread = measurement.spread();
    assert_eq!(spread.minimum, Bytes(100));
    assert_eq!(spread.p5, Bytes(100));
    assert_eq!(spread.median, Bytes(100));
    assert_eq!(spread.p95, Bytes(200));
    assert_eq!(spread.maximum, Bytes(200));
}

#[test]
fn identical_samples_have_no_spread() {
    let measurement = Measurement::of(Count(5), Count(5), [Count(5)], conditions());
    let spread = measurement.spread();
    assert_eq!(spread.minimum, spread.maximum);
    assert_eq!(measurement.n(), 3);
}

#[test]
fn a_rank_above_a_hundred_is_not_a_percentile() {
    assert!(Percentile::new(101).is_none());
    assert!(Percentile::new(255).is_none());
    assert_eq!(Percentile::new(0).map(Percentile::rank), Some(0));
    assert_eq!(Percentile::new(100).map(Percentile::rank), Some(100));
}

#[test]
fn the_rendering_carries_the_conditions_the_count_and_the_spread() {
    let rendered = ten().to_string();
    assert!(rendered.contains("n=10"), "{rendered}");
    assert!(rendered.contains("p5–p95"), "{rendered}");
    assert!(
        rendered.contains(BuildIdentity::current().version),
        "{rendered} omits the instrument"
    );
    assert!(
        rendered.contains("hardware_state=unknown"),
        "{rendered} omits an unknown condition"
    );
}

#[test]
fn an_unread_condition_stays_unknown() {
    let floor = Floor::nothing_known();
    assert_eq!(floor.known_count(), 0);
    assert_eq!(floor.entries().len(), 13);
    for (question, value) in floor.entries() {
        assert!(!value.is_known(), "{question} claims to be known");
        assert_eq!(value.to_string(), "unknown");
    }
}

#[test]
fn a_read_condition_reports_what_was_read() {
    let floor = Floor {
        context_length: Attested::Known(ConditionValue::integer(4096)),
        ..Floor::nothing_known()
    };
    assert_eq!(floor.known_count(), 1);
    assert_eq!(
        floor.context_length.known(),
        Some(&ConditionValue::integer(4096))
    );
    assert_eq!(floor.context_length.to_string(), "4096");
    assert_eq!(floor.thermal_state.to_string(), "unknown");
}

#[test]
fn the_conditions_carry_the_instrument() {
    assert_eq!(conditions().mcf(), BuildIdentity::current());
}

#[test]
fn quantities_render_with_their_units() {
    assert_eq!(Bytes(512).to_string(), "512 B");
    assert_eq!(Bytes(1024).to_string(), "1024 B (1.0 KiB)");
    assert_eq!(Bytes(1_610_612_736).to_string(), "1610612736 B (1.5 GiB)");
    assert_eq!(Bytes(0).human(), "0 B");
    assert_eq!(PartsPerMillion(500).to_string(), "500 ppm");
    assert_eq!(Count(0).to_string(), "0");
}

#[test]
fn a_prohibition_is_still_a_measurement() {
    let wakeups = Measurement::of(Count(0), Count(0), [Count(0)], conditions());
    assert_eq!(wakeups.maximum(), Count(0));
    assert_eq!(wakeups.n(), 3);
}

#[test]
fn an_estimate_renders_as_an_estimate_with_its_basis() {
    let banded = Estimate::band(Count(10), Count(30), Basis::LocalHistory);
    let rendered = banded.to_string();
    assert!(rendered.contains("estimate"), "{rendered}");
    assert!(rendered.contains("local history"), "{rendered}");
    assert!(
        rendered.contains("10") && rendered.contains("30"),
        "{rendered}"
    );
}

#[test]
fn a_point_estimate_is_still_an_estimate() {
    let point = Estimate::point(Bytes(4096), Basis::Declared);
    assert!(point.is_point());
    assert_eq!(point.low(), point.high());
    assert!(point.to_string().contains("estimate"), "{point}");
}

#[test]
fn a_band_is_ordered_however_it_was_written() {
    let forwards = Estimate::band(Count(1), Count(9), Basis::LocalHistory);
    let backwards = Estimate::band(Count(9), Count(1), Basis::LocalHistory);
    assert_eq!(forwards, backwards);
    assert_eq!(forwards.low(), Count(1));
    assert_eq!(forwards.high(), Count(9));
}

#[test]
fn a_corpus_basis_carries_its_sample_count() {
    let thin = Estimate::point(Count(5), Basis::Corpus { reports: 2 });
    let thick = Estimate::point(Count(5), Basis::Corpus { reports: 400 });
    assert_ne!(thin, thick);
    assert!(thin.to_string().contains("n=2"), "{thin}");
    assert!(thick.to_string().contains("n=400"), "{thick}");
}

#[test]
fn an_estimate_and_a_measurement_do_not_look_alike() {
    let measurement = Measurement::of(Count(10), Count(30), [Count(20)], conditions());
    let estimate = Estimate::band(Count(10), Count(30), Basis::LocalHistory);
    assert!(measurement.to_string().contains("n=3"));
    assert!(!measurement.to_string().contains("estimate"));
    assert!(estimate.to_string().contains("estimate"));
    assert!(!estimate.to_string().contains("n=3"));
}
