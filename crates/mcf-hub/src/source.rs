//! Where a repository's contents come from, as an interface rather than as a
//! network client.
//!
//! **Why this exists before the network does.** B-028 asks for a complete,
//! deterministic simulated hub so that *every M1 test runs against it with no
//! network* — and a simulation is only possible if the thing being simulated
//! has a boundary. This is that boundary: the small set of questions MCF asks a
//! hub, stated as a trait, so the acquisition path is written once and exercised
//! against a laboratory's hub that behaves badly on purpose (D26, B19).
//!
//! **It is deliberately small.** Four questions — what does this repository
//! publish, what are its terms, give me this file, and who am I to you — and
//! each of them is something MCF already needs an answer to. A wider interface
//! would be a wider surface for a hostile hub to reach through (§3.7), and
//! §3.13 refuses generality nobody asked for.
//!
//! **What a source promises, and what it does not.** It promises to answer or
//! to fail in a classified way (A2): every method returns MCF's own failure
//! type, and there is no path through this interface that hangs or panics.
//! It promises nothing about *when* — a real hub is slow, throttled and
//! occasionally absent, and B7 makes each of those a defined outcome rather
//! than an exception.
//!
//! **Nothing here fetches yet.** [`Source::fetch`] exists so that the fake hub
//! can serve bytes to the tests that need them; the real client, resumption and
//! integrity checking are B-021, and the vendoring decision that a network
//! stack needs has not been made.

use std::path::Path;

use mcf_core::failure::Result;

use crate::reference::Reference;

/// One file a repository publishes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    /// Its path within the repository.
    pub path: String,
    /// How many bytes the hub says it is.
    ///
    /// The hub's claim, not a measurement: B-213 plans against it and B-021
    /// checks it, and the difference between the two is a finding rather than
    /// an error (A21's shape — declared and verified are different states).
    pub size: u64,
}

/// What a repository publishes, and what MCF is allowed to do with it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Listing {
    /// The reference this describes, with the revision resolved where the
    /// source could resolve it.
    pub reference: Reference,
    /// The revision the listing is of.
    ///
    /// A source that could not say leaves it absent rather than repeating what
    /// was asked for (A7): "the default branch, whatever it is" is not a
    /// revision anybody can pin.
    pub revision: Option<String>,
    /// Every file, in the order the source listed them.
    pub entries: Vec<Entry>,
    /// The licence the repository declares, where it declares one.
    ///
    /// Declared, never verified: A21 keeps those apart, and B-023 is where the
    /// difference is surfaced to a user before they use an artifact.
    pub declared_licence: Option<String>,
}

impl Listing {
    /// The entry with this path, if the repository publishes one.
    #[must_use]
    pub fn entry(&self, path: &str) -> Option<&Entry> {
        self.entries.iter().find(|entry| entry.path == path)
    }

    /// Every entry whose path ends with this suffix, which is how a
    /// quantization is chosen from a repository that publishes twenty.
    #[must_use]
    pub fn entries_ending(&self, suffix: &str) -> Vec<&Entry> {
        self.entries
            .iter()
            .filter(|entry| entry.path.ends_with(suffix))
            .collect()
    }

    /// What the whole repository would cost to hold.
    ///
    /// `None` on an overflow rather than a wrapped total: a listing whose sizes
    /// do not add up is a listing MCF will not plan against (B-213).
    #[must_use]
    pub fn total_bytes(&self) -> Option<u64> {
        self.entries
            .iter()
            .try_fold(0_u64, |total, entry| total.checked_add(entry.size))
    }
}

/// What MCF asks a hub.
///
/// Implemented by the real client (B-021) and by the laboratory's simulated hub
/// (B-028). A caller written against this is a caller the laboratory can drive
/// through every failure in the `hub.*` taxonomy without a network.
pub trait Source {
    /// What this source is, for a record and for a message.
    ///
    /// Part of a measurement's conditions the moment an artifact acquired
    /// through it is measured (§3.4), and the difference between *the hub* and
    /// *a mirror somebody stood up* when a result is questioned.
    fn describe(&self) -> String;

    /// What a repository publishes.
    ///
    /// # Errors
    ///
    /// Any `hub.*` failure: the reference names nothing, the repository is
    /// gated, the account is throttled, the hub is unreachable.
    fn list(&self, reference: &Reference) -> Result<Listing>;

    /// Puts one file where the caller asked for it.
    ///
    /// The caller supplies the destination, so a source never decides where
    /// anything on this machine goes — §3.10's habit, and the reason a hostile
    /// source cannot choose a path.
    ///
    /// # Errors
    ///
    /// Any `hub.*` failure, and `artifact.incomplete` where the transfer ended
    /// early. What it must never do is leave a partial file looking like a
    /// whole one, which is B-021's condition and the thing the laboratory's hub
    /// exists to attempt.
    fn fetch(&self, reference: &Reference, entry: &Entry, into: &Path) -> Result<Fetched>;
}

/// What a completed fetch produced.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fetched {
    /// How many bytes arrived.
    pub bytes: u64,
    /// The digest of what arrived, computed while it arrived.
    ///
    /// Computed by the *fetcher* rather than reported by the source: a checksum
    /// a hostile source supplies is a checksum of what it wishes it had sent
    /// (§3.7, B-021).
    pub digest: String,
}
