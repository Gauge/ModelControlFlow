use core::fmt;

use mcf_core::failure::{Category, Failure};

use super::World;

#[derive(Clone, Copy)]
pub struct Scenario {
    pub id: &'static str,
    pub produces: Category,
    pub summary: &'static str,
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    Produced(Failure),
    Unexpected(String),
}

impl Outcome {
    #[must_use]
    pub const fn failure(&self) -> Option<&Failure> {
        match self {
            Self::Produced(failure) => Some(failure),
            Self::Unexpected(_) => None,
        }
    }

    #[must_use]
    pub fn elide(self, world: &World) -> Self {
        match self {
            Self::Produced(failure) => Self::Produced(elide_failure(&failure, world)),
            Self::Unexpected(what) => Self::Unexpected(world.elide(&what)),
        }
    }

    #[must_use]
    pub fn matches(&self, expected: Category) -> bool {
        self.failure()
            .is_some_and(|failure| failure.category() == expected)
    }
}

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

#[derive(Debug)]
pub struct Repetition {
    pub runs: usize,
    pub first: Option<Outcome>,
    pub divergence: Option<(Outcome, usize)>,
}

impl Repetition {
    #[must_use]
    pub const fn is_deterministic(&self) -> bool {
        self.divergence.is_none()
    }

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
