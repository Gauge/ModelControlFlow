use core::fmt;

use crate::attested::Attested;
use crate::build_identity::BuildIdentity;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ConditionValue {
    Text(String),
    Integer(i64),
}

impl ConditionValue {
    pub fn text(value: impl Into<String>) -> Self {
        Self::Text(value.into())
    }

    #[must_use]
    pub const fn integer(value: i64) -> Self {
        Self::Integer(value)
    }
}

impl fmt::Display for ConditionValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Text(value) => f.write_str(value),
            Self::Integer(value) => write!(f, "{value}"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Floor {
    pub hardware_state: Attested<ConditionValue>,
    pub thermal_state: Attested<ConditionValue>,
    pub driver_versions: Attested<ConditionValue>,
    pub runtime_versions: Attested<ConditionValue>,
    pub quantization: Attested<ConditionValue>,
    pub context_length: Attested<ConditionValue>,
    pub batch_shape: Attested<ConditionValue>,
    pub mcf_configuration: Attested<ConditionValue>,
    pub realized_placement: Attested<ConditionValue>,
    pub instrumentation: Attested<ConditionValue>,
    pub artifact_storage: Attested<ConditionValue>,
    pub seed_set: Attested<ConditionValue>,
    pub reuse: Attested<ConditionValue>,
}

impl Floor {
    #[must_use]
    pub const fn nothing_known() -> Self {
        Self {
            hardware_state: Attested::Unknown,
            thermal_state: Attested::Unknown,
            driver_versions: Attested::Unknown,
            runtime_versions: Attested::Unknown,
            quantization: Attested::Unknown,
            context_length: Attested::Unknown,
            batch_shape: Attested::Unknown,
            mcf_configuration: Attested::Unknown,
            realized_placement: Attested::Unknown,
            instrumentation: Attested::Unknown,
            artifact_storage: Attested::Unknown,
            seed_set: Attested::Unknown,
            reuse: Attested::Unknown,
        }
    }

    #[must_use]
    pub fn entries(&self) -> [(&'static str, &Attested<ConditionValue>); 13] {
        [
            ("hardware_state", &self.hardware_state),
            ("thermal_state", &self.thermal_state),
            ("driver_versions", &self.driver_versions),
            ("runtime_versions", &self.runtime_versions),
            ("quantization", &self.quantization),
            ("context_length", &self.context_length),
            ("batch_shape", &self.batch_shape),
            ("mcf_configuration", &self.mcf_configuration),
            ("realized_placement", &self.realized_placement),
            ("instrumentation", &self.instrumentation),
            ("artifact_storage", &self.artifact_storage),
            ("seed_set", &self.seed_set),
            ("reuse", &self.reuse),
        ]
    }

    #[must_use]
    pub fn known_count(&self) -> usize {
        self.entries()
            .iter()
            .filter(|(_, value)| value.is_known())
            .count()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Conditions {
    mcf: BuildIdentity,
    floor: Floor,
}

impl Conditions {
    #[must_use]
    pub const fn new(mcf: BuildIdentity, floor: Floor) -> Self {
        Self { mcf, floor }
    }

    #[must_use]
    pub const fn mcf(&self) -> BuildIdentity {
        self.mcf
    }

    #[must_use]
    pub const fn floor(&self) -> &Floor {
        &self.floor
    }
}

impl fmt::Display for Conditions {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.mcf)?;
        for (question, value) in self.floor.entries() {
            write!(f, "; {question}={value}")?;
        }
        Ok(())
    }
}
