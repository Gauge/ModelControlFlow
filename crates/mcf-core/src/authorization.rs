#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum Gated {
    UntrustedExecution,
    LargeIrrecoverableUse,
    NetworkExposure,
    Destruction,
    Publication,
}

pub const GATED: [Gated; 5] = [
    Gated::UntrustedExecution,
    Gated::LargeIrrecoverableUse,
    Gated::NetworkExposure,
    Gated::Destruction,
    Gated::Publication,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Asking {
    ByCommand {
        command: &'static str,
        and: &'static str,
    },
    NoPathExists {
        why: &'static str,
    },
}

impl Gated {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::UntrustedExecution => "untrusted_execution",
            Self::LargeIrrecoverableUse => "large_irrecoverable_use",
            Self::NetworkExposure => "network_exposure",
            Self::Destruction => "destruction",
            Self::Publication => "publication",
        }
    }

    #[must_use]
    pub const fn clause(self) -> &'static str {
        match self {
            Self::UntrustedExecution => "§6.4",
            Self::LargeIrrecoverableUse => "§6.14",
            Self::NetworkExposure => "§6.12",
            Self::Destruction => "§3.11",
            Self::Publication => "§3.20",
        }
    }

    #[must_use]
    pub const fn asking(self) -> Asking {
        match self {
            Self::UntrustedExecution => Asking::NoPathExists {
                why: "MCF executes nothing it acquires: every place shipped code starts a \
                      process is declared, and a model file's contents are data however they \
                      read (B-025)",
            },
            Self::LargeIrrecoverableUse => Asking::ByCommand {
                command: "mcf pull",
                and: "the file is named. A repository asked for without one is answered with \
                      what it publishes, the sizes, and which variants would run here — and \
                      nothing is acquired (B-029, B-213)",
            },
            Self::NetworkExposure => Asking::ByCommand {
                command: "mcf host",
                and: "`--open on` is said with it and an API key is set, since a hold reachable \
                      from the network without one answers anyone; the window's *Reachable \
                      from the network* switch is the same asking, and `mcf unhost` or Stop \
                      revokes it",
            },
            Self::Destruction => Asking::ByCommand {
                command: "mcf rm",
                and: "a reason is stated and the plan it authorizes is the one that was \
                      previewed. Nothing is deleted at all without --purge (B-027)",
            },
            Self::Publication => Asking::NoPathExists {
                why: "MCF sends nothing anywhere: there is no destination, no address to \
                      configure and no path that opens one. `mcf share` writes a file and \
                      renders every row it holds, which is producing rather than sending \
                      (B-160, A24)",
            },
        }
    }
}

impl core::fmt::Display for Gated {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{} ({})", self.as_str(), self.clause())
    }
}

#[cfg(test)]
mod tests;
