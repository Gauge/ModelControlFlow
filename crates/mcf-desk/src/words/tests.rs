use super::{
    UNMEASURED, against_reading, does_it_fit, grouped, holds_up, remembers, size_in_words,
    speed_in_words, wakes_in,
};

#[test]
fn a_measurement_becomes_a_sentence_a_person_can_use() {
    assert_eq!(
        speed_in_words(Some(155.0)).as_deref(),
        Some("116 words a second")
    );
    assert_eq!(
        against_reading(Some(155.0)).as_deref(),
        Some("about 29× faster than you can read")
    );
}

#[test]
fn nothing_measured_produces_no_sentence_at_all() {
    assert_eq!(speed_in_words(None), None);
    assert_eq!(against_reading(None), None);
    assert_eq!(remembers(None), None);
    assert_eq!(size_in_words(None), None);
    assert_eq!(wakes_in(None), None);
    assert_eq!(holds_up(None, Some(2.0)), None);
    assert_eq!(does_it_fit(None, Some(16), "memory"), UNMEASURED);
}

#[test]
fn a_zero_rate_is_not_a_slow_model_but_an_absent_measurement() {
    assert_eq!(speed_in_words(Some(0.0)), None);
    assert_eq!(speed_in_words(Some(-1.0)), None);
    assert_eq!(speed_in_words(Some(f64::NAN)), None);
    assert_eq!(against_reading(Some(f64::INFINITY)), None);
}

#[test]
fn a_slow_model_is_told_plainly_and_not_as_a_fraction() {
    assert_eq!(
        against_reading(Some(2.0)).as_deref(),
        Some("slower than you can read")
    );
    assert_eq!(
        against_reading(Some(5.3)).as_deref(),
        Some("about as fast as you can read")
    );
}

#[test]
fn a_conversation_length_is_rounded_to_what_the_conversion_can_carry() {
    assert_eq!(
        remembers(Some(32_768)).as_deref(),
        Some("about 25,000 words")
    );
    assert_eq!(remembers(Some(4_096)).as_deref(), Some("about 3,100 words"));
    assert_eq!(remembers(Some(0)), None);
}

#[test]
fn thousands_are_grouped_where_a_person_reads_them() {
    assert_eq!(grouped(0), "0");
    assert_eq!(grouped(999), "999");
    assert_eq!(grouped(1_000), "1,000");
    assert_eq!(grouped(25_000), "25,000");
    assert_eq!(grouped(1_234_567), "1,234,567");
}

#[test]
fn a_size_reads_as_a_person_would_say_it() {
    assert_eq!(
        size_in_words(Some(5_020_000_000)).as_deref(),
        Some("5.0 GB")
    );
    assert_eq!(
        size_in_words(Some(14_000_000_000)).as_deref(),
        Some("14 GB")
    );
    assert_eq!(size_in_words(Some(270_000_000)).as_deref(), Some("270 MB"));
}

#[test]
fn whether_it_fits_reads_the_same_way_either_way() {
    assert_eq!(
        does_it_fit(Some(5_000_000_000), Some(16_000_000_000), "graphics card"),
        "Uses 5.0 GB of your 16 GB of graphics card"
    );
    assert_eq!(
        does_it_fit(Some(40_000_000_000), Some(16_000_000_000), "graphics card"),
        "Needs 40 GB, and there is 16 GB of graphics card"
    );
}

#[test]
fn the_fall_off_is_reported_as_whether_it_matters() {
    assert_eq!(
        holds_up(Some(6.43), Some(7.66)).as_deref(),
        Some("Stays fast")
    );
    assert_eq!(
        holds_up(Some(10.0), Some(15.0)).as_deref(),
        Some("Slows a little")
    );
    assert_eq!(
        holds_up(Some(10.0), Some(40.0)).as_deref(),
        Some("Slows to about a 4th of its speed")
    );
}

#[test]
fn waking_up_is_never_reported_as_a_bare_zero() {
    assert_eq!(wakes_in(Some(0.4)).as_deref(), Some("under a second"));
    assert_eq!(wakes_in(Some(1.9)).as_deref(), Some("2 seconds"));
    assert_eq!(wakes_in(Some(240.0)).as_deref(), Some("4 minutes"));
    assert_eq!(wakes_in(Some(-1.0)), None);
}

#[test]
fn the_conditions_of_every_conversion_can_be_shown() {
    let said = super::basis();
    assert!(
        said.contains("0.75"),
        "the word-per-token ratio is not stated: {said}"
    );
    assert!(
        said.contains("240"),
        "the reading speed is not stated: {said}"
    );
}
