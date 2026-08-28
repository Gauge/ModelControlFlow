//! What a trial reused from the one before it (B-081, §6.13, §6.2).
//!
//! **§6.13's own words:** *caching, reuse and adaptation are permitted, and are
//! expected under §VII — but anything that could change a result must be
//! visible in that result's conditions. A measurement taken with a warm cache
//! is a different measurement from one taken cold, and MCF must know which it
//! produced.*
//!
//! The concrete case MCF already has is residency. D41 holds one model in the
//! daemon after its first request, and F35 measured what that is worth: 0.8
//! seconds of load against 6.8 seconds of forward passes for the shortest
//! request. A trial that paid the load and one that did not are two different
//! measurements, and a set that mixes them is not one measurement at all.
//!
//! **A mixed run says so rather than averaging over it.** That is the whole
//! point of putting this in the condition floor: a floor value of *mixed* is a
//! visible statement that the trials were not alike, and it flows into
//! `Isolation` for free — two arms that differ in warmth *and* in the thing
//! under test are confounded, and A8 withholds the delta.
//!
//! **What this does not do.** §6.13's corollary is sharper than recording:
//! *the benchmark path may not adapt.* Making that true needs MCF to be able
//! to tell the daemon to release what it holds, which is a control it does not
//! have. Recording it is what can be done today, and a run that came out mixed
//! is one whose result says why.

use core::fmt;

/// What one trial found already loaded.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Warmth {
    /// The engine loaded the model for this trial.
    Cold,
    /// The model was already resident when the trial arrived.
    Warm,
    /// The engine did not say.
    ///
    /// Not a guess in either direction (A7). An engine MCF starts per request
    /// and one MCF has not taught to report are both here, and both make the
    /// measurement weaker in the same way: nobody can tell what it reused.
    Unstated,
}

impl Warmth {
    /// What a daemon's account means by its `loaded` field.
    ///
    /// The strings are the daemon's, and are matched rather than parsed
    /// loosely: a state MCF has not been taught is [`Warmth::Unstated`] rather
    /// than whichever of the two it superficially resembles (§7.30, A7).
    #[must_use]
    pub fn from_account(loaded: Option<&str>) -> Self {
        match loaded {
            Some("resident" | "resident_in_server") => Self::Warm,
            Some("loaded" | "loaded_for_this_request" | "per_request_subprocess") => Self::Cold,
            _ => Self::Unstated,
        }
    }
}

impl fmt::Display for Warmth {
    fn fmt(&self, form: &mut fmt::Formatter<'_>) -> fmt::Result {
        form.write_str(match *self {
            Self::Cold => "cold",
            Self::Warm => "warm",
            Self::Unstated => "unstated",
        })
    }
}

/// What a whole run reused, which is only a condition if the trials agreed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Reuse {
    /// Every trial was alike.
    Uniform(Warmth),
    /// They were not.
    ///
    /// **The state this module exists for.** A run that loaded the model for
    /// some trials and not for others has measured two things and reported
    /// one. It is not refused here — the trials happened and A1 keeps them —
    /// but it is named, and naming it is what stops it being averaged over.
    Mixed {
        /// How many trials paid the load.
        cold: usize,
        /// How many did not.
        warm: usize,
        /// How many the engine said nothing about.
        unstated: usize,
    },
    /// There were no trials.
    Nothing,
}

impl Reuse {
    /// What a sequence of trials amounts to.
    #[must_use]
    pub fn over(trials: impl IntoIterator<Item = Warmth>) -> Self {
        let (mut cold, mut warm, mut unstated) = (0_usize, 0_usize, 0_usize);
        for held in trials {
            match held {
                Warmth::Cold => cold = cold.saturating_add(1),
                Warmth::Warm => warm = warm.saturating_add(1),
                Warmth::Unstated => unstated = unstated.saturating_add(1),
            }
        }
        match (cold, warm, unstated) {
            (0, 0, 0) => Self::Nothing,
            (_, 0, 0) => Self::Uniform(Warmth::Cold),
            (0, _, 0) => Self::Uniform(Warmth::Warm),
            (0, 0, _) => Self::Uniform(Warmth::Unstated),
            _ => Self::Mixed {
                cold,
                warm,
                unstated,
            },
        }
    }

    /// Whether the trials were alike.
    #[must_use]
    pub const fn is_uniform(&self) -> bool {
        matches!(*self, Self::Uniform(_) | Self::Nothing)
    }

    /// The condition, as the record and the floor write it.
    ///
    /// One string, because that is what a [`ConditionValue`] is and because a
    /// mixed run has to be legible in the same place a uniform one is — a
    /// reader scanning the floor must not have to know that *mixed* is
    /// recorded somewhere else.
    ///
    /// [`ConditionValue`]: mcf_core::measurement::ConditionValue
    #[must_use]
    pub fn condition(&self) -> String {
        match self {
            Self::Uniform(Warmth::Cold) => {
                "cold: every trial loaded the model for itself".to_owned()
            }
            Self::Uniform(Warmth::Warm) => {
                "warm: the model was already resident for every trial".to_owned()
            }
            Self::Uniform(Warmth::Unstated) => {
                "unstated: the engine did not say what any trial reused".to_owned()
            }
            Self::Mixed {
                cold,
                warm,
                unstated,
            } => format!(
                "MIXED: {cold} trial(s) loaded the model, {warm} found it resident, {unstated} \
                 unstated — this is not one measurement (§6.13)"
            ),
            Self::Nothing => "no trials".to_owned(),
        }
    }
}

impl fmt::Display for Reuse {
    fn fmt(&self, form: &mut fmt::Formatter<'_>) -> fmt::Result {
        form.write_str(&self.condition())
    }
}

#[cfg(test)]
mod tests;
