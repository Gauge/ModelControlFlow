//! The machine, as the experimental apparatus.
//!
//! §3.8: *MCF's measurements are only as good as its understanding of what it
//! is measuring on. Hardware is not a static fact to be read once at install;
//! it is a time-varying condition.* This module reads it, every time it is
//! asked, and never caches — a cached reading standing in for a live one is
//! A7's plausible substitute wearing a cache.
//!
//! **D25 draws the boundary this module exists to apply.** A device is
//! [`Characterized`] when MCF can read, at measurement time, its identity, its
//! driver and runtime versions, its available memory and its thermal state —
//! the four readings §3.8 needs in order to tell "this model is slow" from
//! "this machine was busy". Anything less is [`AttemptedUncharacterized`], and
//! it names which readings are missing rather than saying only that something
//! is.
//!
//! **A vendor is supported by having a route, never a branch.** [`Route`] is
//! the whole extension mechanism: each route declares which readings it can
//! supply and produces a [`Reading`]. Adding a vendor is adding a route, and no
//! measurement path anywhere contains a test for which vendor it is dealing
//! with — B28's principle applied to hardware rather than to models.
//!
//! [`Characterized`]: Characterization::Characterized
//! [`AttemptedUncharacterized`]: Characterization::AttemptedUncharacterized

mod accelerator;
mod nvml;
mod processor;
mod route_files;

pub use accelerator::{Accelerator, Characterization, Missing, Reading, Route, routes};
pub use processor::{Memory, PowerProfile, Processor};

use core::fmt;

use crate::attested::Attested;

/// Everything MCF can currently say about the machine it is running on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Machine {
    /// The processor.
    pub processor: Processor,
    /// Host memory.
    pub memory: Memory,
    /// The accelerators MCF found, in the order the routes reported them.
    ///
    /// Empty is a result, not a gap: A9 makes "there is no accelerator here" a
    /// thing worth saying, and §3.2 makes every result taken afterwards
    /// processor-derived and marked.
    pub accelerators: Vec<Accelerator>,
    /// The host's power or performance profile, where the platform publishes
    /// one.
    pub power_profile: Attested<PowerProfile>,
}

impl Machine {
    /// Reads the machine now.
    ///
    /// Never fails. Every reading MCF cannot take is [`Attested::Unknown`] and
    /// every device it cannot fully interrogate is
    /// [`Characterization::AttemptedUncharacterized`] — §3.2's degrade-and-say-so,
    /// with A7 forbidding the plausible substitute. A machine with no
    /// accelerator, no thermal sensor and no governor produces a complete,
    /// honest profile in which almost everything is `unknown`.
    #[must_use]
    pub fn read() -> Self {
        Self {
            processor: processor::read_processor(),
            memory: processor::read_memory(),
            accelerators: accelerator::read_all(),
            power_profile: processor::read_power_profile(),
        }
    }

    /// Whether every accelerator present is characterized.
    ///
    /// `true` on a machine with none, which is correct rather than a
    /// convenience: there is no device whose state MCF is failing to read.
    /// What such a machine cannot do is produce an accelerator result at all,
    /// and that is [`Machine::accelerators`] being empty.
    #[must_use]
    pub fn every_accelerator_is_characterized(&self) -> bool {
        self.accelerators
            .iter()
            .all(|device| device.characterization() == Characterization::Characterized)
    }
}

impl fmt::Display for Machine {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "{}", self.processor)?;
        writeln!(f, "{}", self.memory)?;
        if self.accelerators.is_empty() {
            writeln!(f, "accelerators: none present")?;
        } else {
            for device in &self.accelerators {
                writeln!(f, "{device}")?;
            }
        }
        write!(f, "power profile: {}", self.power_profile)
    }
}

#[cfg(test)]
mod tests;
