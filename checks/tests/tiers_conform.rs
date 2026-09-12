#![allow(clippy::expect_used, clippy::panic)]

use mcf_checks::tiers::{Cadence, TIERS, ci_script};
use mcf_checks::workspace::root;

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

fn ages_script() -> String {
    std::fs::read_to_string(root().join("scripts").join("check-tier-ages.sh"))
        .expect("scripts/check-tier-ages.sh is readable")
}

#[test]
fn the_register_is_exactly_the_disciplines_d10_names() {
    let mut declared: Vec<&str> = TIERS.iter().map(|tier| tier.id).collect();
    let mut expected: Vec<&str> = D10_DISCIPLINES.to_vec();
    declared.sort_unstable();
    expected.sort_unstable();
    assert_eq!(declared, expected);
}

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

#[test]
fn every_scheduled_tier_is_reported_when_it_does_not_run() {
    let script = ci_source();
    let (_, report) = script
        .split_once("=== not run in this invocation")
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

#[test]
fn every_scheduled_tier_is_stamped_when_it_passes() {
    let script = ci_source();
    for tier in TIERS {
        if tier.gates() {
            continue;
        }
        assert!(
            script.contains(&format!("tier_stamp \"$root\" {}", tier.id)),
            "scripts/ci.sh does not stamp the {} tier when it passes",
            tier.id
        );
    }
}

#[test]
fn the_age_check_knows_exactly_the_scheduled_tiers() {
    let script = ages_script();
    let listed = script
        .split_once("declare -a scheduled=(")
        .and_then(|(_, rest)| rest.split_once(')'))
        .map(|(names, _)| names.to_owned())
        .expect("scripts/check-tier-ages.sh lists the scheduled tiers");
    let mut listed: Vec<&str> = listed.split_whitespace().collect();

    let mut scheduled: Vec<&str> = TIERS
        .iter()
        .filter(|tier| !tier.gates())
        .map(|tier| tier.id)
        .collect();

    listed.sort_unstable();
    scheduled.sort_unstable();
    assert_eq!(listed, scheduled);
}

#[test]
fn the_age_check_names_the_flag_that_clears_each_refusal() {
    let script = ages_script();
    for tier in TIERS.iter().filter(|tier| !tier.gates()) {
        let Some(flag) = tier.cadence.flag() else {
            continue;
        };
        assert!(
            script.contains(&format!("--{flag}")),
            "scripts/check-tier-ages.sh does not name --{flag} for the {} tier",
            tier.id
        );
    }
}
