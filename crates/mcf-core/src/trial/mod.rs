mod seed;
mod series;

pub use seed::{Draw, NotASeedSet, STANDARD, SeedSet, published};
pub use series::{Series, Thinning};

use core::fmt;

use crate::measurement::{Conditions, Measurement, Quantity};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Arm(String);

impl Arm {
    #[must_use]
    pub fn new(name: impl Into<String>) -> Self {
        Self(name.into())
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Arm {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SessionId(String);

impl SessionId {
    #[must_use]
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for SessionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Position(pub u32);

impl fmt::Display for Position {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "#{}", self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Trial<Q: Quantity> {
    value: Q,
    arm: Arm,
    position: Position,
    session: SessionId,
    drew: Draw,
}

impl<Q: Quantity> Trial<Q> {
    #[must_use]
    pub const fn new(
        value: Q,
        arm: Arm,
        position: Position,
        session: SessionId,
        drew: Draw,
    ) -> Self {
        Self {
            value,
            arm,
            position,
            session,
            drew,
        }
    }

    #[must_use]
    pub const fn value(&self) -> Q {
        self.value
    }

    #[must_use]
    pub const fn arm(&self) -> &Arm {
        &self.arm
    }

    #[must_use]
    pub const fn position(&self) -> Position {
        self.position
    }

    #[must_use]
    pub const fn session(&self) -> &SessionId {
        &self.session
    }

    #[must_use]
    pub const fn drew(&self) -> &Draw {
        &self.drew
    }
}

impl<Q: Quantity> fmt::Display for Trial<Q> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} arm={} {} session={} {}",
            self.value, self.arm, self.position, self.session, self.drew
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Trials<Q: Quantity> {
    trials: Vec<Trial<Q>>,
}

impl<Q: Quantity> Trials<Q> {
    #[must_use]
    pub fn from(trials: impl IntoIterator<Item = Trial<Q>>) -> Self {
        Self {
            trials: trials.into_iter().collect(),
        }
    }

    #[must_use]
    pub fn all(&self) -> &[Trial<Q>] {
        &self.trials
    }

    #[must_use]
    pub fn arms(&self) -> Vec<Arm> {
        let mut arms: Vec<Arm> = self.trials.iter().map(|t| t.arm.clone()).collect();
        arms.sort();
        arms.dedup();
        arms
    }

    #[must_use]
    pub fn of_arm(&self, arm: &Arm) -> Vec<&Trial<Q>> {
        let mut found: Vec<&Trial<Q>> = self.trials.iter().filter(|t| &t.arm == arm).collect();
        found.sort_by_key(|trial| trial.position);
        found
    }

    #[must_use]
    pub fn measure(&self, arm: &Arm, conditions: Conditions) -> Option<Measurement<Q>> {
        Measurement::from_samples(self.of_arm(arm).into_iter().map(Trial::value), conditions)
    }

    #[must_use]
    pub fn paired_with(&self, left: &Arm, right: &Arm) -> Paired<'_, Q> {
        let a = self.of_arm(left);
        let b = self.of_arm(right);
        let pairs = a.len().min(b.len());
        Paired {
            pairs: a.into_iter().zip(b).take(pairs).collect(),
            unpaired: self
                .trials
                .iter()
                .filter(|t| &t.arm == left || &t.arm == right)
                .count()
                .saturating_sub(pairs.saturating_mul(2)),
        }
    }

    #[must_use]
    pub fn seeds_repeated_within_an_arm(&self) -> Vec<(Arm, u64)> {
        let mut found = Vec::new();
        for arm in self.arms() {
            let mut seen: Vec<u64> = Vec::new();
            for trial in self.of_arm(&arm) {
                if !trial.drew.is_seeded() {
                    continue;
                }
                let drawn = trial.drew.seed();
                if seen.contains(&drawn) {
                    found.push((arm.clone(), drawn));
                } else {
                    seen.push(drawn);
                }
            }
        }
        found.sort();
        found.dedup();
        found
    }

    #[must_use]
    pub fn is_balanced(&self) -> bool {
        let counts: Vec<usize> = self
            .arms()
            .iter()
            .map(|arm| self.of_arm(arm).len())
            .collect();
        counts.windows(2).all(|pair| pair.first() == pair.last())
    }
}

#[derive(Debug)]
pub struct Paired<'a, Q: Quantity> {
    pub pairs: Vec<(&'a Trial<Q>, &'a Trial<Q>)>,
    pub unpaired: usize,
}

impl<Q: Quantity> Paired<'_, Q> {
    #[must_use]
    pub fn len(&self) -> usize {
        self.pairs.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.pairs.is_empty()
    }

    #[must_use]
    pub fn left_was_smaller(&self) -> Vec<bool> {
        self.pairs
            .iter()
            .map(|(left, right)| left.value() < right.value())
            .collect()
    }
}

#[cfg(test)]
mod tests;
