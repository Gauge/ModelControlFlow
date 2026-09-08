use super::{conditions, floor};
use crate::attested::Attested;
use crate::hardware::Machine;

#[test]
fn what_the_machine_reports_is_captured_and_the_rest_is_not_invented() {
    let machine = Machine::read();
    let captured = floor(&machine, None, "the tests", "full", None);

    assert!(captured.mcf_configuration.is_known());
    if machine.processor.model.is_known() {
        assert!(captured.hardware_state.is_known());
    }

    for (question, value) in [
        ("quantization", &captured.quantization),
        ("context_length", &captured.context_length),
        ("batch_shape", &captured.batch_shape),
        ("realized_placement", &captured.realized_placement),
    ] {
        assert!(
            !value.is_known(),
            "{question} claims a value, and nothing runs a model yet"
        );
    }
}

#[test]
fn the_characterization_verdict_is_part_of_the_conditions() {
    let machine = Machine::read();
    let captured = floor(&machine, None, "the tests", "full", None);
    let Attested::Known(described) = &captured.hardware_state else {
        return;
    };
    let described = described.to_string();
    if machine.accelerators.is_empty() {
        assert!(described.contains("no accelerator"), "{described}");
    } else {
        assert!(
            described.contains("characterized") || described.contains("uncharacterized"),
            "{described}"
        );
    }
    assert!(described.contains("load"), "{described}");
}

#[test]
fn a_machine_with_no_accelerator_captures_a_complete_honest_floor() {
    let machine = Machine::read_through(&[]);
    let captured = floor(&machine, None, "the tests", "full", None);
    assert_eq!(captured.entries().len(), 13);
    assert!(!captured.driver_versions.is_known());
    assert!(!captured.runtime_versions.is_known());
    let sensors = crate::hardware::thermal::sensors();
    match crate::hardware::thermal::processor(&sensors) {
        Some(_) => {
            let described = captured
                .thermal_state
                .known()
                .map(ToString::to_string)
                .unwrap_or_default();
            assert!(
                described.contains("processor"),
                "this machine publishes a processor sensor, so the floor must carry it: \
                 {described}"
            );
        }
        None => assert!(
            !captured.thermal_state.is_known(),
            "a machine with no processor sensor and no accelerator has no thermal reading, and \
             substituting a board zone is the mistake F91 corrected (A7)"
        ),
    }
}

#[test]
fn the_conditions_carry_the_instrument() {
    let captured = conditions(&Machine::read(), None, "the tests", "full", None);
    assert_eq!(
        captured.mcf(),
        crate::build_identity::BuildIdentity::current()
    );
    assert!(captured.to_string().contains("the tests"));
}
