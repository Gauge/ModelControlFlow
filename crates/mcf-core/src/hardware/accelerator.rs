use core::fmt;

use crate::attested::Attested;
use crate::measurement::Bytes;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Missing {
    Identity,
    Versions,
    Memory,
    Thermal,
}

impl Missing {
    pub const ALL: [Self; 4] = [Self::Identity, Self::Versions, Self::Memory, Self::Thermal];

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Characterization {
    Characterized,
    AttemptedUncharacterized { missing: Vec<Missing> },
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

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Reading {
    pub vendor: Attested<String>,
    pub model: Attested<String>,
    pub driver: Attested<String>,
    pub runtime: Attested<String>,
    pub memory_total: Attested<Bytes>,
    pub memory_available: Attested<Bytes>,
    pub temperature_c: Attested<u32>,
}

impl Reading {
    #[must_use]
    pub fn nothing_known() -> Self {
        Self::default()
    }

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Accelerator {
    index: usize,
    reading: Reading,
    routes: Vec<&'static str>,
    disagreements: Vec<&'static str>,
}

impl Accelerator {
    #[must_use]
    pub const fn index(&self) -> usize {
        self.index
    }

    #[must_use]
    pub const fn reading(&self) -> &Reading {
        &self.reading
    }

    #[must_use]
    pub fn routes(&self) -> &[&'static str] {
        &self.routes
    }

    #[must_use]
    pub fn disagreements(&self) -> &[&'static str] {
        &self.disagreements
    }

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

pub trait Route {
    fn name(&self) -> &'static str;

    fn covers(&self) -> &'static [Missing];

    fn probe(&self) -> Vec<Reading>;
}

#[must_use]
pub fn routes() -> Vec<Box<dyn Route>> {
    vec![
        Box::new(super::route_files::Files),
        Box::new(super::nvml::VendorLibrary),
        Box::new(super::route_amdgpu::Amdgpu::default()),
    ]
}

pub(super) fn read_through(routes: &[Box<dyn Route>]) -> Vec<Accelerator> {
    let mut devices: Vec<Accelerator> = Vec::new();
    for route in routes {
        for (index, reading) in route.probe().into_iter().enumerate() {
            let another = devices
                .get(index)
                .is_some_and(|held| disagree(&held.reading.vendor, &reading.vendor));
            let at = if another { devices.len() } else { index };
            match devices.get_mut(at) {
                Some(device) => {
                    let mut disagreements = device.reading.disagreements_with(&reading);
                    device.disagreements.append(&mut disagreements);
                    device.disagreements.sort_unstable();
                    device.disagreements.dedup();
                    device.reading = device.reading.clone().filled_from(&reading);
                    device.routes.push(route.name());
                }
                None => devices.push(Accelerator {
                    index: at,
                    reading,
                    routes: vec![route.name()],
                    disagreements: Vec::new(),
                }),
            }
        }
    }
    devices
}
