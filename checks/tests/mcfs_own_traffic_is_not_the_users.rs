//! Suite data and user traffic are different categories, structurally
//! (B-146, §6.8, A17, A25, B9, F114).
//!
//! **§6.8 says why the separation must be structural**: *benchmark suites —
//! whose content is fixture data, not user data — may be recorded in full, and
//! this distinction is exactly why suite data and user traffic must be
//! structurally separated rather than separated by convention.* A flag on one
//! store is separation by convention. It is right until something forgets to
//! set it, and what it protects is the operator's most sensitive text.
//!
//! **It was not academic.** Seven hours after MCF's content store began holding
//! anything, 614 files were in it, and most were MCF's own probe traffic — the
//! chat-template probe's three constant questions and a model's answers — filed
//! beside a person's `mcf run` and indistinguishable from it (F114).
//!
//! **What is structural here.** Two types with no conversion (`Whose::User`,
//! `Whose::Fixture`), two directories, and a store that is opened *for* a
//! category and can write only there. Which one a turn is travels on the wire
//! with the request, because the daemon cannot tell a probe's constant question
//! from a person's prompt by looking at it — they arrive on the same socket in
//! the same shape.

// Every item in this file is test code; see the note in `taxonomy_agreement.rs`.
#![allow(clippy::panic, clippy::expect_used)]

use mcf_record::content::Whose;

fn read(relative: &str) -> String {
    let path = mcf_checks::workspace::root().join(relative);
    std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("{} is readable: {error}", path.display()))
}

/// The two categories live in two places and neither is the other.
#[test]
fn the_two_categories_are_two_directories() {
    assert_ne!(Whose::User.directory(), Whose::Fixture.directory());
    assert_ne!(Whose::User.as_str(), Whose::Fixture.as_str());
    // A third category would be a third retention policy nobody has decided
    // (DEC-005), and a reader meeting one would have to guess which of the two
    // rules it follows.
    for name in ["suite", "benchmark", "other", ""] {
        assert_eq!(
            Whose::parse(name),
            None,
            "{name:?} parses as a category, and there are two"
        );
    }
}

/// The category is not a setting.
///
/// "Cannot be defeated by configuration" is B-146's done-when. What decides is
/// the surface that made the request, in code, at the point the request is
/// built — so there is nothing to configure and nothing to get wrong twice.
#[test]
fn nothing_reads_the_category_from_configuration() {
    for file in [
        "crates/mcf-record/src/content.rs",
        "crates/mcf-serve/src/control.rs",
        "crates/mcf-serve/src/daemon.rs",
    ] {
        let source = read(file);
        for setting in ["env::var", "var_os", "config", "setting"] {
            assert!(
                !source.contains(&format!("{setting}(\"MCF_WHOSE")),
                "{file} reads the content category from the environment, which makes §6.8's \
                 split a configuration rather than a structure (B-146)"
            );
        }
    }
    assert!(
        !read("crates/mcf-record/src/content.rs").contains("impl Default for Whose"),
        "a default category is a category something can fall into without saying so"
    );
}

/// Every surface that asks for a generation says whose text it is.
///
/// The compiler requires the field — there is no `Default` — so what this adds
/// is that no site fills it in with the *wrong* one by copying a neighbour: MCF
/// asking a model something is fixture, and a person's prompt is theirs.
#[test]
fn each_surface_files_its_traffic_where_it_belongs() {
    for (file, expected) in [
        ("crates/mcf-cli/src/run.rs", "Whose::User"),
        ("crates/mcf-cli/src/bench.rs", "Whose::User"),
        ("crates/mcf-cli/src/crosscheck.rs", "Whose::Fixture"),
        ("crates/mcf-serve/src/probes.rs", "Whose::Fixture"),
        ("crates/mcf-serve/src/cost.rs", "Whose::Fixture"),
    ] {
        let source = read(file);
        assert!(
            source.contains(expected),
            "{file} builds a generation request and does not name {expected}. A probe's \
             constant question is fixture data and a person's prompt is theirs (§6.8, B-146)"
        );
        let other = if expected.ends_with("User") {
            "Whose::Fixture"
        } else {
            "Whose::User"
        };
        assert!(
            !source.contains(other),
            "{file} names both categories, so which one its traffic is depends on which line \
             ran (B-146)"
        );
    }
}

/// A store can only write where its category says.
#[test]
fn a_store_writes_only_in_its_own_directory() {
    let source = read("crates/mcf-record/src/content.rs");
    assert!(
        source.contains("pub fn open_for(path: &Path, whose: Whose)"),
        "a store must be opened for a category, so that what it writes is decided once"
    );
    assert!(
        source.contains("whose: Whose,") && source.contains("pub const fn whose(&self)"),
        "and must carry the category it was opened for, or nothing can check what it holds"
    );
    // `keep` writes under `self.path`, which came from the category. A `keep`
    // that took a path or a category would be one a caller could aim.
    let (_, keep) = source
        .split_once("pub fn keep(")
        .expect("the store keeps content");
    let body = keep.split_once("\n    }").map_or(keep, |(held, _)| held);
    for aimed in ["whose:", "Whose::", "directory("] {
        assert!(
            !body.contains(aimed),
            "`keep` takes the category as an argument ({aimed}), so a caller can file a \
             person's text in MCF's store by passing a different value (B-146)"
        );
    }
}

/// And the record says which kind of turn it was.
///
/// Whose text a generation was is a *condition* of it, not content: a turn MCF
/// asked for is a different experiment from one a person asked for, and §3.4
/// makes that a thing the record carries.
#[test]
fn the_record_says_who_asked() {
    let daemon = read("crates/mcf-serve/src/daemon.rs");
    assert!(
        daemon.contains("\"asked_by\""),
        "a generation's record must say whose turn it was, or the two categories are \
         distinguishable in the store and not in the record (§3.4, B-146)"
    );
}
