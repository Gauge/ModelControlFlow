//! What a hosting settings may and may not do.

use super::{DEFAULT_PORT, Hosting, LOOPBACK};
use mcf_record::json::Value;

fn on_a_card() -> Hosting {
    Hosting::recommended(
        "llama.cpp-cuda",
        "NVIDIA",
        true,
        32_768,
        Some(32),
        true,
        None,
    )
}

/// The recommendation puts the model on the card when it fits.
///
/// **This is the defect that made the check worth writing.** The engine was
/// started with `-ngl 0` written into the source, so MCF resolved a model to a
/// graphics card, said *runs on NVIDIA*, and ran it on the processor. Measured
/// on this machine the difference was 158 against 770 tokens a second (F133).
#[test]
fn a_model_that_fits_on_the_card_is_put_on_the_card() {
    let recommended = on_a_card();
    assert!(
        recommended.gpu_layers > 0,
        "a model that fits on the card was recommended onto the processor"
    );
    let arguments = recommended.arguments("/models/a.gguf", "127.0.0.1");
    let at = arguments
        .iter()
        .position(|held| held == "--n-gpu-layers")
        .expect("the layer count is passed to the engine");
    assert_ne!(
        arguments.get(at + 1).map(String::as_str),
        Some("0"),
        "the engine is still being told to use no layers: {arguments:?}"
    );
}

/// A model that does not fit stays on the processor.
///
/// Not a failure and not a warning: a model too large for the card runs, and
/// where it runs is a fact about this machine rather than a fault (A9).
#[test]
fn a_model_that_does_not_fit_stays_on_the_processor() {
    let recommended = Hosting::recommended(
        "llama.cpp-cuda",
        "NVIDIA",
        true,
        8_192,
        Some(32),
        false,
        None,
    );
    assert_eq!(recommended.gpu_layers, 0);
    assert!(!recommended.flash_attention);
}

/// Every setting is listed, with what MCF recommended beside it.
#[test]
fn every_setting_is_shown_with_what_was_recommended() {
    let recommended = on_a_card();
    let listed = recommended.listed(&recommended);
    assert!(
        listed.len() >= 10,
        "only {} settings are listed",
        listed.len()
    );
    for setting in &listed {
        assert!(!setting.name.is_empty());
        assert!(!setting.value.is_empty(), "{} has no value", setting.name);
        assert!(
            !setting.because.is_empty(),
            "{} says nothing about what it does",
            setting.name
        );
    }
    // Nothing has been moved, so nothing is reported as moved.
    assert!(recommended.differs_from(&recommended).is_empty());
}

/// A setting somebody changed is reported as changed.
///
/// §3.15 and A6: a run under a changed setting is not a run under the
/// recommended one, and the record carries both so the two can disagree in
/// writing.
#[test]
fn a_setting_that_was_moved_says_so() {
    let recommended = on_a_card();
    let mut chosen = recommended.clone();
    chosen.gpu_layers = 0;
    chosen.context = 4_096;
    let moved = chosen.differs_from(&recommended);
    assert_eq!(moved.len(), 2, "{moved:?}");
    assert!(
        moved.iter().any(|said| said.contains("put it on")),
        "{moved:?}"
    );
    assert!(
        moved.iter().any(|said| said.contains("context window")),
        "{moved:?}"
    );
}

/// A key is a condition, and never a value the record holds.
///
/// A25 and §3.20: a record is something MCF publishes, and a secret in a
/// published record is a secret nobody meant to publish.
#[test]
fn a_key_is_recorded_as_present_and_never_as_itself() {
    let mut chosen = on_a_card();
    chosen.api_key = Some("a-secret-nobody-should-see".to_owned());
    let written = chosen.to_value().to_line();
    assert!(
        !written.contains("a-secret-nobody-should-see"),
        "the key itself reached the record: {written}"
    );
    assert!(written.contains("api_key_set"), "{written}");
    // And the interface says only that one is set.
    let listed = chosen.listed(&chosen);
    let key = listed
        .iter()
        .find(|setting| setting.name == "API key")
        .expect("the key is listed");
    assert_eq!(key.value, "set");
}

/// A client that mentioned nothing gets the recommendation, not a zero.
///
/// A7 and D43: a caller who said nothing has not asked for a setting's lowest
/// value, and MCF never changes a value under somebody who set one.
#[test]
fn an_unmentioned_setting_keeps_its_recommendation() {
    let recommended = on_a_card();
    let nothing: [(&str, Value); 0] = [];
    let read = Hosting::from_value(&Value::map(nothing), &recommended);
    assert_eq!(read, recommended);

    // And one that was mentioned is taken as said, including a zero.
    let asked = Value::map([("gpu_layers", Value::Integer(0))]);
    let read = Hosting::from_value(&asked, &recommended);
    assert_eq!(read.gpu_layers, 0, "an explicit zero was overridden");
    assert_eq!(
        read.context, recommended.context,
        "an unmentioned field moved"
    );
}

/// A hosted model is reachable on this computer and nowhere else.
///
/// §3.7: a model bound to every interface is a model on the network, and that
/// is a decision somebody makes rather than one MCF makes for them.
#[test]
fn a_hosted_model_is_on_this_computer_only() {
    let recommended = on_a_card();
    assert_eq!(recommended.port, DEFAULT_PORT);
    assert!(
        recommended
            .address()
            .starts_with(&format!("http://{LOOPBACK}:"))
    );
    let arguments = recommended.arguments("/models/a.gguf", LOOPBACK);
    let at = arguments
        .iter()
        .position(|held| held == "--host")
        .expect("the engine is told what to bind");
    assert_eq!(arguments.get(at + 1).map(String::as_str), Some(LOOPBACK));
}
