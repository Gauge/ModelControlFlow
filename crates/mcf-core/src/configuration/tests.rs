use super::{Configuration, Engine, Placement, Quantization, Sampling, Thousandths, Weights};
use crate::attested::Attested;
use crate::build_identity::BuildIdentity;
use crate::measurement::{ConditionValue, Conditions, Count, Floor, Measurement};
use crate::provenance::{Repository, Revision};

fn configuration() -> Configuration {
    Configuration {
        weights: Weights {
            repository: Attested::Known(Repository::new("example-org/example-27B")),
            revision: Attested::Known(Revision::new("aa11bb22cc33")),
            digest: Attested::Unknown,
        },
        quantization: Attested::Known(Quantization::new("Q4_K_M")),
        context_length: Attested::Known(8192),
        engine: Engine::new("example-engine", Some("b4021".to_owned())),
        placement: Placement::AcceleratorPreferred,
        sampling: Sampling {
            temperature: Attested::Known(Thousandths(700)),
            top_p: Attested::Known(Thousandths(950)),
            top_k: Attested::Known(40),
            min_p: Attested::Known(Thousandths(50)),
            repetition_penalty: Attested::Known(Thousandths(1_050)),
            max_output_tokens: Attested::Known(2048),
        },
    }
}

#[test]
fn the_same_configuration_on_two_machines_is_one_identity() {
    let here = configuration();
    let there = configuration();
    assert!(here.is_the_same_thing_as(&there));

    let laptop = Conditions::new(
        BuildIdentity::current(),
        Floor {
            hardware_state: Attested::Known(ConditionValue::text("a laptop with no accelerator")),
            ..Floor::nothing_known()
        },
    );
    let workstation = Conditions::new(
        BuildIdentity::current(),
        Floor {
            hardware_state: Attested::Known(ConditionValue::text("a workstation with one")),
            ..Floor::nothing_known()
        },
    );
    assert_ne!(laptop, workstation);

    let a = Measurement::of(Count(10), Count(12), [], laptop);
    let b = Measurement::of(Count(30), Count(33), [], workstation);
    assert_ne!(a.conditions(), b.conditions());
    assert!(here.is_the_same_thing_as(&there));
}

#[test]
fn sampling_belongs_to_the_identity() {
    let mut warmer = configuration();
    warmer.sampling.temperature = Attested::Known(Thousandths(800));
    assert!(!configuration().is_the_same_thing_as(&warmer));
}

#[test]
fn the_engine_build_belongs_to_the_identity() {
    let mut rebuilt = configuration();
    rebuilt.engine = Engine::new("example-engine", Some("b4022".to_owned()));
    assert!(!configuration().is_the_same_thing_as(&rebuilt));
}

#[test]
fn the_declared_placement_belongs_to_the_identity() {
    let mut on_processor = configuration();
    on_processor.placement = Placement::ProcessorOnly;
    assert!(!configuration().is_the_same_thing_as(&on_processor));
}

#[test]
fn every_identity_field_distinguishes() {
    let base = configuration();

    let mut other_weights = base.clone();
    other_weights.weights.revision = Attested::Known(Revision::new("ffffffffffff"));

    let mut other_quantization = base.clone();
    other_quantization.quantization = Attested::Known(Quantization::new("Q5_K_M"));

    let mut other_context = base.clone();
    other_context.context_length = Attested::Known(4096);

    for variant in [other_weights, other_quantization, other_context] {
        assert!(
            !base.is_the_same_thing_as(&variant),
            "{variant} compares equal to {base}"
        );
    }
}

#[test]
fn an_unset_parameter_is_not_the_same_as_any_value() {
    let mut unset = configuration();
    unset.sampling.top_k = Attested::Unknown;
    assert!(!configuration().is_the_same_thing_as(&unset));
    assert!(unset.to_string().contains("top_k=unknown"), "{unset}");
}

#[test]
fn nothing_set_sets_nothing() {
    let sampling = Sampling::nothing_set();
    for (parameter, value) in sampling.entries() {
        assert_eq!(value, "unknown", "{parameter} claims a value");
    }
}

#[test]
fn thousandths_render_as_the_decimal_they_are() {
    assert_eq!(Thousandths(700).to_string(), "0.700");
    assert_eq!(Thousandths(1_050).to_string(), "1.050");
    assert_eq!(Thousandths(0).to_string(), "0.000");
    assert_eq!(Thousandths(2_000).to_string(), "2.000");
}

#[test]
fn a_decimal_is_read_as_thousandths_without_a_float() {
    assert_eq!("0.7".parse(), Ok(Thousandths(700)));
    assert_eq!("1".parse(), Ok(Thousandths(1_000)));
    assert_eq!(".5".parse(), Ok(Thousandths(500)));
    assert_eq!("0.9500".parse(), Ok(Thousandths(950)));
    assert_eq!(" 1.050 ".parse(), Ok(Thousandths(1_050)));
    assert_eq!("0.0001".parse::<Thousandths>(), Err(()));
    assert_eq!("warm".parse::<Thousandths>(), Err(()));
    assert_eq!("".parse::<Thousandths>(), Err(()));
    assert_eq!("-0.5".parse::<Thousandths>(), Err(()));
}

#[test]
fn adjacent_thousandths_are_distinguishable() {
    assert_ne!(Thousandths(700), Thousandths(701));
    assert!(Thousandths(700) < Thousandths(701));
}

#[test]
fn the_realized_layout_is_a_condition() {
    let floor = Floor {
        realized_placement: Attested::Known(ConditionValue::text("28 of 32 layers offloaded")),
        ..Floor::nothing_known()
    };
    assert_eq!(floor.known_count(), 1);
    assert_eq!(floor.entries().len(), 13);
    assert!(
        floor
            .entries()
            .iter()
            .any(|(question, _)| *question == "realized_placement")
    );
}

#[test]
fn the_rendering_names_every_part_of_the_identity() {
    let rendered = configuration().to_string();
    for expected in [
        "example-org/example-27B",
        "aa11bb22cc33",
        "Q4_K_M",
        "8192",
        "example-engine",
        "b4021",
        "accelerator preferred",
        "temperature=0.700",
    ] {
        assert!(
            rendered.contains(expected),
            "{rendered:?} omits {expected:?}"
        );
    }
}
