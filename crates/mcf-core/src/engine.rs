//! Which implementation of inference produced a result, and what that permits.
//!
//! D31 and B65: MCF ships a second implementation of inference — its own,
//! deliberately slow, written to be read — so that a model no vendored engine
//! will run still runs, and so that the vendored engine has something to be
//! checked against (A19). The condition of that is a prohibition: **a stand-in
//! cannot produce a timing.**
//!
//! A throughput figure from a naive kernel measures the naive kernel. It says
//! nothing about the model and nothing about the machine, and publishing one
//! would be worse than publishing nothing (P1). So the prohibition is enforced
//! the way A11 and B37 keep a simulated duration from becoming a performance
//! number: **in the type**. [`Engine`] is a type parameter, `Vendored` and
//! `StandIn` are different types, and only one of them can produce a
//! [`Timing`].
//!
//! **This exists before the stand-in does**, deliberately, for the reason
//! B-220's restoration ledger was built before anything was permitted to change
//! the environment. A prohibition added after the thing it prohibits is a
//! prohibition somebody has already worked around.
//!
//! **The second job it does.** With no way to report a speed, there is no
//! reason to optimize the stand-in — so the slope from *stand-in* to *our own
//! engine* has no first step, and §7.4's reading is untouched.

use core::fmt;
use core::marker::PhantomData;

use crate::degradation::{Degradation, Degraded};
use crate::failure::{Attribution, Category, Disposition, Failure, Subsystem};
use crate::measurement::{Conditions, Measurement, Quantity};

/// Which implementation of inference this is.
///
/// A sealed-by-convention marker, like `ClockKind`. Two implementors, and
/// adding a third is a decision about what MCF is willing to call an engine
/// (D23's tiers) rather than a convenience.
pub trait Engine: Copy + fmt::Debug + 'static {
    /// The engine's name, as it appears in a result's conditions and in a
    /// configuration's identity (D17, intent v16).
    const NAME: &'static str;

    /// Whether a result from this engine may be read as a timing.
    ///
    /// B65. `false` for a stand-in, and the type system is what actually
    /// enforces it — this constant exists so a surface can *say* which it is
    /// looking at, not so a caller can branch on it.
    const MAY_BE_TIMED: bool;
}

/// An engine MCF vendored, built and pinned (B64, D23 tier one).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Vendored;

impl Engine for Vendored {
    const NAME: &'static str = "vendored";
    const MAY_BE_TIMED: bool = true;
}

/// An engine MCF provisioned — installed, built and pinned itself, in an
/// environment it controls — and drives as a subprocess (D39, B-032, B-367).
///
/// A real engine: it may be timed, and nothing it produces is marked degraded.
/// What makes it a condition rather than a circumstance is that every result
/// names the component, its pinned commit and the prefix it ran from, so that
/// the same request through another engine differs in a recorded field and in
/// nothing else.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Provisioned;

impl Engine for Provisioned {
    const NAME: &'static str = "provisioned";
    const MAY_BE_TIMED: bool = true;
}

/// MCF's own deliberately slow implementation (D31).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StandIn;

impl Engine for StandIn {
    const NAME: &'static str = "stand-in";
    const MAY_BE_TIMED: bool = false;
}

/// A run through an engine, and the results it is allowed to produce.
///
/// The type parameter is what carries the permission. A function that takes a
/// `Run<Vendored>` cannot be handed a `Run<StandIn>`, and there is no
/// conversion between them — a stand-in run does not *become* a vendored one by
/// being checked, exactly as an estimate does not become a measurement (A20).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Run<E: Engine> {
    engine: PhantomData<E>,
    build: String,
}

impl<E: Engine> Run<E> {
    /// A run through an engine at a stated build.
    ///
    /// The build is required, not optional: intent v16 makes it part of a
    /// configuration's identity, because an engine that changes silently
    /// colours every measurement taken after it.
    #[must_use]
    pub fn at_build(build: impl Into<String>) -> Self {
        Self {
            engine: PhantomData,
            build: build.into(),
        }
    }

    /// Which build.
    #[must_use]
    pub fn build(&self) -> &str {
        &self.build
    }

    /// Which implementation.
    #[must_use]
    pub const fn engine_name() -> &'static str {
        E::NAME
    }

    /// A behaviour-class result: did the call parse, did the loop terminate,
    /// did the format hold.
    ///
    /// Available from **either** engine, because those outcomes do not depend
    /// on how fast the arithmetic was (B31). This is the whole of what a
    /// stand-in is for.
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
    /// A timing-class result.
    ///
    /// Defined on `Run<Vendored>` alone. That is B65's enforcement: there is no
    /// such method on `Run<StandIn>`, so a timing from a stand-in is not a rule
    /// somebody might break but a program that does not compile.
    #[must_use]
    pub fn timing<Q: Quantity>(&self, measured: Measurement<Q>) -> Timing<Q> {
        Timing {
            measured,
            build: self.build.clone(),
        }
    }
}

impl Run<StandIn> {
    /// The mark every result from a stand-in carries (A5, D31).
    ///
    /// A stand-in result is a result taken under reduced capability — the
    /// capability being *the engine the artifact was meant to run on* — and A5
    /// makes an unmarked degraded result a corrupted one.
    #[must_use]
    pub fn mark<T>(&self, observed: Behaviour<T>) -> Degraded<Behaviour<T>> {
        // B21 measures a failure record by whether the laboratory can rebuild
        // the failure from it, and a mark with nothing attached tells a reader
        // that something was lost without saying what. Which implementation ran
        // and at which build is also a condition of the result (intent v16), so
        // it has to be here whether or not anybody rebuilds anything.
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
            // Unreachable: the disposition above is `degraded`, which is the
            // only thing `from_failure` requires. Written rather than
            // `expect`ed because A2 forbids the panicking construct, and the
            // fallback is the same classification with no context rather than a
            // different failure.
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

/// Something observed about a model's behaviour.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Behaviour<T> {
    observed: T,
    engine: &'static str,
    build: String,
}

impl<T> Behaviour<T> {
    /// What was observed.
    #[must_use]
    pub const fn observed(&self) -> &T {
        &self.observed
    }

    /// Which implementation produced it.
    #[must_use]
    pub const fn engine(&self) -> &'static str {
        self.engine
    }

    /// At which build.
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

/// A timing, which only a vendored engine can produce.
///
/// There is deliberately no constructor here and no `Timing::new`: the only way
/// to obtain one is [`Run<Vendored>::timing`], and `Run<StandIn>` has no such
/// method.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Timing<Q: Quantity> {
    measured: Measurement<Q>,
    build: String,
}

impl<Q: Quantity> Timing<Q> {
    /// The measurement, with its conditions and its spread.
    #[must_use]
    pub const fn measured(&self) -> &Measurement<Q> {
        &self.measured
    }

    /// The engine build it was taken through, which is part of the
    /// configuration's identity (intent v16).
    #[must_use]
    pub fn build(&self) -> &str {
        &self.build
    }

    /// The conditions it was taken under.
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
