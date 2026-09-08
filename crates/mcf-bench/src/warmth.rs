use core::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Warmth {
    Cold,
    Warm,
    Unstated,
}

impl Warmth {
    #[must_use]
    pub fn from_account(loaded: Option<&str>) -> Self {
        match loaded {
            Some("resident" | "resident_in_server" | "resident_in_hosted_server") => Self::Warm,
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Reuse {
    Uniform(Warmth),
    Mixed {
        cold: usize,
        warm: usize,
        unstated: usize,
    },
    Nothing,
}

impl Reuse {
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

    #[must_use]
    pub const fn is_uniform(&self) -> bool {
        matches!(*self, Self::Uniform(_) | Self::Nothing)
    }

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
