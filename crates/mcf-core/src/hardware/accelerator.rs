//! Accelerators, and D25's boundary applied to each of them.
//!
//! Four readings decide whether a device is characterized, and they are the
//! ones §3.8 needs in order to tell "this model is slow" from "this machine
//! was busy":
//!
//! * **identity** — which device this is;
//! * **versions** — the driver and runtime in the path, which change what the
//!   same silicon does;
//! * **memory** — how much of the device's memory a new allocation could
//!   obtain, which decides whether a model fits;
//! * **thermal** — its temperature or throttle state, which is the difference
//!   between a slow model and a hot one.
//!
//! A device is [`Characterization::Characterized`] when all four are readable
//! *now*, and [`Characterization::AttemptedUncharacterized`] otherwise, naming
//! which are missing. The state is per read, never cached (D25, §3.8).
//!
//! **Routes, not branches.** A [`Route`] declares which readings it can supply
//! and produces a [`Reading`]. Two routes are compiled in: the files a driver
//! publishes, and the vendor's management library over the C ABI. Nothing
//! outside this module knows which route answered, and no measurement path
//! contains a test for which vendor a device is (B28).
//!
//! **Cross-checked by:** two independent routes. The driver files and the
//! vendor library are read separately and their disagreement is recorded
//! rather than resolved (`routes_disagree_about`), which is A19 built into
//! the reading itself.

use core::fmt;

use crate::attested::Attested;
use crate::measurement::Bytes;

/// One reading MCF needs about a device.
///
/// Named individually so that an uncharacterized device says *which* reading
/// it is missing. "Something is unknown" is the report A2 calls worse than a
/// crash, scaled down to a field.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Missing {
    /// Which device this is.
    Identity,
    /// The driver and runtime versions.
    Versions,
    /// How much device memory is available.
    Memory,
    /// The device's temperature or throttle state.
    Thermal,
}

impl Missing {
    /// Every reading D25 requires, in the order it states them.
    pub const ALL: [Self; 4] = [Self::Identity, Self::Versions, Self::Memory, Self::Thermal];

    /// The reading's name, as it appears in a report.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Identity => "identity",
            Self::Versions => "versions",
            Self::Memory => "memory",
            Self::Thermal => "thermal",
        }
    }
}

impl fmt::Display for Missing {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Whether MCF can say what this device is doing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Characterization {
    /// All four readings were taken. Results are comparable and contributable.
    Characterized,
    /// The device is present and usable and at least one reading was not
    /// taken. Results are marked degraded (A5), are not comparable with
    /// characterized ones (A8), and are not contributable (B54).
    AttemptedUncharacterized {
        /// Which readings were not taken.
        missing: Vec<Missing>,
    },
}

impl fmt::Display for Characterization {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Characterized => f.write_str("characterized"),
            Self::AttemptedUncharacterized { missing } => {
                let names: Vec<&str> = missing.iter().map(|m| m.as_str()).collect();
                write!(
                    f,
                    "attempted, uncharacterized (missing {})",
                    names.join(", ")
                )
            }
        }
    }
}

/// What one route learned about one device.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Reading {
    /// The vendor.
    pub vendor: Attested<String>,
    /// The model.
    pub model: Attested<String>,
    /// The driver version.
    pub driver: Attested<String>,
    /// The compute runtime version, where the device has one distinct from the
    /// driver.
    pub runtime: Attested<String>,
    /// Total device memory.
    pub memory_total: Attested<Bytes>,
    /// Device memory a new allocation could obtain.
    pub memory_available: Attested<Bytes>,
    /// Temperature in whole degrees Celsius.
    pub temperature_c: Attested<u32>,
}

impl Reading {
    /// A reading in which nothing is known.
    ///
    /// The honest starting point for a route: every question asked, none
    /// answered. Named as well as derived, so that a call site can say what it
    /// means rather than saying `Default`.
    #[must_use]
    pub fn nothing_known() -> Self {
        Self::default()
    }

    /// Fills in from another reading anything this one does not know.
    ///
    /// Merging is one-directional on purpose: a route never overwrites a
    /// reading another route already took. Where two routes disagree that is a
    /// finding about one of them (A8) and not something to resolve by
    /// preferring whichever ran last — [`disagreements_with`] is how it is
    /// surfaced instead.
    ///
    /// [`disagreements_with`]: Reading::disagreements_with
    #[must_use]
    pub fn filled_from(mut self, other: &Self) -> Self {
        fill(&mut self.vendor, &other.vendor);
        fill(&mut self.model, &other.model);
        fill(&mut self.driver, &other.driver);
        fill(&mut self.runtime, &other.runtime);
        fill(&mut self.memory_total, &other.memory_total);
        fill(&mut self.memory_available, &other.memory_available);
        fill(&mut self.temperature_c, &other.temperature_c);
        self
    }

    /// The fields both readings claim to know and disagree about.
    ///
    /// A8: two readings of one thing that differ are not two data points. A
    /// disagreement is a finding about a route, and it is reported rather than
    /// silently resolved.
    #[must_use]
    pub fn disagreements_with(&self, other: &Self) -> Vec<&'static str> {
        let mut found = Vec::new();
        if disagree(&self.vendor, &other.vendor) {
            found.push("vendor");
        }
        if disagree(&self.model, &other.model) {
            found.push("model");
        }
        if disagree(&self.driver, &other.driver) {
            found.push("driver");
        }
        if disagree(&self.runtime, &other.runtime) {
            found.push("runtime");
        }
        if disagree(&self.memory_total, &other.memory_total) {
            found.push("memory_total");
        }
        found
    }

    /// Which of D25's four readings this does not have.
    #[must_use]
    pub fn missing(&self) -> Vec<Missing> {
        let mut missing = Vec::new();
        if !self.model.is_known() {
            missing.push(Missing::Identity);
        }
        if !self.driver.is_known() {
            missing.push(Missing::Versions);
        }
        if !self.memory_available.is_known() {
            missing.push(Missing::Memory);
        }
        if !self.temperature_c.is_known() {
            missing.push(Missing::Thermal);
        }
        missing
    }
}

fn fill<T: Clone>(into: &mut Attested<T>, from: &Attested<T>) {
    if let (false, Attested::Known(value)) = (into.is_known(), from) {
        *into = Attested::Known(value.clone());
    }
}

fn disagree<T: PartialEq>(left: &Attested<T>, right: &Attested<T>) -> bool {
    match (left, right) {
        (Attested::Known(a), Attested::Known(b)) => a != b,
        _ => false,
    }
}

/// One accelerator, as MCF currently sees it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Accelerator {
    index: usize,
    reading: Reading,
    routes: Vec<&'static str>,
    disagreements: Vec<&'static str>,
}

impl Accelerator {
    /// Where it sits in the order the routes reported.
    ///
    /// Not an identity. A device's position can change between boots, and
    /// nothing that has to be stable across runs may use it.
    #[must_use]
    pub const fn index(&self) -> usize {
        self.index
    }

    /// What MCF read about it.
    #[must_use]
    pub const fn reading(&self) -> &Reading {
        &self.reading
    }

    /// Which routes answered.
    #[must_use]
    pub fn routes(&self) -> &[&'static str] {
        &self.routes
    }

    /// Fields two routes both claimed and disagreed about (A8).
    #[must_use]
    pub fn disagreements(&self) -> &[&'static str] {
        &self.disagreements
    }

    /// D25's verdict for this device, now.
    #[must_use]
    pub fn characterization(&self) -> Characterization {
        let missing = self.reading.missing();
        if missing.is_empty() {
            Characterization::Characterized
        } else {
            Characterization::AttemptedUncharacterized { missing }
        }
    }
}

impl fmt::Display for Accelerator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "accelerator #{}: {} {} · driver {} · runtime {} · {} of {} free · {} °C · {} [routes: {}]",
            self.index,
            self.reading.vendor,
            self.reading.model,
            self.reading.driver,
            self.reading.runtime,
            self.reading.memory_available,
            self.reading.memory_total,
            self.reading.temperature_c,
            self.characterization(),
            self.routes.join(", "),
        )?;
        if !self.disagreements.is_empty() {
            write!(
                f,
                " [routes disagree about: {}]",
                self.disagreements.join(", ")
            )?;
        }
        Ok(())
    }
}

/// A way of asking a machine about its accelerators.
///
/// The whole extension mechanism. A route states its name and which of D25's
/// readings it can in principle supply, and returns one [`Reading`] per device
/// it found. It never fails: a route that cannot run found nothing, which is a
/// result (A9) and not an error.
pub trait Route {
    /// The route's name, as it appears beside a reading.
    fn name(&self) -> &'static str;

    /// Which of D25's readings this route can supply when it works.
    ///
    /// Declared rather than inferred from what it returned, so that a route
    /// that *should* have supplied a reading and did not is distinguishable
    /// from one that never claimed to (A21's shape: declared and observed are
    /// different things).
    fn covers(&self) -> &'static [Missing];

    /// Asks the machine.
    fn probe(&self) -> Vec<Reading>;
}

/// Every route compiled in, in the order they are consulted.
///
/// The order matters only for which route's name appears first; readings are
/// merged without overwriting, so the answer does not depend on it.
#[must_use]
pub fn routes() -> Vec<Box<dyn Route>> {
    vec![
        Box::new(super::route_files::Files),
        Box::new(super::nvml::VendorLibrary),
    ]
}

/// Reads every accelerator the given routes can see.
pub(super) fn read_through(routes: &[Box<dyn Route>]) -> Vec<Accelerator> {
    let mut devices: Vec<Accelerator> = Vec::new();
    for route in routes {
        for (index, reading) in route.probe().into_iter().enumerate() {
            match devices.get_mut(index) {
                Some(device) => {
                    let mut disagreements = device.reading.disagreements_with(&reading);
                    device.disagreements.append(&mut disagreements);
                    device.disagreements.sort_unstable();
                    device.disagreements.dedup();
                    device.reading = device.reading.clone().filled_from(&reading);
                    device.routes.push(route.name());
                }
                None => devices.push(Accelerator {
                    index,
                    reading,
                    routes: vec![route.name()],
                    disagreements: Vec::new(),
                }),
            }
        }
    }
    devices
}
