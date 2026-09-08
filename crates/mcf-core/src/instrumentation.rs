use core::fmt;

use crate::measurement::PartsPerMillion;

#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Profile {
    NothingBeyondTheClock,
    Light {
        watcher: String,
        residual: PartsPerMillion,
    },
    Deep {
        watcher: String,
    },
}

impl Profile {
    #[must_use]
    pub const fn suits_timing(&self) -> bool {
        match self {
            Self::NothingBeyondTheClock | Self::Light { .. } => true,
            Self::Deep { .. } => false,
        }
    }
}

impl fmt::Display for Profile {
    fn fmt(&self, form: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NothingBeyondTheClock => form.write_str("nothing beyond the clock"),
            Self::Light { watcher, residual } => write!(
                form,
                "{watcher}, which moved the measurement by {}.{}% as characterized against the \
                 same run unwatched (B31)",
                residual.0.wrapping_div(10_000),
                residual.0.wrapping_div(1_000).wrapping_rem(10)
            ),
            Self::Deep { watcher } => write!(
                form,
                "{watcher}, deep — which makes this a debugging run and not a timing one \
                 (B-164, §6.2)"
            ),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NotATiming {
    pub watcher: String,
}

impl fmt::Display for NotATiming {
    fn fmt(&self, form: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            form,
            "this run was watched by {}, so a timing taken under it is a timing of the \
             instrument as much as of the model. Deep instrumentation has no single residual \
             to carry as a condition — the overhead depends on what the model did — so the \
             result is a debugging observation and not a measurement (B31, §6.2, §6.25)",
            self.watcher
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Timed<T> {
    value: T,
    under: Profile,
}

impl<T> Timed<T> {
    pub fn new(value: T, under: Profile) -> Result<Self, NotATiming> {
        match &under {
            Profile::Deep { watcher } => Err(NotATiming {
                watcher: watcher.clone(),
            }),
            Profile::NothingBeyondTheClock | Profile::Light { .. } => Ok(Self { value, under }),
        }
    }

    #[must_use]
    pub const fn value(&self) -> &T {
        &self.value
    }

    #[must_use]
    pub const fn under(&self) -> &Profile {
        &self.under
    }
}

#[cfg(test)]
mod tests;
