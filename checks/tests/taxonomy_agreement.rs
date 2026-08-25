//! The taxonomy in the document and the taxonomy in the code are one list.
//!
//! A13 states the principle for the laboratory's fault catalogue; B-010 makes
//! that check and waits on DEC-021. This is the half that is available now,
//! and it stands on its own: `crates/mcf-core/src/failure/category.rs` is
//! generated from `doc/taxonomy.md` and then committed, so without a check the
//! two drift the first time someone edits one of them.
//!
//! The comparison is exact in both directions. A code in the document with no
//! variant is an unimplemented claim; a variant with no row in the document is
//! a code that was never designed, and C5 makes it permanent the moment it
//! reaches a record.

// Every item in this file is test code. clippy's in-test allowance keys on the
// `#[test]` attribute, so the helpers the tests share need it stated: a helper
// whose precondition does not hold has nothing to return and no record to write
// to, and failing loudly is the honest outcome (B-003 draws the line at
// non-test code).
#![allow(clippy::expect_used, clippy::panic)]

use mcf_checks::taxonomy::Taxonomy;
use mcf_core::failure::{Attribution, Category, Disposition, Domain};

fn taxonomy() -> Taxonomy {
    Taxonomy::read().expect("doc/taxonomy.md is readable")
}

/// The codes are the same list, in the same order. Order matters because the
/// document's order is the domains' order, and reading either as a reference
/// should reach the same place.
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

/// Every code means what the document says it means. A variant whose
/// documentation has drifted is a variant whose consumers were told something
/// the document no longer says.
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

/// Every code is filed under the domain whose table it appears in.
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

/// The domains are the same list, in the same order, with the same subjects.
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

/// The other two axes are the same lists too. They are a smaller surface than
/// the codes and a more load-bearing one: §3.8's question and §3.2's mark are
/// answered with these values.
#[test]
fn the_axes_are_the_same_lists() {
    let taxonomy = taxonomy();
    let attributions: Vec<&str> = Attribution::ALL.iter().map(|a| a.as_str()).collect();
    assert_eq!(taxonomy.attributions, attributions);
    let dispositions: Vec<&str> = Disposition::ALL.iter().map(|d| d.as_str()).collect();
    assert_eq!(taxonomy.dispositions, dispositions);
}

/// Wherever the documentation states the taxonomy's size, it states the size
/// the taxonomy actually is.
///
/// Two documents summarize it — the README's document map and the backlog's
/// DEC-010 row — and a summary that has drifted from what it summarizes is the
/// stale-figure defect that makes a register unusable for deciding what is
/// left. B-041 generalizes this to every citation in `doc/`; this is the
/// instance that exists because the taxonomy is the first document the code
/// has an opinion about.
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

/// `internal.unclassified` is a tracked defect metric with a target of zero,
/// so no code outside its own definition may construct it. When a real
/// unclassified failure appears the fix is a new category (A13), not a call
/// site here.
#[test]
fn nothing_constructs_the_unclassified_category() {
    let mut offenders = Vec::new();
    for path in rust_sources(&mcf_checks::workspace::root().join("crates")) {
        // `mcf-core::failure` is where the metric is defined and where the
        // tests that assert it exists live. The exemption is the module, not
        // the file, because the predicate that reports the metric has to name
        // the variant too — and that module is small, is entirely about the
        // taxonomy, and is read by anyone changing it.
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
