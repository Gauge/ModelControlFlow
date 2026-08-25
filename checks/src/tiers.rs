//! The test tiers, declared once as data (B-191, D10, §6.34).
//!
//! D10 names ten disciplines that share the word "test", and §6.34 resolves the
//! tension between them: *tier the suites, gate on the fast one, schedule the
//! heavy ones, and report the age of every tier*. This module is that list,
//! written as a declaration the checks in `tests/tiers_conform.rs` compare the
//! repository against — the same shape [`workspace`] uses for the crate split,
//! and for the same reason: a paragraph describing an architecture drifts, and
//! a table the build compares against cannot.
//!
//! **Why a register at all.** A tier that exists but is not run is worse than
//! one that does not exist, because the suite reports the same green either
//! way. So each entry names the command that runs it and the file that holds
//! it, and the checks fail when a command is not in `scripts/ci.sh`, when a
//! file is not in the tree, or when `doc/build.md` describes a different set of
//! tiers than this one.
//!
//! **What is not here.** Tier *ages* — B-185 — and the mutation *floor* —
//! B-186. Both need somewhere to keep a previous result, which is B-300's
//! journal-and-index work. Until they exist, `scripts/ci.sh` prints which tiers
//! did not run in a given invocation, which is the honest half that is
//! available: "did not run" read as "passed" is A2's silent failure aimed at
//! the suite.
//!
//! [`workspace`]: crate::workspace

use std::path::PathBuf;

/// When a tier runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cadence {
    /// Runs on every change, in `scripts/ci.sh` with no flag.
    ///
    /// B19 and B38 make this tier's two obligations hermetic and fast: it is
    /// run constantly, and a gate people skip does not gate.
    Gating,
    /// Runs on a schedule and before a release, behind the named flag.
    Scheduled {
        /// The flag `scripts/ci.sh` takes, without its leading dashes.
        flag: &'static str,
    },
}

impl Cadence {
    /// The flag that runs this tier, if it takes one.
    #[must_use]
    pub const fn flag(self) -> Option<&'static str> {
        match self {
            Self::Gating => None,
            Self::Scheduled { flag } => Some(flag),
        }
    }
}

/// One tier of the suite.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Tier {
    /// What it is called, in `doc/build.md` and in this file.
    pub id: &'static str,
    /// The discipline D10 names, in one line.
    pub covers: &'static str,
    /// When it runs.
    pub cadence: Cadence,
    /// The command that runs it, as `scripts/ci.sh` invokes it.
    ///
    /// A substring of the script, checked to be present: a tier whose command
    /// is not in the one script that gates a change is a tier nobody runs.
    pub command: &'static str,
    /// Where it lives, relative to the workspace root.
    ///
    /// Every path must exist. A register naming a file that was deleted is a
    /// register that reads as coverage MCF does not have.
    pub holds: &'static [&'static str],
}

impl Tier {
    /// Whether this tier gates every change.
    #[must_use]
    pub const fn gates(&self) -> bool {
        matches!(self.cadence, Cadence::Gating)
    }
}

/// The ten tiers D10 names, in the order D10 names them.
///
/// The order is not arbitrary: it runs from the cheapest and most local to the
/// most expensive and most global, which is also the order in which a defect is
/// cheapest to find. A tier that could be moved earlier in this list without
/// getting slower should be.
pub const TIERS: &[Tier] = &[
    Tier {
        id: "unit",
        covers: "logic, in the crate that owns it",
        cadence: Cadence::Gating,
        command: "cargo test --workspace --locked --offline",
        holds: &[
            "crates/mcf-core/src/measurement/tests.rs",
            "crates/mcf-record/src/journal/tests.rs",
            "checks/src/property.rs",
        ],
    },
    Tier {
        id: "property",
        covers: "invariants MCF claims universally, over generated inputs",
        cadence: Cadence::Gating,
        command: "cargo test --workspace --locked --offline",
        holds: &["checks/tests/properties.rs", "checks/src/property.rs"],
    },
    Tier {
        id: "functional",
        covers: "behaviour at the API surface, and the rules the workspace enforces about itself",
        cadence: Cadence::Gating,
        command: "cargo test --workspace --locked --offline",
        holds: &[
            "checks/tests",
            "crates/mcf-record/tests",
            "crates/mcf-lab/tests",
        ],
    },
    Tier {
        id: "whole-system",
        covers: "the binary as a process, against a real record, including restart and a kill",
        cadence: Cadence::Gating,
        command: "cargo test --workspace --locked --offline",
        holds: &["crates/mcf-cli/tests/whole_system.rs"],
    },
    Tier {
        id: "fault-injection",
        covers: "every failure MCF claims to handle, reproduced from the laboratory's catalogue",
        cadence: Cadence::Gating,
        command: "cargo test --workspace --locked --offline",
        holds: &[
            "crates/mcf-lab/tests/scenarios_reproduce.rs",
            "checks/tests/fault_catalogue.rs",
        ],
    },
    Tier {
        id: "fuzz",
        covers: "the parsers that read bytes MCF did not write",
        cadence: Cadence::Scheduled { flag: "with-fuzz" },
        command: "cargo test --locked --offline -p mcf-checks --test fuzz -- --ignored --nocapture",
        holds: &["checks/tests/fuzz.rs", "checks/src/fuzz.rs"],
    },
    Tier {
        id: "load",
        covers: "MCF's claims under many callers at once, against the simulated laboratory",
        cadence: Cadence::Scheduled { flag: "with-load" },
        command: "cargo test --locked --offline -p mcf-checks --test load -- --ignored --nocapture",
        holds: &["checks/tests/load.rs"],
    },
    Tier {
        id: "soak",
        covers: "drift over a long run: descriptors, directories, memory that grows with operations",
        cadence: Cadence::Scheduled { flag: "with-soak" },
        command: "cargo test --locked --offline -p mcf-checks --test soak -- --ignored --nocapture --test-threads=1",
        holds: &["checks/tests/soak.rs"],
    },
    Tier {
        id: "performance",
        covers: "D24's budgets, on the release artifact, read as D27 says to read them",
        cadence: Cadence::Scheduled {
            flag: "with-budget",
        },
        command: "cargo test --release --locked --offline -p mcf-cli --test budget -- --ignored --nocapture",
        holds: &["crates/mcf-cli/tests/budget.rs"],
    },
    Tier {
        id: "mutation",
        covers: "the test of the tests: a suite that does not fail when the code is broken",
        cadence: Cadence::Scheduled {
            flag: "with-mutation",
        },
        command: "scripts/check-mutants.sh",
        holds: &["scripts/check-mutants.sh"],
    },
];

/// The tier with this identifier, if there is one.
#[must_use]
pub fn find(id: &str) -> Option<&'static Tier> {
    TIERS.iter().find(|tier| tier.id == id)
}

/// The path `scripts/ci.sh` lives at.
#[must_use]
pub fn ci_script() -> PathBuf {
    crate::workspace::root().join("scripts").join("ci.sh")
}

#[cfg(test)]
mod tests {
    use super::{Cadence, TIERS, find};

    /// Every tier is named once. A duplicate identifier would make the checks
    /// in `tests/tiers_conform.rs` pass twice on one tier and never on
    /// another.
    #[test]
    fn the_identifiers_are_unique() {
        let mut seen: Vec<&str> = TIERS.iter().map(|tier| tier.id).collect();
        seen.sort_unstable();
        let count = seen.len();
        seen.dedup();
        assert_eq!(seen.len(), count, "two tiers share an identifier");
    }

    /// Every scheduled tier has a flag, and no gating tier does. The
    /// distinction is the whole point of the register: a gating tier behind a
    /// flag would not gate.
    #[test]
    fn only_scheduled_tiers_have_flags() {
        for tier in TIERS {
            match tier.cadence {
                Cadence::Gating => assert!(tier.gates() && tier.cadence.flag().is_none()),
                Cadence::Scheduled { flag } => {
                    assert!(!tier.gates());
                    assert!(!flag.is_empty(), "{} has an empty flag", tier.id);
                    assert!(
                        !flag.starts_with('-'),
                        "{} spells its flag with dashes; the register holds the name",
                        tier.id
                    );
                }
            }
        }
    }

    #[test]
    fn a_tier_can_be_found_by_name() {
        assert_eq!(find("mutation").map(|tier| tier.id), Some("mutation"));
        assert!(find("integration").is_none());
    }
}
