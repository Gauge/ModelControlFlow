#![allow(clippy::expect_used, clippy::panic)]

use mcf_checks::taxonomy::Taxonomy;
use mcf_core::failure::{Attribution, Category, Disposition, Domain};

fn taxonomy() -> Taxonomy {
    Taxonomy::read().expect("doc/taxonomy.md is readable")
}

#[test]
fn the_codes_are_the_same_list_in_the_same_order() {
    let taxonomy = taxonomy();
    let documented: Vec<&str> = taxonomy.codes.iter().map(|c| c.code.as_str()).collect();
    let implemented: Vec<&str> = Category::ALL.iter().map(|c| c.code()).collect();
    assert_eq!(
        documented, implemented,
        "doc/taxonomy.md and mcf_core::failure::Category disagree"
    );
}

#[test]
fn every_code_means_what_the_document_says() {
    for entry in &taxonomy().codes {
        let category = Category::from_code(&entry.code)
            .unwrap_or_else(|| panic!("{} has no variant", entry.code));
        assert_eq!(
            category.meaning(),
            entry.meaning,
            "{} means something different in the code",
            entry.code
        );
    }
}

#[test]
fn every_code_is_in_the_domain_whose_table_holds_it() {
    for entry in &taxonomy().codes {
        let category = Category::from_code(&entry.code)
            .unwrap_or_else(|| panic!("{} has no variant", entry.code));
        assert_eq!(
            category.domain().prefix(),
            entry.domain_prefix,
            "{} is filed under the wrong domain",
            entry.code
        );
    }
}

#[test]
fn the_domains_are_the_same_list() {
    let documented = taxonomy().domains;
    let implemented = Domain::ALL;
    assert_eq!(documented.len(), implemented.len());
    for (position, (entry, domain)) in documented.iter().zip(implemented).enumerate() {
        assert_eq!(entry.number, position + 1, "the document skips a number");
        assert_eq!(entry.prefix, domain.prefix());
        assert_eq!(entry.subject, domain.subject(), "{} drifted", entry.prefix);
    }
}

#[test]
fn the_axes_are_the_same_lists() {
    let taxonomy = taxonomy();
    let attributions: Vec<&str> = Attribution::ALL.iter().map(|a| a.as_str()).collect();
    assert_eq!(taxonomy.attributions, attributions);
    let dispositions: Vec<&str> = Disposition::ALL.iter().map(|d| d.as_str()).collect();
    assert_eq!(taxonomy.dispositions, dispositions);
}

#[test]
fn the_documentation_states_the_size_the_taxonomy_is() {
    let taxonomy = taxonomy();
    let stated = format!(
        "three axes, {} domains, {} codes",
        spell(taxonomy.domains.len()),
        taxonomy.codes.len()
    );
    let root = mcf_checks::workspace::root();
    for document in ["README.md", "doc/backlog.md"] {
        let source = std::fs::read_to_string(root.join(document))
            .unwrap_or_else(|error| panic!("{document} is readable: {error}"));
        assert!(
            source.contains(&stated),
            "{document} does not state {stated:?}"
        );
    }
}

fn spell(count: usize) -> String {
    match count {
        16 => "sixteen".to_owned(),
        other => other.to_string(),
    }
}

#[test]
fn nothing_constructs_the_unclassified_category() {
    let mut offenders = Vec::new();
    for path in rust_sources(&mcf_checks::workspace::root().join("crates")) {
        if path.components().any(|c| c.as_os_str() == "failure") {
            continue;
        }
        let source = std::fs::read_to_string(&path).expect("a source file is readable");
        for (number, line) in source.lines().enumerate() {
            if line.contains("InternalUnclassified") {
                offenders.push(format!("{}:{}", path.display(), number + 1));
            }
        }
    }
    assert!(
        offenders.is_empty(),
        "internal.unclassified is constructed at {offenders:?}; its target is zero, \
         and the fix is a new category with a scenario (A13)"
    );
}

fn rust_sources(directory: &std::path::Path) -> Vec<std::path::PathBuf> {
    let mut found = Vec::new();
    let Ok(entries) = std::fs::read_dir(directory) else {
        return found;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            found.extend(rust_sources(&path));
        } else if path.extension().is_some_and(|e| e == "rs") {
            found.push(path);
        }
    }
    found.sort();
    found
}
