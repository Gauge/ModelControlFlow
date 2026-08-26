//! What one line of the journal holds.
//!
//! An entry is an *event*: something happened, and this is what MCF knew about
//! it at the time. B4 makes that the only reason to write — time passing is
//! not one — so there is no entry kind for "a sample of the current state".
//!
//! Every entry carries four things and cannot be built without them: an
//! identifier, a kind, when it was recorded, and its body. The first three are
//! the envelope every reader can interpret, including a reader from a later
//! version that does not understand the body — which is what §7.30 needs from
//! a format that travels between versions.

use core::fmt;

use mcf_core::time::Timestamp;

use crate::json::Value;

/// What kind of event an entry records.
///
/// Small on purpose. A kind is a thing consumers switch on, so adding one is a
/// decision about the record's public shape (§7.30) rather than a convenience,
/// and the same discipline the failure taxonomy states for its domains applies
/// here.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum EntryKind {
    /// What the machine was, read at a moment MCF was asked to look.
    MachineProfile,
    /// A classified failure (A2).
    Failure,
    /// What MCF's own operation cost, measured (§3.8).
    SelfCost,
    /// The trials of one session (D16, B56).
    ///
    /// Trials rather than results: what is recorded is what was observed, and
    /// every summary is projected from these when a question is asked.
    Trials,
    /// An artifact arrived on this machine, with everything about where it
    /// came from (B-029, §3.6).
    ///
    /// Written at the moment of acquisition rather than derived from the
    /// sidecar beside the artifact, because the two answer different questions:
    /// the sidecar says what this file is, and the record says what happened on
    /// this machine and when. An artifact that is later moved by hand keeps the
    /// first and cannot change the second.
    ArtifactAcquired,
    /// An artifact left this machine, and who said it could (B-027, §3.11).
    ///
    /// A kind of its own rather than a failure or a note, because it is the one
    /// event whose record has to outlive the thing it is about: after a removal
    /// the artifact is gone and this line is all there is. §3.11 forbids
    /// reclaiming space without a decision, and a decision nobody wrote down is
    /// indistinguishable from an automatic one.
    ArtifactRemoved,
}

impl EntryKind {
    /// Every kind, in the order they were defined.
    pub const ALL: [Self; 6] = [
        Self::MachineProfile,
        Self::Failure,
        Self::SelfCost,
        Self::Trials,
        Self::ArtifactAcquired,
        Self::ArtifactRemoved,
    ];

    /// The kind's name, as it appears in the record.
    ///
    /// Stable for life (C5): once written it travels between machines and
    /// versions, and a renamed kind is a record nobody can read twice.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::MachineProfile => "machine_profile",
            Self::Failure => "failure",
            Self::SelfCost => "self_cost",
            Self::Trials => "trials",
            Self::ArtifactAcquired => "artifact_acquired",
            Self::ArtifactRemoved => "artifact_removed",
        }
    }

    /// The kind a written name refers to, if this build knows it.
    ///
    /// `None` rather than a fallback: a kind this version does not know is a
    /// record written by a newer schema, and deciding what to do about that
    /// belongs to the reader with the context (§7.30).
    #[must_use]
    pub fn parse(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|kind| kind.as_str() == name)
    }
}

impl fmt::Display for EntryKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// An entry's identifier.
///
/// Built from the kind, the moment and a sequence number, so that it is
/// meaningful to a reader, sorts into the order things happened, and stays
/// unique when two events land in the same nanosecond. C5 makes it stable for
/// life once written.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EntryId(String);

impl EntryId {
    /// Composes an identifier.
    #[must_use]
    pub fn new(kind: EntryKind, at: Timestamp, sequence: u64) -> Self {
        let civil = at.civil_utc();
        Self(format!(
            "{}_{:04}-{:02}-{:02}T{:02}-{:02}-{:02}Z_{sequence:04}",
            kind.as_str(),
            civil.year,
            civil.month,
            civil.day,
            civil.hour,
            civil.minute,
            civil.second,
        ))
    }

    /// The identifier, as written.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for EntryId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// One recorded event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    id: EntryId,
    kind: EntryKind,
    recorded_at: Timestamp,
    body: Value,
}

impl Entry {
    /// Records an event.
    ///
    /// There is no constructor that omits the moment or the kind, and none
    /// that takes a body alone: an entry nobody can classify or place in time
    /// is a line, not a record.
    #[must_use]
    pub fn new(kind: EntryKind, recorded_at: Timestamp, sequence: u64, body: Value) -> Self {
        Self {
            id: EntryId::new(kind, recorded_at, sequence),
            kind,
            recorded_at,
            body,
        }
    }

    /// Its identifier.
    #[must_use]
    pub const fn id(&self) -> &EntryId {
        &self.id
    }

    /// What kind of event it was.
    #[must_use]
    pub const fn kind(&self) -> EntryKind {
        self.kind
    }

    /// When it was recorded.
    #[must_use]
    pub const fn recorded_at(&self) -> Timestamp {
        self.recorded_at
    }

    /// What MCF knew about it.
    #[must_use]
    pub const fn body(&self) -> &Value {
        &self.body
    }

    /// The entry as one JSON value.
    ///
    /// The timestamp is written twice on purpose: once as text a reader can
    /// see, and once as nanoseconds a reader can sort and compare exactly.
    /// Rendering only the text would make every consumer re-parse a calendar;
    /// storing only the number would make the record unreadable to a person,
    /// which §3.3 permits ranking second but not omitting.
    #[must_use]
    pub fn to_value(&self) -> Value {
        Value::map([
            ("id", Value::text(self.id.as_str())),
            ("kind", Value::text(self.kind.as_str())),
            ("recorded_at", Value::text(self.recorded_at.to_string())),
            (
                "recorded_at_utc_nanos",
                Value::Integer(i64::try_from(self.recorded_at.utc_nanos()).unwrap_or(i64::MAX)),
            ),
            ("body", self.body.clone()),
        ])
    }
}
