//! Trials, which are the record — and summaries, which are not.
//!
//! D16 and B56: *every trial is a row carrying its value, its arm, its
//! position in the interleaving and its session. A summary is computed at query
//! time and never written in place of the trials that produced it.* B56's
//! violation is a stored mean, which it calls *a question nobody can ask
//! again*.
//!
//! **There is no mean in MCF, anywhere.** That is the strongest available form
//! of B-270's *a stored mean does not compile*: `Measurement` reports order
//! statistics and has no arithmetic at all, [`Quantity`] requires only [`Ord`],
//! and nothing in this crate averages anything. A mean is not forbidden by a
//! rule here; it has no representation. The check in
//! `checks/tests/no_summary_is_persisted.rs` is what keeps it that way.
//!
//! **Why the three fields.** §3.27 makes the paired comparison the durable
//! output: arms are interleaved within one session so that drift in thermal
//! state, contention and clock affects both equally, and the reported quantity
//! is the paired difference rather than the difference of two summaries. That
//! is only reconstructible from the record if each trial says which arm it was,
//! where it sat in the interleaving, and which session it belonged to — so
//! those are fields a [`Trial`] cannot be built without.
//!
//! **What is not here yet.** The seed each trial drew from is B61's and arrives
//! with B-290; the comparison type that consumes these is B53's and arrives
//! with B-250. What this module owes them is that the record already contains
//! what they will need.
//!
//! [`Quantity`]: crate::measurement::Quantity

mod series;

pub use series::{Series, Thinning};

use core::fmt;

use crate::measurement::{Conditions, Measurement, Quantity};

/// Which side of a comparison a trial belongs to.
///
/// Named by the caller rather than enumerated: an arm is *a configuration under
/// test*, and there is no fixed number of them. Two configurations differing
/// only in quantization are two arms, and so are eight.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Arm(String);

impl Arm {
    /// Names an arm.
    #[must_use]
    pub fn new(name: impl Into<String>) -> Self {
        Self(name.into())
    }

    /// The name.
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

/// One run of one set of arms, held together.
///
/// B53 makes a comparison assembled from separate sessions a weaker claim that
/// must be labelled as one, so the session is what tells two trials apart from
/// two trials that merely look alike.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SessionId(String);

impl SessionId {
    /// Names a session.
    #[must_use]
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// The identifier.
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

/// Where a trial sat in its session's interleaving.
///
/// Zero-based, counting every trial in the session across all arms. It is what
/// makes *A, B, A, B* legible as an interleaving rather than as two blocks, and
/// what lets a reader see whether going first was an advantage (B53).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Position(pub u32);

impl fmt::Display for Position {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "#{}", self.0)
    }
}

/// One observation, with everything a later question needs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Trial<Q: Quantity> {
    value: Q,
    arm: Arm,
    position: Position,
    session: SessionId,
}

impl<Q: Quantity> Trial<Q> {
    /// Records a trial.
    ///
    /// All four are arguments and none has a default. A trial with no arm is a
    /// number; a trial with no position cannot be checked for order effects; a
    /// trial with no session can be compared with one from a different
    /// afternoon without anybody noticing (B53).
    #[must_use]
    pub const fn new(value: Q, arm: Arm, position: Position, session: SessionId) -> Self {
        Self {
            value,
            arm,
            position,
            session,
        }
    }

    /// What was observed.
    #[must_use]
    pub const fn value(&self) -> Q {
        self.value
    }

    /// Which arm it belongs to.
    #[must_use]
    pub const fn arm(&self) -> &Arm {
        &self.arm
    }

    /// Where it sat in the interleaving.
    #[must_use]
    pub const fn position(&self) -> Position {
        self.position
    }

    /// Which session it belongs to.
    #[must_use]
    pub const fn session(&self) -> &SessionId {
        &self.session
    }
}

impl<Q: Quantity> fmt::Display for Trial<Q> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} arm={} {} session={}",
            self.value, self.arm, self.position, self.session
        )
    }
}

/// The trials of one session, and the projections they support.
///
/// Everything below is computed when it is asked for. Nothing here is stored,
/// and nothing here can be stored: the type has no serialization and the record
/// holds trials.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Trials<Q: Quantity> {
    trials: Vec<Trial<Q>>,
}

impl<Q: Quantity> Trials<Q> {
    /// Collects trials.
    #[must_use]
    pub fn from(trials: impl IntoIterator<Item = Trial<Q>>) -> Self {
        Self {
            trials: trials.into_iter().collect(),
        }
    }

    /// Every trial, in the order given.
    #[must_use]
    pub fn all(&self) -> &[Trial<Q>] {
        &self.trials
    }

    /// The arms present, sorted.
    #[must_use]
    pub fn arms(&self) -> Vec<Arm> {
        let mut arms: Vec<Arm> = self.trials.iter().map(|t| t.arm.clone()).collect();
        arms.sort();
        arms.dedup();
        arms
    }

    /// The trials of one arm, in interleaving order.
    #[must_use]
    pub fn of_arm(&self, arm: &Arm) -> Vec<&Trial<Q>> {
        let mut found: Vec<&Trial<Q>> = self.trials.iter().filter(|t| &t.arm == arm).collect();
        found.sort_by_key(|trial| trial.position);
        found
    }

    /// A measurement of one arm, projected from its trials.
    ///
    /// `None` where the arm has fewer than two trials, because §3.4 makes a
    /// single-shot reading an anecdote and [`Measurement`] cannot hold one.
    #[must_use]
    pub fn measure(&self, arm: &Arm, conditions: Conditions) -> Option<Measurement<Q>> {
        Measurement::from_samples(self.of_arm(arm).into_iter().map(Trial::value), conditions)
    }

    /// Trials of two arms paired by their place in the interleaving.
    ///
    /// The *k*-th trial of one arm with the *k*-th of the other, which is what
    /// §3.27 means by pairing: the two saw the same thermal state, the same
    /// contention and the same afternoon, so what differs between them is the
    /// arm. Unpaired trials are dropped from the pairing and their count is
    /// returned, because A1 forbids losing the fact that they existed.
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

    /// Whether every arm has the same number of trials.
    ///
    /// B53 wants arms interleaved rather than run in blocks; unequal counts are
    /// the first sign that something stopped early, and that is a condition of
    /// the comparison rather than something to even up quietly.
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

/// Two arms' trials, paired by interleaving position.
#[derive(Debug)]
pub struct Paired<'a, Q: Quantity> {
    /// The pairs, in interleaving order.
    pub pairs: Vec<(&'a Trial<Q>, &'a Trial<Q>)>,
    /// How many trials of either arm had no partner.
    pub unpaired: usize,
}

impl<Q: Quantity> Paired<'_, Q> {
    /// How many pairs there are.
    #[must_use]
    pub fn len(&self) -> usize {
        self.pairs.len()
    }

    /// Whether there are none.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.pairs.is_empty()
    }

    /// For each pair, whether the left arm's value was the smaller.
    ///
    /// A comparison of *values*, not a subtraction of them: [`Quantity`] is
    /// ordered and not arithmetic, so what a pairing yields here is the sign of
    /// each difference rather than its size. The effect size B55 asks a
    /// recommendation to carry needs arithmetic on a quantity that has it, and
    /// that arrives with the comparison type (B-250).
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
