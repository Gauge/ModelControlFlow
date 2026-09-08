#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::collections::BTreeMap;

use super::{Active, Role, billions, label_agrees, of, role_of};
use crate::gguf::{Model, Tensor, TensorKind, Value};

fn tensor(name: &str, dimensions: &[u64], kind: TensorKind) -> Tensor {
    Tensor {
        name: name.to_owned(),
        dimensions: dimensions.to_vec(),
        kind,
        offset: 0,
    }
}

fn model(architecture: &str, declared: &[(&str, Value)], tensors: Vec<Tensor>) -> Model {
    let mut metadata = BTreeMap::new();
    metadata.insert(
        "general.architecture".to_owned(),
        Value::Text(architecture.to_owned()),
    );
    for (key, value) in declared {
        metadata.insert((*key).to_owned(), value.clone());
    }
    Model {
        version: 3,
        metadata,
        tensors,
        data_offset: 0,
        alignment: 32,
    }
}

fn dense() -> Model {
    let mut tensors = vec![
        tensor("token_embd.weight", &[64, 256], TensorKind::Q4_K),
        tensor("output.weight", &[64, 256], TensorKind::Q6_K),
        tensor("output_norm.weight", &[64], TensorKind::F32),
    ];
    for block in 0..2 {
        let named = |leaf: &str| format!("blk.{block}.{leaf}");
        tensors.extend([
            tensor(&named("attn_q.weight"), &[64, 64], TensorKind::Q4_K),
            tensor(&named("attn_k.weight"), &[64, 32], TensorKind::Q4_K),
            tensor(&named("attn_v.weight"), &[64, 32], TensorKind::Q6_K),
            tensor(&named("attn_output.weight"), &[64, 64], TensorKind::Q4_K),
            tensor(&named("attn_norm.weight"), &[64], TensorKind::F32),
            tensor(&named("ffn_up.weight"), &[64, 128], TensorKind::Q4_K),
            tensor(&named("ffn_gate.weight"), &[64, 128], TensorKind::Q4_K),
            tensor(&named("ffn_down.weight"), &[128, 64], TensorKind::Q6_K),
            tensor(&named("ffn_norm.weight"), &[64], TensorKind::F32),
        ]);
    }
    model(
        "llama",
        &[
            ("llama.block_count", Value::Integer(2)),
            ("llama.embedding_length", Value::Integer(64)),
            ("llama.attention.head_count", Value::Integer(4)),
            ("llama.attention.head_count_kv", Value::Integer(2)),
            ("llama.feed_forward_length", Value::Integer(128)),
            ("llama.vocab_size", Value::Integer(256)),
            ("general.parameter_count", Value::Integer(106_816)),
            (
                "tokenizer.ggml.tokens",
                Value::List(vec![Value::Text("a".to_owned()); 256]),
            ),
        ],
        tensors,
    )
}

#[test]
fn every_tensor_is_placed_by_its_name() {
    assert_eq!(role_of("token_embd.weight"), Role::Embedding);
    assert_eq!(role_of("output.weight"), Role::Output);
    assert_eq!(role_of("output_norm.weight"), Role::NormsAndBiases);
    assert_eq!(role_of("blk.3.attn_q.weight"), Role::Attention);
    assert_eq!(role_of("blk.3.attn_q.bias"), Role::NormsAndBiases);
    assert_eq!(role_of("blk.3.attn_k_norm.weight"), Role::NormsAndBiases);
    assert_eq!(
        role_of("blk.3.post_attention_norm.weight"),
        Role::NormsAndBiases
    );
    assert_eq!(role_of("blk.3.ffn_up.weight"), Role::FeedForward);
    assert_eq!(role_of("blk.3.ffn_up_shexp.weight"), Role::FeedForward);
    assert_eq!(role_of("blk.3.ffn_up_exps.weight"), Role::Experts);
    assert_eq!(role_of("blk.3.ffn_up_exps.bias"), Role::Experts);
    assert_eq!(role_of("blk.3.ffn_gate_inp.weight"), Role::Routing);
    assert_eq!(role_of("blk.3.attn_kv_a_mqa.weight"), Role::Attention);
    assert_eq!(role_of("rope_freqs.weight"), Role::Other);
}

#[test]
fn a_dense_model_is_counted_from_its_shapes() {
    let counted = of(&dense());
    let per_block = 64 * 64 * 2 + 64 * 32 * 2 + 3 * 64 * 128 + 2 * 64;
    assert_eq!(counted.elements, 2 * 64 * 256 + 2 * per_block + 64);
    assert_eq!(counted.blocks, 2);
    assert!(!counted.output_tied);
    assert!(counted.active.is_none());
    assert_eq!(counted.unsized_tensors, 0);
    let attention = counted
        .roles
        .iter()
        .find(|(role, _)| *role == Role::Attention)
        .map(|(_, share)| share)
        .unwrap();
    assert_eq!(attention.tensors, 8);
    assert_eq!(attention.elements, 2 * (64 * 64 * 2 + 64 * 32 * 2));
    let attention_bytes: u64 = dense()
        .tensors
        .iter()
        .filter(|held| held.name.contains("attn_") && !held.name.contains("norm"))
        .map(|held| held.bytes().unwrap())
        .sum();
    assert_eq!(attention.bytes, Some(attention_bytes));
    assert!(attention.hundredths_of_a_bit().is_some());
    let bytes: u64 = dense()
        .tensors
        .iter()
        .map(|held| held.bytes().unwrap())
        .sum();
    assert_eq!(counted.bytes, Some(bytes));
    assert_eq!(
        counted.kinds.first().map(|(kind, _)| *kind),
        Some(TensorKind::Q4_K)
    );
}

#[test]
fn the_header_is_set_against_the_directory_and_agrees() {
    let counted = of(&dense());
    let row = |what: &str| {
        counted
            .agreements
            .iter()
            .find(|held| held.what == what)
            .unwrap_or_else(|| panic!("a row for {what}"))
            .clone()
    };
    assert_eq!(row("blocks").agrees, Some(true));
    assert_eq!(row("embedding width").agrees, Some(true));
    assert_eq!(row("vocabulary, embedding rows").agrees, Some(true));
    assert_eq!(row("vocabulary, tokens listed").agrees, Some(true));
    assert_eq!(row("attention heads").agrees, Some(true));
    assert_eq!(row("key/value heads").agrees, Some(true));
    assert_eq!(row("feed-forward width").agrees, Some(true));
    assert!(counted.agreements.iter().all(|held| held.what != "experts"));
    assert_eq!(row("parameters").declared.as_deref(), Some("106816"));
    assert_eq!(row("parameters").agrees, Some(true));
}

#[test]
fn a_header_that_disagrees_with_its_own_directory_is_said_to() {
    let mut lying = dense();
    lying.metadata.insert(
        "llama.attention.head_count_kv".to_owned(),
        Value::Integer(4),
    );
    lying
        .metadata
        .insert("llama.block_count".to_owned(), Value::Integer(3));
    let counted = of(&lying);
    let disagreeing: Vec<&str> = counted
        .agreements
        .iter()
        .filter(|held| held.agrees == Some(false))
        .map(|held| held.what)
        .collect();
    assert_eq!(disagreeing, vec!["blocks", "key/value heads"]);
    let heads = counted
        .agreements
        .iter()
        .find(|held| held.what == "key/value heads")
        .unwrap();
    assert_eq!(heads.declared.as_deref(), Some("4"));
    assert_eq!(heads.observed.as_deref(), Some("2"));
}

#[test]
fn a_figure_the_header_does_not_state_is_not_a_disagreement() {
    let mut quiet = dense();
    quiet.metadata.remove("llama.vocab_size");
    let counted = of(&quiet);
    let rows = counted
        .agreements
        .iter()
        .find(|held| held.what == "vocabulary, embedding rows")
        .unwrap();
    assert_eq!(rows.declared, None);
    assert_eq!(rows.observed.as_deref(), Some("256"));
    assert_eq!(rows.agrees, None);
}

#[test]
fn a_mixture_counts_what_a_token_activates() {
    let tensors = vec![
        tensor("token_embd.weight", &[64, 256], TensorKind::Q4_K),
        tensor("output_norm.weight", &[64], TensorKind::F32),
        tensor("blk.0.attn_q.weight", &[64, 64], TensorKind::Q4_K),
        tensor("blk.0.attn_k.weight", &[64, 32], TensorKind::Q4_K),
        tensor("blk.0.attn_v.weight", &[64, 32], TensorKind::Q4_K),
        tensor("blk.0.attn_output.weight", &[64, 64], TensorKind::Q4_K),
        tensor("blk.0.ffn_gate_inp.weight", &[64, 8], TensorKind::F32),
        tensor("blk.0.ffn_up_exps.weight", &[64, 32, 8], TensorKind::Q4_K),
        tensor("blk.0.ffn_gate_exps.weight", &[64, 32, 8], TensorKind::Q4_K),
        tensor("blk.0.ffn_down_exps.weight", &[32, 64, 8], TensorKind::Q4_K),
    ];
    let mixture = model(
        "mixture",
        &[
            ("mixture.expert_count", Value::Integer(8)),
            ("mixture.expert_used_count", Value::Integer(2)),
            ("mixture.expert_feed_forward_length", Value::Integer(32)),
            ("general.size_label", Value::Text("0.0B-A0.0B".to_owned())),
        ],
        tensors,
    );
    let counted = of(&mixture);
    assert!(counted.output_tied);
    let experts = 3 * 64 * 32 * 8;
    let outside = counted.elements - experts;
    assert_eq!(
        counted.active,
        Some(Active {
            experts: 8,
            used: 2,
            elements: outside + 3 * 64 * 32 * 2,
        })
    );
    let row = |what: &str| {
        counted
            .agreements
            .iter()
            .find(|held| held.what == what)
            .unwrap_or_else(|| panic!("a row for {what}"))
            .clone()
    };
    assert_eq!(row("experts").agrees, Some(true));
    assert_eq!(row("expert feed-forward width").agrees, Some(true));
    assert_eq!(
        row("parameters").declared.as_deref(),
        Some("0.0B-A0.0B (a label)")
    );
    assert_eq!(row("parameters").agrees, Some(true));
    assert_eq!(
        row("parameters a token activates").declared.as_deref(),
        Some("A0.0B (a label)")
    );
    assert_eq!(row("parameters a token activates").agrees, Some(true));
}

#[test]
fn an_encoding_the_reader_cannot_size_leaves_the_bytes_unknown() {
    let mut odd = dense();
    odd.tensors.push(tensor(
        "blk.1.ffn_up_exps.weight",
        &[64, 32, 8],
        TensorKind::Unknown(40),
    ));
    let counted = of(&odd);
    assert_eq!(
        counted.bytes, None,
        "a total with a hole in it is not a total (A7)"
    );
    assert_eq!(counted.unsized_tensors, 1);
    assert_eq!(counted.elements, of(&dense()).elements + 64 * 32 * 8);
}

#[test]
fn a_label_agrees_to_its_own_resolution() {
    assert_eq!(label_agrees("8B", 8_250_000_000), Some(true));
    assert_eq!(label_agrees("8B", 7_500_000_000), Some(true));
    assert_eq!(label_agrees("8B", 8_999_000_000), Some(true));
    assert_eq!(label_agrees("8B", 9_000_000_000), Some(false));
    assert_eq!(label_agrees("8B", 7_400_000_000), Some(false));
    assert_eq!(label_agrees("30B", 30_532_122_624), Some(true));
    assert_eq!(label_agrees("2.6B", 2_640_000_000), Some(true));
    assert_eq!(label_agrees("2.6B", 2_699_000_000), Some(true));
    assert_eq!(label_agrees("2.6B", 2_700_000_000), Some(false));
    assert_eq!(label_agrees("64x2.6B", 2_600_000_000), None);
    assert_eq!(label_agrees("large", 1), None);
}

#[test]
fn billions_are_written_to_one_decimal() {
    assert_eq!(billions(8_250_000_000), "8.3");
    assert_eq!(billions(8_249_999_999), "8.2");
    assert_eq!(billions(30_530_000_000), "30.5");
    assert_eq!(billions(1_000), "0.0");
}

#[test]
fn one_token_is_costed_from_the_widths_the_header_names() {
    let mut file = dense();
    file.metadata
        .insert("llama.context_length".to_owned(), Value::Integer(1024));
    file.metadata
        .insert("llama.attention.key_length".to_owned(), Value::Integer(16));
    let counted = of(&file);
    let work = super::work::of(&file, &counted);
    assert_eq!(work.multiply_adds, counted.elements - 64 * 256);
    assert_eq!(work.head_width, Some(16));
    assert_eq!(work.queries_per_key, Some(2));
    assert_eq!(work.attention_at_context, Some(4 * 32 * 1024 * 2));
    assert_eq!(
        work.cache,
        super::work::Cache::Sized {
            per_token: 2 * 32 * 2 * 2,
            key_heads: 2,
            per_head: 32,
            latent: false,
            at_context: Some((1024, 1024 * 2 * 32 * 2 * 2)),
            sliding_window: None,
            attending: (2, 2),
            recurrent: 0,
        }
    );
}

#[test]
fn a_head_width_the_header_omits_is_the_embedding_over_the_heads() {
    let file = dense();
    let counted = of(&file);
    let work = super::work::of(&file, &counted);
    assert_eq!(work.head_width, Some(16));
    assert_eq!(work.attention_at_context, None);
    assert!(matches!(
        work.cache,
        super::work::Cache::Sized {
            per_token: 256,
            at_context: None,
            ..
        }
    ));
}

#[test]
fn a_latent_cache_is_a_key_with_no_value() {
    let mut file = dense();
    for (key, value) in [
        ("llama.attention.kv_lora_rank", 12),
        ("llama.attention.key_length", 16),
        ("llama.attention.value_length", 12),
        ("llama.attention.head_count_kv", 1),
        ("llama.context_length", 1024),
    ] {
        file.metadata.insert(key.to_owned(), Value::Integer(value));
    }
    let counted = of(&file);
    let work = super::work::of(&file, &counted);
    assert_eq!(
        work.cache,
        super::work::Cache::Sized {
            per_token: 16 * 2 * 2,
            key_heads: 1,
            per_head: 16,
            latent: true,
            at_context: Some((1024, 1024 * 16 * 2 * 2)),
            sliding_window: None,
            attending: (2, 2),
            recurrent: 0,
        }
    );
    assert_eq!(work.attention_at_context, Some(4 * 28 * 1024 * 2));
}

#[test]
fn a_tied_output_head_multiplies_the_embedding_table_once() {
    let file = model(
        "llama",
        &[],
        vec![
            tensor("token_embd.weight", &[64, 256], TensorKind::Q4_K),
            tensor("blk.0.attn_q.weight", &[64, 64], TensorKind::Q4_K),
        ],
    );
    let counted = of(&file);
    let work = super::work::of(&file, &counted);
    assert!(counted.output_tied);
    assert_eq!(work.multiply_adds, 64 * 256 + 64 * 64);
}

fn spoken() -> Model {
    let tokens = [
        "<|end|>",
        "Ġthe",
        "the",
        "Ġ123",
        "12",
        "<0x41>",
        "Ġ",
        "<|start|>",
    ];
    let types = [3, 1, 1, 1, 1, 6, 1, 3];
    model(
        "llama",
        &[
            ("tokenizer.ggml.model", Value::Text("gpt2".to_owned())),
            ("tokenizer.ggml.pre", Value::Text("llama-bpe".to_owned())),
            (
                "tokenizer.ggml.tokens",
                Value::List(
                    tokens
                        .iter()
                        .map(|held| Value::Text((*held).to_owned()))
                        .collect(),
                ),
            ),
            (
                "tokenizer.ggml.token_type",
                Value::List(types.iter().map(|held| Value::Integer(*held)).collect()),
            ),
            (
                "tokenizer.ggml.merges",
                Value::List(vec![Value::Text("t h".to_owned()); 3]),
            ),
            ("tokenizer.ggml.eos_token_id", Value::Integer(0)),
            ("tokenizer.ggml.bos_token_id", Value::Integer(7)),
            ("tokenizer.ggml.padding_token_id", Value::Integer(99)),
            ("tokenizer.ggml.add_bos_token", Value::Bool(false)),
            (
                "tokenizer.chat_template",
                Value::Text(
                    "{% for m in messages %}<|start|>{{ m.role }}\n{{ m.content }}<|end|>{% \
                     endfor %}{% if tools %}{% endif %}"
                        .to_owned(),
                ),
            ),
        ],
        vec![],
    )
}

#[test]
fn a_vocabulary_is_counted_from_its_list() {
    let counted = super::vocabulary::of(&spoken());
    assert_eq!(counted.tokens, 8);
    assert_eq!(counted.model.as_deref(), Some("gpt2"));
    assert_eq!(counted.pre.as_deref(), Some("llama-bpe"));
    assert_eq!(counted.merges, Some(3));
    assert_eq!(
        counted.kinds,
        Some(vec![
            (super::vocabulary::Kind::Normal, 5),
            (super::vocabulary::Kind::Control, 2),
            (super::vocabulary::Kind::Byte, 1),
        ])
    );
    assert_eq!(counted.adds_beginning, Some(false));
    assert_eq!(counted.longest, Some(("<|start|>".to_owned(), 9)));
    assert_eq!(counted.word_starts, 3);
    assert_eq!(counted.digit_tokens, (2, 3));
}

#[test]
fn a_named_token_is_spelled_from_the_list_or_shown_to_be_beyond_it() {
    let counted = super::vocabulary::of(&spoken());
    let named = |what: &str| {
        counted
            .named
            .iter()
            .find(|held| held.what == what)
            .unwrap_or_else(|| panic!("a row for {what}"))
            .clone()
    };
    assert_eq!(named("end of text").spelled.as_deref(), Some("<|end|>"));
    assert_eq!(
        named("beginning of text").spelled.as_deref(),
        Some("<|start|>")
    );
    let padding = named("padding");
    assert_eq!(padding.identifier, 99);
    assert_eq!(padding.spelled, None);
    assert!(!counted.named.iter().any(|held| held.what == "end of turn"));
}

#[test]
fn a_template_is_read_for_the_markers_and_variables_it_uses() {
    let counted = super::vocabulary::of(&spoken());
    let template = counted.template.unwrap();
    assert_eq!(
        template.markers,
        Some(vec!["<|end|>".to_owned(), "<|start|>".to_owned()])
    );
    assert_eq!(template.mentions, vec!["tools"]);
    assert!(template.bytes > 0);
}

#[test]
fn a_vocabulary_without_types_says_so_rather_than_guessing() {
    let mut file = spoken();
    file.metadata.remove("tokenizer.ggml.token_type");
    let counted = super::vocabulary::of(&file);
    assert_eq!(counted.kinds, None);
    assert_eq!(counted.template.unwrap().markers, None);
}

#[test]
fn blocks_that_differ_are_grouped_and_only_the_attending_ones_are_cached() {
    use super::blocks::{Feed, Mixing, ranges};
    let mut tensors = vec![
        tensor("token_embd.weight", &[64, 256], TensorKind::Q4_K),
        tensor("output_norm.weight", &[64], TensorKind::F32),
    ];
    for block in 0..4_u64 {
        let named = |leaf: &str| format!("blk.{block}.{leaf}");
        if block == 3 {
            tensors.extend([
                tensor(&named("attn_q.weight"), &[64, 64], TensorKind::Q4_K),
                tensor(&named("attn_k.weight"), &[64, 32], TensorKind::Q4_K),
                tensor(&named("attn_v.weight"), &[64, 32], TensorKind::Q4_K),
                tensor(&named("attn_output.weight"), &[64, 64], TensorKind::Q4_K),
            ]);
        } else {
            let kind = if block == 0 {
                TensorKind::Q6_K
            } else {
                TensorKind::Q4_K
            };
            tensors.extend([
                tensor(&named("attn_qkv.weight"), &[64, 128], kind),
                tensor(&named("ssm_conv1d.weight"), &[4, 128], TensorKind::F32),
                tensor(&named("ssm_out.weight"), &[64, 64], kind),
            ]);
        }
        tensors.extend([
            tensor(&named("ffn_gate_inp.weight"), &[64, 8], TensorKind::F32),
            tensor(&named("ffn_up_exps.weight"), &[64, 32, 8], TensorKind::Q4_K),
            tensor(
                &named("ffn_down_exps.weight"),
                &[32, 64, 8],
                TensorKind::Q4_K,
            ),
            tensor(&named("ffn_up_shexp.weight"), &[64, 32], TensorKind::Q4_K),
            tensor(&named("ffn_down_shexp.weight"), &[32, 64], TensorKind::Q4_K),
        ]);
    }
    let file = model(
        "hybrid",
        &[
            ("hybrid.block_count", Value::Integer(4)),
            ("hybrid.embedding_length", Value::Integer(64)),
            ("hybrid.context_length", Value::Integer(1024)),
            ("hybrid.attention.head_count", Value::Integer(4)),
            ("hybrid.attention.head_count_kv", Value::Integer(2)),
            ("hybrid.attention.key_length", Value::Integer(16)),
            ("hybrid.expert_count", Value::Integer(8)),
            ("hybrid.expert_used_count", Value::Integer(2)),
        ],
        tensors,
    );
    let counted = of(&file);
    let census = &counted.census;
    assert_eq!(census.families.len(), 2, "{census:?}");
    let recurrent = &census.families[0];
    assert_eq!(recurrent.blocks, vec![0, 1, 2]);
    assert_eq!(recurrent.shape.mixing, Mixing::Recurrent);
    assert_eq!(
        recurrent.shape.feed,
        Feed::Experts {
            count: 8,
            shared: true
        }
    );
    let (least, most) = recurrent.bits.expect("every block is sized");
    assert!(least < most, "{least} {most}");
    let attending = &census.families[1];
    assert_eq!(attending.blocks, vec![3]);
    assert_eq!(attending.shape.mixing, Mixing::Attention);
    assert_eq!((census.attending, census.recurrent), (1, 3));
    assert_eq!(census.outside.tensors, 2);
    assert_eq!(ranges(&[0, 1, 2, 4, 5, 6, 8, 9], 2), "0–2, 4–6, …");
    assert_eq!(ranges(&[3, 7, 11], 5), "3, 7, 11");

    let work = super::work::of(&file, &counted);
    assert_eq!(work.attention_at_context, Some(4 * 32 * 1024));
    assert_eq!(
        work.cache,
        super::work::Cache::Sized {
            per_token: 2 * 32 * 2,
            at_context: Some((1024, 1024 * 2 * 32 * 2)),
            sliding_window: None,
            key_heads: 2,
            per_head: 32,
            latent: false,
            attending: (1, 4),
            recurrent: 3,
        }
    );
    assert_eq!(role_of("blk.0.ssm_conv1d.weight"), Role::Recurrent);
    assert_eq!(role_of("blk.0.ssm_a"), Role::Recurrent);
    assert_eq!(role_of("blk.0.ssm_norm.weight"), Role::NormsAndBiases);
    assert!(
        counted
            .roles
            .iter()
            .any(|(role, share)| *role == Role::Recurrent && share.tensors == 6),
        "{:?}",
        counted.roles
    );
}

#[test]
fn heads_are_read_off_the_output_projection() {
    let mut tensors = vec![tensor("token_embd.weight", &[64, 256], TensorKind::Q4_K)];
    let named = |leaf: &str| format!("blk.0.{leaf}");
    tensors.extend([
        tensor(&named("attn_q.weight"), &[64, 128], TensorKind::Q4_K),
        tensor(&named("attn_k.weight"), &[64, 32], TensorKind::Q4_K),
        tensor(&named("attn_v.weight"), &[64, 32], TensorKind::Q4_K),
        tensor(&named("attn_output.weight"), &[64, 64], TensorKind::Q4_K),
    ]);
    let file = model(
        "gated",
        &[
            ("gated.block_count", Value::Integer(1)),
            ("gated.embedding_length", Value::Integer(64)),
            ("gated.attention.head_count", Value::Integer(4)),
            ("gated.attention.head_count_kv", Value::Integer(2)),
            ("gated.attention.key_length", Value::Integer(16)),
            ("gated.attention.value_length", Value::Integer(16)),
        ],
        tensors,
    );
    let counted = of(&file);
    let heads = counted
        .agreements
        .iter()
        .find(|held| held.what == "attention heads")
        .expect("the header declares heads");
    assert_eq!(
        (
            heads.declared.as_deref(),
            heads.observed.as_deref(),
            heads.agrees
        ),
        (Some("4"), Some("4"), Some(true))
    );

    let mut tensors = vec![tensor("token_embd.weight", &[64, 256], TensorKind::Q4_K)];
    tensors.extend([
        tensor(&named("attn_kv_a_mqa.weight"), &[64, 72], TensorKind::Q4_K),
        tensor(&named("attn_output.weight"), &[80, 64], TensorKind::Q4_K),
    ]);
    let key_value_heads = Value::Integer(1);
    let file = model(
        "latent",
        &[
            ("latent.block_count", Value::Integer(1)),
            ("latent.embedding_length", Value::Integer(64)),
            ("latent.attention.head_count", Value::Integer(5)),
            ("latent.attention.head_count_kv", key_value_heads),
            ("latent.attention.key_length", Value::Integer(72)),
            ("latent.attention.value_length", Value::Integer(64)),
            ("latent.attention.key_length_mla", Value::Integer(32)),
            ("latent.attention.value_length_mla", Value::Integer(16)),
        ],
        tensors,
    );
    let counted = of(&file);
    let heads = counted
        .agreements
        .iter()
        .find(|held| held.what == "attention heads")
        .expect("the header declares heads");
    assert_eq!(
        (
            heads.declared.as_deref(),
            heads.observed.as_deref(),
            heads.agrees
        ),
        (Some("5"), Some("5"), Some(true))
    );
    let key_heads = counted
        .agreements
        .iter()
        .find(|held| held.what == "key/value heads")
        .expect("the header declares key/value heads");
    assert_eq!(
        (key_heads.observed.as_deref(), key_heads.agrees),
        (Some("1"), Some(true))
    );
}
