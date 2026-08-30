//! The declared shape of the workspace, and how to read the real one.
//!
//! B-001 fixes the crate split and the direction of its dependency edges. This
//! module states that shape once, as data, so that the checks in
//! `tests/workspace_shape.rs` compare the repository against a declaration
//! rather than against a second copy of itself.

use std::path::{Path, PathBuf};

use crate::manifest::{Manifest, ManifestError};

/// One member crate, and what it is permitted to depend on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Member {
    /// The crate's package name.
    pub name: &'static str,
    /// Its directory, relative to the workspace root.
    pub path: &'static str,
    /// The MCF crates it is permitted to depend on.
    ///
    /// The list is exact, not a maximum: a crate that does not yet take an
    /// edge it is permitted still declares it here only when it takes it, so
    /// the check reports both a missing edge and an added one.
    pub depends_on: &'static [&'static str],
}

/// The workspace as B-001 declares it.
///
/// The order is the layering: every entry's `depends_on` names only entries
/// above it, which is what makes the graph acyclic by inspection as well as by
/// test.
pub const MEMBERS: &[Member] = &[
    Member {
        name: "mcf-core",
        path: "crates/mcf-core",
        depends_on: &[],
    },
    Member {
        name: "mcf-record",
        path: "crates/mcf-record",
        depends_on: &["mcf-core"],
    },
    Member {
        name: "mcf-standin",
        path: "crates/mcf-standin",
        // MCF's own implementation of inference (D31, B-360). It depends on
        // `mcf-core` for the failure type and the engine handles, and on
        // nothing else: it is an engine, not an adapter, and the crate boundary
        // is what keeps a stand-in's output from reaching a surface except
        // through the handle that carries its mark (A5, B65).
        depends_on: &["mcf-core"],
    },
    Member {
        name: "mcf-helper",
        path: "crates/mcf-helper",
        // The privileged helper (B-190, D35, §6.32). It links `mcf-core` and
        // nothing else, on purpose: *auditable* is half of what §6.32 asks for,
        // and a program that runs with rights the daemon does not have should
        // be readable in one sitting. Nothing depends on it — the daemon starts
        // it as a process rather than calling into it, which is the whole point
        // of the split.
        depends_on: &["mcf-core"],
    },
    Member {
        name: "mcf-hub",
        path: "crates/mcf-hub",
        depends_on: &["mcf-core", "mcf-record"],
    },
    Member {
        name: "mcf-serve",
        path: "crates/mcf-serve",
        // `mcf-hub` because a daemon is asked what models this machine is
        // holding, and what is held is the store's answer rather than a second
        // reader that could disagree with it (B-030, §VI). `mcf-standin`
        // because the daemon serves generations (B-034), and MCF's own engine
        // is the one that runs in its process — a provisioned engine will be
        // a supervised subprocess, which is an edge to a process rather than
        // to a crate (B-032).
        depends_on: &["mcf-core", "mcf-record", "mcf-hub", "mcf-standin"],
    },
    Member {
        name: "mcf-lab",
        path: "crates/mcf-lab",
        // `mcf-standin`, `mcf-hub` and `mcf-serve` because A13 requires a
        // scenario for every category MCF's code constructs, and a scenario
        // that produced one by hand would be a scenario about a mock (D26).
        // That is why the laboratory sits above everything it reproduces
        // failures for, and why this list grows when a new crate starts
        // classifying something.
        depends_on: &[
            "mcf-core",
            "mcf-helper",
            "mcf-hub",
            "mcf-record",
            "mcf-serve",
            "mcf-standin",
        ],
    },
    Member {
        name: "mcf-bench",
        path: "crates/mcf-bench",
        depends_on: &["mcf-core", "mcf-record", "mcf-serve"],
    },
    Member {
        name: "mcf-tui",
        path: "crates/mcf-tui",
        // A second surface, and a client of the control plane exactly as the
        // command line is (A22, B-072). It depends on `mcf-serve` for the
        // requests it sends and on `mcf-record` for the shape MCF answers in;
        // it depends on no store, no engine and no laboratory, because a
        // surface that could reach past the wire would be a surface with a
        // capability the wire does not have.
        depends_on: &["mcf-core", "mcf-record", "mcf-serve"],
    },
    Member {
        name: "mcf-desk",
        path: "crates/mcf-desk",
        // `mcf-tui` because the window draws the CONSOLE's screens rather than
        // screens of its own: one layout, two scales, and neither surface can
        // drift from the other. The edge points this way because the console is
        // the one that works with no display attached, which A22 makes the
        // surface everything else is measured against.
        depends_on: &["mcf-core", "mcf-record", "mcf-serve", "mcf-tui"],
    },
    Member {
        name: "mcf-cli",
        path: "crates/mcf-cli",
        // `mcf-standin` because `mcf run` drives MCF's own engine: D31 put it
        // there so that a model no vendored engine will run still runs, marked,
        // and a surface that could not reach it would be a capability only a
        // test could use (A22).
        depends_on: &[
            "mcf-core",
            "mcf-record",
            "mcf-standin",
            "mcf-lab",
            "mcf-hub",
            "mcf-serve",
            "mcf-bench",
            "mcf-tui",
            "mcf-desk",
        ],
    },
    Member {
        name: "mcf-prototype-adversarial",
        path: "prototypes/adversarial",
        // The §7.19 prototype (B-002). Ships nothing, and nothing depends on
        // it; it exists to produce evidence for DEC-019, DEC-008 and DEC-004,
        // and is superseded by B-013, B-033 and B-011.
        depends_on: &["mcf-core"],
    },
    Member {
        name: "mcf-prototype-kernel-slope",
        path: "prototypes/kernel-slope",
        // The §7.4 evidence (DEC-004): how much of an inference kernel's speed
        // is reachable from safe, portable Rust. Ships nothing, and nothing
        // depends on it.
        depends_on: &["mcf-core"],
    },
    Member {
        name: "mcf-prototype-thread-scaling",
        path: "prototypes/thread-scaling",
        // The B-366 evidence: what threads do to MCF's own engine — whether the
        // answer moved (it must not), how much faster, and how much noisier.
        // Ships nothing and nothing depends on it. It reaches for
        // `mcf-standin` because the subject *is* that engine: a partition timed
        // against a mock would be a timing of the mock, and B65's prohibition
        // is on MCF publishing such a number rather than on MCF knowing what
        // its own code costs (F8, F12 and F52 are the precedent).
        depends_on: &["mcf-core", "mcf-standin"],
    },
    Member {
        name: "mcf-prototype-timing-noise",
        path: "prototypes/timing-noise",
        // The DEC-007 evidence: how much an identical run's timing varies on a
        // machine, and how many repeats that implies (F51, F52, F53). Ships
        // nothing and nothing depends on it. It reaches for `mcf-bench`
        // because the second half of its job is demonstrating that crate's
        // stopping condition against real timings rather than against a
        // fixture — an instrument that proved the rule on invented numbers
        // would have proved nothing (B-083).
        depends_on: &["mcf-bench", "mcf-core"],
    },
    Member {
        name: "mcf-checks",
        path: "checks",
        // Development dependencies only: the taxonomy agreement check reads
        // `mcf_core::failure`, the fault catalogue check reads
        // `mcf_lab::CATALOGUE`, and the property, fuzz, load and soak tiers
        // examine `mcf_record`'s codec and journal and `mcf_standin`'s model
        // reader. Nothing this crate builds ships.
        depends_on: &[
            // `mcf-bench` so that the seed-set tier can ask whether two
            // distributions differ with the same arithmetic every other
            // comparison uses (B-291, D19). A second implementation of that
            // question inside the checks would be a second answer to it (A6).
            "mcf-bench",
            "mcf-core",
            // `mcf-helper` so that `the_daemon_holds_no_privilege` can compare
            // the helper's surface against D35 in both directions. Nothing in
            // the checks ships (§6.32's audit is about what an operator runs).
            "mcf-helper",
            "mcf-hub",
            "mcf-lab",
            "mcf-record",
            "mcf-standin",
        ],
    },
];

/// The workspace root, derived from this crate's own manifest directory.
///
/// `CARGO_MANIFEST_DIR` is set by cargo for every build, so the checks locate
/// the repository the same way whatever directory the suite is run from
/// (B19: the suite runs on a laptop, and a check that depends on the current
/// working directory is a check that fails on somebody else's).
#[must_use]
pub fn root() -> PathBuf {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .parent()
        .map_or_else(|| manifest_dir.to_path_buf(), Path::to_path_buf)
}

/// What went wrong reading a manifest, with the file it was reading.
#[derive(Debug)]
pub enum ReadError {
    /// The file could not be read.
    Io(PathBuf, std::io::Error),
    /// The file was read and could not be understood.
    Parse(PathBuf, ManifestError),
}

impl std::fmt::Display for ReadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(path, error) => write!(f, "{}: {error}", path.display()),
            Self::Parse(path, error) => write!(f, "{}: {error}", path.display()),
        }
    }
}

impl std::error::Error for ReadError {}

/// Reads a manifest from a path relative to the workspace root.
///
/// # Errors
///
/// Returns a [`ReadError`] naming the file, when it cannot be read or cannot
/// be understood.
pub fn read(relative: &str) -> Result<Manifest, ReadError> {
    let path = root().join(relative);
    let source = std::fs::read_to_string(&path).map_err(|e| ReadError::Io(path.clone(), e))?;
    Manifest::parse(&source).map_err(|e| ReadError::Parse(path, e))
}

/// Reads a member crate's manifest.
///
/// # Errors
///
/// As [`read`].
pub fn read_member(member: &Member) -> Result<Manifest, ReadError> {
    read(&format!("{}/Cargo.toml", member.path))
}

/// The MCF crates a manifest declares as dependencies, sorted.
///
/// Non-MCF dependencies are excluded: B-001 constrains the shape of this
/// workspace, and B15 governs outside dependencies separately.
#[must_use]
pub fn declared_mcf_dependencies(manifest: &Manifest) -> Vec<String> {
    let mut names: Vec<String> = ["dependencies", "dev-dependencies", "build-dependencies"]
        .into_iter()
        .flat_map(|table| manifest.keys(table))
        .filter(|name| name.starts_with("mcf-"))
        .map(str::to_owned)
        .collect();
    names.sort_unstable();
    names.dedup();
    names
}
