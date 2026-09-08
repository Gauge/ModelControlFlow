#![allow(clippy::panic, clippy::expect_used)]

use mcf_record::content::Whose;

fn read(relative: &str) -> String {
    let path = mcf_checks::workspace::root().join(relative);
    std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("{} is readable: {error}", path.display()))
}

#[test]
fn the_two_categories_are_two_directories() {
    assert_ne!(Whose::User.directory(), Whose::Fixture.directory());
    assert_ne!(Whose::User.as_str(), Whose::Fixture.as_str());
    for name in ["suite", "benchmark", "other", ""] {
        assert_eq!(
            Whose::parse(name),
            None,
            "{name:?} parses as a category, and there are two"
        );
    }
}

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

#[test]
fn each_surface_files_its_traffic_where_it_belongs() {
    for (file, expected) in [
        ("crates/mcf-cli/src/run.rs", "Whose::User"),
        ("crates/mcf-cli/src/bench.rs", "Whose::User"),
        ("crates/mcf-serve/src/daemon.rs", "Whose::Fixture"),
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

#[test]
fn the_record_says_who_asked() {
    let daemon = read("crates/mcf-serve/src/daemon.rs");
    assert!(
        daemon.contains("\"asked_by\""),
        "a generation's record must say whose turn it was, or the two categories are \
         distinguishable in the store and not in the record (§3.4, B-146)"
    );
}
