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
