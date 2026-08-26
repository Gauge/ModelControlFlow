//! What somebody said, what MCF found, and the difference between them (B-050,
//! B-058, §3.18, A21).
//!
//! **The type exists so that the two cannot be confused.** §3.18 forbids MCF
//! treating a model card's claims as facts, and A21 makes *declared* and
//! *verified* different states rather than degrees of confidence. Held as two
//! fields that are separately optional, a declaration cannot become a
//! verification by being copied into the same variable — which is how it
//! happens when both are a `String`.
//!
//! **Three states, and the fourth is the interesting one.** Nothing known;
//! something declared and unchecked; something MCF checked. And when both exist
//! and disagree, a **divergence** — which B-058 calls, correctly, often the most
//! useful thing MCF can say about a model. A model whose card says one thing and
//! whose weights say another is not a model with bad metadata; it is a model
//! somebody should look at before measuring anything on it.
//!
//! **What it is not.** A confidence level, a score, or a place to put a
//! default. There is no `unwrap_or`: the way to read a capability MCF has not
//! verified is to ask for the declaration *as a declaration*, which is the one
//! sentence this whole module exists to make somebody write.

/// What is known about one claim.
///
/// The two halves are kept apart on purpose: `declared` is somebody else's
/// statement and `verified` is MCF's own observation, and no operation here
/// turns one into the other.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Capability<T> {
    declared: Option<T>,
    verified: Option<T>,
}

/// Which of the four situations a capability is in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum State {
    /// Nobody said and MCF has not looked.
    Unknown,
    /// Somebody said, and MCF has not looked.
    Declared,
    /// MCF looked. Whether anybody said the same thing is
    /// [`Capability::declaration`]'s question.
    Verified,
    /// Both, and they disagree.
    Diverged,
}

impl<T> Capability<T> {
    /// Nothing is known.
    #[must_use]
    pub const fn unknown() -> Self {
        Self {
            declared: None,
            verified: None,
        }
    }

    /// Somebody said this, and MCF has not checked it.
    #[must_use]
    pub const fn declared(claim: T) -> Self {
        Self {
            declared: Some(claim),
            verified: None,
        }
    }

    /// MCF observed this itself.
    #[must_use]
    pub const fn verified(found: T) -> Self {
        Self {
            declared: None,
            verified: Some(found),
        }
    }

    /// Adds a declaration to what MCF observed, or an observation to what was
    /// declared — whichever this is missing.
    ///
    /// Deliberately not a setter for either half: a caller writes
    /// `.and_declared(x)` or `.and_verified(x)`, and what they are adding is in
    /// the name at the call site rather than in a field somewhere above it.
    #[must_use]
    pub fn and_declared(mut self, claim: T) -> Self {
        self.declared = Some(claim);
        self
    }

    /// The same, from the other side.
    #[must_use]
    pub fn and_verified(mut self, found: T) -> Self {
        self.verified = Some(found);
        self
    }

    /// What somebody said, as a declaration.
    ///
    /// Named so that a caller reading it has written the word *declaration* —
    /// which is the point (§3.18).
    #[must_use]
    pub const fn declaration(&self) -> Option<&T> {
        self.declared.as_ref()
    }

    /// What MCF observed.
    #[must_use]
    pub const fn observation(&self) -> Option<&T> {
        self.verified.as_ref()
    }
}

impl<T: PartialEq> Capability<T> {
    /// Which situation this is.
    #[must_use]
    pub fn state(&self) -> State {
        match (&self.declared, &self.verified) {
            (None, None) => State::Unknown,
            (Some(_), None) => State::Declared,
            (None, Some(_)) => State::Verified,
            (Some(declared), Some(verified)) => {
                if declared == verified {
                    State::Verified
                } else {
                    State::Diverged
                }
            }
        }
    }

    /// What was said and what was found, when they disagree.
    ///
    /// `None` when they do not, or when there is nothing to compare. B-058's
    /// finding, in the shape a surface or a record can carry it.
    #[must_use]
    pub fn divergence(&self) -> Option<(&T, &T)> {
        match (&self.declared, &self.verified) {
            (Some(declared), Some(verified)) if declared != verified => Some((declared, verified)),
            _ => None,
        }
    }

    /// Whether MCF may act on this as a fact.
    ///
    /// True only when MCF observed it and nothing contradicts the observation.
    /// A declaration is never enough, which is the whole of §3.18 in one
    /// method — and the reason there is no `unwrap_or`.
    #[must_use]
    pub fn is_established(&self) -> bool {
        matches!(self.state(), State::Verified)
    }
}

impl State {
    /// The state's name, as a record writes it.
    ///
    /// Stable for life (C5).
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Unknown => "unknown",
            Self::Declared => "declared",
            Self::Verified => "verified",
            Self::Diverged => "diverged",
        }
    }
}

impl core::fmt::Display for State {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl<T: PartialEq + core::fmt::Display> core::fmt::Display for Capability<T> {
    /// What a surface shows: the value and which kind of knowing it is, never
    /// one without the other (§3.18, A21).
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match (&self.declared, &self.verified) {
            (None, None) => f.write_str("unknown"),
            (Some(declared), None) => write!(f, "{declared} (declared, unverified)"),
            (None, Some(verified)) => write!(f, "{verified} (verified; nothing declared it)"),
            (Some(declared), Some(verified)) if declared == verified => {
                write!(f, "{verified} (verified, and the declaration agrees)")
            }
            (Some(declared), Some(verified)) => write!(
                f,
                "{verified} (verified) — DIVERGES from the declared {declared}"
            ),
        }
    }
}

impl<T> Default for Capability<T> {
    fn default() -> Self {
        Self::unknown()
    }
}

#[cfg(test)]
mod tests;
