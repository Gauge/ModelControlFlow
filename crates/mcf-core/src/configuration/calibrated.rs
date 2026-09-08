use core::fmt;

use super::Sampling;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Chosen {
    DeclaredByArtifact { read_from: String },
    MeasuredHere { sweep: String },
    PinnedByLaboratory { laboratory: String },
    McfsOwn { because: String },
}

impl Chosen {
    #[must_use]
    pub const fn is_measured(&self) -> bool {
        matches!(*self, Self::MeasuredHere { .. })
    }

    #[must_use]
    pub const fn is_pinned(&self) -> bool {
        matches!(*self, Self::PinnedByLaboratory { .. })
    }
}

impl fmt::Display for Chosen {
    fn fmt(&self, form: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DeclaredByArtifact { read_from } => write!(
                form,
                "declared by the artifact ({read_from}) and unverified here — adopted because the \
                 publisher knows what this model was trained for, and marked because knowing that \
                 is not having measured it (A21, B60)"
            ),
            Self::MeasuredHere { sweep } => {
                write!(form, "measured best on this machine by {sweep}")
            }
            Self::PinnedByLaboratory { laboratory } => write!(
                form,
                "pinned by {laboratory} as its own declared method — results taken this way stay \
                 apart from those that inherited the artifact's recommendation (B60)"
            ),
            Self::McfsOwn { because } => write!(
                form,
                "MCF's own, because {because} — not the model's recommendation and not measured \
                 (B60)"
            ),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Calibrated {
    sampling: Sampling,
    chosen: Chosen,
}

impl Calibrated {
    #[must_use]
    pub fn declared_by_artifact(sampling: Sampling, read_from: impl Into<String>) -> Self {
        Self {
            sampling,
            chosen: Chosen::DeclaredByArtifact {
                read_from: read_from.into(),
            },
        }
    }

    #[must_use]
    pub fn measured_here(sampling: Sampling, sweep: impl Into<String>) -> Self {
        Self {
            sampling,
            chosen: Chosen::MeasuredHere {
                sweep: sweep.into(),
            },
        }
    }

    #[must_use]
    pub fn pinned_by_laboratory(sampling: Sampling, laboratory: impl Into<String>) -> Self {
        Self {
            sampling,
            chosen: Chosen::PinnedByLaboratory {
                laboratory: laboratory.into(),
            },
        }
    }

    #[must_use]
    pub fn mcfs_own(sampling: Sampling, because: impl Into<String>) -> Self {
        Self {
            sampling,
            chosen: Chosen::McfsOwn {
                because: because.into(),
            },
        }
    }

    #[must_use]
    pub const fn sampling(&self) -> &Sampling {
        &self.sampling
    }

    #[must_use]
    pub const fn chosen(&self) -> &Chosen {
        &self.chosen
    }

    #[must_use]
    pub fn comparable_with(&self, other: &Self) -> bool {
        match (&self.chosen, &other.chosen) {
            (
                Chosen::PinnedByLaboratory { laboratory: one },
                Chosen::PinnedByLaboratory { laboratory: other },
            ) => one == other,
            (Chosen::PinnedByLaboratory { .. }, _) | (_, Chosen::PinnedByLaboratory { .. }) => {
                false
            }
            _ => true,
        }
    }
}

impl fmt::Display for Calibrated {
    fn fmt(&self, form: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(form, "{} — {}", self.sampling, self.chosen)
    }
}

#[cfg(test)]
mod tests;
