//! What a scenario is, and what running one produces.

use core::fmt;

use mcf_core::failure::{Category, Failure};

use super::World;

/// One deterministic reproduction of one failure.
///
/// A `fn` pointer rather than a boxed closure, and a `const` catalogue rather
/// than a registry: B32 refuses a lab API, a discovery mechanism and a
/// configuration language, and a table of function pointers is the shape that
/// cannot grow into one.
#[derive(Clone, Copy)]
pub struct Scenario {
    /// What it is called. Stable for life (C5): a failure record names the
    /// scenario that rebuilds it, and a renamed scenario breaks every record
    /// that pointed at it.
    pub id: &'static str,
    /// The category this scenario exists to produce.
    ///
    /// Declared rather than inferred from what it returned, so that a scenario
    /// which stops producing what it claims is a failure of the check rather
    /// than a silent change of subject (A21's shape).
    pub produces: Category,
    /// What it does, in one line.
    pub summary: &'static str,
    /// The scenario itself.
    pub run: fn(&World) -> Outcome,
}

impl fmt::Debug for Scenario {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Scenario")
            .field("id", &self.id)
            .field("produces", &self.produces)
            .finish_non_exhaustive()
    }
}

impl fmt::Display for Scenario {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} → {} ({})", self.id, self.produces, self.summary)
    }
}

/// What running a scenario produced.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    /// The failure the scenario exists to produce.
    Produced(Failure),
    /// Something else happened, and this is what.
    ///
    /// Not an error to the caller. A scenario that stopped reproducing its
    /// failure is a finding — about the scenario, or about MCF having changed
    /// underneath it — and a finding is reported, not thrown.
    Unexpected(String),
}

impl Outcome {
    /// The failure, if the scenario produced one.
    #[must_use]
    pub const fn failure(&self) -> Option<&Failure> {
        match self {
            Self::Produced(failure) => Some(failure),
            Self::Unexpected(_) => None,
        }
    }

    /// The same outcome with the run's scratch path replaced by a
    /// placeholder.
    ///
    /// Applied by [`run`] before an outcome is returned, so nothing downstream
    /// has to remember to. A failure carrying a path that changes every run
    /// would make §3.17's *reproduces exactly* unachievable for a reason that
    /// has nothing to do with the failure.
    ///
    /// [`run`]: crate::run
    #[must_use]
    pub fn elide(self, world: &World) -> Self {
        match self {
            Self::Produced(failure) => Self::Produced(elide_failure(&failure, world)),
            Self::Unexpected(what) => Self::Unexpected(world.elide(&what)),
        }
    }

    /// Whether this is the category the scenario claims to produce.
    #[must_use]
    pub fn matches(&self, expected: Category) -> bool {
        self.failure()
            .is_some_and(|failure| failure.category() == expected)
    }
}

/// Rebuilds a failure with every path in it elided.
///
/// Rebuilt rather than mutated: [`Failure`] has no setter, because §3.6's
/// habit — a record rather than a claim its holder can restate — applies to a
/// failure as much as to a provenance.
fn elide_failure(failure: &Failure, world: &World) -> Failure {
    let mut rebuilt = Failure::new(
        failure.category(),
        failure.attribution(),
        failure.disposition(),
        failure.subsystem(),
        world.elide(failure.detail()),
    );
    for entry in failure.context() {
        rebuilt = rebuilt.with_context(entry.key, world.elide(&entry.value));
    }
    match failure.cause() {
        Some(cause) => rebuilt.caused_by(elide_failure(cause, world)),
        None => rebuilt,
    }
}

impl fmt::Display for Outcome {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Produced(failure) => write!(f, "{failure}"),
            Self::Unexpected(what) => write!(f, "unexpected: {what}"),
        }
    }
}

/// What repeating a scenario established.
#[derive(Debug)]
pub struct Repetition {
    /// How many times it ran.
    pub runs: usize,
    /// What the first run produced.
    pub first: Option<Outcome>,
    /// The first run that disagreed, and which one it was.
    pub divergence: Option<(Outcome, usize)>,
}

impl Repetition {
    /// Whether every run agreed.
    #[must_use]
    pub const fn is_deterministic(&self) -> bool {
        self.divergence.is_none()
    }

    /// A sentence stating what happened, whichever way it went.
    #[must_use]
    pub fn statement(&self) -> String {
        match (&self.first, &self.divergence) {
            (None, _) => "0 runs".to_owned(),
            (Some(first), None) => {
                format!(
                    "{} runs, {} identical outcomes: {first}",
                    self.runs, self.runs
                )
            }
            (Some(first), Some((other, at))) => format!(
                "{} runs, DIVERGED at run {at}\n  first: {first}\n  then:  {other}",
                self.runs
            ),
        }
    }
}
