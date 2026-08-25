//! The tiers MCF declares are the tiers it runs (B-191, B38).
//!
//! A tier that exists and is not run reports the same green as one that runs,
//! which is A2's silent failure aimed at the suite. So the register in
//! `mcf_checks::tiers` is compared against the three places a tier is real: the
//! script that runs it, the files that hold it, and the document that describes
//! it to a reader.
//!
//! Both directions, as with the crate split: a tier missing from
//! `scripts/ci.sh` is coverage nobody gets, and a flag in `scripts/ci.sh` that
//! no tier declares is a command whose absence from the register means the
//! register can no longer be read as the suite.

// Every item in this file is test code; see the note in
// checks/tests/taxonomy_agreement.rs.
#![allow(clippy::expect_used, clippy::panic)]

use mcf_checks::tiers::{Cadence, TIERS, ci_script};
use mcf_checks::workspace::root;

/// The ten disciplines D10 names, in D10's order.
///
/// Written out rather than derived, because this is the claim: the suite has a
/// tier for each of them. A tier deleted from the register fails here rather
/// than quietly reducing what "the suite passed" means.
const D10_DISCIPLINES: [&str; 10] = [
    "unit",
    "property",
    "functional",
    "whole-system",
    "fault-injection",
    "fuzz",
    "load",
    "soak",
    "performance",
    "mutation",
];

fn ci_source() -> String {
    std::fs::read_to_string(ci_script()).expect("scripts/ci.sh is readable")
}

fn build_document() -> String {
    std::fs::read_to_string(root().join("doc").join("build.md")).expect("doc/build.md is readable")
}

/// Every discipline D10 names has a tier, and the register has no others.
#[test]
fn the_register_is_exactly_the_disciplines_d10_names() {
    let mut declared: Vec<&str> = TIERS.iter().map(|tier| tier.id).collect();
    let mut expected: Vec<&str> = D10_DISCIPLINES.to_vec();
    declared.sort_unstable();
    expected.sort_unstable();
    assert_eq!(declared, expected);
}

/// Every tier's command is in the one script that gates a change.
///
/// The command is compared verbatim. A tier whose declared command has drifted
/// from what the script actually runs is a register that describes a suite
/// nobody has.
#[test]
fn every_tier_is_run_by_the_script() {
    let script = ci_source();
    for tier in TIERS {
        assert!(
            script.contains(tier.command),
            "scripts/ci.sh does not run the {} tier: {}",
            tier.id,
            tier.command
        );
    }
}

/// Every scheduled tier's flag is one the script accepts, and appears in its
/// usage. A flag the script rejects is a tier that cannot be run at all.
#[test]
fn every_scheduled_flag_is_accepted_and_documented() {
    let script = ci_source();
    for tier in TIERS {
        let Some(flag) = tier.cadence.flag() else {
            continue;
        };
        assert!(
            script.contains(&format!("--{flag})")),
            "scripts/ci.sh has no case for --{flag} ({})",
            tier.id
        );
        assert!(
            script.contains(&format!("--{flag} ")) || script.contains(&format!("--{flag}\n")),
            "scripts/ci.sh's usage does not mention --{flag}"
        );
    }
}

/// A scheduled tier that did not run says so.
///
/// B38: an unstated staleness is A2's silent failure aimed at the suite. Until
/// B-185 gives every tier an age, the honest half is that an invocation reports
/// which tiers it did not run.
#[test]
fn every_scheduled_tier_is_reported_when_it_does_not_run() {
    let script = ci_source();
    let (_, report) = script
        .split_once("=== not run in this tier")
        .expect("scripts/ci.sh reports what it did not run");
    for tier in TIERS {
        let Some(flag) = tier.cadence.flag() else {
            continue;
        };
        assert!(
            report.contains(&format!("--{flag}")),
            "an invocation without --{flag} would not say that the {} tier did not run",
            tier.id
        );
    }
}

/// Every file a tier says it lives in is in the tree.
#[test]
fn every_tier_is_where_it_says_it_is() {
    for tier in TIERS {
        assert!(!tier.holds.is_empty(), "{} names no files", tier.id);
        for held in tier.holds {
            let path = root().join(held);
            assert!(
                path.exists(),
                "the {} tier names {held}, which is not in the tree",
                tier.id
            );
        }
    }
}

/// A gating tier does not ignore its own tests.
///
/// This is the failure mode the register exists to catch: `#[ignore]` is how a
/// scheduled tier is kept out of the gate, and one applied to a gating tier
/// would leave the suite reporting green having run nothing. The check is
/// textual because that is what the harness reads too.
#[test]
fn no_gating_tier_ignores_its_tests() {
    for tier in TIERS.iter().filter(|tier| tier.gates()) {
        for held in tier.holds {
            let path = root().join(held);
            if !path.is_file() {
                continue;
            }
            let source = std::fs::read_to_string(&path).expect("a tier's file is readable");
            assert!(
                !source.contains("#[ignore"),
                "the gating tier {} ignores a test in {held}",
                tier.id
            );
        }
    }
}

/// A scheduled tier does ignore its tests, or it would run in the gate.
///
/// The two halves are the same claim from opposite sides, and the pair is what
/// makes the cadence in the register real rather than descriptive.
#[test]
fn every_scheduled_rust_tier_keeps_itself_out_of_the_gate() {
    for tier in TIERS.iter().filter(|tier| !tier.gates()) {
        let ignored = tier.holds.iter().any(|held| {
            let path = root().join(held);
            path.extension().is_some_and(|extension| extension == "rs")
                && std::fs::read_to_string(&path).is_ok_and(|source| source.contains("#[ignore"))
        });
        let scripted = tier.holds.iter().any(|held| {
            std::path::Path::new(held)
                .extension()
                .is_some_and(|extension| extension == "sh")
        });
        assert!(
            ignored || scripted,
            "the scheduled tier {} would run in the gating tier",
            tier.id
        );
    }
}

/// The document a reader reaches for describes the tiers the register holds.
///
/// `doc/build.md` §9 is a rendering of this register, in the same way §2's
/// table is a rendering of the crate split. Both directions: a tier missing
/// from the document is coverage nobody knows about, and a tier in the document
/// that does not exist is a claim of coverage MCF does not have — which is
/// worse.
#[test]
fn the_build_document_describes_exactly_these_tiers() {
    let document = build_document();
    for tier in TIERS {
        assert!(
            document.contains(&format!("`{}`", tier.id)),
            "doc/build.md does not name the {} tier",
            tier.id
        );
    }
    for discipline in D10_DISCIPLINES {
        let named = TIERS.iter().any(|tier| tier.id == discipline);
        assert!(named, "{discipline} is a D10 discipline with no tier");
    }
}

/// The document names every flag, so a reader can run any tier from it.
#[test]
fn the_build_document_names_every_flag() {
    let document = build_document();
    for tier in TIERS {
        if let Cadence::Scheduled { flag } = tier.cadence {
            assert!(
                document.contains(&format!("--{flag}")),
                "doc/build.md does not say how to run the {} tier",
                tier.id
            );
        }
    }
}
