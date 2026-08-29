//! Every sensor is carried; none is chosen for the reader.

use super::{Kind, Sensor, kind_of, processor};

fn sensor(chip: &str, label: Option<&str>, millidegrees: i64) -> Sensor {
    Sensor {
        kind: kind_of(chip, label),
        chip: chip.to_owned(),
        label: label.map(str::to_owned),
        millidegrees,
        critical_millidegrees: None,
    }
}

#[test]
fn a_compute_die_is_told_from_a_control_temperature() {
    // A Ryzen publishes both: `Tctl` is derived by firmware and may carry an
    // offset; `Tccd1` is a die.
    assert_eq!(kind_of("k10temp", Some("Tccd1")), Kind::ProcessorDie);
    assert_eq!(kind_of("k10temp", Some("Tctl")), Kind::ProcessorPackage);
    assert_eq!(kind_of("coretemp", Some("Core 3")), Kind::ProcessorDie);
    assert_eq!(
        kind_of("coretemp", Some("Package id 0")),
        Kind::ProcessorPackage
    );
}

#[test]
fn the_acpi_zone_is_not_mistaken_for_a_processor() {
    // The failure F53 recorded as a fact: this board's ACPI zone reads 16.8 °C
    // while the processor is at 70 °C, and reading it as *the* temperature is
    // what closed off DEC-007's thermal half.
    assert_eq!(kind_of("acpitz", None), Kind::Board);
    let held = [
        sensor("acpitz", None, 16_800),
        sensor("k10temp", Some("Tccd1"), 66_500),
    ];
    let found = processor(&held).expect("a die sensor is present");
    assert_eq!(
        found.millidegrees, 66_500,
        "the board zone must not answer for the processor"
    );
}

#[test]
fn a_die_reading_is_preferred_over_a_package_one() {
    let held = [
        sensor("k10temp", Some("Tctl"), 70_250),
        sensor("k10temp", Some("Tccd1"), 66_500),
    ];
    let found = processor(&held).expect("both are processor sensors");
    assert_eq!(
        found.label.as_deref(),
        Some("Tccd1"),
        "a control temperature can carry a vendor offset; a die sensor is the thing doing the \
         work — even when it reads lower"
    );
}

#[test]
fn a_machine_with_no_processor_sensor_answers_nothing() {
    let held = [
        sensor("acpitz", None, 16_800),
        sensor("nvme", Some("Composite"), 32_850),
    ];
    assert!(
        processor(&held).is_none(),
        "A7: no processor sensor is unknown, and substituting a drive or a board zone would be \
         the failure this module exists to correct"
    );
}

#[test]
fn an_unrecognised_chip_keeps_its_reading() {
    let held = sensor("some_future_driver", Some("Tdie"), 55_000);
    assert_eq!(held.kind, Kind::Unclassified);
    assert_eq!(
        held.millidegrees, 55_000,
        "A1: a chip this build does not recognise loses its label, never its reading"
    );
}

#[test]
fn a_missing_critical_point_says_so_rather_than_implying_headroom() {
    let shown = sensor("k10temp", Some("Tccd1"), 66_500).to_string();
    assert!(
        shown.contains("critical point not published"),
        "a reader not told the limit is missing will assume there is room: {shown}"
    );
}

/// This machine, read for real.
#[test]
fn this_machine_is_read_without_inventing_anything() {
    let held = super::sensors();
    for one in &held {
        assert!(
            one.millidegrees > -50_000 && one.millidegrees < 200_000,
            "{one} is outside anything a thermometer on a working machine reports"
        );
        assert!(
            !one.chip.is_empty(),
            "every reading names the chip that published it"
        );
    }
    // Not an assertion that this machine has sensors — a container or another
    // platform may have none, and that is a finding rather than a failure.
    if let Some(found) = processor(&held) {
        assert!(
            matches!(found.kind, Kind::ProcessorDie | Kind::ProcessorPackage),
            "the processor reading must come from a processor sensor: {found}"
        );
    }
}

/// **A sentinel is not a temperature.** An `NVMe` drive on this machine
/// publishes `temp2_max` as 65261850 — 65261.8 °C — which rendered as a
/// critical point beside real ones. A number no thermometer produced,
/// presented with the confidence of one that was measured: the same defect as
/// F90's contention reading above the machine's ceiling.
#[test]
fn an_out_of_range_limit_is_read_as_absent() {
    for one in super::sensors() {
        if let Some(limit) = one.critical_millidegrees {
            assert!(
                (-50_000..=200_000).contains(&limit),
                "{one} publishes a critical point no silicon reaches; absent is what such a \
                 value means (A7)"
            );
        }
    }
}
