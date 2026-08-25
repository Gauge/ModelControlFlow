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
        name: "mcf-lab",
        path: "crates/mcf-lab",
        depends_on: &["mcf-core", "mcf-record"],
    },
    Member {
        name: "mcf-hub",
        path: "crates/mcf-hub",
        depends_on: &["mcf-core", "mcf-record"],
    },
    Member {
        name: "mcf-serve",
        path: "crates/mcf-serve",
        depends_on: &["mcf-core", "mcf-record"],
    },
    Member {
        name: "mcf-bench",
        path: "crates/mcf-bench",
        depends_on: &["mcf-core", "mcf-record", "mcf-serve"],
    },
    Member {
        name: "mcf-cli",
        path: "crates/mcf-cli",
        depends_on: &[
            "mcf-core",
            "mcf-record",
            "mcf-lab",
            "mcf-hub",
            "mcf-serve",
            "mcf-bench",
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
        name: "mcf-checks",
        path: "checks",
        // Development dependencies only: the taxonomy agreement check reads
        // `mcf_core::failure`, the fault catalogue check reads
        // `mcf_lab::CATALOGUE`, and the property, fuzz, load and soak tiers
        // examine `mcf_record`'s codec and journal. Nothing this crate builds
        // ships.
        depends_on: &["mcf-core", "mcf-lab", "mcf-record"],
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
