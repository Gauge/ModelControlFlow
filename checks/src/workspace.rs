use std::path::{Path, PathBuf};

use crate::manifest::{Manifest, ManifestError};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Member {
    pub name: &'static str,
    pub path: &'static str,
    pub depends_on: &'static [&'static str],
}

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
        depends_on: &["mcf-core"],
    },
    Member {
        name: "mcf-helper",
        path: "crates/mcf-helper",
        depends_on: &["mcf-core"],
    },
    Member {
        name: "mcf-hub",
        path: "crates/mcf-hub",
        depends_on: &["mcf-core", "mcf-record"],
    },
    Member {
        name: "mcf-optimize",
        path: "crates/mcf-optimize",
        depends_on: &["mcf-core", "mcf-record"],
    },
    Member {
        name: "mcf-serve",
        path: "crates/mcf-serve",
        depends_on: &["mcf-core", "mcf-record", "mcf-hub", "mcf-standin"],
    },
    Member {
        name: "mcf-tui",
        path: "crates/mcf-tui",
        depends_on: &["mcf-core", "mcf-record", "mcf-serve"],
    },
    Member {
        name: "mcf-desk",
        path: "crates/mcf-desk",
        depends_on: &[
            "mcf-core",
            "mcf-optimize",
            "mcf-record",
            "mcf-serve",
            "mcf-tui",
        ],
    },
    Member {
        name: "mcf-cli",
        path: "crates/mcf-cli",
        depends_on: &[
            "mcf-core",
            "mcf-record",
            "mcf-standin",
            "mcf-hub",
            "mcf-serve",
            "mcf-tui",
            "mcf-desk",
        ],
    },
    Member {
        name: "mcf-prototype-adversarial",
        path: "prototypes/adversarial",
        depends_on: &["mcf-core"],
    },
    Member {
        name: "mcf-prototype-kernel-slope",
        path: "prototypes/kernel-slope",
        depends_on: &["mcf-core"],
    },
    Member {
        name: "mcf-prototype-thread-scaling",
        path: "prototypes/thread-scaling",
        depends_on: &["mcf-core", "mcf-standin"],
    },
    Member {
        name: "mcf-checks",
        path: "checks",
        depends_on: &[
            "mcf-core",
            "mcf-helper",
            "mcf-hub",
            "mcf-record",
            "mcf-serve",
            "mcf-standin",
        ],
    },
];

#[must_use]
pub fn root() -> PathBuf {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .parent()
        .map_or_else(|| manifest_dir.to_path_buf(), Path::to_path_buf)
}

#[derive(Debug)]
pub enum ReadError {
    Io(PathBuf, std::io::Error),
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

pub fn read(relative: &str) -> Result<Manifest, ReadError> {
    let path = root().join(relative);
    let source = std::fs::read_to_string(&path).map_err(|e| ReadError::Io(path.clone(), e))?;
    Manifest::parse(&source).map_err(|e| ReadError::Parse(path, e))
}

pub fn read_member(member: &Member) -> Result<Manifest, ReadError> {
    read(&format!("{}/Cargo.toml", member.path))
}

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
