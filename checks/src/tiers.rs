use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cadence {
    Gating,
    Scheduled { flag: &'static str },
}

impl Cadence {
    #[must_use]
    pub const fn flag(self) -> Option<&'static str> {
        match self {
            Self::Gating => None,
            Self::Scheduled { flag } => Some(flag),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Tier {
    pub id: &'static str,
    pub covers: &'static str,
    pub cadence: Cadence,
    pub command: &'static str,
    pub holds: &'static [&'static str],
}

impl Tier {
    #[must_use]
    pub const fn gates(&self) -> bool {
        matches!(self.cadence, Cadence::Gating)
    }
}

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
        holds: &[
            "crates/mcf-cli/tests/whole_system.rs",
            "crates/mcf-cli/tests/untrusted_cannot_elevate.rs",
        ],
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
        holds: &[
            "scripts/check-mutants.sh",
            "checks/tests/the_mutation_catalogue_still_fits.rs",
        ],
    },
];

#[must_use]
pub fn find(id: &str) -> Option<&'static Tier> {
    TIERS.iter().find(|tier| tier.id == id)
}

#[must_use]
pub fn ci_script() -> PathBuf {
    crate::workspace::root().join("scripts").join("ci.sh")
}

#[cfg(test)]
mod tests {
    use super::{Cadence, TIERS, find};

    #[test]
    fn the_identifiers_are_unique() {
        let mut seen: Vec<&str> = TIERS.iter().map(|tier| tier.id).collect();
        seen.sort_unstable();
        let count = seen.len();
        seen.dedup();
        assert_eq!(seen.len(), count, "two tiers share an identifier");
    }

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
