//! The anatomy on the wire says what the counting said, and no more.

use super::{Said, SaidCache, encode};
use mcf_record::json::Value;
use mcf_standin::gguf::{Model, Tensor, TensorKind, Value as Header};

/// Four blocks, three keeping a recurrent state and one attending, with a
/// dense feed-forward in each; an embedding table outside them.
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
    // 64×256 + 3×(64×64 + 64×128) + (64×32 + 64×128)
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

/// The header's declarations sit beside what the directory shows, never
/// merged into one figure (A21).
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

/// The cache is sized over the one block that attends, and the wire says
/// so — the same figure placement uses (F150).
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

/// What the window reads is what the daemon wrote — through the JSON it
/// crosses the socket as, not the value in memory.
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
