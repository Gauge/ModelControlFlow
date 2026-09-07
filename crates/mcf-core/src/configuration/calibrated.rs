//! Where a sampling setting came from, which is not the same question as what
//! it is (B60, D18, A21, B-281).
//!
//! **B60's violation is *a global default temperature applied to every model,
//! which measures each of them under settings some were never designed for*.**
//! The rule against it is not *have no defaults* — a sampler has to be told
//! something — it is that **the choice is attributed**: the artifact's own
//! recommendation is adopted where there is one and marked *declared,
//! unverified* until a sweep has tested it; a laboratory may pin its own only
//! as declared method, with its results kept apart from labs that inherit; and
//! MCF's own choice, where nothing else spoke, is named as MCF's.
//!
//! So [`Sampling`] holds the values and [`Calibrated`] holds the values
//! **together with whose they are**, and there is no constructor that omits
//! the second. A caller that wants a sampling configuration has to say where
//! it came from, which is the whole rule expressed once.
//!
//! **Identity is untouched.** D18 makes the *values* part of a configuration's
//! identity, because they change the distribution; where they came from does
//! not change the distribution and so is a condition rather than identity
//! (D17). Two configurations with the same values are the same runnable thing
//! however each arrived at them — and that is exactly why the attribution has
//! to travel separately rather than be inferred from the numbers.
//!
//! **What "kept apart" means.** A laboratory that pins its own sampling is
//! answering a question about the sampler, not about the model as its publisher
//! intended it. Its results are not wrong; they are about something else. So
//! [`Calibrated::comparable_with`] refuses to put a pinned set beside an
//! inherited one, and the refusal is the same shape A8 gives everywhere else:
//! two things differing in more than the thing under test.
//!
//! [`Sampling`]: super::Sampling

use core::fmt;

use super::Sampling;

/// Whose choice a sampling configuration is.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Chosen {
    /// The artifact recommends it, and nothing has tested it here.
    ///
    /// A21's state: read, never believed. It is adopted because the publisher
    /// knows what the model was trained for and MCF does not, and it is marked
    /// unverified because knowing what it was trained for is not the same as
    /// having measured it on this machine.
    DeclaredByArtifact {
        /// Where MCF read it — the file's own metadata, or the repository's
        /// generation configuration. Named, because *the artifact says so* is
        /// not checkable and *this key in this file says so* is.
        read_from: String,
    },
    /// A sweep on this machine measured it best (B-280, D18).
    ///
    /// The only state in which MCF may call a sampling setting *good* here.
    /// Nothing produces it yet: the sweep is B-280 and is open, and a variant
    /// that exists and is unreachable is the honest way to say so.
    MeasuredHere {
        /// Which sweep, so the measurement can be found again.
        sweep: String,
    },
    /// A laboratory pinned its own, as declared method.
    ///
    /// Permitted by B60 and quarantined by it: results taken this way stay
    /// apart from results that inherited the artifact's recommendation.
    PinnedByLaboratory {
        /// Which laboratory, so a reader knows whose method this is.
        laboratory: String,
    },
    /// MCF's own, because nothing else said anything.
    ///
    /// The state B60 is most wary of, and the reason it is a *named* state
    /// rather than an absence: a house choice that says it is a house choice
    /// is a condition a reader can weigh, and one that does not is a hidden
    /// default (§3.15).
    McfsOwn {
        /// Why, in one line an operator can act on.
        because: String,
    },
}

impl Chosen {
    /// Whether this choice has been measured rather than declared.
    #[must_use]
    pub const fn is_measured(&self) -> bool {
        matches!(*self, Self::MeasuredHere { .. })
    }

    /// Whether a laboratory imposed it.
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

/// A sampling configuration, and whose choice it is.
///
/// The pair, because neither is usable without the other: values with no
/// attribution are a hidden default, and an attribution with no values is a
/// sentence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Calibrated {
    sampling: Sampling,
    chosen: Chosen,
}

impl Calibrated {
    /// What the artifact recommends, read from a named place.
    #[must_use]
    pub fn declared_by_artifact(sampling: Sampling, read_from: impl Into<String>) -> Self {
        Self {
            sampling,
            chosen: Chosen::DeclaredByArtifact {
                read_from: read_from.into(),
            },
        }
    }

    /// What a sweep measured best here.
    #[must_use]
    pub fn measured_here(sampling: Sampling, sweep: impl Into<String>) -> Self {
        Self {
            sampling,
            chosen: Chosen::MeasuredHere {
                sweep: sweep.into(),
            },
        }
    }

    /// What a laboratory pinned as its own method.
    #[must_use]
    pub fn pinned_by_laboratory(sampling: Sampling, laboratory: impl Into<String>) -> Self {
        Self {
            sampling,
            chosen: Chosen::PinnedByLaboratory {
                laboratory: laboratory.into(),
            },
        }
    }

    /// MCF's own choice, with the reason it had to make one.
    ///
    /// The reason is an argument rather than a constant because it differs:
    /// *the artifact recommends nothing* and *the operator asked for this* are
    /// different sentences and a reader needs the right one.
    #[must_use]
    pub fn mcfs_own(sampling: Sampling, because: impl Into<String>) -> Self {
        Self {
            sampling,
            chosen: Chosen::McfsOwn {
                because: because.into(),
            },
        }
    }

    /// The values.
    #[must_use]
    pub const fn sampling(&self) -> &Sampling {
        &self.sampling
    }

    /// Whose choice they are.
    #[must_use]
    pub const fn chosen(&self) -> &Chosen {
        &self.chosen
    }

    /// Whether results taken under these two may be put side by side (B60).
    ///
    /// A laboratory's pinned method is quarantined from everything else, in
    /// both directions — including from another laboratory's pin, since two
    /// laboratories that each imposed their own are two methods rather than
    /// one. Everything that did *not* pin is comparable: an artifact's
    /// recommendation, a sweep's measurement and MCF's own choice are all
    /// answers to *how should this model be sampled*, and A8 already refuses
    /// the case where the answers differ.
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
