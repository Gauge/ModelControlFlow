#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Capability<T> {
    declared: Option<T>,
    verified: Option<T>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum State {
    Unknown,
    Declared,
    Verified,
    Diverged,
}

impl<T> Capability<T> {
    #[must_use]
    pub const fn unknown() -> Self {
        Self {
            declared: None,
            verified: None,
        }
    }

    #[must_use]
    pub const fn declared(claim: T) -> Self {
        Self {
            declared: Some(claim),
            verified: None,
        }
    }

    #[must_use]
    pub const fn verified(found: T) -> Self {
        Self {
            declared: None,
            verified: Some(found),
        }
    }

    #[must_use]
    pub fn and_declared(mut self, claim: T) -> Self {
        self.declared = Some(claim);
        self
    }

    #[must_use]
    pub fn and_verified(mut self, found: T) -> Self {
        self.verified = Some(found);
        self
    }

    #[must_use]
    pub const fn declaration(&self) -> Option<&T> {
        self.declared.as_ref()
    }

    #[must_use]
    pub const fn observation(&self) -> Option<&T> {
        self.verified.as_ref()
    }
}

impl<T: PartialEq> Capability<T> {
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

    #[must_use]
    pub fn divergence(&self) -> Option<(&T, &T)> {
        match (&self.declared, &self.verified) {
            (Some(declared), Some(verified)) if declared != verified => Some((declared, verified)),
            _ => None,
        }
    }

    #[must_use]
    pub fn is_established(&self) -> bool {
        matches!(self.state(), State::Verified)
    }
}

impl State {
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
