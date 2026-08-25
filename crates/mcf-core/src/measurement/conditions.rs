//! The conditions a measurement is bound to.
//!
//! §3.4: the unit of scientific output is not a number, it is a number bound
//! to the conditions that produced it. §3.3 fixes what the minimum set is and
//! calls it the floor — "everything that varies *and could change a result*" —
//! and says explicitly that the list does not shrink under §VII.
//!
//! [`Floor`] is that list as a struct with no `Default` and no constructor.
//! Rust requires every field of a struct literal, so a caller cannot omit one,
//! and adding a field to the floor breaks every call site — which is the
//! correct amount of friction for adding to a set the intent document says
//! never shrinks (§3.16, B16).
//!
//! Every field is [`Attested`], never a bare value. A7 forbids filling an
//! unknown with a plausible value, so a condition MCF could not read is
//! `Unknown` and stays that way; nothing here can be quietly defaulted.
//!
//! [`Attested`]: crate::attested::Attested
//!
//! **The values are deliberately shallow.** A condition is a
//! [`ConditionValue`] — text or an integer — rather than a rich type, because
//! the subsystems that produce them do not exist yet: the hardware profiler is
//! B-013 (blocked on DEC-008) and the capture path is B-007. What is fixed
//! here is the *set of questions*, which is the part §3.3 makes a floor. The
//! representations sharpen as the producers arrive.

use core::fmt;

use crate::attested::Attested;
use crate::build_identity::BuildIdentity;

/// One condition's value.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ConditionValue {
    /// A name, a version string, a shape written out.
    Text(String),
    /// A count, a length, a temperature in millidegrees.
    Integer(i64),
}

impl ConditionValue {
    /// A textual condition.
    pub fn text(value: impl Into<String>) -> Self {
        Self::Text(value.into())
    }

    /// An integral condition.
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

/// The §3.3 floor: everything that varies and could change a result.
///
/// Nine questions, each answered or explicitly unanswered. Eight are the
/// intent document's own list, in its own order; the ninth is D17's realized
/// placement. The set does not shrink.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Floor {
    /// What the machine is: its processors, its memory, its accelerators.
    pub hardware_state: Attested<ConditionValue>,
    /// Its thermal condition, which changes what the same machine does.
    pub thermal_state: Attested<ConditionValue>,
    /// The versions of the drivers in the path.
    pub driver_versions: Attested<ConditionValue>,
    /// The versions of the runtimes and engines in the path.
    ///
    /// B64 makes this a condition of every absolute figure MCF reports: an
    /// absolute number describes the stack MCF ships, not the hardware's
    /// ceiling.
    pub runtime_versions: Attested<ConditionValue>,
    /// The quantization of the weights under test.
    pub quantization: Attested<ConditionValue>,
    /// The context length in force.
    pub context_length: Attested<ConditionValue>,
    /// The batch shape in force.
    pub batch_shape: Attested<ConditionValue>,
    /// MCF's own configuration, as a digest of the settings in force.
    ///
    /// MCF's *version* is not here: it is [`Conditions::mcf`], which cannot be
    /// unknown because the running binary knows what it is.
    pub mcf_configuration: Attested<ConditionValue>,
    /// The layout that actually resulted — which devices held which layers.
    ///
    /// The ninth question, and the one that does not come from §3.3's list.
    /// Intent v16 splits placement in two: the *declared* intent belongs to the
    /// configuration's identity ([`Placement`]) and the *realized* layout is a
    /// condition, because it names hardware and B57 keeps hardware out of
    /// identity. Divergence between the two is a finding — it is how a
    /// configuration visibly fails to transfer.
    ///
    /// §3.3 says the floor never shrinks. It does not say it never grows, and
    /// "everything that varies and could change a result" plainly reaches this.
    ///
    /// [`Placement`]: crate::configuration::Placement
    pub realized_placement: Attested<ConditionValue>,
}

impl Floor {
    /// A floor in which nothing is known.
    ///
    /// Not a default, and deliberately not named one. It is the honest state
    /// of a measurement taken before any of the producers exist — every
    /// question asked, none answered — and it is what a caller starts from
    /// when it will fill in only the conditions it can actually read. A
    /// measurement built on it is a measurement whose conditions are all
    /// `unknown`, which is a weak claim that says so rather than a strong one
    /// that lies (A7).
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
        }
    }

    /// Each condition, paired with the question it answers, in §3.3's order.
    ///
    /// A surface renders from this rather than from eight field accesses, so a
    /// condition added to the floor appears on every surface without any of
    /// them being edited — A6's "any surface that drops its conditions is
    /// doing damage", made hard to do by accident.
    #[must_use]
    pub fn entries(&self) -> [(&'static str, &Attested<ConditionValue>); 9] {
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
        ]
    }

    /// How many of the floor's questions MCF could answer.
    ///
    /// Reported rather than thresholded: A9 makes a measurement taken under
    /// mostly-unknown conditions a real result, and how much was known is part
    /// of how far it travels.
    #[must_use]
    pub fn known_count(&self) -> usize {
        self.entries()
            .iter()
            .filter(|(_, value)| value.is_known())
            .count()
    }
}

/// Everything a measurement is bound to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Conditions {
    mcf: BuildIdentity,
    floor: Floor,
}

impl Conditions {
    /// Binds a floor to the instrument that read it.
    #[must_use]
    pub const fn new(mcf: BuildIdentity, floor: Floor) -> Self {
        Self { mcf, floor }
    }

    /// The instrument: which MCF, built by which compiler, for which target.
    #[must_use]
    pub const fn mcf(&self) -> BuildIdentity {
        self.mcf
    }

    /// The floor.
    #[must_use]
    pub const fn floor(&self) -> &Floor {
        &self.floor
    }
}

impl fmt::Display for Conditions {
    /// Every condition, none omitted.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.mcf)?;
        for (question, value) in self.floor.entries() {
            write!(f, "; {question}={value}")?;
        }
        Ok(())
    }
}
