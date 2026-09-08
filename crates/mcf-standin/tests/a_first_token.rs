#![allow(clippy::expect_used, clippy::panic)]

use mcf_standin::llama::{Cache, load};
use mcf_standin::sample::Settings;
use mcf_standin::session::{Request, Stopped, generate};
use mcf_standin::tokenizer::Vocabulary;
use mcf_standin::{gguf, ops};

const TOKENS: [&str; 20] = [
    "<s>",
    "\u{2581}",
    "y",
    "e",
    "s",
    "n",
    "o",
    "m",
    "a",
    "b",
    "\u{2581}y",
    "\u{2581}ye",
    "\u{2581}yes",
    "\u{2581}n",
    "\u{2581}no",
    "\u{2581}m",
    "\u{2581}ma",
    "\u{2581}may",
    "\u{2581}mayb",
    "\u{2581}maybe",
];

const WIDTH: usize = TOKENS.len();

fn scores() -> Vec<f32> {
    TOKENS
        .iter()
        .map(|token| match token.chars().count() {
            0 | 1 => -9.0,
            length => -9.0 + f32::from(u8::try_from(length).unwrap_or(0)),
        })
        .collect()
}

fn wide() -> u64 {
    u64::try_from(WIDTH).unwrap_or(0)
}

fn a_model() -> Vec<u8> {
    let metadata = vec![
        text("general.architecture", "llama"),
        text("tokenizer.ggml.model", "llama"),
        integer("llama.block_count", 1),
        integer("llama.embedding_length", WIDTH),
        integer("llama.attention.head_count", 1),
        integer("llama.feed_forward_length", WIDTH),
        integer("llama.context_length", 16),
        integer("tokenizer.ggml.bos_token_id", 0),
        token_list("tokenizer.ggml.tokens", &TOKENS),
        score_list("tokenizer.ggml.scores", &scores()),
    ];

    let mut table = vec![0.0_f32; WIDTH * WIDTH];
    for token in 0..WIDTH {
        if let Some(slot) = table.get_mut(token * WIDTH + token) {
            *slot = 1.0;
        }
    }

    let square = vec![0.0_f32; WIDTH * WIDTH];
    let ones = vec![1.0_f32; WIDTH];
    let mut tensors: Vec<(String, Vec<u64>, Vec<f32>)> = vec![
        ("token_embd.weight".to_owned(), vec![wide(), wide()], table),
        ("output_norm.weight".to_owned(), vec![wide()], ones.clone()),
    ];
    for (name, dimensions, values) in [
        ("attn_norm.weight", vec![wide()], ones.clone()),
        ("attn_q.weight", vec![wide(), wide()], square.clone()),
        ("attn_k.weight", vec![wide(), wide()], square.clone()),
        ("attn_v.weight", vec![wide(), wide()], square.clone()),
        ("attn_output.weight", vec![wide(), wide()], square.clone()),
        ("ffn_norm.weight", vec![wide()], ones),
        ("ffn_gate.weight", vec![wide(), wide()], square.clone()),
        ("ffn_up.weight", vec![wide(), wide()], square.clone()),
        ("ffn_down.weight", vec![wide(), wide()], square),
    ] {
        tensors.push((format!("blk.0.{name}"), dimensions, values));
    }

    write(&metadata, &tensors)
}

fn text(key: &str, value: &str) -> (String, u32, Vec<u8>) {
    let mut bytes = length(value.len()).to_vec();
    bytes.extend_from_slice(value.as_bytes());
    (key.to_owned(), 8, bytes)
}

fn integer(key: &str, value: usize) -> (String, u32, Vec<u8>) {
    let value = u32::try_from(value).unwrap_or(0);
    (key.to_owned(), 5, value.to_le_bytes().to_vec())
}

fn token_list(key: &str, values: &[&str]) -> (String, u32, Vec<u8>) {
    let mut bytes = 8_u32.to_le_bytes().to_vec();
    bytes.extend_from_slice(&length(values.len()));
    for value in values {
        bytes.extend_from_slice(&length(value.len()));
        bytes.extend_from_slice(value.as_bytes());
    }
    (key.to_owned(), 9, bytes)
}

fn score_list(key: &str, values: &[f32]) -> (String, u32, Vec<u8>) {
    let mut bytes = 6_u32.to_le_bytes().to_vec();
    bytes.extend_from_slice(&length(values.len()));
    for value in values {
        bytes.extend_from_slice(&value.to_bits().to_le_bytes());
    }
    (key.to_owned(), 9, bytes)
}

fn length(value: usize) -> [u8; 8] {
    u64::try_from(value).unwrap_or(0).to_le_bytes()
}

fn write(metadata: &[(String, u32, Vec<u8>)], tensors: &[(String, Vec<u64>, Vec<f32>)]) -> Vec<u8> {
    let mut out = b"GGUF".to_vec();
    out.extend_from_slice(&3_u32.to_le_bytes());
    out.extend_from_slice(&length(tensors.len()));
    out.extend_from_slice(&length(metadata.len()));
    for (key, kind, value) in metadata {
        out.extend_from_slice(&length(key.len()));
        out.extend_from_slice(key.as_bytes());
        out.extend_from_slice(&kind.to_le_bytes());
        out.extend_from_slice(value);
    }
    let mut offset = 0_u64;
    for (name, dimensions, values) in tensors {
        out.extend_from_slice(&length(name.len()));
        out.extend_from_slice(name.as_bytes());
        out.extend_from_slice(&u32::try_from(dimensions.len()).unwrap_or(0).to_le_bytes());
        for dimension in dimensions {
            out.extend_from_slice(&dimension.to_le_bytes());
        }
        out.extend_from_slice(&0_u32.to_le_bytes());
        out.extend_from_slice(&offset.to_le_bytes());
        offset = offset.saturating_add(u64::try_from(values.len() * 4).unwrap_or(0));
    }
    let padding = (32 - (out.len() % 32)) % 32;
    out.extend(std::iter::repeat_n(0_u8, padding));
    for (_, _, values) in tensors {
        for value in values {
            out.extend_from_slice(&value.to_le_bytes());
        }
    }
    out
}

#[test]
fn text_goes_in_and_text_comes_out_marked() {
    let bytes = a_model();
    let file = gguf::parse(&bytes).expect("the test's own file reads");
    let vocabulary = Vocabulary::read(&file).expect("the vocabulary reads");
    let model = load(&file, &bytes).expect("the model loads");

    let prompt = vocabulary.encode("yes", true).expect("it segments");
    let yes = TOKENS
        .iter()
        .position(|token| *token == "\u{2581}yes")
        .expect("the vocabulary spells the word");
    assert_eq!(prompt, vec![0, yes], "<s> then the word");

    let marked = generate(
        &model,
        "stand-in, this test",
        &Request {
            prompt,
            limit: 3,
            settings: Settings::Greedy,
            seed: 0,
            stop: Vec::new(),
        },
    )
    .expect("it runs");

    assert!(
        marked.degradation().to_string().contains("engine"),
        "the result does not say which engine produced it"
    );
    let generated = marked.value().observed();
    assert_eq!(generated.prompt_length, 2);
    assert_eq!(generated.stopped, Stopped::AtLimit);

    assert_eq!(generated.tokens, vec![yes, yes, yes]);
    assert_eq!(vocabulary.decode(&generated.tokens), " yes yes yes");
}

#[test]
fn a_generation_stops_where_it_is_told_to() {
    let bytes = a_model();
    let file = gguf::parse(&bytes).expect("reads");
    let vocabulary = Vocabulary::read(&file).expect("reads");
    let model = load(&file, &bytes).expect("loads");

    let no = TOKENS
        .iter()
        .position(|token| *token == "\u{2581}no")
        .expect("the vocabulary spells the word");
    let marked = generate(
        &model,
        "stand-in, this test",
        &Request {
            prompt: vocabulary.encode("no", true).expect("segments"),
            limit: 8,
            settings: Settings::Greedy,
            seed: 0,
            stop: vec![no],
        },
    )
    .expect("runs");

    let generated = marked.value().observed();
    assert_eq!(generated.stopped, Stopped::AtStopToken { token: no });
    assert!(generated.tokens.is_empty());
    assert_eq!(vocabulary.decode(&generated.tokens), "");
}

#[test]
fn the_logits_and_the_generation_agree() {
    let bytes = a_model();
    let file = gguf::parse(&bytes).expect("reads");
    let model = load(&file, &bytes).expect("loads");

    let mut cache = Cache::for_model(&model.shape);
    let logits = model.forward(1, 0, &mut cache).expect("runs");
    assert_eq!(ops::argmax(&logits), Some(1));
}
