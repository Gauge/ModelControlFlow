use super::{RUNTIME_OVERHEAD, Requirement, Shape, USABLE_PER_CENT, Verdict, assess, plan};
use mcf_core::failure::Category;
use mcf_core::measurement::Bytes;

const GIB: u64 = 1024 * 1024 * 1024;

fn usable(available: u64) -> u64 {
    available
        .checked_mul(USABLE_PER_CENT)
        .and_then(|scaled| scaled.checked_div(100))
        .unwrap_or(0)
}

fn shape() -> Shape {
    Shape {
        blocks: 32,
        key_value_heads: 8,
        per_head: 256,
        cache: mcf_core::configuration::CacheType::default(),
    }
}

fn variant(name: &str, weights: u64) -> Requirement {
    Requirement {
        name: name.to_owned(),
        weights: Bytes(weights),
        shape: shape(),
    }
}

#[test]
fn a_token_costs_what_the_shape_says() {
    assert_eq!(shape().bytes_per_token(), Some(131_072));
    assert_eq!(131_072 * 8192, GIB);
}

#[test]
fn what_a_variant_needs_is_the_three_things_added() {
    let variant = variant("q4_k_m.gguf", 5 * GIB);
    let needs = variant.at_context(8192).expect("it adds up");
    assert_eq!(needs, Bytes(5 * GIB + GIB + RUNTIME_OVERHEAD.0));
}

#[test]
fn a_variant_that_fits_says_what_is_left() {
    let variant = variant("q4_k_m.gguf", 5 * GIB);
    let verdict = assess(&variant, 8192, Bytes(16 * GIB)).expect("assessable");
    let left = usable(16 * GIB);
    match verdict {
        Verdict::Fits { needs, headroom } => {
            assert_eq!(needs, Bytes(5 * GIB + GIB + RUNTIME_OVERHEAD.0));
            assert_eq!(headroom, Bytes(left - needs.0));
        }
        other => panic!("expected it to fit: {other:?}"),
    }
    assert!(verdict_of(&variant, 8192, 16 * GIB).is_runnable());
}

fn verdict_of(variant: &Requirement, context: u64, available: u64) -> Verdict {
    assess(variant, context, Bytes(available)).expect("assessable")
}

#[test]
fn a_context_that_does_not_fit_is_answered_with_one_that_does() {
    let variant = variant("q8_0.gguf", 12 * GIB);
    match verdict_of(&variant, 32_768, 16 * GIB) {
        Verdict::FitsWithoutContextHeadroom {
            longest_context, ..
        } => {
            let floor = 12 * GIB + RUNTIME_OVERHEAD.0;
            let room = usable(16 * GIB) - floor;
            assert_eq!(
                longest_context,
                room.checked_div(131_072).unwrap_or(0),
                "the answer is how many tokens the remaining room holds"
            );
            assert!(
                longest_context > 0 && longest_context < 32_768,
                "a shorter context that fits is the useful answer: {longest_context}"
            );
        }
        other => panic!("expected a shorter context: {other:?}"),
    }
}

#[test]
fn a_variant_that_does_not_fit_says_by_how_much() {
    let variant = variant("bf16.gguf", 131 * GIB);
    match verdict_of(&variant, 8192, 24 * GIB) {
        Verdict::DoesNotFit { needs, short_by } => {
            assert_eq!(needs, Bytes(131 * GIB + RUNTIME_OVERHEAD.0));
            assert_eq!(short_by, Bytes(needs.0 - usable(24 * GIB)));
        }
        other => panic!("expected a refusal: {other:?}"),
    }
    assert!(!verdict_of(&variant, 8192, 24 * GIB).is_runnable());
}

#[test]
fn a_plan_classifies_every_variant_a_repository_publishes() {
    let sizes = [
        ("iq1_s.gguf", 2 * GIB),
        ("q2_k.gguf", 3 * GIB),
        ("q4_k_m.gguf", 5 * GIB),
        ("q5_k_m.gguf", 6 * GIB),
        ("q6_k.gguf", 7 * GIB),
        ("q8_0.gguf", 9 * GIB),
        ("f16.gguf", 16 * GIB),
        ("bf16.gguf", 16 * GIB),
    ];
    let requirements: Vec<Requirement> = sizes
        .iter()
        .map(|(name, weights)| variant(name, *weights))
        .collect();

    let planned = plan(&requirements, 8192, Bytes(12 * GIB)).expect("a plan");
    assert_eq!(planned.len(), sizes.len(), "every variant is in the plan");

    let fits = planned
        .iter()
        .filter(|(_, verdict)| matches!(verdict, Verdict::Fits { .. }))
        .count();
    let shorter = planned
        .iter()
        .filter(|(_, verdict)| matches!(verdict, Verdict::FitsWithoutContextHeadroom { .. }))
        .count();
    let refused = planned
        .iter()
        .filter(|(_, verdict)| matches!(verdict, Verdict::DoesNotFit { .. }))
        .count();

    assert_eq!(
        fits + shorter + refused,
        sizes.len(),
        "every one is classified"
    );
    assert!(fits > 0 && refused > 0, "{planned:?}");
    assert!(matches!(
        planned.first().map(|(_, verdict)| verdict),
        Some(Verdict::Fits { .. })
    ));
}

#[test]
fn a_plan_does_not_fill_the_machine() {
    let exact = Requirement {
        name: "exact".to_owned(),
        weights: Bytes(usable(10 * GIB) - RUNTIME_OVERHEAD.0),
        shape: Shape {
            blocks: 1,
            key_value_heads: 1,
            per_head: 1,
            cache: mcf_core::configuration::CacheType::default(),
        },
    };
    assert!(matches!(
        verdict_of(&exact, 0, 10 * GIB),
        Verdict::Fits { .. }
    ));

    let over = Requirement {
        weights: Bytes(exact.weights.0 + 1),
        ..exact
    };
    assert!(matches!(
        verdict_of(&over, 0, 10 * GIB),
        Verdict::DoesNotFit { .. }
    ));
}

#[test]
fn arithmetic_that_overflows_refuses_to_plan() {
    let absurd = Requirement {
        name: "hostile.gguf".to_owned(),
        weights: Bytes(u64::MAX),
        shape: Shape {
            blocks: u64::MAX,
            key_value_heads: u64::MAX,
            per_head: u64::MAX,
            cache: mcf_core::configuration::CacheType::default(),
        },
    };
    let failure = assess(&absurd, 8192, Bytes(16 * GIB)).expect_err("it cannot be planned");
    assert_eq!(failure.category(), Category::ArtifactFormatMalformed);
    assert!(
        failure
            .context()
            .iter()
            .any(|entry| entry.value.contains("hostile.gguf")),
        "the refusal does not name which variant"
    );

    let requirements = vec![variant("fine.gguf", GIB), absurd];
    assert!(plan(&requirements, 8192, Bytes(16 * GIB)).is_err());
}

#[test]
fn the_cache_is_sized_by_the_key_value_heads() {
    let grouped = Shape {
        key_value_heads: 8,
        ..shape()
    };
    let ungrouped = Shape {
        key_value_heads: 32,
        ..shape()
    };
    assert_eq!(
        ungrouped.bytes_per_token().unwrap_or(0),
        grouped.bytes_per_token().unwrap_or(0).saturating_mul(4),
        "a four-fold grouping is a four-fold difference in the cache"
    );
}

#[test]
fn a_shape_is_read_from_a_configuration() {
    let stated = mcf_record::json::parse(
        r#"{"num_hidden_layers":28,"num_key_value_heads":8,"head_dim":128,"hidden_size":1024,
            "num_attention_heads":16}"#,
    )
    .expect("JSON");
    assert_eq!(
        Shape::from_configuration(&stated, mcf_core::configuration::CacheType::default()),
        Some(Shape {
            blocks: 28,
            key_value_heads: 8,
            per_head: 256,
            cache: mcf_core::configuration::CacheType::default(),
        })
    );

    let derived = mcf_record::json::parse(
        r#"{"num_hidden_layers":32,"num_key_value_heads":8,"hidden_size":4096,
            "num_attention_heads":32}"#,
    )
    .expect("JSON");
    assert_eq!(
        Shape::from_configuration(&derived, mcf_core::configuration::CacheType::default())
            .map(|shape| shape.per_head),
        Some(256)
    );
}

#[test]
fn a_nested_configuration_is_read_where_the_model_puts_it() {
    let nested = mcf_record::json::parse(
        r#"{"model_type":"a-multimodal-model","vision_config":{"num_hidden_layers":27},
            "text_config":{"num_hidden_layers":64,"num_key_value_heads":4,"head_dim":256,
            "hidden_size":5120,"num_attention_heads":24,"rms_norm_eps":1e-06}}"#,
    )
    .expect("JSON");
    assert_eq!(
        Shape::from_configuration(&nested, mcf_core::configuration::CacheType::default()),
        Some(Shape {
            blocks: 64,
            key_value_heads: 4,
            per_head: 512,
            cache: mcf_core::configuration::CacheType::default(),
        }),
        "the transformer's own fields were not read"
    );
}

#[test]
fn only_the_blocks_that_cache_are_counted() {
    let hybrid = mcf_record::json::parse(
        r#"{"num_hidden_layers":8,"num_key_value_heads":4,"head_dim":128,
            "layer_types":["linear_attention","linear_attention","linear_attention",
            "full_attention","linear_attention","linear_attention","linear_attention",
            "full_attention"]}"#,
    )
    .expect("JSON");
    assert_eq!(
        Shape::from_configuration(&hybrid, mcf_core::configuration::CacheType::default())
            .map(|shape| shape.blocks),
        Some(2),
        "every block was counted, and only two of them cache"
    );

    let plain = mcf_record::json::parse(
        r#"{"num_hidden_layers":8,"num_key_value_heads":4,"head_dim":128}"#,
    )
    .expect("JSON");
    assert_eq!(
        Shape::from_configuration(&plain, mcf_core::configuration::CacheType::default())
            .map(|shape| shape.blocks),
        Some(8)
    );

    let cacheless = mcf_record::json::parse(
        r#"{"num_hidden_layers":4,"num_key_value_heads":4,"head_dim":128,
            "layer_types":["linear_attention","linear_attention","linear_attention",
            "linear_attention"]}"#,
    )
    .expect("JSON");
    assert_eq!(
        Shape::from_configuration(&cacheless, mcf_core::configuration::CacheType::default()),
        None
    );
}

#[test]
fn a_configuration_that_does_not_say_produces_no_shape() {
    for incomplete in [
        r#"{"num_key_value_heads":8,"head_dim":128}"#,
        r#"{"num_hidden_layers":28,"head_dim":128}"#,
        r#"{"num_hidden_layers":28,"num_key_value_heads":8}"#,
        r#"{"num_hidden_layers":28,"num_key_value_heads":8,"hidden_size":1024}"#,
        r#"{"num_hidden_layers":0,"num_key_value_heads":8,"head_dim":128}"#,
        r#"{"num_hidden_layers":28,"num_key_value_heads":8,"hidden_size":1024,"num_attention_heads":0}"#,
        r#"{"num_hidden_layers":"twenty-eight","num_key_value_heads":8,"head_dim":128}"#,
        "{}",
    ] {
        let value = mcf_record::json::parse(incomplete).expect("JSON");
        assert_eq!(
            Shape::from_configuration(&value, mcf_core::configuration::CacheType::default()),
            None,
            "a shape was invented from {incomplete}"
        );
    }
}

#[test]
fn the_cache_is_sized_by_the_grouped_heads_not_the_query_heads() {
    let value = mcf_record::json::parse(
        r#"{"num_hidden_layers":32,"num_key_value_heads":8,"num_attention_heads":32,"head_dim":128}"#,
    )
    .expect("JSON");
    let shape = Shape::from_configuration(&value, mcf_core::configuration::CacheType::default())
        .expect("a shape");
    let grouped = shape.bytes_per_token().expect("it multiplies out");

    let ungrouped = Shape {
        key_value_heads: 32,
        ..shape
    }
    .bytes_per_token()
    .expect("it multiplies out");
    assert_eq!(
        ungrouped,
        grouped.saturating_mul(4),
        "the two readings differ by the grouping factor, which is the point"
    );
}

#[test]
fn a_plan_is_recorded_whichever_way_it_came_out() {
    use mcf_record::json::Value;

    let verdicts = vec![
        (
            "small.gguf".to_owned(),
            Verdict::Fits {
                needs: Bytes(1_000),
                headroom: Bytes(500),
            },
        ),
        (
            "medium.gguf".to_owned(),
            Verdict::FitsWithoutContextHeadroom {
                needs: Bytes(1_400),
                longest_context: 512,
            },
        ),
        (
            "enormous.gguf".to_owned(),
            Verdict::DoesNotFit {
                needs: Bytes(140_000_000_000),
                short_by: Bytes(120_000_000_000),
            },
        ),
    ];
    let recorded = super::planned(&verdicts, 4096, Bytes(1_500));

    assert_eq!(
        recorded.get("context").and_then(Value::as_integer),
        Some(4096),
        "*this fits* means nothing without the length it fits at (A6)"
    );
    let variants = recorded
        .get("variants")
        .and_then(Value::as_list)
        .expect("the variants are a list");
    assert_eq!(
        variants.len(),
        3,
        "every variant is kept, not only the good news"
    );

    let outcomes: Vec<&str> = variants
        .iter()
        .filter_map(|held| held.get("outcome").and_then(Value::as_text))
        .collect();
    assert_eq!(
        outcomes,
        ["fits", "fits_at_a_shorter_context", "does_not_fit"],
        "each outcome names itself"
    );

    let refused = variants.last().expect("the last variant");
    assert_eq!(
        refused.get("short_by_bytes").and_then(Value::as_integer),
        Some(120_000_000_000),
        "the refusal carries by how much, which is what makes it actionable (§6.3)"
    );
    assert_eq!(
        refused.get("needs_bytes").and_then(Value::as_integer),
        Some(140_000_000_000)
    );
}

#[test]
fn every_outcome_has_a_name_and_none_of_them_is_a_failure() {
    for verdict in [
        Verdict::Fits {
            needs: Bytes(1),
            headroom: Bytes(1),
        },
        Verdict::FitsWithoutContextHeadroom {
            needs: Bytes(1),
            longest_context: 1,
        },
        Verdict::DoesNotFit {
            needs: Bytes(1),
            short_by: Bytes(1),
        },
    ] {
        let name = verdict.outcome();
        assert!(!name.is_empty());
        assert!(
            !name.contains("fail") && !name.contains("error"),
            "all three are answers to *will this run here*, which A9 makes results: {name}"
        );
    }
}
