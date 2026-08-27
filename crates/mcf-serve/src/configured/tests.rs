//! What a derived configuration has to survive.

use super::{Addressing, forget, read, write};
use mcf_standin::tokenizer::Piece;

fn an_addressing() -> Addressing {
    Addressing {
        name: "im_start…im_end as assistant".to_owned(),
        before: vec![
            Piece::Marker("<|im_start|>".to_owned()),
            Piece::Text("user\n".to_owned()),
        ],
        after: vec![
            Piece::Marker("<|im_end|>".to_owned()),
            Piece::Text("\n".to_owned()),
        ],
        probe: "chat-template".to_owned(),
        at: "2026-08-27T00:00:00Z".to_owned(),
        build: "0.1.0-m0".to_owned(),
        conditions: "engine: provisioned".to_owned(),
    }
}

/// Somewhere to write, that goes when the test does.
fn scratch(name: &str) -> std::path::PathBuf {
    let at = std::env::temp_dir().join(format!("mcf-configured-{name}-{}", std::process::id()));
    let _cleared = std::fs::remove_dir_all(&at);
    std::fs::create_dir_all(&at).expect("a scratch directory");
    at
}

/// It goes to a file and comes back the same, markers and text told apart.
///
/// The distinction is the whole point: a marker is a control token the model
/// was trained on, and text is what a person wrote. A round trip that lost it
/// would turn one into the other, which is the defect F37 is about.
#[test]
fn an_addressing_survives_the_round_trip() {
    let home = scratch("round-trip");
    let model = std::path::Path::new("/models/a-model.gguf");
    let written = write(&home, model, &an_addressing()).expect("it was written");
    assert!(written.exists());
    assert_eq!(read(&home, model), Some(an_addressing()));
    let _gone = std::fs::remove_dir_all(&home);
}

/// A configuration that cannot say where it came from is not read at all.
///
/// B-059 asks that every derived value answer *why this value*. A file missing
/// its probe answers *because it is written here*, which is the answer a
/// default gives — and a default wearing a measurement's clothes is worse than
/// no measurement (A21, A7).
#[test]
fn a_configuration_without_its_provenance_is_not_read() {
    let home = scratch("no-provenance");
    let model = std::path::Path::new("/models/a-model.gguf");
    let path = super::path_for(&home, model);
    std::fs::create_dir_all(path.parent().expect("a parent")).expect("the directory");
    std::fs::write(
        &path,
        r#"{"model":"/models/a-model.gguf","addressing":{"name":"x","before":[],"after":[]}}"#,
    )
    .expect("the file");
    assert_eq!(
        read(&home, model),
        None,
        "a value nobody can account for is not a configuration"
    );
    let _gone = std::fs::remove_dir_all(&home);
}

/// Nothing configured reads as nothing configured, not as an error.
#[test]
fn no_configuration_is_not_a_failure() {
    let home = scratch("absent");
    let model = std::path::Path::new("/models/never-configured.gguf");
    assert_eq!(read(&home, model), None);
    assert!(
        !forget(&home, model).expect("forgetting nothing is not a failure"),
        "and it says there was nothing to forget"
    );
    let _gone = std::fs::remove_dir_all(&home);
}

/// Two models do not share a configuration.
#[test]
fn each_model_is_its_own() {
    let home = scratch("two-models");
    let one = std::path::Path::new("/models/one.gguf");
    let other = std::path::Path::new("/models/other.gguf");
    write(&home, one, &an_addressing()).expect("written");
    assert!(read(&home, one).is_some());
    assert_eq!(read(&home, other), None);
    let _gone = std::fs::remove_dir_all(&home);
}

/// What was decided can be undone, and the undoing is reported.
#[test]
fn forgetting_says_whether_there_was_anything() {
    let home = scratch("forget");
    let model = std::path::Path::new("/models/a-model.gguf");
    write(&home, model, &an_addressing()).expect("written");
    assert!(forget(&home, model).expect("it went"));
    assert_eq!(read(&home, model), None);
    assert!(!forget(&home, model).expect("nothing left"));
    let _gone = std::fs::remove_dir_all(&home);
}

/// The provenance line names the probe and the moment, because that is what a
/// reader needs to decide whether to believe it.
#[test]
fn the_provenance_says_which_probe_and_when() {
    let said = an_addressing().provenance();
    assert!(said.contains("chat-template"), "{said}");
    assert!(said.contains("2026-08-27"), "{said}");
    assert!(said.contains("provisioned"), "{said}");
}
