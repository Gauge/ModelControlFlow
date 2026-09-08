use core::fmt;
use std::io::Read as _;

use crate::attested::Attested;
use crate::digest::{Digest, Sha256};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SourceRevision {
    Known(&'static str),
    Unknown,
}

impl fmt::Display for SourceRevision {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Known(revision) => f.write_str(revision),
            Self::Unknown => f.write_str("unknown"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BuildIdentity {
    pub version: &'static str,
    pub revision: SourceRevision,
    pub rustc: &'static str,
    pub target: &'static str,
    pub profile: &'static str,
}

impl BuildIdentity {
    #[must_use]
    pub const fn current() -> Self {
        Self {
            version: env!("CARGO_PKG_VERSION"),
            revision: match option_env!("MCF_BUILD_COMMIT") {
                Some(revision) => SourceRevision::Known(revision),
                None => SourceRevision::Unknown,
            },
            rustc: env!("MCF_BUILD_RUSTC"),
            target: env!("MCF_BUILD_TARGET"),
            profile: env!("MCF_BUILD_PROFILE"),
        }
    }
}

impl fmt::Display for BuildIdentity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "MCF {}  (revision {}, {}, target {}, profile {})",
            self.version, self.revision, self.rustc, self.target, self.profile
        )
    }
}

static INSTRUMENT: std::sync::OnceLock<Attested<Digest>> = std::sync::OnceLock::new();

#[must_use]
pub fn instrument() -> Attested<Digest> {
    INSTRUMENT
        .get_or_init(|| {
            let Ok(path) = std::env::current_exe() else {
                return Attested::Unknown;
            };
            let Ok(file) = std::fs::File::open(path) else {
                return Attested::Unknown;
            };
            let mut reader = std::io::BufReader::new(file);
            let mut hasher = Sha256::new();
            let mut buffer = vec![0_u8; 64 * 1024];
            loop {
                match reader.read(&mut buffer) {
                    Ok(0) => break,
                    Ok(taken) => match buffer.get(..taken) {
                        Some(held) => hasher.update(held),
                        None => return Attested::Unknown,
                    },
                    Err(_) => return Attested::Unknown,
                }
            }
            Attested::Known(hasher.finish())
        })
        .to_owned()
}

const SHORT: usize = 12;

#[must_use]
pub fn identifier() -> String {
    let version = BuildIdentity::current().version;
    match instrument() {
        Attested::Known(digest) => {
            let hex = digest.hex();
            format!("{version}+{}", hex.get(..SHORT).unwrap_or(&hex))
        }
        Attested::Unknown => format!("{version}+unknown"),
    }
}

#[must_use]
pub fn stand_in_engine() -> String {
    format!("MCF's own stand-in, build {}", identifier())
}

#[cfg(test)]
mod tests {
    use super::{BuildIdentity, SourceRevision};

    #[test]
    fn version_is_the_manifest_version() {
        assert_eq!(BuildIdentity::current().version, env!("CARGO_PKG_VERSION"));
    }

    #[test]
    fn target_is_a_triple_and_not_empty() {
        let identity = BuildIdentity::current();
        assert!(
            identity.target.matches('-').count() >= 2,
            "target {:?} is not a target triple",
            identity.target
        );
    }

    #[test]
    fn rustc_reports_itself() {
        assert!(
            BuildIdentity::current().rustc.starts_with("rustc "),
            "rustc field is {:?}",
            BuildIdentity::current().rustc
        );
    }

    #[test]
    fn profile_is_a_known_profile() {
        let profile = BuildIdentity::current().profile;
        assert!(
            profile == "debug" || profile == "release",
            "unexpected profile {profile:?}"
        );
    }

    #[test]
    fn unknown_revision_renders_as_unknown() {
        assert_eq!(SourceRevision::Unknown.to_string(), "unknown");
    }

    #[test]
    fn known_revision_renders_verbatim() {
        assert_eq!(SourceRevision::Known("4f2ac91").to_string(), "4f2ac91");
    }

    #[test]
    fn display_names_every_field() {
        let identity = BuildIdentity::current();
        let rendered = identity.to_string();
        for field in [
            identity.version,
            identity.rustc,
            identity.target,
            identity.profile,
        ] {
            assert!(rendered.contains(field), "{rendered:?} omits {field:?}");
        }
    }
}
