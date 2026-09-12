use super::{COMPONENTS, Component};

fn llamas() -> Vec<&'static Component> {
    COMPONENTS
        .iter()
        .filter(|held| held.name.starts_with("llama.cpp"))
        .collect()
}

#[test]
fn every_component_names_an_image_by_digest_not_by_tag_alone() {
    for held in COMPONENTS {
        assert!(
            held.image_digest.starts_with("sha256:"),
            "{} names no digest",
            held.name
        );
        assert_eq!(
            held.image_digest.len(),
            71,
            "{} names a digest of the wrong length",
            held.name
        );
        assert!(
            held.image_digest
                .get(7..)
                .is_some_and(|hex| hex.chars().all(|ch| ch.is_ascii_hexdigit())),
            "{} names a digest that is not hexadecimal",
            held.name
        );
    }
}

#[test]
fn every_component_pins_a_full_commit() {
    for held in COMPONENTS {
        assert_eq!(held.commit.len(), 40, "{} pins a short commit", held.name);
        assert!(
            held.commit.chars().all(|ch| ch.is_ascii_hexdigit()),
            "{} pins something that is not a commit",
            held.name
        );
    }
}

#[test]
fn the_reference_engine_is_one_commit_across_every_back_end() {
    let built = llamas();
    assert!(built.len() >= 3, "a processor build and two card builds");
    let Some(first) = built.first() else {
        panic!("the reference engine is in the table");
    };
    for held in &built {
        assert_eq!(
            held.commit, first.commit,
            "{} is built from a different commit, so two figures from MCF would not be \
             comparable",
            held.name
        );
        assert_eq!(
            held.source, first.source,
            "{} is a different source",
            held.name
        );
    }
}

#[test]
fn every_back_end_builds_the_same_tools() {
    let built = llamas();
    let Some(first) = built.first() else {
        panic!("the reference engine is in the table");
    };
    for held in &built {
        assert_eq!(
            held.targets, first.targets,
            "{} builds a different set of tools",
            held.name
        );
        assert!(
            held.targets.contains(&"llama-server"),
            "{} builds no server to host with",
            held.name
        );
    }
}

#[test]
fn no_two_components_share_a_name() {
    let mut seen = std::collections::BTreeSet::new();
    for held in COMPONENTS {
        assert!(seen.insert(held.name), "{} appears twice", held.name);
    }
}
