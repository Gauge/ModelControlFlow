use super::{Energy, PerSecond};
use crate::price::{Cost, PricePerKwh};

fn measured(millijoules: u64, rate: u32, counter: &str) -> Energy {
    Energy::Measured {
        millijoules,
        rate: PerSecond(rate),
        counter: counter.to_owned(),
    }
}

#[test]
fn only_a_reading_yields_joules() {
    assert_eq!(
        measured(4_200, 10, "package").measured_millijoules(),
        Some(4_200)
    );
    for empty in [
        Energy::Modelled {
            millijoules: 4_200,
            by: "the vendor's model".to_owned(),
        },
        Energy::Unknown {
            why: "this platform exposes no interface".to_owned(),
        },
    ] {
        assert_eq!(
            empty.measured_millijoules(),
            None,
            "a modelled or absent figure must not leave as a number (B39, A20)"
        );
        assert!(!empty.is_measured());
    }
}

#[test]
fn nothing_readable_is_unknown_rather_than_zero() {
    let shown = Energy::Unknown {
        why: "this platform exposes no interface".to_owned(),
    }
    .to_string();
    assert!(shown.contains("Not zero"), "{shown}");
    assert!(
        shown.contains("not derived from utilization"),
        "B39's exact violation is deriving a figure from utilization, and the sentence must \
         name it: {shown}"
    );
}

#[test]
fn a_modelled_figure_says_so_where_a_reader_sees_it() {
    let shown = Energy::Modelled {
        millijoules: 4_200,
        by: "the vendor's model".to_owned(),
    }
    .to_string();
    assert!(shown.contains("MODELLED"), "{shown}");
    assert!(shown.contains("A20"), "{shown}");
}

#[test]
fn the_sampling_rate_travels_with_the_reading() {
    let shown = measured(4_200, 10, "package").to_string();
    assert!(
        shown.contains("sampled 10 time(s) a second"),
        "energy read at one hertz across a two-second run has seen two samples, and a reading \
         whose rate does not travel cannot be compared with one taken at another (§3.4): \
         {shown}"
    );
    assert!(
        shown.contains("package"),
        "and which counter it was: {shown}"
    );
}

#[test]
fn two_readings_compare_only_at_the_same_rate_from_the_same_counter() {
    let held = measured(4_200, 10, "package");
    assert!(held.comparable_with(&measured(4_000, 10, "package")));
    assert!(
        !held.comparable_with(&measured(4_000, 1, "package")),
        "different rates saw different fractions of their runs (A8)"
    );
    assert!(
        !held.comparable_with(&measured(4_000, 10, "the accelerator")),
        "and a package-level figure is not a device-level one"
    );
}

#[test]
fn a_modelled_figure_compares_with_nothing() {
    let modelled = Energy::Modelled {
        millijoules: 4_200,
        by: "the vendor's model".to_owned(),
    };
    assert!(!modelled.comparable_with(&measured(4_200, 10, "package")));
    assert!(
        !modelled.comparable_with(&modelled.clone()),
        "two models are two authors' opinions rather than two readings"
    );
}

#[test]
fn a_measured_energy_costs_what_the_price_says() {
    // One kilowatt-hour is 3.6e9 millijoules; at 0.28 it costs 0.28.
    let energy = Energy::Measured {
        millijoules: 3_600_000_000,
        rate: PerSecond(1),
        counter: "a counter".to_owned(),
    };
    let price = PricePerKwh::parse("0.28").expect("a price");
    assert_eq!(
        energy.cost_at(price),
        Some(Cost {
            millionths: 280_000
        })
    );
}

#[test]
fn an_energy_nobody_measured_has_no_cost() {
    let price = PricePerKwh::parse("0.28").expect("a price");
    let modelled = Energy::Modelled {
        millijoules: 3_600_000_000,
        by: "an estimate".to_owned(),
    };
    let unknown = Energy::Unknown {
        why: "no counter here".to_owned(),
    };
    assert_eq!(modelled.cost_at(price), None, "an estimate was priced");
    assert_eq!(unknown.cost_at(price), None, "an unknown was priced");
}
