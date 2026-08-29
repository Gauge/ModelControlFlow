//! Fitment against numbers worked out by hand.
//!
//! Every case below states its arithmetic in the comment above it, so what is
//! being checked is the *rule* rather than a second implementation of the same
//! multiplication (A19).

use super::{RUNTIME_OVERHEAD, Requirement, Shape, USABLE_PER_CENT, Verdict, assess, plan};
use mcf_core::failure::Category;
use mcf_core::measurement::Bytes;

const GIB: u64 = 1024 * 1024 * 1024;

/// The usable share of a machine's memory, as the module computes it.
///
/// Written as a function rather than inline because the workspace denies
/// integer division: a silently truncated quotient is a wrong number, and a
/// test's arithmetic is not exempt from that.
fn usable(available: u64) -> u64 {
    available
        .checked_mul(USABLE_PER_CENT)
        .and_then(|scaled| scaled.checked_div(100))
        .unwrap_or(0)
}

/// A shape like an 8-billion-parameter llama: 32 blocks, 8 key/value heads of
/// 128, cached at half precision.
fn shape() -> Shape {
    Shape {
        blocks: 32,
        key_value_heads: 8,
        head_dimension: 128,
        bytes_per_element: 2,
    }
}

fn variant(name: &str, weights: u64) -> Requirement {
    Requirement {
        name: name.to_owned(),
        weights: Bytes(weights),
        shape: shape(),
    }
}

/// 2 × 32 blocks × 8 heads × 128 wide × 2 bytes = 131 072 bytes a token, which
/// is 128 KiB — the figure anyone who has sized a cache will recognise.
#[test]
fn a_token_costs_what_the_shape_says() {
    assert_eq!(shape().bytes_per_token(), Some(131_072));
    // And at 8192 tokens that is exactly 1 GiB.
    assert_eq!(131_072 * 8192, GIB);
}

/// Weights plus cache plus the runtime's overhead, added up.
#[test]
fn what_a_variant_needs_is_the_three_things_added() {
    let variant = variant("q4_k_m.gguf", 5 * GIB);
    let needs = variant.at_context(8192).expect("it adds up");
    assert_eq!(needs, Bytes(5 * GIB + GIB + RUNTIME_OVERHEAD.0));
}

/// A machine with room says so, and says how much is left.
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

/// The weights fit, the context does not, and the answer is the longest context
/// that would — which is a configuration an operator can actually take.
#[test]
fn a_context_that_does_not_fit_is_answered_with_one_that_does() {
    // 12 GiB of weights on a 16 GiB machine: usable is 14.4 GiB, the floor is
    // 12.5 GiB, so about 1.9 GiB is left for a cache costing 128 KiB a token.
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

/// Nothing fits, and the answer says by how much — which is §6.3's *needs 131
/// GiB, you have 24*, as a number rather than a sentence.
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

/// A plan is PR3's sentence: how many fit, how many fit shorter, how many do
/// not — over every variant a repository publishes, without fetching any.
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
    // Usable is 10.8 GiB; 2 GiB of weights plus 1 GiB of cache plus 0.5 GiB of
    // overhead is 3.5 GiB, so the smallest certainly fits.
    assert!(matches!(
        planned.first().map(|(_, verdict)| verdict),
        Some(Verdict::Fits { .. })
    ));
}

/// The plan leaves the machine some memory. A plan that filled it exactly is a
/// plan that was wrong.
#[test]
fn a_plan_does_not_fill_the_machine() {
    // Exactly the usable share, which fits; one byte more, which does not.
    let exact = Requirement {
        name: "exact".to_owned(),
        weights: Bytes(usable(10 * GIB) - RUNTIME_OVERHEAD.0),
        shape: Shape {
            blocks: 1,
            key_value_heads: 1,
            head_dimension: 1,
            bytes_per_element: 2,
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

/// A shape from a hostile or broken file is a refusal to plan rather than a
/// plan built on a wrapped number (§3.7).
#[test]
fn arithmetic_that_overflows_refuses_to_plan() {
    let absurd = Requirement {
        name: "hostile.gguf".to_owned(),
        weights: Bytes(u64::MAX),
        shape: Shape {
            blocks: u64::MAX,
            key_value_heads: u64::MAX,
            head_dimension: u64::MAX,
            bytes_per_element: 2,
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

    // And one bad variant refuses the whole plan rather than being dropped from
    // it: a plan missing a row nobody mentioned is worse than no plan (A1).
    let requirements = vec![variant("fine.gguf", GIB), absurd];
    assert!(plan(&requirements, 8192, Bytes(16 * GIB)).is_err());
}

/// Grouped-query attention is the general case, and using the query count
/// instead of the key/value count overstates a cache by the grouping factor —
/// which would refuse models that fit.
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

/// A shape is read from the model's own configuration, in both the shapes
/// configurations are written in.
#[test]
fn a_shape_is_read_from_a_configuration() {
    let stated = mcf_record::json::parse(
        r#"{"num_hidden_layers":28,"num_key_value_heads":8,"head_dim":128,"hidden_size":1024,
            "num_attention_heads":16}"#,
    )
    .expect("JSON");
    assert_eq!(
        Shape::from_configuration(&stated, 2),
        Some(Shape {
            blocks: 28,
            key_value_heads: 8,
            head_dimension: 128,
            bytes_per_element: 2,
        })
    );

    // An older configuration states no head width, and it is the same number
    // written another way.
    let derived = mcf_record::json::parse(
        r#"{"num_hidden_layers":32,"num_key_value_heads":8,"hidden_size":4096,
            "num_attention_heads":32}"#,
    )
    .expect("JSON");
    assert_eq!(
        Shape::from_configuration(&derived, 2).map(|shape| shape.head_dimension),
        Some(128)
    );
}

/// A multimodal repository publishes one configuration describing several
/// models, and the transformer's fields are under `text_config`.
///
/// The reference model is exactly this shape, and before MCF looked there it
/// could not plan for it at all ([findings.md](../../../doc/findings.md) F16).
#[test]
fn a_nested_configuration_is_read_where_the_model_puts_it() {
    let nested = mcf_record::json::parse(
        r#"{"model_type":"a-multimodal-model","vision_config":{"num_hidden_layers":27},
            "text_config":{"num_hidden_layers":64,"num_key_value_heads":4,"head_dim":256,
            "hidden_size":5120,"num_attention_heads":24,"rms_norm_eps":1e-06}}"#,
    )
    .expect("JSON");
    assert_eq!(
        Shape::from_configuration(&nested, 2),
        Some(Shape {
            blocks: 64,
            key_value_heads: 4,
            head_dimension: 256,
            bytes_per_element: 2,
        }),
        "the transformer's own fields were not read"
    );
}

/// Only the blocks that cache are counted.
///
/// A model whose configuration lists its layer types caches in the
/// full-attention ones and not in the linear ones. Counting every block
/// overstates the cache by the ratio between them — fourfold on the reference
/// model, which lists sixty-four layers of which sixteen are full attention
/// (F16) — and an operator would be told a variant does not fit that does.
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
        Shape::from_configuration(&hybrid, 2).map(|shape| shape.blocks),
        Some(2),
        "every block was counted, and only two of them cache"
    );

    // Where nothing lists the layers, every block caches — which is what a
    // transformer without a hybrid attention scheme does.
    let plain = mcf_record::json::parse(
        r#"{"num_hidden_layers":8,"num_key_value_heads":4,"head_dim":128}"#,
    )
    .expect("JSON");
    assert_eq!(
        Shape::from_configuration(&plain, 2).map(|shape| shape.blocks),
        Some(8)
    );

    // A configuration listing layers of which none caches is a shape MCF does
    // not understand, and a plan claiming a variant costs no memory would be
    // worse than no plan (A7).
    let cacheless = mcf_record::json::parse(
        r#"{"num_hidden_layers":4,"num_key_value_heads":4,"head_dim":128,
            "layer_types":["linear_attention","linear_attention","linear_attention",
            "linear_attention"]}"#,
    )
    .expect("JSON");
    assert_eq!(Shape::from_configuration(&cacheless, 2), None);
}

/// A configuration that does not say is not guessed at: the grouping factor is
/// exactly what a guess gets wrong, and an operator would be told a variant
/// does not fit that does (A7).
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
            Shape::from_configuration(&value, 2),
            None,
            "a shape was invented from {incomplete}"
        );
    }
}

/// The grouping factor is read from the key/value heads and not from the
/// attention heads, which is the mistake that overstates a cache fourfold.
#[test]
fn the_cache_is_sized_by_the_grouped_heads_not_the_query_heads() {
    let value = mcf_record::json::parse(
        r#"{"num_hidden_layers":32,"num_key_value_heads":8,"num_attention_heads":32,"head_dim":128}"#,
    )
    .expect("JSON");
    let shape = Shape::from_configuration(&value, 2).expect("a shape");
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

/// **A9's other half in the record.** *Does not fit here* is written as an
/// outcome, with the numbers that produced it — not as a failure and not as an
/// absence (B-086, §6.3).
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

/// The outcome names are stable for life, because a record written today is
/// read by a build that does not exist yet (C5).
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
