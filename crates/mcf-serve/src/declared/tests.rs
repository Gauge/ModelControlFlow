//! What is declared, what is asked for, and which pairs of the two are
//! refused before an engine is started.

use super::{Declared, Rope, Scaling, Started};

/// Nothing is on unless somebody turned it on.
#[test]
fn nothing_is_asked_for_by_default() {
    let nothing = Started::default();
    assert!(!nothing.asks_anything());
    assert!(nothing.arguments().is_empty());
    assert_eq!(nothing.said(), "nothing beyond the plain load");
}

/// The switches the engine is started with, in the engine's own spelling.
#[test]
fn the_switches_are_the_engines_own() {
    let asked = Started {
        draft_head: true,
        rope: Some(Scaling::Yarn),
        factor: Some(4),
        window: None,
    };
    assert_eq!(
        asked.arguments(),
        vec![
            "--spec-type".to_owned(),
            "draft-mtp".to_owned(),
            "--rope-scaling".to_owned(),
            "yarn".to_owned(),
            "--rope-scale".to_owned(),
            "4".to_owned(),
        ]
    );
    assert_eq!(asked.said(), "the draft head, rope scaling yarn ×4");
}

/// What the control plane carries comes back as what was asked.
#[test]
fn what_was_asked_survives_the_wire() {
    for asked in [
        Started::default(),
        Started {
            draft_head: true,
            rope: None,
            factor: None,
            window: None,
        },
        Started {
            draft_head: false,
            rope: Some(Scaling::Off),
            factor: None,
            window: None,
        },
        Started {
            draft_head: true,
            rope: Some(Scaling::Linear),
            factor: Some(8),
            window: None,
        },
        Started {
            draft_head: false,
            rope: None,
            factor: None,
            window: Some(8192),
        },
    ] {
        assert_eq!(Started::from_value(&asked.to_value()), asked);
    }
}

/// A client that said nothing has not asked for anything.
#[test]
fn silence_on_the_wire_asks_for_nothing() {
    assert_eq!(
        Started::from_value(&mcf_record::json::Value::Null),
        Started::default()
    );
}

/// A draft head asked of a file that has none is refused, and named as the
/// file's shortfall rather than the asker's mistake.
#[test]
fn a_draft_head_no_file_declares_is_refused() {
    let asked = Started {
        draft_head: true,
        ..Started::default()
    };
    let refusal = asked
        .against(&Declared::default())
        .expect_err("a draft head that is not there");
    assert_eq!(
        refusal.category(),
        mcf_core::failure::Category::ConfigUnsatisfiable
    );
    assert_eq!(
        refusal.attribution(),
        mcf_core::failure::Attribution::Artifact
    );
    let declares_one = Declared {
        draft_head: Some(1),
        ..Declared::default()
    };
    assert!(asked.against(&declares_one).is_ok());
}

/// A scaling with no factor anywhere would stretch by one, so it is refused
/// rather than started and reported.
#[test]
fn a_scaling_that_stretches_nothing_is_refused() {
    let asked = Started {
        rope: Some(Scaling::Yarn),
        ..Started::default()
    };
    assert!(asked.against(&Declared::default()).is_err());
    // The file's own factor is a factor.
    let declares_one = Declared {
        rope: Some(Rope {
            kind: "yarn".to_owned(),
            factor: Some("32".to_owned()),
            trained: Some(4096),
        }),
        ..Declared::default()
    };
    assert!(asked.against(&declares_one).is_ok());
    // So is one somebody asked for.
    let with_factor = Started {
        factor: Some(4),
        ..asked
    };
    assert!(with_factor.against(&Declared::default()).is_ok());
    // Turning the file's own scaling off needs no factor at all.
    let off = Started {
        rope: Some(Scaling::Off),
        ..Started::default()
    };
    assert!(off.against(&declares_one).is_ok());
}

/// A factor with nothing to apply it to is the asker's to fix.
#[test]
fn a_factor_with_no_scaling_is_refused() {
    let asked = Started {
        factor: Some(4),
        ..Started::default()
    };
    let refusal = asked
        .against(&Declared::default())
        .expect_err("a factor with no scaling");
    assert_eq!(refusal.attribution(), mcf_core::failure::Attribution::User);
}

/// The sentence the account carries for a model hosted without a feature
/// its file declares — and nothing where the feature was started, or where
/// the file declares none.
#[test]
fn what_was_left_in_the_file_is_said() {
    let declares_one = Declared {
        draft_head: Some(1),
        ..Declared::default()
    };
    let said = declares_one
        .not_started(Started::default())
        .expect("a draft head nobody asked for");
    assert!(said.contains("a draft head of 1 layer,"), "{said}");
    assert_eq!(
        declares_one.not_started(Started {
            draft_head: true,
            ..Started::default()
        }),
        None
    );
    assert_eq!(Declared::default().not_started(Started::default()), None);
    let declares_two = Declared {
        draft_head: Some(2),
        ..Declared::default()
    };
    let said = declares_two
        .not_started(Started::default())
        .expect("two layers");
    assert!(said.contains("2 layers"), "{said}");
}

/// A file MCF cannot read declares nothing, and nothing is claimed about it.
#[test]
fn a_file_that_is_not_a_model_declares_nothing() {
    let directory = std::env::temp_dir().join(format!("mcf-declared-{}", std::process::id()));
    let _made = std::fs::create_dir_all(&directory);
    let path = directory.join("not-a-model.gguf");
    let _written = std::fs::write(&path, b"nothing like a header");
    assert!(super::header(&path).is_none());
    assert_eq!(Declared::of(&path), Declared::default());
    let _gone = std::fs::remove_file(&path);
}

/// The three words the engine takes, and nothing else.
#[test]
fn only_the_engines_own_words_are_scalings() {
    assert_eq!(Scaling::from_word("yarn"), Some(Scaling::Yarn));
    assert_eq!(Scaling::from_word("linear"), Some(Scaling::Linear));
    assert_eq!(Scaling::from_word("none"), Some(Scaling::Off));
    assert_eq!(Scaling::from_word("Yarn"), None);
    assert_eq!(Scaling::from_word("stretched"), None);
}
