//! No module that measures ships without saying what checks it (B-390, A19,
//! §6.16, F94).
//!
//! **The gap F94 found.** A19 — *anything reported is tested against an
//! independently known value* — was cited in fifty-eight files and none of the
//! ones that measure the machine. Four instrument defects followed, each with
//! an independent source available and unconsulted. The tier that compares
//! them was built; this is the half that stops the next instrument arriving
//! the way those four did.
//!
//! **Every module in the measuring crates declares one of three things**, in
//! its own module documentation where the next person to edit it will read it:
//!
//! - `**Cross-checked by test:**` — naming a test in the instrument tier,
//!   which this file looks up;
//! - `**Cross-checked by:**` — naming something that is not a test, such as
//!   two independent routes compared inside the reading itself;
//! - `**Not an instrument:**` — and why it measures nothing;
//! - `**Cross-check owed (B-390):**` — and what would serve.
//!
//! **The third is the honest one and it is a ratchet.** Five modules measure
//! something and are compared against nothing; pretending otherwise would be
//! worse than the gap. So the debt is counted, and the count may fall and may
//! not rise. A new instrument cannot be added to it — it has to be
//! cross-checked, or argued to measure nothing, and both of those are visible
//! in review.
//!
//! **Why a marker and not a list here.** A list in this file is a list that
//! goes stale the week it is written (F79) and, worse, is read by nobody
//! editing the instrument. A marker sits in the module it is about.

// Every item in this file is test code; see the note in `taxonomy_agreement.rs`.
#![allow(clippy::panic, clippy::expect_used, clippy::unwrap_used)]

use std::path::PathBuf;

/// The crates whose modules measure, or reduce measurements to a claim.
const MEASURING: [&str; 2] = ["crates/mcf-core/src/hardware", "crates/mcf-bench/src"];

/// How many modules measure something and are checked against nothing.
///
/// **May fall. May not rise.** Lowering it is the work B-390 names; raising it
/// is adding an instrument nobody can check, which is what this file exists to
/// stop.
const OWED: usize = 5;

/// A claim that names a test, which is looked up.
///
/// Separate from the prose form on purpose. The first version of this check
/// tried to tell a test name from any other backticked identifier and could
/// not — it read `routes_disagree_about`, a field, as a test that did not
/// exist. A claim that has to be *stated* rather than inferred is the same
/// lesson as `LocallyMeasured::new` not being a `From` (B-167): a claim should
/// be written.
const CROSS_CHECKED_BY_TEST: &str = "**Cross-checked by test:**";
const CROSS_CHECKED: &str = "**Cross-checked by:**";
const NOT_AN_INSTRUMENT: &str = "**Not an instrument:**";
const OWED_MARK: &str = "**Cross-check owed (B-390):**";

fn modules() -> Vec<PathBuf> {
    let root = mcf_checks::workspace::root();
    let mut found = Vec::new();
    for directory in MEASURING {
        let Ok(entries) = std::fs::read_dir(root.join(directory)) else {
            panic!("{directory} is a directory in this workspace");
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let named = path
                .file_name()
                .and_then(|held| held.to_str())
                .unwrap_or("");
            // `tests.rs` is a test module, not an instrument.
            if path.extension().is_some_and(|held| held == "rs") && named != "tests.rs" {
                found.push(path);
            }
        }
    }
    assert!(
        found.len() > 15,
        "only {} modules were found, so this check is reading the wrong tree",
        found.len()
    );
    found
}

/// Every module says which of the three it is.
#[test]
fn every_measuring_module_declares_what_checks_it() {
    let mut silent = Vec::new();
    for path in modules() {
        let source = std::fs::read_to_string(&path).unwrap_or_default();
        let declared = source.contains(CROSS_CHECKED)
            || source.contains(CROSS_CHECKED_BY_TEST)
            || source.contains(NOT_AN_INSTRUMENT)
            || source.contains(OWED_MARK);
        if !declared {
            silent.push(path.display().to_string());
        }
    }
    assert!(
        silent.is_empty(),
        "these modules measure, or sit beside things that do, and say nothing about what \
         checks them. A19 asks that anything reported be tested against an independently \
         known value, and four instrument defects followed from not asking (F94): {silent:#?}"
    );
}

/// The debt is what it is, and no larger.
#[test]
fn the_cross_checks_owed_do_not_grow() {
    let mut owed = Vec::new();
    for path in modules() {
        if std::fs::read_to_string(&path)
            .unwrap_or_default()
            .contains(OWED_MARK)
        {
            owed.push(path.display().to_string());
        }
    }
    assert!(
        owed.len() <= OWED,
        "{} modules now measure something and are checked against nothing, up from {OWED}. \
         Lowering this is the work B-390 names; raising it is adding an instrument nobody can \
         check: {owed:#?}",
        owed.len()
    );
    assert_eq!(
        owed.len(),
        OWED,
        "the debt has fallen to {}, which is good — lower `OWED` to match, so it cannot \
         quietly rise again",
        owed.len()
    );
}

/// A module that claims a cross-check names one that exists.
#[test]
fn a_named_cross_check_is_a_test_that_exists() {
    let tier = std::fs::read_to_string(
        mcf_checks::workspace::root()
            .join("checks/tests/instruments_agree_with_an_independent_source.rs"),
    )
    .expect("the instrument tier exists");
    let mut missing = Vec::new();
    for path in modules() {
        let source = std::fs::read_to_string(&path).unwrap_or_default();
        let Some((_, after)) = source.split_once(CROSS_CHECKED_BY_TEST) else {
            continue;
        };
        // The first backticked token after the marker is the test's name.
        // Only this form is looked up: the prose form names something that is
        // not a test and has nothing to look up.
        let Some(named) = after.split('`').nth(1) else {
            missing.push(format!(
                "{}: names no test after the marker",
                path.display()
            ));
            continue;
        };
        if !tier.contains(&format!("fn {named}")) {
            missing.push(format!("{}: names `{named}`", path.display()));
        }
    }
    assert!(
        missing.is_empty(),
        "a module claims a cross-check that is not a test in the instrument tier — which is a \
         cross-check that reads as done and is not: {missing:#?}"
    );
}
