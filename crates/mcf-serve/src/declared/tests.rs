use super::{Declared, Rope, Scaling, Started};

#[test]
fn nothing_is_asked_for_by_default() {
    let nothing = Started::default();
    assert!(!nothing.asks_anything());
    assert!(nothing.arguments().is_empty());
    assert_eq!(nothing.said(), "nothing beyond the plain load");
}

#[test]
fn the_switches_are_the_engines_own() {
    let asked = Started {
        effort: None,
        temperature: None,
        top_p: None,
        top_k: None,
        draft_head: true,
        rope: Some(Scaling::Yarn),
        factor: Some(4),
        window: None,
        ..Started::default()
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

#[test]
fn what_was_asked_survives_the_wire() {
    for asked in [
        Started::default(),
        Started {
            draft_head: true,
            rope: None,
            factor: None,
            window: None,
            ..Started::default()
        },
        Started {
            draft_head: false,
            rope: Some(Scaling::Off),
            factor: None,
            window: None,
            ..Started::default()
        },
        Started {
            draft_head: true,
            rope: Some(Scaling::Linear),
            factor: Some(8),
            window: None,
            ..Started::default()
        },
        Started {
            draft_head: false,
            rope: None,
            factor: None,
            window: Some(8192),
            ..Started::default()
        },
    ] {
        assert_eq!(Started::from_value(&asked.to_value()), asked);
    }
}

#[test]
fn silence_on_the_wire_asks_for_nothing() {
    assert_eq!(
        Started::from_value(&mcf_record::json::Value::Null),
        Started::default()
    );
}

#[test]
fn a_draft_head_no_file_declares_is_refused() {
    let asked = Started {
        effort: None,
        temperature: None,
        top_p: None,
        top_k: None,
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

#[test]
fn a_scaling_that_stretches_nothing_is_refused() {
    let asked = Started {
        effort: None,
        temperature: None,
        top_p: None,
        top_k: None,
        rope: Some(Scaling::Yarn),
        ..Started::default()
    };
    assert!(asked.against(&Declared::default()).is_err());
    let declares_one = Declared {
        rope: Some(Rope {
            kind: "yarn".to_owned(),
            factor: Some("32".to_owned()),
            trained: Some(4096),
        }),
        ..Declared::default()
    };
    assert!(asked.against(&declares_one).is_ok());
    let with_factor = Started {
        factor: Some(4),
        ..asked
    };
    assert!(with_factor.against(&Declared::default()).is_ok());
    let off = Started {
        rope: Some(Scaling::Off),
        ..Started::default()
    };
    assert!(off.against(&declares_one).is_ok());
}

#[test]
fn a_factor_with_no_scaling_is_refused() {
    let asked = Started {
        effort: None,
        temperature: None,
        top_p: None,
        top_k: None,
        factor: Some(4),
        ..Started::default()
    };
    let refusal = asked
        .against(&Declared::default())
        .expect_err("a factor with no scaling");
    assert_eq!(refusal.attribution(), mcf_core::failure::Attribution::User);
}

#[test]
fn what_was_left_in_the_file_is_said() {
    let declares_one = Declared {
        draft_head: Some(1),
        ..Declared::default()
    };
    let said = declares_one
        .not_started(&Started::default())
        .expect("a draft head nobody asked for");
    assert!(said.contains("a draft head of 1 layer,"), "{said}");
    assert_eq!(
        declares_one.not_started(&Started {
            draft_head: true,
            ..Started::default()
        }),
        None
    );
    assert_eq!(Declared::default().not_started(&Started::default()), None);
    let declares_two = Declared {
        draft_head: Some(2),
        ..Declared::default()
    };
    let said = declares_two
        .not_started(&Started::default())
        .expect("two layers");
    assert!(said.contains("2 layers"), "{said}");
}

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

#[test]
fn only_the_engines_own_words_are_scalings() {
    assert_eq!(Scaling::from_word("yarn"), Some(Scaling::Yarn));
    assert_eq!(Scaling::from_word("linear"), Some(Scaling::Linear));
    assert_eq!(Scaling::from_word("none"), Some(Scaling::Off));
    assert_eq!(Scaling::from_word("Yarn"), None);
    assert_eq!(Scaling::from_word("stretched"), None);
}

#[test]
fn the_tuning_switches_reach_the_engine_in_the_order_it_reads_them() {
    let asked = Started {
        effort: None,
        temperature: None,
        top_p: None,
        top_k: None,
        draft_head: true,
        drafted: Some(2),
        rope: Some(Scaling::Yarn),
        factor: Some(2),
        trained: Some(262_144),
        lift: Some(524_288),
        architecture: Some("llama".to_owned()),
        thinking: Some(4096),
        window: Some(524_288),
    };
    assert_eq!(
        asked.arguments(),
        vec![
            "--spec-type".to_owned(),
            "draft-mtp".to_owned(),
            "--spec-draft-n-max".to_owned(),
            "2".to_owned(),
            "--rope-scaling".to_owned(),
            "yarn".to_owned(),
            "--rope-scale".to_owned(),
            "2".to_owned(),
            "--yarn-orig-ctx".to_owned(),
            "262144".to_owned(),
            "--override-kv".to_owned(),
            "llama.context_length=int:524288".to_owned(),
            "--reasoning-budget".to_owned(),
            "4096".to_owned(),
        ]
    );
}

#[test]
fn lifting_the_ceiling_needs_the_architecture_the_header_named() {
    let without = Started {
        lift: Some(524_288),
        ..Started::default()
    };
    assert!(
        without.arguments().is_empty(),
        "a ceiling cannot be lifted without the architecture whose key carries it"
    );
    let with = Started {
        lift: Some(524_288),
        architecture: Some("gpt-oss".to_owned()),
        ..Started::default()
    };
    assert_eq!(
        with.arguments(),
        vec![
            "--override-kv".to_owned(),
            "gpt-oss.context_length=int:524288".to_owned(),
        ]
    );
}

/// A budget of nothing is a setting and not an absence — but it does not reach the engine
/// as a nought, because the engine reads a nought as no budget at all and lets the model
/// think until it is done. Measured on a model whose template opens its own thinking
/// section: two hundred and seventy three characters of thinking asked for at nought, none
/// at one. This test asserted the nought for as long as nobody had tried it.
#[test]
fn a_thinking_budget_of_zero_is_a_setting_not_an_absence() {
    let cut = Started {
        thinking: Some(0),
        ..Started::default()
    };
    assert!(cut.asks_anything());
    assert_eq!(
        cut.arguments(),
        vec!["--reasoning-budget".to_owned(), "1".to_owned()],
        "the smallest budget there is, which is the one that means nothing"
    );
    assert!(cut.said().contains("cut off at once"), "{}", cut.said());
}

/// Thinking off is said to the engine both ways it can be said, because which of them
/// lands is a property of the template and the arguments are built without one in hand.
/// Measured on this machine: a template reading a thinking switch stopped opening its
/// section under `--reasoning off`, and gpt-oss — which reads no switch — ignored the flag
/// and went on asking for medium reasoning, so the budget is what stops that one.
#[test]
fn thinking_off_is_said_the_two_ways_a_template_might_hear_it() {
    let off = Started {
        effort: Some(crate::thinking::OFF.to_owned()),
        ..Started::default()
    };
    assert_eq!(
        off.arguments(),
        vec![
            "--reasoning".to_owned(),
            "off".to_owned(),
            "--reasoning-budget".to_owned(),
            "1".to_owned(),
        ],
    );
    assert!(
        !off.arguments()
            .iter()
            .any(|held| held == "--reasoning-effort"),
        "off is MCF's word and no template knows it, so it is never handed over as a level"
    );
}

/// A budget the person asked for is the budget they get: MCF's own way of saying off does
/// not overrule it.
#[test]
fn a_budget_that_was_asked_for_is_not_replaced_by_the_one_that_means_off() {
    let off = Started {
        effort: Some(crate::thinking::OFF.to_owned()),
        thinking: Some(64),
        ..Started::default()
    };
    let said = off.arguments();
    assert_eq!(
        said.iter()
            .filter(|held| *held == "--reasoning-budget")
            .count(),
        1,
        "one budget, not two: {said:?}"
    );
    assert!(said.contains(&"64".to_owned()), "{said:?}");
}

/// A named level still reaches the engine as a level.
#[test]
fn a_level_is_handed_over_as_a_level() {
    let asked = Started {
        effort: Some("high".to_owned()),
        ..Started::default()
    };
    assert_eq!(
        asked.arguments(),
        vec!["--reasoning-effort".to_owned(), "high".to_owned()]
    );
}

#[test]
fn every_new_switch_survives_the_wire() {
    let asked = Started {
        effort: None,
        temperature: None,
        top_p: None,
        top_k: None,
        draft_head: false,
        drafted: Some(5),
        rope: None,
        factor: None,
        trained: Some(131_072),
        lift: Some(1_048_576),
        architecture: Some("nemotron_h_moe".to_owned()),
        thinking: Some(6144),
        window: Some(1_048_576),
    };
    assert_eq!(Started::from_value(&asked.to_value()), asked);
}
