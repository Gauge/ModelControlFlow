//! The failure classification, as types.
//!
//! This module is `doc/taxonomy.md` expressed so that the compiler can hold
//! it. A2 requires every failure be classified against the taxonomy,
//! attributed to a subsystem and persisted with its context; a classification
//! that lives only in a document is a classification a tired author will
//! approximate with a string.
//!
//! Three axes, not one tree, exactly as the taxonomy argues: [`Category`] says
//! *what failed*, [`Attribution`] says *whose failure it is*, and
//! [`Disposition`] says *what MCF did*. A single tree would have to encode the
//! product of the three.
//!
//! **This file is generated from the taxonomy and then committed.** It is not
//! regenerated at build time: a build that reads a Markdown file to decide what
//! compiles is a build with an undeclared input (§3.12). The check that the two
//! have not drifted is a test — `checks/tests/taxonomy_agreement.rs` — which
//! fails when a code, a meaning, a domain or an axis value differs. Adding a
//! leaf therefore means editing both, in one change, with a laboratory scenario
//! (A13).
//!
//! Codes are stable for life (C5): never reused, never renamed, deprecated only
//! in favour of a named successor, because once §XIV ships they travel between
//! machines and versions.

// Three exhaustive matches over 110 variants each. `too_many_lines` and
// `match_same_arms` are both true of them and both wrong to act on: the value
// of these tables is that they can be read top to bottom against the taxonomy,
// and splitting them or collapsing the arms of `domain` — where identical
// bodies are the whole point, since a domain is what a set of codes shares —
// would destroy the property the agreement check depends on.
#![allow(clippy::too_many_lines, clippy::match_same_arms)]

use core::fmt;

/// The sixteen domains of the taxonomy.
///
/// Domains are the part consumers switch on, which is why the taxonomy makes
/// adding one a decision rather than a cheap edit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum Domain {
    /// `hub.*` — the model source.
    Hub,
    /// `transfer.*` — getting bytes here.
    Transfer,
    /// `artifact.*` — a local model artifact.
    Artifact,
    /// `engine.*` — the supervised inference process.
    Engine,
    /// `accel.*` — the accelerator.
    Accel,
    /// `resource.*` — the machine's own limits.
    Resource,
    /// `record.*` — the record store.
    Record,
    /// `config.*` — configuration.
    Config,
    /// `probe.*` — capability probing.
    Probe,
    /// `lab.*` — laboratory execution.
    Lab,
    /// `model.*` — the model under test's behaviour.
    Model,
    /// `sandbox.*` — containment.
    Sandbox,
    /// `platform.*` — the operating system and privilege.
    Platform,
    /// `time.*` — the clock.
    Time,
    /// `exchange.*` — identifiers and contributions.
    Exchange,
    /// `internal.*` — MCF's own invariants.
    Internal,
}

impl Domain {
    /// Every domain, in the taxonomy's own order.
    pub const ALL: [Self; 16] = [
        Self::Hub,
        Self::Transfer,
        Self::Artifact,
        Self::Engine,
        Self::Accel,
        Self::Resource,
        Self::Record,
        Self::Config,
        Self::Probe,
        Self::Lab,
        Self::Model,
        Self::Sandbox,
        Self::Platform,
        Self::Time,
        Self::Exchange,
        Self::Internal,
    ];

    /// The domain's prefix, as it appears in a code.
    #[must_use]
    pub const fn prefix(self) -> &'static str {
        match self {
            Self::Hub => "hub",
            Self::Transfer => "transfer",
            Self::Artifact => "artifact",
            Self::Engine => "engine",
            Self::Accel => "accel",
            Self::Resource => "resource",
            Self::Record => "record",
            Self::Config => "config",
            Self::Probe => "probe",
            Self::Lab => "lab",
            Self::Model => "model",
            Self::Sandbox => "sandbox",
            Self::Platform => "platform",
            Self::Time => "time",
            Self::Exchange => "exchange",
            Self::Internal => "internal",
        }
    }

    /// What the domain covers, as the taxonomy states it.
    #[must_use]
    pub const fn subject(self) -> &'static str {
        match self {
            Self::Hub => "the model source",
            Self::Transfer => "getting bytes here",
            Self::Artifact => "a local model artifact",
            Self::Engine => "the supervised inference process",
            Self::Accel => "the accelerator",
            Self::Resource => "the machine's own limits",
            Self::Record => "the record store",
            Self::Config => "configuration",
            Self::Probe => "capability probing",
            Self::Lab => "laboratory execution",
            Self::Model => "the model under test's behaviour",
            Self::Sandbox => "containment",
            Self::Platform => "the operating system and privilege",
            Self::Time => "the clock",
            Self::Exchange => "identifiers and contributions",
            Self::Internal => "MCF's own invariants",
        }
    }
}

impl fmt::Display for Domain {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.prefix())
    }
}

/// What failed: the taxonomy's 111 leaf codes.
///
/// `#[non_exhaustive]` because the taxonomy's extension policy makes adding a
/// leaf cheap, and a caller outside this crate that matches exhaustively today
/// would break on the next leaf — which is a reason not to add one, and A13
/// wants the opposite pressure.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum Category {
    /// No route to the hub
    HubUnreachable,
    /// Throttled, with or without a retry hint
    HubRateLimited,
    /// Credentials absent
    HubAuthRequired,
    /// Credentials present and refused
    HubAuthRejected,
    /// Terms not accepted for this account
    HubAccessGated,
    /// Repository or revision does not exist
    HubRefNotFound,
    /// Tag repointed between resolve and fetch
    HubRefMoved,
    /// No config, card or licence
    HubMetadataAbsent,
    /// Present and unparseable
    HubMetadataMalformed,
    /// Declares an architecture the weights are not
    HubMetadataDeceptive,
    /// Licence text present, terms unmatchable
    HubLicenceUnparseable,
    /// Terms forbid the attempted use
    HubLicenceForbidsUse,
    /// No progress past the deadline
    TransferStalled,
    /// Stream ended before the declared length
    TransferTruncated,
    /// Content changed between manifest and fetch
    TransferMutated,
    /// Bytes arrived and do not verify
    TransferChecksumMismatch,
    /// Connection lost; resumable
    TransferInterrupted,
    /// Certificate or handshake failure
    TransferTls,
    /// Referenced and not present
    ArtifactMissing,
    /// Present and fails verification (§7.49's re-check)
    ArtifactCorrupt,
    /// Present and cannot be read at all
    ArtifactUnreadable,
    /// A format MCF does not read
    ArtifactFormatUnsupported,
    /// A format MCF reads, malformed
    ArtifactFormatMalformed,
    /// Some shards present, others absent
    ArtifactIncomplete,
    /// Member escapes the extraction root
    ArtifactArchiveTraversal,
    /// Expands beyond its declared size
    ArtifactArchiveOversized,
    /// Held, with unknown fields (§3.6)
    ArtifactProvenanceIncomplete,
    /// Binary absent at spawn
    EngineSpawnNotFound,
    /// Permission or platform refusal
    EngineSpawnRefused,
    /// Died before first output
    EngineExitImmediate,
    /// Died after partial output
    EngineExitMidstream,
    /// Killed by signal, including the OOM killer
    EngineExitSignal,
    /// Alive, silent, past deadline
    EngineHangNoOutput,
    /// Output MCF cannot parse
    EngineProtocolMalformed,
    /// Incompatible engine interface
    EngineProtocolVersion,
    /// Engine declines the model
    EngineLoadRefused,
    /// No vendored engine supports this artifact (D23)
    EngineUnavailable,
    /// None present
    AccelAbsent,
    /// Present, not characterized (§7.8)
    AccelUnrecognized,
    /// No driver
    AccelDriverAbsent,
    /// Driver present, interrogation failed
    AccelDriverQueryFailed,
    /// Runtime and driver disagree
    AccelDriverVersionMismatch,
    /// Allocation refused
    AccelMemoryExhausted,
    /// Free but unallocatable
    AccelMemoryFragmented,
    /// Sustained throttle
    AccelThermalCeiling,
    /// Device reset mid-operation
    AccelReset,
    /// Disappeared from the bus
    AccelLost,
    /// No space, operation in flight
    ResourceDiskExhausted,
    /// Volume remounted read-only
    ResourceDiskReadonly,
    /// Quota refused the write
    ResourceDiskQuota,
    /// Host allocation refused
    ResourceMemoryExhausted,
    /// Allocatable but degraded
    ResourceMemoryPressure,
    /// Descriptor limit
    ResourceFdExhausted,
    /// Another process holds what was needed (B24)
    ResourceContended,
    /// Store cannot be opened for writing
    RecordUnwritable,
    /// Derived database damaged; journal intact (D20)
    RecordCorruptIndex,
    /// Journal damaged
    RecordCorruptJournal,
    /// Rebuilt, with a stated gap
    RecordReplayIncomplete,
    /// Written by a version this one cannot read (§7.30)
    RecordSchemaUnknown,
    /// Retention limit reached (§7.5)
    RecordBudgetExhausted,
    /// Value outside the permitted domain
    ConfigInvalid,
    /// Coherent and impossible on this machine
    ConfigUnsatisfiable,
    /// Two settings that cannot both hold
    ConfigConflict,
    /// Declared, never probed, and required to be (A21)
    ConfigUnverified,
    /// Realized differs from declared (D23, placement)
    ConfigIdentityMismatch,
    /// Neither confirms nor denies (§3.18)
    ProbeInconclusive,
    /// No result within its bound
    ProbeTimeout,
    /// Output the probe cannot grade
    ProbeMalformedResponse,
    /// Probe does not apply to this artifact
    ProbeUnsupported,
    /// Declared and verified disagree — a *finding*, not an error
    ProbeDivergence,
    /// Environment could not be constructed
    LabSetupFailed,
    /// Residue left behind (B58)
    LabTeardownFailed,
    /// Capability verified absent (B40) — not a failure
    LabGateNotApplicable,
    /// Capability unestablished (B40)
    LabGateUnknown,
    /// Machine not quiet for a timing run (B-217)
    LabPreconditionContended,
    /// Declared maximum duration reached
    LabBudgetExceeded,
    /// Stopped by the operator; partial preserved
    LabInterrupted,
    /// Supplied workload cannot be graded (B42)
    LabWorkloadUngradable,
    /// Conditions moved mid-run; result unsound
    LabConditionsInvalidated,
    /// Unparseable or missing required arguments
    ModelToolMalformedCall,
    /// Plausible but incorrect tool
    ModelToolWrongSelection,
    /// Tool that does not exist
    ModelToolHallucinated,
    /// Repeats an action with unchanged state
    ModelLoopNoProgress,
    /// Runs to the turn or token budget
    ModelStopNever,
    /// Stops with the task incomplete
    ModelStopPremature,
    /// Receives an error and repeats unchanged
    ModelRecoveryNone,
    /// Correct content, wrong required format
    ModelFormatViolated,
    /// A stated constraint unmet
    ModelInstructionIgnored,
    /// Declined the task
    ModelRefused,
    /// Produced nothing
    ModelOutputEmpty,
    /// Reached for something absent (A14) — recorded, never permitted
    SandboxEscapeAttempted,
    /// Environment quota refused a write
    SandboxQuotaDisk,
    /// Environment limit reached
    SandboxQuotaMemory,
    /// Turn limit reached
    SandboxTurnBudget,
    /// Environment could not be built
    SandboxConstructFailed,
    /// Outside §7.35's declared scope
    PlatformUnsupported,
    /// Elevation refused (A26)
    PlatformPrivilegeDenied,
    /// No mechanism on this platform
    PlatformPrivilegeUnavailable,
    /// Boxing, pinning or yielding unsupported here (§7.43)
    PlatformMechanismUnavailable,
    /// Something changed could not be restored (A27)
    PlatformRestoreFailed,
    /// Wall clock stepped back mid-measurement
    TimeJumpBackward,
    /// Large forward step
    TimeJumpForward,
    /// No monotonic source (D9)
    TimeMonotonicUnavailable,
    /// Unparseable
    ExchangeIdentifierMalformed,
    /// Names something unobtainable (§XV)
    ExchangeIdentifierUnresolvable,
    /// Resolvable and cannot run here — a complete answer (§6.3)
    ExchangeReproduceImpossible,
    /// Reproduced; numbers differ — a *finding* (§6.29)
    ExchangeReproduceDivergent,
    /// Contribution written by an uninterpretable version
    ExchangeSchemaUnreadable,
    /// Terms not shown before sending — a defect (D21)
    ExchangeTermsAbsent,
    /// A state the type system was meant to prevent
    InternalInvariantViolated,
    /// A failure that fits nothing above
    InternalUnclassified,
}

impl Category {
    /// Every category, in the taxonomy's own order.
    pub const ALL: [Self; 111] = [
        Self::HubUnreachable,
        Self::HubRateLimited,
        Self::HubAuthRequired,
        Self::HubAuthRejected,
        Self::HubAccessGated,
        Self::HubRefNotFound,
        Self::HubRefMoved,
        Self::HubMetadataAbsent,
        Self::HubMetadataMalformed,
        Self::HubMetadataDeceptive,
        Self::HubLicenceUnparseable,
        Self::HubLicenceForbidsUse,
        Self::TransferStalled,
        Self::TransferTruncated,
        Self::TransferMutated,
        Self::TransferChecksumMismatch,
        Self::TransferInterrupted,
        Self::TransferTls,
        Self::ArtifactMissing,
        Self::ArtifactCorrupt,
        Self::ArtifactUnreadable,
        Self::ArtifactFormatUnsupported,
        Self::ArtifactFormatMalformed,
        Self::ArtifactIncomplete,
        Self::ArtifactArchiveTraversal,
        Self::ArtifactArchiveOversized,
        Self::ArtifactProvenanceIncomplete,
        Self::EngineSpawnNotFound,
        Self::EngineSpawnRefused,
        Self::EngineExitImmediate,
        Self::EngineExitMidstream,
        Self::EngineExitSignal,
        Self::EngineHangNoOutput,
        Self::EngineProtocolMalformed,
        Self::EngineProtocolVersion,
        Self::EngineLoadRefused,
        Self::EngineUnavailable,
        Self::AccelAbsent,
        Self::AccelUnrecognized,
        Self::AccelDriverAbsent,
        Self::AccelDriverQueryFailed,
        Self::AccelDriverVersionMismatch,
        Self::AccelMemoryExhausted,
        Self::AccelMemoryFragmented,
        Self::AccelThermalCeiling,
        Self::AccelReset,
        Self::AccelLost,
        Self::ResourceDiskExhausted,
        Self::ResourceDiskReadonly,
        Self::ResourceDiskQuota,
        Self::ResourceMemoryExhausted,
        Self::ResourceMemoryPressure,
        Self::ResourceFdExhausted,
        Self::ResourceContended,
        Self::RecordUnwritable,
        Self::RecordCorruptIndex,
        Self::RecordCorruptJournal,
        Self::RecordReplayIncomplete,
        Self::RecordSchemaUnknown,
        Self::RecordBudgetExhausted,
        Self::ConfigInvalid,
        Self::ConfigUnsatisfiable,
        Self::ConfigConflict,
        Self::ConfigUnverified,
        Self::ConfigIdentityMismatch,
        Self::ProbeInconclusive,
        Self::ProbeTimeout,
        Self::ProbeMalformedResponse,
        Self::ProbeUnsupported,
        Self::ProbeDivergence,
        Self::LabSetupFailed,
        Self::LabTeardownFailed,
        Self::LabGateNotApplicable,
        Self::LabGateUnknown,
        Self::LabPreconditionContended,
        Self::LabBudgetExceeded,
        Self::LabInterrupted,
        Self::LabWorkloadUngradable,
        Self::LabConditionsInvalidated,
        Self::ModelToolMalformedCall,
        Self::ModelToolWrongSelection,
        Self::ModelToolHallucinated,
        Self::ModelLoopNoProgress,
        Self::ModelStopNever,
        Self::ModelStopPremature,
        Self::ModelRecoveryNone,
        Self::ModelFormatViolated,
        Self::ModelInstructionIgnored,
        Self::ModelRefused,
        Self::ModelOutputEmpty,
        Self::SandboxEscapeAttempted,
        Self::SandboxQuotaDisk,
        Self::SandboxQuotaMemory,
        Self::SandboxTurnBudget,
        Self::SandboxConstructFailed,
        Self::PlatformUnsupported,
        Self::PlatformPrivilegeDenied,
        Self::PlatformPrivilegeUnavailable,
        Self::PlatformMechanismUnavailable,
        Self::PlatformRestoreFailed,
        Self::TimeJumpBackward,
        Self::TimeJumpForward,
        Self::TimeMonotonicUnavailable,
        Self::ExchangeIdentifierMalformed,
        Self::ExchangeIdentifierUnresolvable,
        Self::ExchangeReproduceImpossible,
        Self::ExchangeReproduceDivergent,
        Self::ExchangeSchemaUnreadable,
        Self::ExchangeTermsAbsent,
        Self::InternalInvariantViolated,
        Self::InternalUnclassified,
    ];

    /// The dotted code, which is what travels in a record and between
    /// machines (§XIV).
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::HubUnreachable => "hub.unreachable",
            Self::HubRateLimited => "hub.rate_limited",
            Self::HubAuthRequired => "hub.auth.required",
            Self::HubAuthRejected => "hub.auth.rejected",
            Self::HubAccessGated => "hub.access.gated",
            Self::HubRefNotFound => "hub.ref.not_found",
            Self::HubRefMoved => "hub.ref.moved",
            Self::HubMetadataAbsent => "hub.metadata.absent",
            Self::HubMetadataMalformed => "hub.metadata.malformed",
            Self::HubMetadataDeceptive => "hub.metadata.deceptive",
            Self::HubLicenceUnparseable => "hub.licence.unparseable",
            Self::HubLicenceForbidsUse => "hub.licence.forbids_use",
            Self::TransferStalled => "transfer.stalled",
            Self::TransferTruncated => "transfer.truncated",
            Self::TransferMutated => "transfer.mutated",
            Self::TransferChecksumMismatch => "transfer.checksum_mismatch",
            Self::TransferInterrupted => "transfer.interrupted",
            Self::TransferTls => "transfer.tls",
            Self::ArtifactMissing => "artifact.missing",
            Self::ArtifactCorrupt => "artifact.corrupt",
            Self::ArtifactUnreadable => "artifact.unreadable",
            Self::ArtifactFormatUnsupported => "artifact.format.unsupported",
            Self::ArtifactFormatMalformed => "artifact.format.malformed",
            Self::ArtifactIncomplete => "artifact.incomplete",
            Self::ArtifactArchiveTraversal => "artifact.archive.traversal",
            Self::ArtifactArchiveOversized => "artifact.archive.oversized",
            Self::ArtifactProvenanceIncomplete => "artifact.provenance.incomplete",
            Self::EngineSpawnNotFound => "engine.spawn.not_found",
            Self::EngineSpawnRefused => "engine.spawn.refused",
            Self::EngineExitImmediate => "engine.exit.immediate",
            Self::EngineExitMidstream => "engine.exit.midstream",
            Self::EngineExitSignal => "engine.exit.signal",
            Self::EngineHangNoOutput => "engine.hang.no_output",
            Self::EngineProtocolMalformed => "engine.protocol.malformed",
            Self::EngineProtocolVersion => "engine.protocol.version",
            Self::EngineLoadRefused => "engine.load.refused",
            Self::EngineUnavailable => "engine.unavailable",
            Self::AccelAbsent => "accel.absent",
            Self::AccelUnrecognized => "accel.unrecognized",
            Self::AccelDriverAbsent => "accel.driver.absent",
            Self::AccelDriverQueryFailed => "accel.driver.query_failed",
            Self::AccelDriverVersionMismatch => "accel.driver.version_mismatch",
            Self::AccelMemoryExhausted => "accel.memory.exhausted",
            Self::AccelMemoryFragmented => "accel.memory.fragmented",
            Self::AccelThermalCeiling => "accel.thermal.ceiling",
            Self::AccelReset => "accel.reset",
            Self::AccelLost => "accel.lost",
            Self::ResourceDiskExhausted => "resource.disk.exhausted",
            Self::ResourceDiskReadonly => "resource.disk.readonly",
            Self::ResourceDiskQuota => "resource.disk.quota",
            Self::ResourceMemoryExhausted => "resource.memory.exhausted",
            Self::ResourceMemoryPressure => "resource.memory.pressure",
            Self::ResourceFdExhausted => "resource.fd.exhausted",
            Self::ResourceContended => "resource.contended",
            Self::RecordUnwritable => "record.unwritable",
            Self::RecordCorruptIndex => "record.corrupt.index",
            Self::RecordCorruptJournal => "record.corrupt.journal",
            Self::RecordReplayIncomplete => "record.replay.incomplete",
            Self::RecordSchemaUnknown => "record.schema.unknown",
            Self::RecordBudgetExhausted => "record.budget.exhausted",
            Self::ConfigInvalid => "config.invalid",
            Self::ConfigUnsatisfiable => "config.unsatisfiable",
            Self::ConfigConflict => "config.conflict",
            Self::ConfigUnverified => "config.unverified",
            Self::ConfigIdentityMismatch => "config.identity.mismatch",
            Self::ProbeInconclusive => "probe.inconclusive",
            Self::ProbeTimeout => "probe.timeout",
            Self::ProbeMalformedResponse => "probe.malformed_response",
            Self::ProbeUnsupported => "probe.unsupported",
            Self::ProbeDivergence => "probe.divergence",
            Self::LabSetupFailed => "lab.setup.failed",
            Self::LabTeardownFailed => "lab.teardown.failed",
            Self::LabGateNotApplicable => "lab.gate.not_applicable",
            Self::LabGateUnknown => "lab.gate.unknown",
            Self::LabPreconditionContended => "lab.precondition.contended",
            Self::LabBudgetExceeded => "lab.budget.exceeded",
            Self::LabInterrupted => "lab.interrupted",
            Self::LabWorkloadUngradable => "lab.workload.ungradable",
            Self::LabConditionsInvalidated => "lab.conditions.invalidated",
            Self::ModelToolMalformedCall => "model.tool.malformed_call",
            Self::ModelToolWrongSelection => "model.tool.wrong_selection",
            Self::ModelToolHallucinated => "model.tool.hallucinated",
            Self::ModelLoopNoProgress => "model.loop.no_progress",
            Self::ModelStopNever => "model.stop.never",
            Self::ModelStopPremature => "model.stop.premature",
            Self::ModelRecoveryNone => "model.recovery.none",
            Self::ModelFormatViolated => "model.format.violated",
            Self::ModelInstructionIgnored => "model.instruction.ignored",
            Self::ModelRefused => "model.refused",
            Self::ModelOutputEmpty => "model.output.empty",
            Self::SandboxEscapeAttempted => "sandbox.escape_attempted",
            Self::SandboxQuotaDisk => "sandbox.quota.disk",
            Self::SandboxQuotaMemory => "sandbox.quota.memory",
            Self::SandboxTurnBudget => "sandbox.turn_budget",
            Self::SandboxConstructFailed => "sandbox.construct_failed",
            Self::PlatformUnsupported => "platform.unsupported",
            Self::PlatformPrivilegeDenied => "platform.privilege.denied",
            Self::PlatformPrivilegeUnavailable => "platform.privilege.unavailable",
            Self::PlatformMechanismUnavailable => "platform.mechanism.unavailable",
            Self::PlatformRestoreFailed => "platform.restore_failed",
            Self::TimeJumpBackward => "time.jump.backward",
            Self::TimeJumpForward => "time.jump.forward",
            Self::TimeMonotonicUnavailable => "time.monotonic.unavailable",
            Self::ExchangeIdentifierMalformed => "exchange.identifier.malformed",
            Self::ExchangeIdentifierUnresolvable => "exchange.identifier.unresolvable",
            Self::ExchangeReproduceImpossible => "exchange.reproduce.impossible",
            Self::ExchangeReproduceDivergent => "exchange.reproduce.divergent",
            Self::ExchangeSchemaUnreadable => "exchange.schema.unreadable",
            Self::ExchangeTermsAbsent => "exchange.terms.absent",
            Self::InternalInvariantViolated => "internal.invariant_violated",
            Self::InternalUnclassified => "internal.unclassified",
        }
    }

    /// What the code means, as the taxonomy states it.
    #[must_use]
    pub const fn meaning(self) -> &'static str {
        match self {
            Self::HubUnreachable => "No route to the hub",
            Self::HubRateLimited => "Throttled, with or without a retry hint",
            Self::HubAuthRequired => "Credentials absent",
            Self::HubAuthRejected => "Credentials present and refused",
            Self::HubAccessGated => "Terms not accepted for this account",
            Self::HubRefNotFound => "Repository or revision does not exist",
            Self::HubRefMoved => "Tag repointed between resolve and fetch",
            Self::HubMetadataAbsent => "No config, card or licence",
            Self::HubMetadataMalformed => "Present and unparseable",
            Self::HubMetadataDeceptive => "Declares an architecture the weights are not",
            Self::HubLicenceUnparseable => "Licence text present, terms unmatchable",
            Self::HubLicenceForbidsUse => "Terms forbid the attempted use",
            Self::TransferStalled => "No progress past the deadline",
            Self::TransferTruncated => "Stream ended before the declared length",
            Self::TransferMutated => "Content changed between manifest and fetch",
            Self::TransferChecksumMismatch => "Bytes arrived and do not verify",
            Self::TransferInterrupted => "Connection lost; resumable",
            Self::TransferTls => "Certificate or handshake failure",
            Self::ArtifactMissing => "Referenced and not present",
            Self::ArtifactCorrupt => "Present and fails verification (§7.49's re-check)",
            Self::ArtifactUnreadable => "Present and cannot be read at all",
            Self::ArtifactFormatUnsupported => "A format MCF does not read",
            Self::ArtifactFormatMalformed => "A format MCF reads, malformed",
            Self::ArtifactIncomplete => "Some shards present, others absent",
            Self::ArtifactArchiveTraversal => "Member escapes the extraction root",
            Self::ArtifactArchiveOversized => "Expands beyond its declared size",
            Self::ArtifactProvenanceIncomplete => "Held, with unknown fields (§3.6)",
            Self::EngineSpawnNotFound => "Binary absent at spawn",
            Self::EngineSpawnRefused => "Permission or platform refusal",
            Self::EngineExitImmediate => "Died before first output",
            Self::EngineExitMidstream => "Died after partial output",
            Self::EngineExitSignal => "Killed by signal, including the OOM killer",
            Self::EngineHangNoOutput => "Alive, silent, past deadline",
            Self::EngineProtocolMalformed => "Output MCF cannot parse",
            Self::EngineProtocolVersion => "Incompatible engine interface",
            Self::EngineLoadRefused => "Engine declines the model",
            Self::EngineUnavailable => "No vendored engine supports this artifact (D23)",
            Self::AccelAbsent => "None present",
            Self::AccelUnrecognized => "Present, not characterized (§7.8)",
            Self::AccelDriverAbsent => "No driver",
            Self::AccelDriverQueryFailed => "Driver present, interrogation failed",
            Self::AccelDriverVersionMismatch => "Runtime and driver disagree",
            Self::AccelMemoryExhausted => "Allocation refused",
            Self::AccelMemoryFragmented => "Free but unallocatable",
            Self::AccelThermalCeiling => "Sustained throttle",
            Self::AccelReset => "Device reset mid-operation",
            Self::AccelLost => "Disappeared from the bus",
            Self::ResourceDiskExhausted => "No space, operation in flight",
            Self::ResourceDiskReadonly => "Volume remounted read-only",
            Self::ResourceDiskQuota => "Quota refused the write",
            Self::ResourceMemoryExhausted => "Host allocation refused",
            Self::ResourceMemoryPressure => "Allocatable but degraded",
            Self::ResourceFdExhausted => "Descriptor limit",
            Self::ResourceContended => "Another process holds what was needed (B24)",
            Self::RecordUnwritable => "Store cannot be opened for writing",
            Self::RecordCorruptIndex => "Derived database damaged; journal intact (D20)",
            Self::RecordCorruptJournal => "Journal damaged",
            Self::RecordReplayIncomplete => "Rebuilt, with a stated gap",
            Self::RecordSchemaUnknown => "Written by a version this one cannot read (§7.30)",
            Self::RecordBudgetExhausted => "Retention limit reached (§7.5)",
            Self::ConfigInvalid => "Value outside the permitted domain",
            Self::ConfigUnsatisfiable => "Coherent and impossible on this machine",
            Self::ConfigConflict => "Two settings that cannot both hold",
            Self::ConfigUnverified => "Declared, never probed, and required to be (A21)",
            Self::ConfigIdentityMismatch => "Realized differs from declared (D23, placement)",
            Self::ProbeInconclusive => "Neither confirms nor denies (§3.18)",
            Self::ProbeTimeout => "No result within its bound",
            Self::ProbeMalformedResponse => "Output the probe cannot grade",
            Self::ProbeUnsupported => "Probe does not apply to this artifact",
            Self::ProbeDivergence => "Declared and verified disagree — a *finding*, not an error",
            Self::LabSetupFailed => "Environment could not be constructed",
            Self::LabTeardownFailed => "Residue left behind (B58)",
            Self::LabGateNotApplicable => "Capability verified absent (B40) — not a failure",
            Self::LabGateUnknown => "Capability unestablished (B40)",
            Self::LabPreconditionContended => "Machine not quiet for a timing run (B-217)",
            Self::LabBudgetExceeded => "Declared maximum duration reached",
            Self::LabInterrupted => "Stopped by the operator; partial preserved",
            Self::LabWorkloadUngradable => "Supplied workload cannot be graded (B42)",
            Self::LabConditionsInvalidated => "Conditions moved mid-run; result unsound",
            Self::ModelToolMalformedCall => "Unparseable or missing required arguments",
            Self::ModelToolWrongSelection => "Plausible but incorrect tool",
            Self::ModelToolHallucinated => "Tool that does not exist",
            Self::ModelLoopNoProgress => "Repeats an action with unchanged state",
            Self::ModelStopNever => "Runs to the turn or token budget",
            Self::ModelStopPremature => "Stops with the task incomplete",
            Self::ModelRecoveryNone => "Receives an error and repeats unchanged",
            Self::ModelFormatViolated => "Correct content, wrong required format",
            Self::ModelInstructionIgnored => "A stated constraint unmet",
            Self::ModelRefused => "Declined the task",
            Self::ModelOutputEmpty => "Produced nothing",
            Self::SandboxEscapeAttempted => {
                "Reached for something absent (A14) — recorded, never permitted"
            }
            Self::SandboxQuotaDisk => "Environment quota refused a write",
            Self::SandboxQuotaMemory => "Environment limit reached",
            Self::SandboxTurnBudget => "Turn limit reached",
            Self::SandboxConstructFailed => "Environment could not be built",
            Self::PlatformUnsupported => "Outside §7.35's declared scope",
            Self::PlatformPrivilegeDenied => "Elevation refused (A26)",
            Self::PlatformPrivilegeUnavailable => "No mechanism on this platform",
            Self::PlatformMechanismUnavailable => {
                "Boxing, pinning or yielding unsupported here (§7.43)"
            }
            Self::PlatformRestoreFailed => "Something changed could not be restored (A27)",
            Self::TimeJumpBackward => "Wall clock stepped back mid-measurement",
            Self::TimeJumpForward => "Large forward step",
            Self::TimeMonotonicUnavailable => "No monotonic source (D9)",
            Self::ExchangeIdentifierMalformed => "Unparseable",
            Self::ExchangeIdentifierUnresolvable => "Names something unobtainable (§XV)",
            Self::ExchangeReproduceImpossible => {
                "Resolvable and cannot run here — a complete answer (§6.3)"
            }
            Self::ExchangeReproduceDivergent => "Reproduced; numbers differ — a *finding* (§6.29)",
            Self::ExchangeSchemaUnreadable => "Contribution written by an uninterpretable version",
            Self::ExchangeTermsAbsent => "Terms not shown before sending — a defect (D21)",
            Self::InternalInvariantViolated => "A state the type system was meant to prevent",
            Self::InternalUnclassified => "A failure that fits nothing above",
        }
    }

    /// The domain the code belongs to.
    #[must_use]
    pub const fn domain(self) -> Domain {
        match self {
            Self::HubUnreachable => Domain::Hub,
            Self::HubRateLimited => Domain::Hub,
            Self::HubAuthRequired => Domain::Hub,
            Self::HubAuthRejected => Domain::Hub,
            Self::HubAccessGated => Domain::Hub,
            Self::HubRefNotFound => Domain::Hub,
            Self::HubRefMoved => Domain::Hub,
            Self::HubMetadataAbsent => Domain::Hub,
            Self::HubMetadataMalformed => Domain::Hub,
            Self::HubMetadataDeceptive => Domain::Hub,
            Self::HubLicenceUnparseable => Domain::Hub,
            Self::HubLicenceForbidsUse => Domain::Hub,
            Self::TransferStalled => Domain::Transfer,
            Self::TransferTruncated => Domain::Transfer,
            Self::TransferMutated => Domain::Transfer,
            Self::TransferChecksumMismatch => Domain::Transfer,
            Self::TransferInterrupted => Domain::Transfer,
            Self::TransferTls => Domain::Transfer,
            Self::ArtifactMissing => Domain::Artifact,
            Self::ArtifactCorrupt => Domain::Artifact,
            Self::ArtifactUnreadable => Domain::Artifact,
            Self::ArtifactFormatUnsupported => Domain::Artifact,
            Self::ArtifactFormatMalformed => Domain::Artifact,
            Self::ArtifactIncomplete => Domain::Artifact,
            Self::ArtifactArchiveTraversal => Domain::Artifact,
            Self::ArtifactArchiveOversized => Domain::Artifact,
            Self::ArtifactProvenanceIncomplete => Domain::Artifact,
            Self::EngineSpawnNotFound => Domain::Engine,
            Self::EngineSpawnRefused => Domain::Engine,
            Self::EngineExitImmediate => Domain::Engine,
            Self::EngineExitMidstream => Domain::Engine,
            Self::EngineExitSignal => Domain::Engine,
            Self::EngineHangNoOutput => Domain::Engine,
            Self::EngineProtocolMalformed => Domain::Engine,
            Self::EngineProtocolVersion => Domain::Engine,
            Self::EngineLoadRefused => Domain::Engine,
            Self::EngineUnavailable => Domain::Engine,
            Self::AccelAbsent => Domain::Accel,
            Self::AccelUnrecognized => Domain::Accel,
            Self::AccelDriverAbsent => Domain::Accel,
            Self::AccelDriverQueryFailed => Domain::Accel,
            Self::AccelDriverVersionMismatch => Domain::Accel,
            Self::AccelMemoryExhausted => Domain::Accel,
            Self::AccelMemoryFragmented => Domain::Accel,
            Self::AccelThermalCeiling => Domain::Accel,
            Self::AccelReset => Domain::Accel,
            Self::AccelLost => Domain::Accel,
            Self::ResourceDiskExhausted => Domain::Resource,
            Self::ResourceDiskReadonly => Domain::Resource,
            Self::ResourceDiskQuota => Domain::Resource,
            Self::ResourceMemoryExhausted => Domain::Resource,
            Self::ResourceMemoryPressure => Domain::Resource,
            Self::ResourceFdExhausted => Domain::Resource,
            Self::ResourceContended => Domain::Resource,
            Self::RecordUnwritable => Domain::Record,
            Self::RecordCorruptIndex => Domain::Record,
            Self::RecordCorruptJournal => Domain::Record,
            Self::RecordReplayIncomplete => Domain::Record,
            Self::RecordSchemaUnknown => Domain::Record,
            Self::RecordBudgetExhausted => Domain::Record,
            Self::ConfigInvalid => Domain::Config,
            Self::ConfigUnsatisfiable => Domain::Config,
            Self::ConfigConflict => Domain::Config,
            Self::ConfigUnverified => Domain::Config,
            Self::ConfigIdentityMismatch => Domain::Config,
            Self::ProbeInconclusive => Domain::Probe,
            Self::ProbeTimeout => Domain::Probe,
            Self::ProbeMalformedResponse => Domain::Probe,
            Self::ProbeUnsupported => Domain::Probe,
            Self::ProbeDivergence => Domain::Probe,
            Self::LabSetupFailed => Domain::Lab,
            Self::LabTeardownFailed => Domain::Lab,
            Self::LabGateNotApplicable => Domain::Lab,
            Self::LabGateUnknown => Domain::Lab,
            Self::LabPreconditionContended => Domain::Lab,
            Self::LabBudgetExceeded => Domain::Lab,
            Self::LabInterrupted => Domain::Lab,
            Self::LabWorkloadUngradable => Domain::Lab,
            Self::LabConditionsInvalidated => Domain::Lab,
            Self::ModelToolMalformedCall => Domain::Model,
            Self::ModelToolWrongSelection => Domain::Model,
            Self::ModelToolHallucinated => Domain::Model,
            Self::ModelLoopNoProgress => Domain::Model,
            Self::ModelStopNever => Domain::Model,
            Self::ModelStopPremature => Domain::Model,
            Self::ModelRecoveryNone => Domain::Model,
            Self::ModelFormatViolated => Domain::Model,
            Self::ModelInstructionIgnored => Domain::Model,
            Self::ModelRefused => Domain::Model,
            Self::ModelOutputEmpty => Domain::Model,
            Self::SandboxEscapeAttempted => Domain::Sandbox,
            Self::SandboxQuotaDisk => Domain::Sandbox,
            Self::SandboxQuotaMemory => Domain::Sandbox,
            Self::SandboxTurnBudget => Domain::Sandbox,
            Self::SandboxConstructFailed => Domain::Sandbox,
            Self::PlatformUnsupported => Domain::Platform,
            Self::PlatformPrivilegeDenied => Domain::Platform,
            Self::PlatformPrivilegeUnavailable => Domain::Platform,
            Self::PlatformMechanismUnavailable => Domain::Platform,
            Self::PlatformRestoreFailed => Domain::Platform,
            Self::TimeJumpBackward => Domain::Time,
            Self::TimeJumpForward => Domain::Time,
            Self::TimeMonotonicUnavailable => Domain::Time,
            Self::ExchangeIdentifierMalformed => Domain::Exchange,
            Self::ExchangeIdentifierUnresolvable => Domain::Exchange,
            Self::ExchangeReproduceImpossible => Domain::Exchange,
            Self::ExchangeReproduceDivergent => Domain::Exchange,
            Self::ExchangeSchemaUnreadable => Domain::Exchange,
            Self::ExchangeTermsAbsent => Domain::Exchange,
            Self::InternalInvariantViolated => Domain::Internal,
            Self::InternalUnclassified => Domain::Internal,
        }
    }

    /// The category a dotted code names, if it names one.
    ///
    /// Returns `None` rather than a fallback to
    /// [`Category::InternalUnclassified`]: a code this version cannot read is
    /// `record.schema.unknown`, and deciding which of those it is belongs to
    /// the caller that has the context, not to a parser (A7).
    #[must_use]
    pub fn from_code(code: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|category| category.code() == code)
    }
}

impl fmt::Display for Category {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.code())
    }
}

/// Whose failure this is (B-233, B49, §7.10, §3.1).
///
/// **The failure this exists to prevent.** An out-of-memory caused by another
/// process competing for the machine is a condition of the run. Recorded as
/// the model's, it becomes *this model gave up* — a claim about a model
/// arrived at by measuring a busy afternoon. B-233: every failure of a
/// yielding run classifies to one branch or the other, never ambiguously.
///
/// **Four branches, not two.** *Environment or model* would force MCF's own
/// bugs into *environment*, which is the same error in the other direction —
/// blaming the machine for what MCF did. And an artifact that is corrupt on
/// disk is neither: the run never happened, so there is nothing to attribute
/// to a model that was never asked.
///
/// **Read from the attribution and never from the category**, which is a
/// correction worth stating because the other way round is the obvious design
/// and it is wrong. `probe.inconclusive` is MCF's when its own logic could not
/// decide and the machine's when the machine misbehaved; `engine.unavailable`
/// is MCF's when its stand-in does not implement a format and the machine's
/// when nothing is installed; `config.invalid` is the operator's. A category
/// says *what went wrong*. Only the attribution says *whose*, which is why it
/// is a separate axis that every failure must supply.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum Branch {
    /// The model under test did this: it looped, it ignored the format, it
    /// never stopped, it emitted a call to a tool that does not exist.
    ///
    /// The only branch that is evidence about a model.
    TheModel,
    /// The machine or its surroundings did this: memory exhausted, an
    /// accelerator lost, a disk full, a network unreachable, a clock that
    /// jumped.
    ///
    /// A condition of the run (§3.4). It says what happened *around* a
    /// measurement, and nothing whatever about the thing measured.
    TheEnvironment,
    /// The artifact was not what it needed to be: absent, corrupt, truncated,
    /// in a format nothing here reads.
    ///
    /// Distinct from the model, because the run never happened: there is no
    /// behaviour to attribute to something that was never asked a question.
    TheArtifact,
    /// MCF did this.
    ///
    /// Its own record it cannot write, its own configuration it cannot make
    /// sense of, its own laboratory machinery. Kept separate so that MCF's
    /// bugs cannot be read as the machine's bad luck — which is the same
    /// laundering B-233 forbids, pointed inwards.
    McfItself,
}

impl fmt::Display for Branch {
    fn fmt(&self, form: &mut fmt::Formatter<'_>) -> fmt::Result {
        form.write_str(match self {
            Self::TheModel => "the model under test",
            Self::TheEnvironment => "the environment the run happened in",
            Self::TheArtifact => "the artifact",
            Self::McfItself => "MCF itself",
        })
    }
}
