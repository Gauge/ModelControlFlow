use crate::attested::Attested;

use super::{Busy, unreadable};

fn busy(card: &str, driver: &str, percent: Option<u64>, because: Option<&str>) -> Busy {
    Busy {
        card: card.to_owned(),
        driver: driver.to_owned(),
        percent: percent.map_or(Attested::Unknown, Attested::Known),
        because: because.map(str::to_owned),
    }
}

#[test]
fn an_unreadable_accelerator_never_renders_as_idle() {
    let held = busy(
        "card2",
        "nvidia",
        None,
        Some("NVML is where the vendor put it"),
    );
    let shown = held.to_string();
    assert!(shown.contains("unknown"), "{shown}");
    for wrong in ["0%", "idle", "0 %"] {
        assert!(
            !shown.contains(wrong),
            "A7: a card MCF cannot poll is not a card doing nothing — {wrong} in {shown}"
        );
    }
}

#[test]
fn every_absence_carries_its_reason() {
    let held = busy(
        "card2",
        "nvidia",
        None,
        Some("NVML is where the vendor put it"),
    );
    assert!(
        held.to_string().contains("NVML"),
        "an operator must be able to tell *nothing was competing* from *MCF could not see*"
    );
    assert!(
        busy("card9", "mystery", None, None)
            .to_string()
            .contains("no reason was recorded")
    );
}

#[test]
fn a_reading_renders_as_one() {
    assert!(
        busy("card1", "amdgpu", Some(6), None)
            .to_string()
            .contains("6% busy")
    );
}

#[test]
fn the_unreadable_drivers_are_what_a_support_request_names() {
    let held = [
        busy("card1", "amdgpu", Some(6), None),
        busy("card2", "nvidia", None, Some("NVML")),
        busy("card3", "i915", None, Some("perf counters")),
    ];
    assert_eq!(unreadable(&held), ["i915".to_owned(), "nvidia".to_owned()]);
}

#[test]
fn this_machine_is_read_without_inventing_anything() {
    for one in super::accelerators() {
        assert!(!one.card.is_empty() && !one.driver.is_empty());
        if let Attested::Known(percent) = one.percent {
            assert!(percent <= 100, "{one} reports more than fully busy");
        } else {
            assert!(
                one.because.is_some(),
                "{one} is unknown for no stated reason"
            );
        }
    }
}
