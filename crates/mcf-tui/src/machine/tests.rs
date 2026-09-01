use super::*;

/// A sampler's first look has no rates, because a rate is a difference between
/// two looks and there has only been one.
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

/// A second look has them, on a platform that has the files.
#[test]
fn a_second_look_has_a_load() {
    if std::fs::metadata("/proc/stat").is_err() {
        return; // not a platform with these files; the console still draws
    }
    let mut sampler = Sampler::new();
    let _first = sampler.read();
    // Something to measure, so the load is not trivially zero.
    let mut held = 0_u64;
    for index in 0..2_000_000_u64 {
        held = held.wrapping_add(index);
    }
    assert!(held > 0);
    let second = sampler.read();
    let load = second.processor.load.expect("a second look has a load");
    assert!(load.0 <= 1000, "a load over 100% is not a load: {}", load.0);
}

/// Nothing read here is ever a zero standing in for a missing reading.
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

/// Used is total minus what is free for something new.
#[test]
fn used_memory_is_what_is_not_available() {
    let memory = Memory {
        total: Some(92),
        available: Some(75),
    };
    assert_eq!(memory.used(), Some(17));
}

/// A percentage renders without a float.
#[test]
fn tenths_render_whole() {
    assert_eq!(Tenths(384).whole(), 38);
    assert_eq!(Tenths(1000).whole(), 100);
    assert_eq!(Tenths(0).whole(), 0);
}

/// Reading this machine says something, or says nothing, but never panics.
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

/// A bus address is the same address however the two tools spell it.
///
/// `nvidia-smi` writes `00000000:C2:00.0`; sysfs writes `0000:c2:00.0`. If
/// these did not compare equal, every NVIDIA card would fall back to its sysfs
/// name and lose the occupancy, power and memory only the vendor tool has.
#[test]
fn a_bus_address_survives_either_spelling() {
    assert!(same_slot("00000000:C2:00.0", "0000:c2:00.0"));
    assert!(same_slot("0000:c2:00.0", "00000000:C2:00.0"));
    assert!(same_slot("0000:01:00.0", "0000:01:00.0"));

    // A different slot is a different card, and must never be merged.
    assert!(!same_slot("00000000:C2:00.0", "0000:c1:00.0"));
    assert!(!same_slot("00000000:C2:00.0", "0000:c2:01.0"));
    assert!(!same_slot("00000000:C2:00.0", "0000:c2:00.1"));
    // A second domain is a second machine's worth of bus, not the same slot.
    assert!(!same_slot("00000001:C2:00.0", "0000:c2:00.0"));

    // Nothing that is not an address matches anything.
    assert!(!same_slot("", "0000:c2:00.0"));
    assert!(!same_slot("not-an-address", "0000:c2:00.0"));
}

/// Each vendor publishes a card's own memory its own way, and an integrated
/// card has none.
///
/// A card with no separate memory answers `None` for both rather than zero:
/// a zero here would be read as *a card with no memory left*, which is the
/// opposite of what is true (A7).
#[test]
fn a_cards_memory_is_read_the_way_its_vendor_publishes_it() {
    // Nothing is mounted at these paths in a test, so every vendor answers
    // `None` — what is being pinned is that no vendor is silently treated as
    // another, and that an unknown is never a zero.
    for driver in ["amdgpu", "radeon", "i915", "xe", "nvidia", "unknown"] {
        let (used, total) = card_memory("card-that-is-not-here", driver);
        assert_eq!(used, None, "{driver} invented a used figure");
        assert_eq!(total, None, "{driver} invented a total figure");
    }
}

/// A card's sensors are read from that card, never from a card like it.
///
/// Matching sensors by driver name gave two cards of the same make one card's
/// temperature twice, and the number in `hwmon3` is whatever the kernel handed
/// out at boot — so a hard-coded `hwmon0` reads another device or nothing.
#[test]
fn a_missing_card_has_no_sensors_rather_than_another_cards() {
    assert!(card_hwmon("card-that-is-not-here").is_none());
    assert!(card_slot("card-that-is-not-here").is_none());
    assert!(card_number("card-that-is-not-here", "mem_info_vram_total").is_none());
}
