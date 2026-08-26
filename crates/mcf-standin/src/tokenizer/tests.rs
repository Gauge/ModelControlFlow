//! Segmentations small enough to work out by hand.
//!
//! Every vocabulary below is a handful of tokens with stated scores, so the
//! highest-scoring split is something a reader can verify without running
//! anything — which is what A19 asks of an algorithm whose output is otherwise
//! only checkable against another implementation of itself.

#![allow(clippy::float_cmp)]

use super::{SPACE, Vocabulary};
use crate::gguf;
use mcf_core::failure::Category;

/// Builds a GGUF file carrying only a vocabulary.
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

/// The vocabulary comes out of the file with its scores and its special
/// identifiers.
#[test]
fn a_vocabulary_reads_out_of_a_file() {
    let vocabulary = vocabulary(&["<s>", "▁a", "b"], &[0.0, -1.0, -2.0]);
    assert_eq!(vocabulary.len(), 3);
    assert_eq!(vocabulary.token(1), Some("▁a"));
    assert_eq!(vocabulary.beginning, Some(0));
    assert_eq!(vocabulary.ending, None);
}

/// The segmentation merges pairs, best score first, and keeps merging.
///
/// This test used to assert the opposite — that a higher-scoring *split* beats
/// a longer match — because the tokenizer was a dynamic program over the whole
/// string. [findings.md](../../../../doc/findings.md) F19 found that the models
/// themselves are tokenized by a different algorithm: adjacent symbols merge,
/// highest-scoring pair first, and a pair that spells a token merges whatever
/// the alternatives score. Getting this wrong does not fail loudly; it hands
/// the model a sequence it was never trained on.
#[test]
fn adjacent_symbols_merge_best_first_and_keep_merging() {
    // `▁a` and `b` merge into `▁ab` because the pair spells one, even though
    // `▁ab` scores worse than either half. A dynamic program would keep the
    // split; the algorithm these models use does not.
    let merging = vocabulary(&["<s>", "▁ab", "▁a", "b"], &[0.0, -5.0, -1.0, -1.0]);
    assert_eq!(merging.encode("ab", false).expect("it segments"), vec![1]);

    // Where the pair spells nothing, the symbols stay apart.
    let apart = vocabulary(&["<s>", "▁a", "b"], &[0.0, -1.0, -1.0]);
    assert_eq!(apart.encode("ab", false).expect("it segments"), vec![1, 2]);
}

/// Which merge happens first is the score's decision, and it changes the
/// answer.
///
/// `▁ab` and `bc` both spell tokens; whichever merges first takes the `b`. The
/// higher score wins, which is the whole ordering the algorithm rests on.
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

/// A space is a character, written `▁`, and the text begins with one. Getting
/// that wrong makes a model read fluent text as gibberish.
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

/// The beginning-of-text token is added when asked for and not otherwise.
#[test]
fn the_beginning_token_is_added_only_when_asked_for() {
    let vocabulary = vocabulary(&["<s>", "▁a"], &[0.0, -1.0]);
    assert_eq!(vocabulary.encode("a", true).expect("segments"), vec![0, 1]);
    assert_eq!(vocabulary.encode("a", false).expect("segments"), vec![1]);
}

/// Text the vocabulary does not spell falls back to bytes, which is what makes
/// the segmentation total rather than a source of surprises.
#[test]
fn text_outside_the_vocabulary_falls_back_to_bytes() {
    let vocabulary = vocabulary(
        &["<s>", "▁", "<0x71>", "<0x7A>"],
        &[0.0, -1.0, -10.0, -10.0],
    );
    // `qz` is not in the vocabulary; its two bytes are.
    let encoded = vocabulary.encode("qz", false).expect("segments");
    assert_eq!(encoded, vec![1, 2, 3]);
    assert_eq!(vocabulary.decode(&encoded), " qz");
}

/// Where there is no byte fallback either, the text is refused rather than
/// quietly shortened. A1: what cannot be represented is not dropped.
#[test]
fn text_with_no_representation_at_all_is_refused() {
    let vocabulary = vocabulary(&["<s>", "▁a"], &[0.0, -1.0]);
    let failure = vocabulary
        .encode("q", false)
        .expect_err("q is unrepresentable");
    assert_eq!(failure.category(), Category::ArtifactFormatMalformed);
}

/// Decoding an identifier the vocabulary does not have says so rather than
/// dropping it, so a model producing nonsense does not look like a model
/// producing nothing.
#[test]
fn an_unknown_identifier_decodes_visibly() {
    let vocabulary = vocabulary(&["<s>", "▁a"], &[0.0, -1.0]);
    assert_eq!(vocabulary.decode(&[1, 77]), " a<id 77>");
}

/// A tokenizer this crate does not implement is refused by name — D31's third
/// state, at the level of a vocabulary.
#[test]
fn another_tokenizer_is_refused_by_name() {
    let bytes = file_with(&["a"], &[], "gpt2", None);
    let file = gguf::parse(&bytes).expect("well formed");
    let failure = Vocabulary::read(&file).expect_err("gpt2 is not implemented");
    assert_eq!(failure.category(), Category::EngineUnavailable);
    assert!(
        failure
            .context()
            .iter()
            .any(|entry| entry.value.contains("gpt2")),
        "the refusal does not name what it found"
    );
}

/// Scores and tokens that disagree about how many there are is a file that
/// contradicts itself.
#[test]
fn a_vocabulary_whose_scores_do_not_match_is_refused() {
    let bytes = file_with(&["a", "b", "c"], &[0.0, -1.0], "llama", None);
    let file = gguf::parse(&bytes).expect("well formed");
    let failure = Vocabulary::read(&file).expect_err("two scores for three tokens");
    assert_eq!(failure.category(), Category::ArtifactFormatMalformed);
}

/// A file with no vocabulary at all is a different failure from one with a
/// vocabulary MCF cannot use.
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

/// Encoding and decoding are inverses for text the vocabulary spells, which is
/// the property a caller actually relies on.
#[test]
fn what_is_encoded_decodes_back() {
    // Every intermediate piece is here, which is what a real vocabulary has
    // and what the merge algorithm needs: a token is reached by merging pairs,
    // so a vocabulary holding `▁fox` and not `▁f` cannot build it (F19).
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
