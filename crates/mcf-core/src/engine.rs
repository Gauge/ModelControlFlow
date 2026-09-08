use core::fmt;
use core::marker::PhantomData;

use crate::degradation::{Degradation, Degraded};
use crate::failure::{Attribution, Category, Disposition, Failure, Subsystem};
use crate::measurement::{Conditions, Measurement, Quantity};

pub trait Engine: Copy + fmt::Debug + 'static {
    const NAME: &'static str;

    const MAY_BE_TIMED: bool;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Vendored;

impl Engine for Vendored {
    const NAME: &'static str = "vendored";
    const MAY_BE_TIMED: bool = true;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Provisioned;

impl Engine for Provisioned {
    const NAME: &'static str = "provisioned";
    const MAY_BE_TIMED: bool = true;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StandIn;

impl Engine for StandIn {
    const NAME: &'static str = "stand-in";
    const MAY_BE_TIMED: bool = false;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Run<E: Engine> {
    engine: PhantomData<E>,
    build: String,
}

impl<E: Engine> Run<E> {
    #[must_use]
    pub fn at_build(build: impl Into<String>) -> Self {
        Self {
            engine: PhantomData,
            build: build.into(),
        }
    }

    #[must_use]
    pub fn build(&self) -> &str {
        &self.build
    }

    #[must_use]
    pub const fn engine_name() -> &'static str {
        E::NAME
    }

    #[must_use]
    pub fn behaviour<T>(&self, observed: T) -> Behaviour<T> {
        Behaviour {
            observed,
            engine: E::NAME,
            build: self.build.clone(),
        }
    }
}

impl Run<Vendored> {
    #[must_use]
    pub fn timing<Q: Quantity>(&self, measured: Measurement<Q>) -> Timing<Q> {
        Timing {
            measured,
            build: self.build.clone(),
        }
    }
}

impl Run<StandIn> {
    #[must_use]
    pub fn mark<T>(&self, observed: Behaviour<T>) -> Degraded<Behaviour<T>> {
        let cause = Failure::new(
            Category::EngineUnavailable,
            Attribution::Mcf,
            Disposition::Degraded,
            Subsystem::new("mcf-core::engine"),
            "no vendored engine runs this artifact, so MCF's own stand-in did — \
             which answers behaviour questions and can never report a speed (D31, B65)",
        )
        .with_context("engine", StandIn::NAME)
        .with_context("engine_build", self.build.clone())
        .with_context("results_permitted", "behaviour-class only (B65)");

        let degradation = Degradation::from_failure(cause).unwrap_or_else(|| {
            Degradation::because(
                Category::EngineUnavailable,
                Attribution::Mcf,
                Subsystem::new("mcf-core::engine"),
                "no vendored engine runs this artifact, so MCF's own stand-in did",
            )
        });
        Degraded::new(observed, degradation)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Behaviour<T> {
    observed: T,
    engine: &'static str,
    build: String,
}

impl<T> Behaviour<T> {
    #[must_use]
    pub const fn observed(&self) -> &T {
        &self.observed
    }

    #[must_use]
    pub const fn engine(&self) -> &'static str {
        self.engine
    }

    #[must_use]
    pub fn build(&self) -> &str {
        &self.build
    }
}

impl<T: fmt::Display> fmt::Display for Behaviour<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} (via the {} engine, build {})",
            self.observed, self.engine, self.build
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Timing<Q: Quantity> {
    measured: Measurement<Q>,
    build: String,
}

impl<Q: Quantity> Timing<Q> {
    #[must_use]
    pub const fn measured(&self) -> &Measurement<Q> {
        &self.measured
    }

    #[must_use]
    pub fn build(&self) -> &str {
        &self.build
    }

    #[must_use]
    pub const fn conditions(&self) -> &Conditions {
        self.measured.conditions()
    }
}

impl<Q: Quantity> fmt::Display for Timing<Q> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} (vendored engine, build {})",
            self.measured, self.build
        )
    }
}

#[cfg(test)]
mod tests;
