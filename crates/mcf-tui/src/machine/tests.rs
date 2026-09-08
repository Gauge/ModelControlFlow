use super::*;

#[test]
fn the_first_look_has_no_rates() {
    let mut sampler = Sampler::new();
    let first = sampler.read();
    assert!(
        first.processor.load.is_none(),
        "a load from one reading would be a total presented as a rate"
    );
    for disk in &first.disks {
        assert!(
            disk.read.is_none(),
            "{} reported a rate from one look",
            disk.name
        );
    }
}

#[test]
fn a_second_look_has_a_load() {
    if std::fs::metadata("/proc/stat").is_err() {
        return;
    }
    let mut sampler = Sampler::new();
    let _first = sampler.read();
    let mut held = 0_u64;
    for index in 0..2_000_000_u64 {
        held = held.wrapping_add(index);
    }
    assert!(held > 0);
    let second = sampler.read();
    let load = second.processor.load.expect("a second look has a load");
    assert!(load.0 <= 1000, "a load over 100% is not a load: {}", load.0);
}

#[test]
fn memory_that_cannot_be_read_is_unknown() {
    let memory = Memory {
        total: None,
        available: None,
    };
    assert_eq!(memory.used(), None, "unknown minus unknown is not zero");
    let partial = Memory {
        total: Some(100),
        available: None,
    };
    assert_eq!(partial.used(), None);
}

#[test]
fn used_memory_is_what_is_not_available() {
    let memory = Memory {
        total: Some(92),
        available: Some(75),
    };
    assert_eq!(memory.used(), Some(17));
}

#[test]
fn tenths_render_whole() {
    assert_eq!(Tenths(384).whole(), 38);
    assert_eq!(Tenths(1000).whole(), 100);
    assert_eq!(Tenths(0).whole(), 0);
}

#[test]
fn a_reading_is_always_safe_to_take() {
    let mut sampler = Sampler::new();
    let _first = sampler.read();
    let second = sampler.read();
    if let Some(memory) = second.memory.total {
        assert!(memory > 0, "a machine with no memory is not this one");
    }
    for card in &second.cards {
        assert!(!card.name.is_empty());
    }
}

#[test]
fn a_bus_address_survives_either_spelling() {
    assert!(same_slot("00000000:C2:00.0", "0000:c2:00.0"));
    assert!(same_slot("0000:c2:00.0", "00000000:C2:00.0"));
    assert!(same_slot("0000:01:00.0", "0000:01:00.0"));

    assert!(!same_slot("00000000:C2:00.0", "0000:c1:00.0"));
    assert!(!same_slot("00000000:C2:00.0", "0000:c2:01.0"));
    assert!(!same_slot("00000000:C2:00.0", "0000:c2:00.1"));
    assert!(!same_slot("00000001:C2:00.0", "0000:c2:00.0"));

    assert!(!same_slot("", "0000:c2:00.0"));
    assert!(!same_slot("not-an-address", "0000:c2:00.0"));
}

#[test]
fn a_cards_memory_is_read_the_way_its_vendor_publishes_it() {
    for driver in ["amdgpu", "radeon", "i915", "xe", "nvidia", "unknown"] {
        let (used, total) = card_memory("card-that-is-not-here", driver);
        assert_eq!(used, None, "{driver} invented a used figure");
        assert_eq!(total, None, "{driver} invented a total figure");
    }
}

#[test]
fn a_missing_card_has_no_sensors_rather_than_another_cards() {
    assert!(card_hwmon("card-that-is-not-here").is_none());
    assert!(card_slot("card-that-is-not-here").is_none());
    assert!(card_number("card-that-is-not-here", "mem_info_vram_total").is_none());
}
