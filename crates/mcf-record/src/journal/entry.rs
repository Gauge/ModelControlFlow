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
    /// The daemon started, and what it recovered when it did (B-030, D1).
    ///
    /// Written at the event and not on a timer (§6.9): a daemon that logged
    /// while idle would fail B-031's measurement, and one that recorded nothing
    /// at all would leave *MCF was up between these two moments* unanswerable —
    /// which is a condition of anything measured in between (§3.4).
    DaemonStarted,
    /// The daemon stopped, and on whose word (A26).
    ///
    /// A process that can only be killed leaves no account of why it stopped.
    /// This is the account: what was asked, and by what reason.
    DaemonStopped,
    /// An artifact arrived on this machine, with everything about where it
    /// came from (B-029, §3.6).
    ///
    /// Written at the moment of acquisition rather than derived from the
    /// sidecar beside the artifact, because the two answer different questions:
    /// the sidecar says what this file is, and the record says what happened on
    /// this machine and when. An artifact that is later moved by hand keeps the
    /// first and cannot change the second.
    ArtifactAcquired,
    /// MCF looked upstream at something it holds, and what it found (B-331,
    /// D37).
    ///
    /// Written whether or not anything had changed: *checked and unchanged* is
    /// a fact about a moment, and a record that only kept the bad news could
    /// not answer *when was this last known to be fine* (A1, A7).
    ArtifactChecked,
    /// An artifact left this machine, and who said it could (B-027, §3.11).
    ///
    /// A kind of its own rather than a failure or a note, because it is the one
    /// event whose record has to outlive the thing it is about: after a removal
    /// the artifact is gone and this line is all there is. §3.11 forbids
    /// reclaiming space without a decision, and a decision nobody wrote down is
    /// indistinguishable from an automatic one.
    ArtifactRemoved,
    /// A component was installed, built and pinned into an environment MCF
    /// controls (B-367, D39).
    ///
    /// The record is what makes a provisioned component a *condition* rather
    /// than a circumstance: every measurement taken through it cites an
    /// environment this entry can reproduce — the image by digest, the source
    /// by commit, the packages by exact version (§3.4).
    ComponentProvisioned,
    /// A provisioned component was removed, and why (A27, §3.11).
    ///
    /// The pair to `ComponentProvisioned` the way `ArtifactRemoved` pairs with
    /// `ArtifactAcquired`: after the removal the prefix is gone and this line
    /// is all there is.
    ComponentRemoved,
    /// A model answered a prompt through the daemon, and under what
    /// conditions (B-034, PR9, D20).
    ///
    /// The terminating line of the stream, written down as it was sent: a
    /// client that ignored the conditions still leaves them in the record, and
    /// a client that hung up early leaves the account of what it got.
    Generated,
    /// MCF was told to address a model the way a probe found it wants to be,
    /// and on whose word (D42, D43, B-059).
    ///
    /// The act D43 requires. A probe writes the verified half of a capability
    /// and never a default, so nothing a probe learns changes how MCF talks to
    /// a model until somebody says so — and when somebody does, the change has
    /// to outlive the moment. This entry is what makes a derived configuration
    /// answer *why this value*: which probe, when, under what conditions, and
    /// what MCF did differently afterwards.
    ///
    /// It is also the line that makes measurements across it non-comparable,
    /// which §3.4 requires MCF to say rather than assume.
    ModelConfigured,
    /// What two arms of a comparison were found to do (B-086, B-250, A9).
    ///
    /// **A kind of its own because a null result is a result.** A9: *"no
    /// measurable difference" and "does not fit here" are findings, not
    /// failures.* A comparison that separated its arms, one that established
    /// they are the same to a stated resolution, and one that refused a delta
    /// because more than one variable differed are three outcomes of the same
    /// event, and putting any of them anywhere but here would make it
    /// unfindable next to the others — the `Same` reading in a note, the
    /// refusal in `Failure`, and the register unable to answer *what has been
    /// compared on this machine*.
    ///
    /// The paired differences are written out beside the verdict, because B56
    /// keeps the trials and derives the summary: a comparison whose
    /// distribution was thrown away is a question nobody can re-ask.
    Comparison,
    /// What MCF judged about a variant before a byte of it was fetched
    /// (B-086, B-213, PR3, §6.3).
    ///
    /// *This will not run here, because it needs 131 GiB and you have 24* is a
    /// complete success of §III, and A9 makes it a result rather than a
    /// refusal. Recorded whichever way it came out, so that the register can
    /// answer *what has this machine been told it cannot run* — which is the
    /// question an operator asks before downloading tens of gigabytes a second
    /// time.
    FitmentPlanned,
    /// What was competing for this machine when a measurement could not be
    /// attributed (B-216, PR5, §3.8, B24).
    ///
    /// **B24's refusal with a name attached.** Having been told a measurement
    /// is unattributable, the operator's next question is always *by what?* —
    /// and MCF is the only thing positioned to answer, because it was there
    /// when it happened. This is that answer, kept: PR5 requires the snapshot
    /// persist with the record rather than being a transient thing on a
    /// screen, so that the finding survives the terminal it was printed in
    /// (§3.1).
    ///
    /// Written when something asks for it and at no other time. B4 refuses
    /// ambient sampling and there is no timer here: a snapshot exists because
    /// a run could not decide, or because somebody asked.
    ContentionSnapshot,
    /// What a probe asked a model and what it observed (B-386, B-055, D42,
    /// A1).
    ///
    /// **Because a measurement nobody can find later is the same as one not
    /// taken.** `mcf probe` established figures — the usable context against
    /// the declared one (F42), how a model wants to be addressed (F38) — and
    /// printed them. A terminal scrolls. Every surface that wants a *measured*
    /// figure rather than a declared one was blocked behind that, and B-382
    /// is the item that found it: a prompt's cost could only be stated against
    /// the file's claim, because MCF's own measurement of what the engine
    /// takes existed nowhere it could be read.
    ///
    /// Distinct from [`Self::ModelConfigured`], which is the *act* D43
    /// requires — somebody deciding to address a model differently. This is
    /// the observation that act may or may not follow from. A probe that
    /// changes nothing still measured something (A9), and D42 keeps the two
    /// apart on purpose: a probe writes the verified half of a capability and
    /// never a default.
    ModelProbed,
}

impl EntryKind {
    /// Every kind, in the order they were defined.
    pub const ALL: [Self; 17] = [
        Self::MachineProfile,
        Self::Failure,
        Self::SelfCost,
        Self::Trials,
        Self::DaemonStarted,
        Self::DaemonStopped,
        Self::ArtifactAcquired,
        Self::ArtifactChecked,
        Self::ArtifactRemoved,
        // Appended rather than sorted in: the derived index stores a kind as
        // its position in this list, so the order is part of the format (D20).
        Self::ComponentProvisioned,
        Self::ComponentRemoved,
        Self::Generated,
        Self::ModelConfigured,
        Self::Comparison,
        Self::FitmentPlanned,
        Self::ContentionSnapshot,
        Self::ModelProbed,
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
            Self::DaemonStarted => "daemon_started",
            Self::DaemonStopped => "daemon_stopped",
            Self::ArtifactAcquired => "artifact_acquired",
            Self::ArtifactChecked => "artifact_checked",
            Self::ArtifactRemoved => "artifact_removed",
            Self::ComponentProvisioned => "component_provisioned",
            Self::ComponentRemoved => "component_removed",
            Self::Generated => "generated",
            Self::ModelConfigured => "model_configured",
            Self::Comparison => "comparison",
            Self::FitmentPlanned => "fitment_planned",
            Self::ContentionSnapshot => "contention_snapshot",
            Self::ModelProbed => "model_probed",
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
/// Built from the kind, the moment, **the writer** and that writer's own count,
/// so that it is meaningful to a reader, sorts into the order things happened,
/// and names exactly one entry on a machine where more than one program writes
/// to the record (DEC-037, B-332).
///
/// **Why the writer is in it.** [findings.md](../../../../doc/findings.md) F13
/// measured that concurrent appends do not tear, which left one thing broken:
/// every writer counted its own appends from zero, so two programs writing in
/// the same second produced the same identifier for two different events. A
/// record whose identifiers name two things is a record nothing can cite.
///
/// **Who mints it.** Only [`Writer`], which only a [`crate::journal::Journal`]
/// holds. An entry that has not been written has no identifier, and the type
/// says so — see [`Entry::id`].
///
/// C5 makes an identifier stable for life *once written*: what is in this file
/// is what it has always been, and this changes what a later entry gets rather
/// than what an earlier one had.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EntryId(String);

impl EntryId {
    /// Composes an identifier.
    fn mint(kind: EntryKind, at: Timestamp, writer: &Writer, sequence: u64) -> Self {
        let civil = at.civil_utc();
        Self(format!(
            "{}_{:04}-{:02}-{:02}T{:02}-{:02}-{:02}Z_{}_{sequence:04}",
            kind.as_str(),
            civil.year,
            civil.month,
            civil.day,
            civil.hour,
            civil.minute,
            civil.second,
            writer.as_str(),
        ))
    }

    /// An identifier as it was found in a record.
    ///
    /// Read back exactly as written and never recomputed: an identifier is what
    /// the file says it is, and a reader that rebuilt one from the envelope
    /// would quietly show something the record does not contain (A1).
    #[must_use]
    pub fn as_written(text: &str) -> Self {
        Self(text.to_owned())
    }

    /// The identifier, as written.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Who is writing, distinctly from everybody else writing to the same record.
///
/// **The problem it solves.** MCF has no coordinator between the programs that
/// write to a record, and F13 measured that it does not need one to keep the
/// file readable. What it does need is for two writers never to mint the same
/// identifier, and the cheapest thing that guarantees that is for each writer
/// to carry something no other writer has.
///
/// **What it is made of, and why that is enough.** The process's identifier,
/// the moment this writer was made, and a count of the writers made in this
/// process. Two live processes cannot share a process identifier; a later
/// process that inherits a recycled one was made at a different nanosecond; and
/// two writers inside one process differ by the count. No clock is trusted for
/// *ordering* here — only for distinctness — so a clock that steps backwards
/// costs nothing (D9).
///
/// **It is not a name for a person or a machine.** A25 keeps the record free of
/// anything about the user, and this is a token of the shape `a3f19c04`: it
/// says *some writer*, distinguishes it from *some other writer*, and carries
/// nothing else.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Writer(String);

/// How many writers this process has made.
static WRITERS: core::sync::atomic::AtomicU64 = core::sync::atomic::AtomicU64::new(0);

impl Writer {
    /// Makes a writer distinct from every other one.
    #[must_use]
    pub fn distinct() -> Self {
        let counted = WRITERS.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
        let mut digest = mcf_core::digest::Sha256::new();
        digest.update(&std::process::id().to_le_bytes());
        digest.update(&Timestamp::now().utc_nanos().to_le_bytes());
        digest.update(&counted.to_le_bytes());
        Self(digest.finish().hex().chars().take(8).collect())
    }

    /// A writer with a stated token, for a laboratory that needs a record it
    /// can reproduce byte for byte (§3.17, B27).
    ///
    /// Nothing in MCF's own paths calls this: a scenario that wants the same
    /// identifiers every run says which writer it is, and everything else takes
    /// what [`Writer::distinct`] gives it.
    #[must_use]
    pub fn stated(token: &str) -> Self {
        Self(token.to_owned())
    }

    /// The token, as it appears in an identifier.
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
    /// `None` until a journal writes it.
    ///
    /// An identifier is minted by the writer that appends the entry, so an
    /// entry nobody has written does not have one — which is the whole of
    /// DEC-037's answer expressed as a type rather than as a convention.
    id: Option<EntryId>,
    kind: EntryKind,
    recorded_at: Timestamp,
    body: Value,
}

impl Entry {
    /// Records an event.
    ///
    /// There is no constructor that omits the moment or the kind, and none
    /// that takes a body alone: an entry nobody can classify or place in time
    /// is a line, not a record. There is also none that takes an identifier:
    /// the writer that appends it mints that (DEC-037).
    #[must_use]
    pub fn new(kind: EntryKind, recorded_at: Timestamp, body: Value) -> Self {
        Self {
            id: None,
            kind,
            recorded_at,
            body,
        }
    }

    /// An entry as a record already holds it.
    ///
    /// For a replay, which reads the identifier the writer minted rather than
    /// making one up (A1).
    #[must_use]
    pub fn recorded(id: EntryId, kind: EntryKind, recorded_at: Timestamp, body: Value) -> Self {
        Self {
            id: Some(id),
            kind,
            recorded_at,
            body,
        }
    }

    /// Its identifier, if it has been written.
    ///
    /// `None` means nobody has appended it yet, which is a real state and not a
    /// missing value: identifiers come from writers (A7's habit — the answer to
    /// *what is its identifier* before it is written is *there is not one*).
    #[must_use]
    pub const fn id(&self) -> Option<&EntryId> {
        self.id.as_ref()
    }

    /// The same entry, with the identifier its writer gave it.
    #[must_use]
    pub(crate) fn stamped(mut self, id: EntryId) -> Self {
        self.id = Some(id);
        self
    }

    /// The identifier a writer would give this entry.
    pub(crate) fn identify(&self, writer: &Writer, sequence: u64) -> EntryId {
        EntryId::mint(self.kind, self.recorded_at, writer, sequence)
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
            (
                "id",
                match &self.id {
                    Some(id) => Value::text(id.as_str()),
                    // Only reachable for an entry nobody has written: the
                    // journal stamps every entry before it renders one.
                    None => Value::Null,
                },
            ),
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
