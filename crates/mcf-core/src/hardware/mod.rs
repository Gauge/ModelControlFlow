mod accelerator;
mod contention;
pub mod headroom;
mod load;
mod nvml;
mod processor;
mod route_amdgpu;
mod route_files;
mod scheduling;
mod space;
mod storage;
pub mod thermal;
pub mod utilisation;

pub use accelerator::{Accelerator, Characterization, Missing, Reading, Route, routes};
pub use contention::{
    Competitor, NAMED, OVER, Snapshot, Steadiness, sample as contention, steadiness,
};
pub use load::{LoadAverage, load_average};
pub use processor::{Memory, PowerProfile, Processor};
pub use scheduling::{Attributability, Scheduling, TOLERATED_DELAY_PPM, Watch, scheduling};
pub use space::{Space, on as space_on};
pub use storage::{Storage, children_major_faults, of as storage_of};

use core::fmt;

use crate::attested::Attested;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Machine {
    pub processor: Processor,
    pub memory: Memory,
    pub accelerators: Vec<Accelerator>,
    pub power_profile: Attested<PowerProfile>,
    pub load: Attested<LoadAverage>,
}

#[must_use]
pub fn memory_available_now() -> Option<u64> {
    processor::available_now()
}

impl Machine {
    #[must_use]
    pub fn read() -> Self {
        Self::read_through(&accelerator::routes())
    }

    #[must_use]
    pub fn read_through(routes: &[Box<dyn Route>]) -> Self {
        Self {
            processor: processor::read_processor(),
            memory: processor::read_memory(),
            accelerators: accelerator::read_through(routes),
            power_profile: processor::read_power_profile(),
            load: load::load_average(),
        }
    }

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
        writeln!(f, "power profile: {}", self.power_profile)?;
        write!(f, "load, one minute: {}", self.load)
    }
}

#[cfg(test)]
mod tests;
