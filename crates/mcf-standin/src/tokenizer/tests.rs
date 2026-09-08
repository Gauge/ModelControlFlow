#![allow(clippy::float_cmp)]

use super::{SPACE, Vocabulary};
use crate::gguf;
use mcf_core::failure::Category;

fn file_with(tokens: &[&str], scores: &[f32], kind: &str, bos: Option<u32>) -> Vec<u8> {
    let mut metadata: Vec<(String, u32, Vec<u8>)> = Vec::new();

    let mut model = length(kind.len()).to_vec();
    model.extend_from_slice(kind.as_bytes());
    metadata.push(("tokenizer.ggml.model".to_owned(), 8, model));

    let mut list = 8_u32.to_le_bytes().to_vec();
    list.extend_from_slice(&length(tokens.len()));
    for token in tokens {
        list.extend_from_slice(&length(token.len()));
        list.extend_from_slice(token.as_bytes());
    }
    metadata.push(("tokenizer.ggml.tokens".to_owned(), 9, list));

    if !scores.is_empty() {
        let mut list = 6_u32.to_le_bytes().to_vec();
        list.extend_from_slice(&length(scores.len()));
        for score in scores {
            list.extend_from_slice(&score.to_bits().to_le_bytes());
        }
        metadata.push(("tokenizer.ggml.scores".to_owned(), 9, list));
    }

    if let Some(bos) = bos {
        metadata.push((
            "tokenizer.ggml.bos_token_id".to_owned(),
            5,
            bos.to_le_bytes().to_vec(),
        ));
    }

    let mut out = b"GGUF".to_vec();
    out.extend_from_slice(&3_u32.to_le_bytes());
    out.extend_from_slice(&length(0));
    out.extend_from_slice(&length(metadata.len()));
    for (key, kind, value) in &metadata {
        out.extend_from_slice(&length(key.len()));
        out.extend_from_slice(key.as_bytes());
        out.extend_from_slice(&kind.to_le_bytes());
        out.extend_from_slice(value);
    }
    out
}

fn length(value: usize) -> [u8; 8] {
    u64::try_from(value).unwrap_or(0).to_le_bytes()
}

fn vocabulary(tokens: &[&str], scores: &[f32]) -> Vocabulary {
    let bytes = file_with(tokens, scores, "llama", Some(0));
    let file = gguf::parse(&bytes).expect("the file is well formed");
    Vocabulary::read(&file).expect("the vocabulary reads")
}

#[test]
fn a_vocabulary_reads_out_of_a_file() {
    let vocabulary = vocabulary(&["<s>", "▁a", "b"], &[0.0, -1.0, -2.0]);
    assert_eq!(vocabulary.len(), 3);
    assert_eq!(vocabulary.token(1), Some("▁a"));
    assert_eq!(vocabulary.beginning, Some(0));
    assert_eq!(vocabulary.ending, None);
}

#[test]
fn adjacent_symbols_merge_best_first_and_keep_merging() {
    let merging = vocabulary(&["<s>", "▁ab", "▁a", "b"], &[0.0, -5.0, -1.0, -1.0]);
    assert_eq!(merging.encode("ab", false).expect("it segments"), vec![1]);

    let apart = vocabulary(&["<s>", "▁a", "b"], &[0.0, -1.0, -1.0]);
    assert_eq!(apart.encode("ab", false).expect("it segments"), vec![1, 2]);
}

#[test]
fn the_higher_scoring_merge_takes_the_character_they_share() {
    let left = vocabulary(
        &["<s>", "▁ab", "bc", "▁a", "b", "c"],
        &[0.0, -1.0, -9.0, -3.0, -3.0, -3.0],
    );
    assert_eq!(
        left.encode("abc", false).expect("it segments"),
        vec![1, 5],
        "the better-scoring left merge did not win"
    );

    let right = vocabulary(
        &["<s>", "▁ab", "bc", "▁a", "b", "c"],
        &[0.0, -9.0, -1.0, -3.0, -3.0, -3.0],
    );
    assert_eq!(
        right.encode("abc", false).expect("it segments"),
        vec![3, 2],
        "the better-scoring right merge did not win"
    );
}

#[test]
fn a_space_is_a_character_and_the_text_starts_with_one() {
    let vocabulary = vocabulary(&["<s>", "▁a", "▁b"], &[0.0, -1.0, -1.0]);
    let encoded = vocabulary.encode("a b", false).expect("segments");
    assert_eq!(
        encoded,
        vec![1, 2],
        "the space before b is part of its token"
    );
    assert_eq!(vocabulary.decode(&encoded), " a b");
    assert_eq!(SPACE, '\u{2581}');
}

#[test]
fn the_beginning_token_is_added_only_when_asked_for() {
    let vocabulary = vocabulary(&["<s>", "▁a"], &[0.0, -1.0]);
    assert_eq!(vocabulary.encode("a", true).expect("segments"), vec![0, 1]);
    assert_eq!(vocabulary.encode("a", false).expect("segments"), vec![1]);
}

#[test]
fn text_outside_the_vocabulary_falls_back_to_bytes() {
    let vocabulary = vocabulary(
        &["<s>", "▁", "<0x71>", "<0x7A>"],
        &[0.0, -1.0, -10.0, -10.0],
    );
    let encoded = vocabulary.encode("qz", false).expect("segments");
    assert_eq!(encoded, vec![1, 2, 3]);
    assert_eq!(vocabulary.decode(&encoded), " qz");
}

#[test]
fn text_with_no_representation_at_all_is_refused() {
    let vocabulary = vocabulary(&["<s>", "▁a"], &[0.0, -1.0]);
    let failure = vocabulary
        .encode("q", false)
        .expect_err("q is unrepresentable");
    assert_eq!(failure.category(), Category::ArtifactFormatMalformed);
}

#[test]
fn an_unknown_identifier_decodes_visibly() {
    let vocabulary = vocabulary(&["<s>", "▁a"], &[0.0, -1.0]);
    assert_eq!(vocabulary.decode(&[1, 77]), " a<id 77>");
}

#[test]
fn another_tokenizer_is_refused_by_name() {
    let bytes = file_with(&["a"], &[], "rwkv", None);
    let file = gguf::parse(&bytes).expect("well formed");
    let failure = Vocabulary::read(&file).expect_err("rwkv is not implemented");
    assert_eq!(failure.category(), Category::EngineUnavailable);
    assert!(
        failure
            .context()
            .iter()
            .any(|entry| entry.value.contains("rwkv")),
        "the refusal does not name what it found"
    );
}

#[test]
fn a_vocabulary_whose_scores_do_not_match_is_refused() {
    let bytes = file_with(&["a", "b", "c"], &[0.0, -1.0], "llama", None);
    let file = gguf::parse(&bytes).expect("well formed");
    let failure = Vocabulary::read(&file).expect_err("two scores for three tokens");
    assert_eq!(failure.category(), Category::ArtifactFormatMalformed);
}

#[test]
fn a_file_with_no_vocabulary_says_that_instead() {
    let mut out = b"GGUF".to_vec();
    out.extend_from_slice(&3_u32.to_le_bytes());
    out.extend_from_slice(&length(0));
    out.extend_from_slice(&length(1));
    out.extend_from_slice(&length("tokenizer.ggml.model".len()));
    out.extend_from_slice(b"tokenizer.ggml.model");
    out.extend_from_slice(&8_u32.to_le_bytes());
    out.extend_from_slice(&length("llama".len()));
    out.extend_from_slice(b"llama");

    let file = gguf::parse(&out).expect("well formed");
    let failure = Vocabulary::read(&file).expect_err("there is no vocabulary");
    assert_eq!(failure.category(), Category::ArtifactProvenanceIncomplete);
}

#[test]
fn what_is_encoded_decodes_back() {
    let vocabulary = vocabulary(
        &[
            "<s>", "▁the", "▁quick", "▁brown", "▁fox", "es", "▁", "t", "h", "e", "q", "u", "i",
            "c", "k", "b", "r", "o", "w", "n", "f", "x", "s", "▁t", "▁th", "▁q", "▁qu", "▁qui",
            "▁quic", "▁b", "▁br", "▁bro", "▁brow", "▁f", "▁fo",
        ],
        &[
            0.0, -1.0, -1.0, -1.0, -1.0, -2.0, -9.0, -9.0, -9.0, -9.0, -9.0, -9.0, -9.0, -9.0,
            -9.0, -9.0, -9.0, -9.0, -9.0, -9.0, -9.0, -9.0, -9.0, -5.0, -5.0, -5.0, -5.0, -5.0,
            -5.0, -5.0, -5.0, -5.0, -5.0, -5.0, -5.0,
        ],
    );
    for text in ["the quick brown fox", "the foxes", "quick"] {
        let encoded = vocabulary.encode(text, false).expect("segments");
        assert_eq!(
            vocabulary.decode(&encoded).trim_start(),
            text,
            "{text} came back as {:?}",
            vocabulary.decode(&encoded)
        );
    }
}

fn pairs_file(
    tokens: &[&str],
    merges: &[&str],
    pre: Option<&str>,
    types: Option<&[i32]>,
    add_bos: Option<bool>,
) -> Vec<u8> {
    let mut metadata: Vec<(String, u32, Vec<u8>)> = Vec::new();

    let mut model = length("gpt2".len()).to_vec();
    model.extend_from_slice(b"gpt2");
    metadata.push(("tokenizer.ggml.model".to_owned(), 8, model));

    for (key, strings) in [
        ("tokenizer.ggml.tokens", tokens),
        ("tokenizer.ggml.merges", merges),
    ] {
        let mut list = 8_u32.to_le_bytes().to_vec();
        list.extend_from_slice(&length(strings.len()));
        for text in strings {
            list.extend_from_slice(&length(text.len()));
            list.extend_from_slice(text.as_bytes());
        }
        metadata.push((key.to_owned(), 9, list));
    }

    if let Some(pre) = pre {
        let mut value = length(pre.len()).to_vec();
        value.extend_from_slice(pre.as_bytes());
        metadata.push(("tokenizer.ggml.pre".to_owned(), 8, value));
    }

    if let Some(types) = types {
        let mut list = 5_u32.to_le_bytes().to_vec();
        list.extend_from_slice(&length(types.len()));
        for kind in types {
            list.extend_from_slice(&kind.to_le_bytes());
        }
        metadata.push(("tokenizer.ggml.token_type".to_owned(), 9, list));
    }

    if let Some(add) = add_bos {
        metadata.push((
            "tokenizer.ggml.add_bos_token".to_owned(),
            7,
            vec![u8::from(add)],
        ));
    }
    metadata.push((
        "tokenizer.ggml.bos_token_id".to_owned(),
        5,
        0_u32.to_le_bytes().to_vec(),
    ));

    let mut out = b"GGUF".to_vec();
    out.extend_from_slice(&3_u32.to_le_bytes());
    out.extend_from_slice(&length(0));
    out.extend_from_slice(&length(metadata.len()));
    for (key, kind, value) in &metadata {
        out.extend_from_slice(&length(key.len()));
        out.extend_from_slice(key.as_bytes());
        out.extend_from_slice(&kind.to_le_bytes());
        out.extend_from_slice(value);
    }
    out
}

fn pairs(
    tokens: &[&str],
    merges: &[&str],
    types: Option<&[i32]>,
    add_bos: Option<bool>,
) -> Vocabulary {
    let bytes = pairs_file(
        tokens,
        merges,
        Some(crate::architecture::a_pre_tokenizer(
            crate::bpe::Split::ModernOneDigit,
        )),
        types,
        add_bos,
    );
    let file = gguf::parse(&bytes).expect("well formed");
    Vocabulary::read(&file).expect("a byte-pair vocabulary")
}

#[test]
fn byte_pairs_segment_by_the_merge_list() {
    let vocabulary = pairs(
        &[
            "<s>",
            "t",
            "h",
            "e",
            "\u{120}",
            "th",
            "the",
            "\u{120}the",
            "r",
            "e\u{120}",
        ],
        &["t h", "th e", "\u{120} the"],
        None,
        None,
    );
    assert_eq!(vocabulary.encode("the the", false), Ok(vec![6, 7]));
    assert_eq!(vocabulary.decode(&[6, 7]), "the the");
}

#[test]
fn the_order_of_the_merges_is_what_is_followed() {
    let tokens = ["<s>", "a", "b", "c", "ab", "bc"];
    let first = pairs(&tokens, &["a b", "b c"], None, None);
    let second = pairs(&tokens, &["b c", "a b"], None, None);
    assert_eq!(first.encode("abc", false), Ok(vec![4, 3]));
    assert_eq!(second.encode("abc", false), Ok(vec![1, 5]));
}

#[test]
fn text_no_merge_covers_still_segments() {
    let vocabulary = pairs(&["<s>", "a", "b", "\u{120}"], &["x y"], None, None);
    assert_eq!(vocabulary.encode("a b", false), Ok(vec![1, 3, 2]));
    assert_eq!(vocabulary.decode(&[1, 3, 2]), "a b");
}

#[test]
fn a_multi_byte_character_survives_the_round_trip() {
    let alphabet: Vec<String> = (0..=u8::MAX)
        .map(|byte| crate::bpe::character_of(byte).to_string())
        .collect();
    let tokens: Vec<&str> = std::iter::once("<s>")
        .chain(alphabet.iter().map(String::as_str))
        .collect();
    let vocabulary = pairs(&tokens, &["x y"], None, None);
    for text in [
        "\u{e9}",
        "\u{4f60}\u{597d}",
        "\u{1f600}",
        "na\u{ef}ve caf\u{e9}",
    ] {
        let identifiers = vocabulary
            .encode(text, false)
            .expect("every byte is a token");
        assert_eq!(
            vocabulary.decode(&identifiers),
            text,
            "{text:?} did not come back"
        );
    }
}

#[test]
fn a_user_defined_token_is_matched_whole_however_short() {
    let vocabulary = pairs(
        &["<s>", "a", "b", "\u{120}", "<|marker|>", "  ", "<"],
        &["x y"],
        Some(&[3, 1, 1, 1, 4, 4, 1]),
        None,
    );
    assert_eq!(vocabulary.encode("a<|marker|>b", false), Ok(vec![1, 4, 2]));
    assert_eq!(vocabulary.encode("a  b", false), Ok(vec![1, 5, 2]));
}

#[test]
fn a_control_token_in_ordinary_text_stays_text() {
    let vocabulary = pairs(
        &[
            "<s>",
            "a",
            "b",
            "\u{120}",
            "<|im_start|>",
            "<",
            "|",
            "i",
            "m",
            "_",
            "s",
            "t",
            "r",
            ">",
        ],
        &["x y"],
        Some(&[3, 1, 1, 1, 3, 1, 1, 1, 1, 1, 1, 1, 1, 1]),
        None,
    );
    let identifiers = vocabulary
        .encode("a<|im_start|>b", false)
        .expect("segmented");
    assert!(
        !identifiers.contains(&4),
        "a control token was produced from ordinary text: {identifiers:?}"
    );
}

#[test]
fn the_file_decides_whether_a_beginning_is_added() {
    let tokens = ["<s>", "a", "b", "\u{120}"];
    assert_eq!(
        pairs(&tokens, &["x y"], None, Some(false)).encode("a", true),
        Ok(vec![1])
    );
    assert_eq!(
        pairs(&tokens, &["x y"], None, Some(true)).encode("a", true),
        Ok(vec![0, 1])
    );
    assert_eq!(
        pairs(&tokens, &["x y"], None, None).encode("a", true),
        Ok(vec![0, 1])
    );
}

#[test]
fn what_a_byte_pair_vocabulary_must_carry_is_checked() {
    let empty = pairs_file(
        &["a"],
        &[],
        Some(crate::architecture::a_pre_tokenizer(
            crate::bpe::Split::ModernOneDigit,
        )),
        None,
        None,
    );
    let file = gguf::parse(&empty).expect("well formed");
    let failure = Vocabulary::read(&file).expect_err("no merges is not a merge list");
    assert_eq!(failure.category(), Category::ArtifactProvenanceIncomplete);

    let strange = pairs_file(
        &["a"],
        &["a b"],
        Some("a pre-tokenizer nobody has written"),
        None,
        None,
    );
    let file = gguf::parse(&strange).expect("well formed");
    let failure = Vocabulary::read(&file).expect_err("tekken is not implemented");
    assert_eq!(failure.category(), Category::EngineUnavailable);
    assert!(
        failure
            .context()
            .iter()
            .any(|entry| entry.value.contains("nobody has written")),
        "the refusal does not name what it asked for"
    );
}

#[test]
fn the_empty_string_is_still_segmented() {
    let vocabulary = vocabulary(&["<s>", "\u{2581}", "\u{2581}a"], &[0.0, -1.0, -2.0]);
    assert_eq!(
        vocabulary.encode("", false),
        Ok(vec![1]),
        "the empty string should encode as the space prefix alone"
    );
    let pairs = pairs(&["<s>", "a", "b", "\u{120}"], &["x y"], None, Some(false));
    assert_eq!(pairs.encode("", false), Ok(vec![]));
}

#[test]
fn the_file_decides_whether_a_space_is_prefixed() {
    let vocabulary = vocabulary(
        &["<s>", "\u{2581}", "\u{2581}a", "a"],
        &[0.0, -1.0, -2.0, -3.0],
    );
    assert!(
        vocabulary.adds_a_space_prefix(),
        "a unigram vocabulary that does not say should add the prefix"
    );
}

#[test]
fn the_token_list_reads_where_the_vocabulary_refuses() {
    let bytes = file_with(
        &["<s>", "[INST]", "[/INST]", "hello"],
        &[0.0, 0.0, 0.0, -1.0],
        "tekken",
        Some(0),
    );
    let file = gguf::parse(&bytes).expect("well formed");
    let refused = Vocabulary::read(&file).expect_err("a third scheme is refused");
    assert_eq!(refused.category(), Category::EngineUnavailable);

    let tokens = super::Tokens::read(&file).expect("the list is there regardless");
    assert_eq!(tokens.len(), 4);
    assert!(tokens.has_token("[INST]"));
    assert!(!tokens.has_token("[INST"));
    assert_eq!(tokens.identifier("[/INST]"), Some(2));
    assert_eq!(tokens.token(3), Some("hello"));
    assert_eq!(tokens.token(4), None);
    assert_eq!(tokens.beginning, Some(0));
    assert_eq!(tokens.ending, None);
}

#[test]
fn a_file_with_no_tokens_is_no_list() {
    let mut out = b"GGUF".to_vec();
    out.extend_from_slice(&3_u32.to_le_bytes());
    out.extend_from_slice(&length(0));
    out.extend_from_slice(&length(1));
    out.extend_from_slice(&length("tokenizer.ggml.model".len()));
    out.extend_from_slice(b"tokenizer.ggml.model");
    out.extend_from_slice(&8_u32.to_le_bytes());
    out.extend_from_slice(&length("tekken".len()));
    out.extend_from_slice(b"tekken");
    let file = gguf::parse(&out).expect("well formed");
    let failure = super::Tokens::read(&file).expect_err("nothing is listed");
    assert_eq!(failure.category(), Category::ArtifactProvenanceIncomplete);
}
