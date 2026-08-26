//! The four acts MCF asks about every time (B-039, §6.14).
//!
//! **The line is category, not frequency.** §6.14 resolves the tension between
//! §VI's ease and §3.7's caution in one sentence: MCF picks defaults freely for
//! everything reversible and benign — quantization, context length, runtime,
//! placement — and asks, *every time and however much friction it adds*, before
//! it executes untrusted code, consumes large irrecoverable resources, exposes
//! itself to a network, or destroys an artifact.
//!
//! A gate that softened with repetition would be the thing the rule is written
//! against: the second deletion is not safer than the first, and a tool that
//! stopped asking has decided on the operator's behalf that they meant it.
//!
//! **What this module is.** The four, enumerable, each saying where MCF asks —
//! or that no path exists to ask about yet, which is the honest state of two of
//! them. `checks/tests/the_four_gates.rs` reads this and checks the claims
//! against the tree, so a gate cannot quietly stop existing and a *no path
//! exists* cannot quietly become one.
//!
//! **It is not a permission system.** There are no roles, no policies and no
//! configuration: §VII resists an identity model MCF has no use for, and
//! §3.13 refuses generality nobody asked for. What is here is a list, and the
//! list is checkable.

/// An act MCF asks about every time.
///
/// `#[non_exhaustive]` because adding one is a decision about what MCF gates,
/// which §6.14 states and this enumerates — never a convenience.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum Gated {
    /// Running code that came from somewhere else (§6.4).
    UntrustedExecution,
    /// Spending something that cannot be got back: bandwidth on a metered
    /// link, hours of a download, room on a disk.
    LargeIrrecoverableUse,
    /// Making MCF reachable from another machine (§6.12).
    NetworkExposure,
    /// Destroying an artifact that is here (§3.11).
    Destruction,
}

/// All four, in the order §6.14 names them.
pub const GATED: [Gated; 4] = [
    Gated::UntrustedExecution,
    Gated::LargeIrrecoverableUse,
    Gated::NetworkExposure,
    Gated::Destruction,
];

/// How MCF asks about one of them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Asking {
    /// The act is a command somebody types, and the command is the asking.
    ///
    /// This is what §6.14 means by *asked every time* on a headless surface:
    /// there is no dialogue to raise, and a command that names what it will do
    /// is a better record of consent than a prompt anybody would click through
    /// (A22, §VI).
    ByCommand {
        /// What is typed.
        command: &'static str,
        /// What makes it an authorization rather than a request: the thing
        /// that must be said before MCF will act.
        and: &'static str,
    },
    /// MCF cannot do this at all, so there is nothing to gate yet.
    ///
    /// A stronger statement than a gate, and a weaker position: it holds only
    /// while the capability is absent, and the check that reads this is what
    /// notices when it stops being true.
    NoPathExists {
        /// Why there is none.
        why: &'static str,
    },
}

impl Gated {
    /// What the act is called, in a record and in a message.
    ///
    /// Stable for life (C5): once written it travels.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::UntrustedExecution => "untrusted_execution",
            Self::LargeIrrecoverableUse => "large_irrecoverable_use",
            Self::NetworkExposure => "network_exposure",
            Self::Destruction => "destruction",
        }
    }

    /// The clause that gates it.
    #[must_use]
    pub const fn clause(self) -> &'static str {
        match self {
            Self::UntrustedExecution => "§6.4",
            Self::LargeIrrecoverableUse => "§6.14",
            Self::NetworkExposure => "§6.12",
            Self::Destruction => "§3.11",
        }
    }

    /// Where MCF asks, today.
    ///
    /// Two of the four have no path to gate: MCF runs nothing it acquires and
    /// listens on no network. Saying so here rather than leaving the gate
    /// unbuilt is what lets a check hold the *absence* — B-025 and B-036 are
    /// where each becomes a gate, and this entry is what has to change when
    /// they do.
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
            Self::NetworkExposure => Asking::NoPathExists {
                why: "the control plane is a Unix socket with no bind address, no port and no \
                      flag: exposure is not something a mistake can do because it is not \
                      something MCF can do (B-030, B-036)",
            },
            Self::Destruction => Asking::ByCommand {
                command: "mcf rm",
                and: "a reason is stated and the plan it authorizes is the one that was \
                      previewed. Nothing is deleted at all without --purge (B-027)",
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
