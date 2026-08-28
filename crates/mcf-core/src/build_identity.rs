//! What produced this binary.
//!
//! §3.4 makes MCF's own version and configuration part of the condition set of
//! every measurement, and §3.12 makes the compiler that built it part of the
//! same set: two binaries built from one source tree by different toolchains
//! are two instruments. The M0 mockup's record header carries these four
//! fields for that reason.
//!
//! Every field is captured at compile time from cargo's and rustc's own
//! output, never inferred. The source revision is the interesting case: it is
//! supplied by the build environment through `MCF_BUILD_COMMIT` and is
//! [`SourceRevision::Unknown`] when it is not, because A7 forbids filling an
//! unknown with a plausible value and B36 forbids requiring the user to have
//! `git` installed to build.

use core::fmt;
use std::io::Read as _;

use crate::attested::Attested;
use crate::digest::{Digest, Sha256};

/// The revision of the source tree a binary was built from.
///
/// A7: absence is a variant of the type rather than an empty string, so a
/// caller has to handle it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SourceRevision {
    /// The build environment named the revision.
    Known(&'static str),
    /// The build environment did not name one.
    ///
    /// This is the honest state of a build from an exported tarball or an
    /// unpacked source archive; it is not a defect, and it is not a reason to
    /// substitute a placeholder that reads like a revision.
    Unknown,
}

impl fmt::Display for SourceRevision {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Known(revision) => f.write_str(revision),
            // C8: a surface that must show something it does not know shows
            // `unknown`.
            Self::Unknown => f.write_str("unknown"),
        }
    }
}

/// The identity of the running binary: what it is, and what built it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BuildIdentity {
    /// MCF's own version, from the workspace manifest.
    pub version: &'static str,
    /// The source revision, or [`SourceRevision::Unknown`].
    pub revision: SourceRevision,
    /// The exact compiler release, as rustc reports itself.
    pub rustc: &'static str,
    /// The target triple this binary was compiled for.
    pub target: &'static str,
    /// The cargo profile it was compiled under.
    ///
    /// Recorded because it changes what is measured: the release profile
    /// aborts on panic and enables fat LTO, and D24's budgets are ceilings for
    /// that artifact rather than for a debug one.
    pub profile: &'static str,
}

impl BuildIdentity {
    /// The identity of this binary.
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
    /// Renders as the M0 mockup's header line.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "MCF {}  (revision {}, {}, target {}, profile {})",
            self.version, self.revision, self.rustc, self.target, self.profile
        )
    }
}

/// The digest of the running binary, computed once.
static INSTRUMENT: std::sync::OnceLock<Attested<Digest>> = std::sync::OnceLock::new();

/// What took this measurement, identified exactly (F93, §3.4).
///
/// **Why a digest and not a version.** §3.4 makes MCF's own version part of
/// every measurement's conditions, and the premise is that a reader who knows
/// what took a number can judge it. A version string does not change when an
/// instrument does: three measuring instruments changed in this repository in
/// one working day — the contention reading, the processor temperature, the
/// effect size — and every record on either side of all three says
/// `0.1.0-m0`. The condition that was supposed to identify the instrument
/// identified nothing.
///
/// `MCF_BUILD_COMMIT` exists for this and is honest about being absent (A7),
/// but it is set in exactly one script in the repository and never in the
/// binary an operator builds and runs. A field that is always `Unknown` is not
/// a mechanism.
///
/// **A binary's own contents cannot be forgotten.** No build cooperation, no
/// git, no environment variable, and it works for a binary shipped in a
/// tarball. Two builds that measure differently have different digests by
/// construction, which is the only property needed to partition a record
/// correctly.
///
/// **What it does not do**: say *what* changed. It says *that* the instrument
/// differs, which is enough to know two measurements are not comparable
/// instruments — and the erratum record is where a defect is described.
///
/// `Unknown` where the executable cannot be located or read, which is a
/// capability of the platform rather than a fact about the instrument, and
/// A7 keeps it from becoming a placeholder.
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

#[cfg(test)]
mod tests {
    use super::{BuildIdentity, SourceRevision};

    /// A19: a reported quantity is checked against an independently known
    /// value. The manifest is the independent source for the version.
    #[test]
    fn version_is_the_manifest_version() {
        assert_eq!(BuildIdentity::current().version, env!("CARGO_PKG_VERSION"));
    }

    /// The target triple is what cargo told the build script, and cargo is the
    /// only thing that knows it.
    #[test]
    fn target_is_a_triple_and_not_empty() {
        let identity = BuildIdentity::current();
        assert!(
            identity.target.matches('-').count() >= 2,
            "target {:?} is not a target triple",
            identity.target
        );
    }

    /// rustc's own `--version` output starts with its name; anything else
    /// means the build script captured the wrong thing.
    #[test]
    fn rustc_reports_itself() {
        assert!(
            BuildIdentity::current().rustc.starts_with("rustc "),
            "rustc field is {:?}",
            BuildIdentity::current().rustc
        );
    }

    /// The profile is one of the two the workspace defines, so a record can be
    /// read as naming a real artifact.
    #[test]
    fn profile_is_a_known_profile() {
        let profile = BuildIdentity::current().profile;
        assert!(
            profile == "debug" || profile == "release",
            "unexpected profile {profile:?}"
        );
    }

    /// A7: the absent revision has its own variant and renders as `unknown`,
    /// never as a plausible-looking value.
    #[test]
    fn unknown_revision_renders_as_unknown() {
        assert_eq!(SourceRevision::Unknown.to_string(), "unknown");
    }

    /// And a known one renders verbatim, so a record is not silently rewritten.
    #[test]
    fn known_revision_renders_verbatim() {
        assert_eq!(SourceRevision::Known("4f2ac91").to_string(), "4f2ac91");
    }

    /// The rendered header names every field, so no condition is dropped on the
    /// way to a surface (A6's habit, applied to the identity).
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
