//! Tests for what a listing says about a model published in parts.

use super::{Entry, Listing};

fn listing(paths: &[(&str, u64)]) -> Listing {
    Listing {
        reference: crate::reference::parse("owner/model").expect("a reference"),
        revision: Some("main".to_owned()),
        entries: paths
            .iter()
            .map(|(path, size)| Entry::new(*path, *size))
            .collect(),
        gated: None,
        declared_licence: None,
        lineage: None,
    }
}

/// A model published in parts is one set: every part, in order, however the
/// listing orders them, and nothing that is not a part of it (B-590).
#[test]
fn the_parts_of_a_published_model_are_one_set_in_order() {
    let listing = listing(&[
        ("Q3/model-Q3-00003-of-00003.gguf", 30),
        ("Q3/model-Q3-00001-of-00003.gguf", 10),
        ("Q4/model-Q4-00001-of-00002.gguf", 5),
        ("Q3/model-Q3-00002-of-00003.gguf", 20),
        ("Q4/model-Q4-00002-of-00002.gguf", 5),
        ("mmproj-model.gguf", 1),
        ("README.md", 1),
    ]);
    let set = listing
        .parts_of("Q3/model-Q3-00002-of-00003.gguf")
        .expect("a part of a set");
    assert_eq!(
        set.parts
            .iter()
            .map(|part| part.path.as_str())
            .collect::<Vec<_>>(),
        [
            "Q3/model-Q3-00001-of-00003.gguf",
            "Q3/model-Q3-00002-of-00003.gguf",
            "Q3/model-Q3-00003-of-00003.gguf",
        ]
    );
    assert_eq!(set.of, 3);
    assert!(set.is_whole());
    assert_eq!(set.bytes(), Some(60));

    let other = listing
        .parts_of("Q4/model-Q4-00002-of-00002.gguf")
        .expect("the other set");
    assert_eq!(other.parts.len(), 2);
    assert!(other.is_whole());

    assert!(listing.parts_of("mmproj-model.gguf").is_none());
    assert!(listing.parts_of("README.md").is_none());
}

/// A set the repository publishes only part of is not whole, and says so
/// rather than being fetched (B-590).
#[test]
fn a_set_missing_a_part_is_not_whole() {
    let listing = listing(&[
        ("model-00001-of-00003.gguf", 10),
        ("model-00003-of-00003.gguf", 30),
    ]);
    let set = listing
        .parts_of("model-00001-of-00003.gguf")
        .expect("a part of a set");
    assert_eq!(set.parts.len(), 2);
    assert_eq!(set.of, 3);
    assert!(!set.is_whole());
    // A part numbered past its count, or at nought, is not a part.
    assert!(listing.parts_of("model-00004-of-00003.gguf").is_none());
    assert!(listing.parts_of("model-00000-of-00003.gguf").is_none());
}

/// A repository's variants are what a person chooses between: a
/// quantization published in four files is one of them, named without its
/// part suffix and weighing the whole (B-597).
#[test]
fn a_repositorys_variants_are_one_a_quantization_however_many_files() {
    let listing = listing(&[
        ("BF16/model-BF16-00001-of-00003.gguf", 49_000_000_000),
        ("BF16/model-BF16-00002-of-00003.gguf", 49_000_000_000),
        ("BF16/model-BF16-00003-of-00003.gguf", 11_000_000_000),
        ("model-Q4_K_M.gguf", 7_300_000_000),
        ("mmproj-F16.gguf", 800_000_000),
        ("README.md", 1_000),
    ]);
    let variants = listing.variants();
    assert_eq!(
        variants
            .iter()
            .map(|held| (held.name.as_str(), held.bytes, held.parts))
            .collect::<Vec<_>>(),
        [
            ("BF16/model-BF16.gguf", 109_000_000_000, 3),
            ("model-Q4_K_M.gguf", 7_300_000_000, 1),
            ("mmproj-F16.gguf", 800_000_000, 1),
        ]
    );
    // The file to ask for is the set's first part, which fetches the whole.
    assert_eq!(variants[0].first, "BF16/model-BF16-00001-of-00003.gguf");
    assert!(variants.iter().all(|held| held.whole));
}

/// A part suffix comes off the name and nothing else does (B-597).
#[test]
fn the_part_suffix_comes_off_the_name_and_nothing_else_does() {
    use super::without_the_part;
    assert_eq!(
        without_the_part("BF16/model-BF16-00001-of-00004.gguf"),
        "BF16/model-BF16.gguf"
    );
    assert_eq!(without_the_part("model-Q4_K_M.gguf"), "model-Q4_K_M.gguf");
    assert_eq!(without_the_part("model-1-of-2.gguf"), "model.gguf");
    // Not a part: a name that only looks like one.
    assert_eq!(
        without_the_part("model-best-of-breed.gguf"),
        "model-best-of-breed.gguf"
    );
    assert_eq!(without_the_part("notes.md"), "notes.md");
}
