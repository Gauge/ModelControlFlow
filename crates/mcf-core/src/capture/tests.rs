//! Tests for condition capture.
//!
//! B19 keeps these true on a machine with an accelerator and on one without,
//! which is what the seam is for: the no-accelerator case is produced rather
//! than waited for.

use super::{conditions, floor};
use crate::attested::Attested;
use crate::hardware::Machine;

/// The floor is filled from what the machine actually reports, and what it does
/// not report stays unknown (A7).
#[test]
fn what_the_machine_reports_is_captured_and_the_rest_is_not_invented() {
    let machine = Machine::read();
    let captured = floor(&machine, None, "the tests");

    // Always knowable: MCF's own configuration, and the processor.
    assert!(captured.mcf_configuration.is_known());
    if machine.processor.model.is_known() {
        assert!(captured.hardware_state.is_known());
    }

    // Never knowable at M0: nothing runs a model.
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

/// D25 makes characterization a per-run verdict, so it belongs in the
/// conditions: a measurement taken while a device was uncharacterized is not
/// comparable with one taken while it was (A8).
#[test]
fn the_characterization_verdict_is_part_of_the_conditions() {
    let machine = Machine::read();
    let captured = floor(&machine, None, "the tests");
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
    // What else the machine was doing is in there too (§3.8, B24).
    assert!(described.contains("load"), "{described}");
}

/// B19: the suite runs on a machine with no accelerator. Produced through the
/// seam rather than waited for, and the capture is still complete and honest.
#[test]
fn a_machine_with_no_accelerator_captures_a_complete_honest_floor() {
    let machine = Machine::read_through(&[]);
    let captured = floor(&machine, None, "the tests");
    assert_eq!(captured.entries().len(), 9);
    // No accelerator means no thermal reading and no driver version, and both
    // say so rather than reporting zero.
    assert!(!captured.thermal_state.is_known());
    assert!(!captured.driver_versions.is_known());
    assert!(!captured.runtime_versions.is_known());
}

/// The conditions carry the instrument as well as the floor, and the
/// instrument is never unknown: the running binary knows what it is (§3.4,
/// §3.12).
#[test]
fn the_conditions_carry_the_instrument() {
    let captured = conditions(&Machine::read(), None, "the tests");
    assert_eq!(
        captured.mcf(),
        crate::build_identity::BuildIdentity::current()
    );
    assert!(captured.to_string().contains("the tests"));
}
