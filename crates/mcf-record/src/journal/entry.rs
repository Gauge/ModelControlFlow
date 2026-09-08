use core::fmt;

use mcf_core::time::Timestamp;

use crate::json::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum EntryKind {
    MachineProfile,
    Failure,
    SelfCost,
    Trials,
    DaemonStarted,
    DaemonStopped,
    ArtifactAcquired,
    ArtifactChecked,
    ArtifactRemoved,
    ComponentProvisioned,
    ComponentRemoved,
    Generated,
    ModelConfigured,
    Comparison,
    FitmentPlanned,
    ContentionSnapshot,
    ModelProbed,
    ModelTimed,
    ModelHosted,
    ModelUnhosted,
    CrossChecked,
    PromptReported,
    Readings,
}

impl EntryKind {
    pub const ALL: [Self; 23] = [
        Self::MachineProfile,
        Self::Failure,
        Self::SelfCost,
        Self::Trials,
        Self::DaemonStarted,
        Self::DaemonStopped,
        Self::ArtifactAcquired,
        Self::ArtifactChecked,
        Self::ArtifactRemoved,
        Self::ComponentProvisioned,
        Self::ComponentRemoved,
        Self::Generated,
        Self::ModelConfigured,
        Self::Comparison,
        Self::FitmentPlanned,
        Self::ContentionSnapshot,
        Self::ModelProbed,
        Self::ModelTimed,
        Self::ModelHosted,
        Self::ModelUnhosted,
        Self::CrossChecked,
        Self::PromptReported,
        Self::Readings,
    ];

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
            Self::ModelTimed => "model_timed",
            Self::ModelHosted => "model_hosted",
            Self::ModelUnhosted => "model_unhosted",
            Self::CrossChecked => "cross_checked",
            Self::PromptReported => "prompt_reported",
            Self::Readings => "readings",
        }
    }

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

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EntryId(String);

impl EntryId {
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

    #[must_use]
    pub fn as_written(text: &str) -> Self {
        Self(text.to_owned())
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Writer(String);

static WRITERS: core::sync::atomic::AtomicU64 = core::sync::atomic::AtomicU64::new(0);

impl Writer {
    #[must_use]
    pub fn distinct() -> Self {
        let counted = WRITERS.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
        let mut digest = mcf_core::digest::Sha256::new();
        digest.update(&std::process::id().to_le_bytes());
        digest.update(&Timestamp::now().utc_nanos().to_le_bytes());
        digest.update(&counted.to_le_bytes());
        Self(digest.finish().hex().chars().take(8).collect())
    }

    #[must_use]
    pub fn stated(token: &str) -> Self {
        Self(token.to_owned())
    }

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    id: Option<EntryId>,
    kind: EntryKind,
    recorded_at: Timestamp,
    body: Value,
}

impl Entry {
    #[must_use]
    pub fn new(kind: EntryKind, recorded_at: Timestamp, body: Value) -> Self {
        Self {
            id: None,
            kind,
            recorded_at,
            body,
        }
    }

    #[must_use]
    pub fn recorded(id: EntryId, kind: EntryKind, recorded_at: Timestamp, body: Value) -> Self {
        Self {
            id: Some(id),
            kind,
            recorded_at,
            body,
        }
    }

    #[must_use]
    pub const fn id(&self) -> Option<&EntryId> {
        self.id.as_ref()
    }

    #[must_use]
    pub(crate) fn stamped(mut self, id: EntryId) -> Self {
        self.id = Some(id);
        self
    }

    pub(crate) fn identify(&self, writer: &Writer, sequence: u64) -> EntryId {
        EntryId::mint(self.kind, self.recorded_at, writer, sequence)
    }

    #[must_use]
    pub const fn kind(&self) -> EntryKind {
        self.kind
    }

    #[must_use]
    pub const fn recorded_at(&self) -> Timestamp {
        self.recorded_at
    }

    #[must_use]
    pub const fn body(&self) -> &Value {
        &self.body
    }

    #[must_use]
    pub fn to_value(&self) -> Value {
        Value::map([
            (
                "id",
                match &self.id {
                    Some(id) => Value::text(id.as_str()),
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
