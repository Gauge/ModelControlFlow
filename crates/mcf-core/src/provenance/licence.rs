//! What the artifact says you may do with it.
//!
//! §III requires the licence be legible before use, and B-023 requires that an
//! artifact report its licence *or report it as unknown* — never as a
//! plausible default. Three states are needed and they are genuinely
//! different:
//!
//! * an identifier MCF recognized ([`Licence::Spdx`]);
//! * terms that are present and that MCF could not match to one
//!   ([`Licence::Stated`], which is `hub.licence.unparseable`);
//! * nothing there at all, which is [`Attested::Unknown`] on the field
//!   ([`hub.metadata.absent`]).
//!
//! Collapsing the middle case into the third is the tempting mistake: a
//! repository whose licence file MCF cannot parse is *not* a repository with
//! no licence, and treating it as one would let MCF proceed past terms nobody
//! read.
//!
//! [`Attested::Unknown`]: crate::attested::Attested::Unknown
//! [`hub.metadata.absent`]: crate::failure::Category::HubMetadataAbsent

use core::fmt;

/// The licence an artifact carries.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum Licence {
    /// An SPDX identifier MCF recognized, kept as written.
    ///
    /// MCF does not interpret it here. B-023 surfaces it and §III makes a use
    /// the terms forbid something MCF *states* rather than discovers, but the
    /// matching of terms to uses is `hub.licence.forbids_use`'s business and
    /// belongs where the use is attempted.
    Spdx(String),
    /// Terms are present and MCF could not match them to an identifier.
    ///
    /// The text is not stored here — it is content, and §6.8 keeps content out
    /// of the record store (A25). What is stored is that terms exist and were
    /// not understood, which is what a reader needs in order to go and read
    /// them.
    Stated,
}

impl Licence {
    /// An SPDX identifier.
    #[must_use]
    pub fn spdx(identifier: impl Into<String>) -> Self {
        Self::Spdx(identifier.into())
    }

    /// Whether MCF matched the terms to an identifier.
    #[must_use]
    pub const fn is_identified(&self) -> bool {
        matches!(self, Self::Spdx(_))
    }
}

impl fmt::Display for Licence {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Spdx(identifier) => f.write_str(identifier),
            Self::Stated => f.write_str("stated, unmatched"),
        }
    }
}
