use super::{FAMILIES, PRE_TOKENIZERS, a_pre_tokenizer, pre_tokenizer, rotation};
use crate::bpe::Split;
use crate::ops::Rotation;

#[test]
fn the_rotation_table_is_total_and_defaults_to_the_first_family() {
    assert_eq!(rotation("llama"), Rotation::Interleaved);
    assert_eq!(
        rotation("a family nobody has heard of"),
        Rotation::Interleaved
    );
    assert_eq!(rotation(""), Rotation::Interleaved);
    assert_eq!(rotation("gemma3"), Rotation::Halved);
}

#[test]
fn the_pre_tokenizer_table_lists_only_what_it_implements() {
    for name in PRE_TOKENIZERS {
        assert!(
            pre_tokenizer(name).is_some(),
            "{name} is listed but not implemented"
        );
    }
    assert_eq!(pre_tokenizer("a pre-tokenizer nobody has written"), None);
    for split in [
        Split::Gpt2,
        Split::Gpt2DigitsApart,
        Split::ModernThreeDigits,
        Split::ModernOneDigit,
    ] {
        assert_eq!(
            pre_tokenizer(a_pre_tokenizer(split)),
            Some(split),
            "the name offered for {split:?} does not come back as {split:?}"
        );
    }
}

#[test]
fn the_families_are_a_list_and_not_an_accident() {
    assert!(
        !FAMILIES.is_empty(),
        "an engine for no families is not an engine"
    );
    assert_eq!(FAMILIES.first().copied(), Some("llama"));
    let mut sorted = FAMILIES.to_vec();
    sorted.sort_unstable();
    sorted.dedup();
    assert_eq!(sorted.len(), FAMILIES.len(), "a family is listed twice");
}
