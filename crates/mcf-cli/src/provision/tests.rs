//! What the recipe table has to keep true without a container in sight.

use super::{COMPONENTS, Component, prefix_for, script_for};

/// Every component pins everything a rerun needs: an image digest, a full
/// commit, at least one package and one target.
#[test]
fn every_component_is_fully_pinned() {
    assert!(!COMPONENTS.is_empty());
    for component in COMPONENTS {
        assert!(
            component.image_digest.starts_with("sha256:") && component.image_digest.len() == 71,
            "{}: an image digest pins or it does not: {}",
            component.name,
            component.image_digest
        );
        assert!(
            component.commit.len() == 40 && component.commit.chars().all(|c| c.is_ascii_hexdigit()),
            "{}: a commit pin is the whole hash: {}",
            component.name,
            component.commit
        );
        assert!(!component.packages.is_empty(), "{}", component.name);
        assert!(!component.targets.is_empty(), "{}", component.name);
        assert!(
            component.source.starts_with("https://"),
            "{}: source over an unencrypted wire: {}",
            component.name,
            component.source
        );
    }
}

/// The script quotes nothing from outside the table, checks out the pinned
/// commit, and writes the three files the recording step reads back.
#[test]
fn the_script_is_the_recipe_and_only_the_recipe() {
    for component in COMPONENTS {
        let script = script_for(component);
        assert!(script.contains(component.commit), "{}", component.name);
        assert!(script.contains(component.source), "{}", component.name);
        for needed in ["toolchain.txt", "commit.txt", "errexit"] {
            assert!(script.contains(needed), "{}: no {needed}", component.name);
        }
        // Portability is part of the recipe: a tuned build is a condition
        // nobody can restate elsewhere.
        assert!(
            script.contains("-DGGML_NATIVE=OFF"),
            "{}: the build must not tune to this machine",
            component.name
        );
    }
}

/// Two pins of one component are two prefixes.
#[test]
fn a_prefix_names_the_component_and_the_commit() {
    let component = &COMPONENTS[0];
    let prefix = prefix_for(component, std::path::Path::new("/somewhere"));
    let name = prefix.file_name().and_then(|n| n.to_str()).unwrap_or("");
    assert!(name.starts_with(component.name), "{name}");
    assert!(name.contains('@'), "{name}");
    assert!(name.ends_with(&component.commit[..12]), "{name}");
}

/// A second component could not silently collide with the first.
#[test]
fn component_names_are_unique() {
    let mut names: Vec<&str> = COMPONENTS.iter().map(|c| c.name).collect();
    names.sort_unstable();
    names.dedup();
    assert_eq!(names.len(), COMPONENTS.len());
}

/// The struct stays constructible in tests without a container: this is the
/// compile-time shape the table depends on.
#[test]
fn a_component_is_data() {
    let component = Component {
        name: "example",
        role: "a test",
        image: "example:1",
        image_digest: "sha256:0000000000000000000000000000000000000000000000000000000000000000",
        source: "https://example.invalid/repo.git",
        commit: "0000000000000000000000000000000000000000",
        packages: &["nothing"],
        configure: &[],
        targets: &["something"],
    };
    assert_eq!(component.name, "example");
}
