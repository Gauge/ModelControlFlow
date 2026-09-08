use core::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum DigestAlgorithm {
    Sha256,
}

impl DigestAlgorithm {
    #[must_use]
    pub const fn hex_length(self) -> usize {
        match self {
            Self::Sha256 => 64,
        }
    }

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

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Checksum {
    algorithm: DigestAlgorithm,
    hex: String,
}

impl Checksum {
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

    #[must_use]
    pub fn sha256(hex: &str) -> Option<Self> {
        Self::new(DigestAlgorithm::Sha256, hex)
    }

    #[must_use]
    pub fn of(digest: crate::digest::Digest) -> Self {
        Self {
            algorithm: DigestAlgorithm::Sha256,
            hex: digest.hex(),
        }
    }

    #[must_use]
    pub const fn algorithm(&self) -> DigestAlgorithm {
        self.algorithm
    }

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
