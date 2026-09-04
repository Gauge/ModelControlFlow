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

/// The conditions holding and the answer holding are different questions, and
/// this is the one that needs no trials.
#[test]
fn conditions_that_still_hold_say_so() {
    use super::{Since, since};
    let held = an_addressing();
    assert_eq!(
        since(&held, "engine: provisioned", &an_addressing().build),
        Since::ConditionsHold,
        "the engine's head and the build's version are what is compared"
    );
}

/// A different engine is a moved condition, named, with both sides kept.
///
/// It is deliberately *not* a divergence. F39 measured two engines returning
/// the same verdict on this exact question, so *the evidence was gathered
/// elsewhere* is what MCF knows and *the answer changed* is what it does not
/// (A21).
#[test]
fn a_different_engine_is_a_moved_condition_not_a_disagreement() {
    use super::{Since, since};
    let held = an_addressing();
    let moved = since(&held, "engine: stand-in", &an_addressing().build);
    let Since::ConditionsMoved(moved) = moved else {
        panic!("the engine moved and was not noticed");
    };
    assert_eq!(moved.len(), 1);
    assert_eq!(moved[0].what, "engine");
    assert!(moved[0].was.contains("provisioned"), "{:?}", moved[0]);
    assert!(moved[0].now.contains("stand-in"), "{:?}", moved[0]);
}

/// A different build is a moved condition too, and both can move at once.
#[test]
fn a_different_build_is_a_moved_condition() {
    use super::{Since, since};
    let held = an_addressing();
    let Since::ConditionsMoved(moved) =
        since(&held, "engine: stand-in", "MCF 0.2.0-m1 (rustc 1.99)")
    else {
        panic!("two conditions moved and neither was noticed");
    };
    assert_eq!(moved.len(), 2, "{moved:?}");
    assert!(moved.iter().any(|one| one.what == "build"));
}

/// Two spellings of one engine must not read as two engines.
///
/// The comparison is between a name a probe wrote down and a name another
/// command computes, and when those were *different kinds of name* — "the
/// engine I asked for" against "the engine that resolved" — `mcf explain`
/// reported a moved condition for an engine that had not moved (F45). The
/// spelling is the resolved one on both sides; this pins the shape so that a
/// later change to one has to change the other.
#[test]
fn one_engine_under_two_names_has_not_moved() {
    use super::{Since, since};
    let mut held = an_addressing();
    held.conditions = "provisioned llama.cpp @925e1179947e, through the daemon".to_owned();
    assert_eq!(
        since(
            &held,
            "provisioned llama.cpp @925e1179947e",
            &held.build.clone()
        ),
        Since::ConditionsHold,
        "the same engine, written the same way, has not moved"
    );
}

/// A different build of the same engine *has* moved, which is the whole reason
/// the commit is part of the name.
#[test]
fn another_build_of_the_same_engine_has_moved() {
    use super::{Since, since};
    let mut held = an_addressing();
    held.conditions = "provisioned llama.cpp @925e1179947e, through the daemon".to_owned();
    let Since::ConditionsMoved(moved) = since(
        &held,
        "provisioned llama.cpp @ffffffffffff",
        &held.build.clone(),
    ) else {
        panic!("a different commit of the engine was not noticed");
    };
    assert_eq!(moved.len(), 1);
    assert_eq!(moved[0].what, "engine");
}

/// A budget keeps what share of its turn was thought, and says so — or says
/// nothing where the probe could not tell.
///
/// The number alone reads as the answer's size; on a model that thinks it is
/// the thought's (F172). A budget read back without the share would carry a
/// provenance that no longer says what the number was set against.
#[test]
fn a_budget_says_what_it_was_set_against() {
    let measured = super::Budget {
        tokens: 247,
        before: Some(205),
        probe: "stop-conditions".to_owned(),
        at: "2026-09-04T21:26:00Z".to_owned(),
        build: "0.1.0-m0".to_owned(),
        conditions: "engine: provisioned".to_owned(),
    };
    let read_back =
        super::Budget::from_value(&measured.to_value()).expect("it survives the round trip");
    assert_eq!(read_back, measured);
    let said = measured.provenance();
    assert!(
        said.starts_with("247 tokens, of which up to 205 were spent thinking before the answer"),
        "{said}"
    );

    let unmeasured = super::Budget {
        before: None,
        ..measured
    };
    let value = unmeasured.to_value();
    assert!(
        value.get("before").is_none(),
        "absent rather than null: nothing was measured, which is not nought"
    );
    assert_eq!(
        super::Budget::from_value(&value),
        Some(unmeasured.clone()),
        "and it is read without one"
    );
    assert!(unmeasured.provenance().starts_with("247 tokens — set by"));
}
