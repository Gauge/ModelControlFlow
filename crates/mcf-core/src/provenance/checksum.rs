//! What the bytes hashed to.
//!
//! §3.6 makes the checksum part of an artifact's provenance, and §7.49 makes
//! it something to re-verify before a long run rather than only at acquisition
//! (B-301) — silent disk corruption caught before it produces a garbage result
//! rather than after.

use core::fmt;

/// A digest algorithm MCF is willing to record.
///
/// Enumerated rather than a string, so a record cannot claim an algorithm
/// nothing implements. `#[non_exhaustive]` because adding one is a decision
/// about what MCF verifies, not a convenience.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum DigestAlgorithm {
    /// SHA-256, which is what the hubs publish.
    Sha256,
}

impl DigestAlgorithm {
    /// The number of hexadecimal characters a digest of this algorithm has.
    #[must_use]
    pub const fn hex_length(self) -> usize {
        match self {
            Self::Sha256 => 64,
        }
    }

    /// The name, as it is written in a record.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Sha256 => "sha256",
        }
    }
}

impl fmt::Display for DigestAlgorithm {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// A digest of an artifact's bytes.
///
/// The digest is stored lower-case, which is a normalization rather than an
/// interpretation: `AB` and `ab` are the same digest, and two records that
/// spell one of them differently would compare unequal for no reason.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Checksum {
    algorithm: DigestAlgorithm,
    hex: String,
}

impl Checksum {
    /// A checksum, if the digest is one.
    ///
    /// Returns `None` when the text is not the right length or is not
    /// hexadecimal. A7's habit: a malformed digest is not silently kept as
    /// text that looks like a digest, because a record that holds one is a
    /// record that will fail a comparison nobody can explain.
    #[must_use]
    pub fn new(algorithm: DigestAlgorithm, hex: &str) -> Option<Self> {
        if hex.len() != algorithm.hex_length() {
            return None;
        }
        if !hex.chars().all(|c| c.is_ascii_hexdigit()) {
            return None;
        }
        Some(Self {
            algorithm,
            hex: hex.to_ascii_lowercase(),
        })
    }

    /// A SHA-256 checksum, if the digest is one.
    #[must_use]
    pub fn sha256(hex: &str) -> Option<Self> {
        Self::new(DigestAlgorithm::Sha256, hex)
    }

    /// Which algorithm produced it.
    #[must_use]
    pub const fn algorithm(&self) -> DigestAlgorithm {
        self.algorithm
    }

    /// The digest, lower-case hexadecimal.
    #[must_use]
    pub fn hex(&self) -> &str {
        &self.hex
    }
}

impl fmt::Display for Checksum {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.algorithm, self.hex)
    }
}
