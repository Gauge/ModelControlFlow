use super::{Said, SaidCache, encode};
use mcf_record::json::Value;
use mcf_standin::gguf::{Model, Tensor, TensorKind, Value as Header};

fn a_hybrid() -> Model {
    let tensor = |name: String, dimensions: &[u64]| Tensor {
        name,
        dimensions: dimensions.to_vec(),
        kind: TensorKind::Q4_K,
        offset: 0,
    };
    let mut metadata = std::collections::BTreeMap::new();
    for (key, value) in [
        ("general.architecture", Header::Text("hybrid".to_owned())),
        ("hybrid.block_count", Header::Integer(4)),
        ("hybrid.embedding_length", Header::Integer(64)),
        ("hybrid.context_length", Header::Integer(1024)),
        ("hybrid.attention.head_count", Header::Integer(4)),
        ("hybrid.attention.head_count_kv", Header::Integer(2)),
        ("hybrid.attention.key_length", Header::Integer(16)),
    ] {
        metadata.insert(key.to_owned(), value);
    }
    let mut tensors = vec![tensor("token_embd.weight".to_owned(), &[64, 256])];
    for block in 0..4_u64 {
        if block == 3 {
            tensors.push(tensor(format!("blk.{block}.attn_k.weight"), &[64, 32]));
        } else {
            tensors.push(tensor(format!("blk.{block}.ssm_out.weight"), &[64, 64]));
        }
        tensors.push(tensor(format!("blk.{block}.ffn_up.weight"), &[64, 128]));
    }
    Model {
        version: 3,
        metadata,
        tensors,
        data_offset: 0,
        alignment: 32,
    }
}

#[test]
fn the_counting_reaches_the_wire_as_it_was_counted() {
    let said = encode("hybrid.gguf", &a_hybrid());
    assert_eq!(
        said.get("model").and_then(Value::as_text),
        Some("hybrid.gguf")
    );
    let counted = said.get("counted").expect("counted");
    assert_eq!(counted.get("blocks").and_then(Value::as_integer), Some(4));
    assert_eq!(
        counted.get("attending").and_then(Value::as_integer),
        Some(1)
    );
    assert_eq!(
        counted.get("recurrent").and_then(Value::as_integer),
        Some(3)
    );
    assert_eq!(
        counted.get("elements").and_then(Value::as_integer),
        Some(16_384 + 3 * (4_096 + 8_192) + (2_048 + 8_192))
    );
    let shapes = counted
        .get("block_shapes")
        .and_then(Value::as_list)
        .expect("block shapes");
    assert_eq!(shapes.len(), 2);
    let first = shapes.first().expect("a first shape");
    assert_eq!(
        first.get("mixing").and_then(Value::as_text),
        Some("recurrent")
    );
    assert_eq!(first.get("feed").and_then(Value::as_text), Some("dense"));
    assert_eq!(
        first.get("blocks").and_then(Value::as_list).map(<[_]>::len),
        Some(3)
    );
    assert_eq!(
        first.get("said").and_then(Value::as_text),
        Some(
            "a recurrent state of fixed size, nothing kept per position; one feed-forward every token passes"
        )
    );
    let parts = counted
        .get("parts")
        .and_then(Value::as_list)
        .expect("parts");
    assert!(parts.iter().any(|part| {
        part.get("part").and_then(Value::as_text) == Some("recurrent state")
            && part.get("elements").and_then(Value::as_integer) == Some(3 * 4_096)
    }));
}

#[test]
fn declared_and_observed_sit_side_by_side() {
    let said = encode("hybrid.gguf", &a_hybrid());
    let agreements = said
        .get("agreements")
        .and_then(Value::as_list)
        .expect("agreements");
    let blocks = agreements
        .iter()
        .find(|held| held.get("what").and_then(Value::as_text) == Some("blocks"))
        .expect("the block count is compared");
    assert_eq!(blocks.get("declared").and_then(Value::as_text), Some("4"));
    assert_eq!(blocks.get("observed").and_then(Value::as_text), Some("4"));
    assert_eq!(blocks.get("agrees"), Some(&Value::Bool(true)));
}

#[test]
fn the_cache_on_the_wire_is_the_one_placement_sizes() {
    let said = encode("hybrid.gguf", &a_hybrid());
    let cache = said
        .get("work")
        .and_then(|v| v.get("cache"))
        .expect("a cache");
    assert_eq!(cache.get("sized"), Some(&Value::Bool(true)));
    assert_eq!(
        cache.get("per_token").and_then(Value::as_integer),
        Some(2 * 32 * 2)
    );
    assert_eq!(cache.get("attending").and_then(Value::as_integer), Some(1));
    assert_eq!(cache.get("blocks").and_then(Value::as_integer), Some(4));
    assert_eq!(cache.get("recurrent").and_then(Value::as_integer), Some(3));
    assert_eq!(
        cache.get("at_context").and_then(Value::as_integer),
        Some(1024 * 128)
    );
    assert_eq!(
        cache.get("kept").and_then(Value::as_text),
        Some("32 for a key and a value")
    );
    assert_eq!(cache.get("latent"), Some(&Value::Bool(false)));

    let mut only_recurrent = a_hybrid();
    only_recurrent
        .tensors
        .retain(|tensor| !tensor.name.starts_with("blk.3."));
    let said = encode("recurrent.gguf", &only_recurrent);
    let cache = said
        .get("work")
        .and_then(|v| v.get("cache"))
        .expect("a cache");
    assert_eq!(cache.get("sized"), Some(&Value::Bool(false)));
    assert!(
        cache
            .get("why")
            .and_then(Value::as_text)
            .is_some_and(|why| why.contains("recurrent"))
    );
}

#[test]
fn the_window_reads_what_the_daemon_wrote() {
    let said = encode("hybrid.gguf", &a_hybrid());
    let line = said.to_line();
    let back = mcf_record::json::parse(&line).expect("the line parses");
    let read = Said::from_value(&back).expect("the value reads back");
    assert_eq!(read.model, "hybrid.gguf");
    assert_eq!(read.blocks, 4);
    assert_eq!((read.attending, read.recurrent), (1, 3));
    assert_eq!(read.families.len(), 2);
    assert_eq!(
        read.families.first().map(|f| f.blocks.clone()),
        Some(vec![0, 1, 2])
    );
    assert_eq!(
        read.families.first().map(|f| f.ranged.as_str()),
        Some("0–2")
    );
    assert!(read.parts.iter().any(|part| part.name == "recurrent state"));
    assert!(read.encodings.iter().any(|kind| kind.name == "Q4_K"));
    assert_eq!(
        read.agreements.len(),
        said.get("agreements")
            .and_then(Value::as_list)
            .map_or(0, <[_]>::len)
    );
    match read.cache {
        SaidCache::Sized {
            per_token,
            attending,
            at_context,
            ..
        } => {
            assert_eq!(per_token, 128);
            assert_eq!(attending, (1, 4));
            assert_eq!(at_context, Some((1024, 1024 * 128)));
        }
        SaidCache::Unsized(why) => panic!("the cache is sized: {why}"),
    }
}

fn a_hybrid_that_speaks() -> Model {
    let mut model = a_hybrid();
    let text = |held: &str| Header::Text(held.to_owned());
    for (key, value) in [
        ("tokenizer.ggml.model", text("gpt2")),
        (
            "tokenizer.ggml.tokens",
            Header::List(
                ["<s>", "</s>", "Ġthe", "12", "<|im_end|>"]
                    .into_iter()
                    .map(text)
                    .collect(),
            ),
        ),
        (
            "tokenizer.ggml.token_type",
            Header::List([3, 3, 1, 1, 3].into_iter().map(Header::Integer).collect()),
        ),
        ("tokenizer.ggml.eos_token_id", Header::Integer(1)),
        ("tokenizer.ggml.eot_token_id", Header::Integer(9)),
        ("tokenizer.ggml.add_bos_token", Header::Bool(false)),
        (
            "tokenizer.chat_template",
            text("{{ system }}<|im_end|>{% if add_generation_prompt %}"),
        ),
    ] {
        model.metadata.insert(key.to_owned(), value);
    }
    model
}

#[test]
fn the_vocabulary_on_the_wire_is_the_one_the_console_prints() {
    let said = encode("hybrid.gguf", &a_hybrid_that_speaks());
    let line = said.to_line();
    let back = mcf_record::json::parse(&line).expect("the line parses");
    let read = Said::from_value(&back).expect("the value reads back");
    let spoken = read.vocabulary;
    assert_eq!(spoken.tokens, 5);
    assert_eq!(spoken.segmentation, "gpt2");
    assert_eq!(spoken.word_starts, 1);
    assert_eq!(
        spoken.kinds,
        Some(vec![("text".to_owned(), 2), ("control".to_owned(), 3)])
    );
    assert!(spoken.digits.starts_with("1 tokens, the longest 2 digits"));
    assert_eq!(spoken.longest, Some(("<|im_end|>".to_owned(), 10)));
    assert_eq!(spoken.beginning, "no, the file says so");
    let end = spoken
        .named
        .iter()
        .find(|named| named.what == "end of text")
        .expect("end of text");
    assert_eq!(end.spelled.as_deref(), Some("</s>"));
    assert_eq!(end.beyond, None);
    let turn = spoken
        .named
        .iter()
        .find(|named| named.what == "end of turn")
        .expect("end of turn");
    assert_eq!(turn.identifier, 9);
    assert_eq!(turn.spelled, None);
    assert!(
        turn.beyond
            .as_deref()
            .is_some_and(|why| why.contains("BEYOND THE LIST") && why.contains("holds 5"))
    );
    let template = spoken.template.expect("a template");
    assert_eq!(template.mentions, vec!["system", "add_generation_prompt"]);
    assert_eq!(template.markers, Ok(vec!["<|im_end|>".to_owned()]));

    let quiet = Said::from_value(&encode("hybrid.gguf", &a_hybrid())).expect("reads back");
    assert_eq!(quiet.vocabulary.tokens, 0);
    assert_eq!(quiet.vocabulary.kinds, None);
    assert_eq!(
        quiet.vocabulary.template,
        Err(mcf_standin::anatomy::vocabulary::NO_TEMPLATE.to_owned())
    );
}
