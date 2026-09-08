use core::fmt;

use crate::time::Timestamp;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Observation {
    pub looked_at: Timestamp,
    pub found: Decay,
}

impl Observation {
    #[must_use]
    pub const fn new(looked_at: Timestamp, found: Decay) -> Self {
        Self { looked_at, found }
    }
}

impl fmt::Display for Observation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} — {}", self.looked_at, self.found)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Decay {
    Unchanged,
    RevisionGone {
        revision: String,
    },
    Relicensed {
        was: String,
        now: String,
    },
    Gated {
        how: String,
    },
    Replaced {
        file: String,
        was: String,
        now: String,
    },
    Unreachable {
        said: String,
    },
}

impl Decay {
    #[must_use]
    pub const fn is_a_change(&self) -> bool {
        matches!(
            self,
            Self::RevisionGone { .. }
                | Self::Relicensed { .. }
                | Self::Gated { .. }
                | Self::Replaced { .. }
        )
    }

    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Unchanged => "unchanged",
            Self::RevisionGone { .. } => "revision_gone",
            Self::Relicensed { .. } => "relicensed",
            Self::Gated { .. } => "gated",
            Self::Replaced { .. } => "replaced",
            Self::Unreachable { .. } => "unreachable",
        }
    }
}

impl fmt::Display for Decay {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unchanged => f.write_str("nothing MCF compared has changed"),
            Self::RevisionGone { revision } => {
                write!(f, "the pinned revision {revision} is not there any more")
            }
            Self::Relicensed { was, now } => {
                write!(f, "the declared licence was {was} and is now {now}")
            }
            Self::Gated { how } => write!(f, "the repository is gated now ({how})"),
            Self::Replaced { file, was, now } => write!(
                f,
                "{file} is published with a different digest: was {was}, now {now}"
            ),
            Self::Unreachable { said } if said.starts_with("hub.auth") => write!(
                f,
                "the hub would not say: {said}. That is the same answer it gives for a \
                 repository that is private and one that never existed, so this is what was \
                 observed rather than what happened"
            ),
            Self::Unreachable { said } => write!(
                f,
                "MCF got no answer about it: {said}. That is a fact about reaching the hub \
                 and says nothing about whether anything there has changed"
            ),
        }
    }
}

#[cfg(test)]
mod tests;
