use super::{Knob, Sampling};
use mcf_record::json::Value;

#[test]
fn a_setting_that_is_set_is_handed_to_the_engine_by_its_flag() {
    let mut held = Sampling::default();
    held.set(Knob::DryStrength, Some(800));
    held.set(Knob::DryAllowedRun, Some(3));
    held.set(Knob::MinP, Some(0));
    assert_eq!(
        held.arguments(),
        vec![
            "--min-p",
            "0.000",
            "--dry-multiplier",
            "0.800",
            "--dry-allowed-length",
            "3"
        ],
        "a min-p of nought is sent, because nought and the engine's own 0.05 are not the same"
    );
}

#[test]
fn nothing_set_is_nothing_sent_and_nothing_saved() {
    let held = Sampling::default();
    assert!(held.is_empty());
    assert!(held.arguments().is_empty());
    assert!(held.pairs().is_empty());
}

#[test]
fn what_is_saved_reads_back_as_it_was() {
    let mut held = Sampling::default();
    for (at, knob) in Knob::ALL.into_iter().enumerate() {
        let (least, _) = knob.reach();
        held.set(knob, Some(least + u32::try_from(at).unwrap_or(0)));
    }
    let saved = Value::map(held.pairs());
    assert_eq!(Sampling::from_value(&saved), held);
}

#[test]
fn a_typed_value_is_read_in_the_settings_own_grain_and_kept_inside_its_reach() {
    assert_eq!(Knob::PresencePenalty.read("1.5"), Ok(1500));
    assert_eq!(Knob::DryWindow.read("256"), Ok(256));
    assert!(Knob::DryWindow.read("2.5").is_err(), "a window is a count");
    let why = Knob::RepeatPenalty
        .read("0.9")
        .expect_err("below one rewards repeating");
    assert!(why.contains("runs from 1.000 to 2.000"), "{why}");
    assert!(Knob::MinP.read("1.5").is_err(), "min-p is a share");
}

#[test]
fn every_setting_says_what_it_does_in_a_sentence_of_its_own() {
    for knob in Knob::ALL {
        let said = knob.because();
        assert!(
            said.split_whitespace().count() >= 8,
            "{}: {said}",
            knob.label()
        );
        assert!(
            !said
                .to_lowercase()
                .starts_with(&knob.label().to_lowercase()),
            "{} restates its own name",
            knob.label()
        );
    }
}
