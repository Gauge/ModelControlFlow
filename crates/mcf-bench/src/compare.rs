//! A comparison, which can only be built out of paired trials (B-250, B53,
//! §3.27).
//!
//! **The failure this type exists to prevent.** Thirty runs of A, then thirty
//! runs of B, subtracted. It is the obvious way to write a benchmark and it
//! reports the afternoon's drift as a difference between configurations. F51
//! measured the size of that: sixteen competing processes moved a run's median
//! by sixty-six percent while widening its spread only from four percent to
//! nine. A level shift that large lands on every trial of whichever arm was
//! running when it arrived, in the same direction, and **no repeat count
//! removes it** — the stopping condition in [`enough`] is
//! defenceless against it, because more trials of a shifted arm are more
//! trials of a shifted arm.
//!
//! Interleaving is the defence. A drift that arrives between two adjacent runs
//! lands on one of them; a drift that lasts longer lands on both and cancels
//! in the difference. So B-250's condition is that **block-then-subtract does
//! not compile**: there is no constructor here that takes two sequences of
//! timings and calls them a comparison. The three ways in are
//!
//! 1. [`Interleaving`], which runs the arms alternately itself, randomizing
//!    which goes first in each pair so that going first is not an advantage;
//! 2. [`Comparison::from_trials`], which reads a session back out of the
//!    record and **checks the interleaving from the positions** — trials that
//!    were run in blocks are refused by name, as
//!    [`NotComparable::RanInBlocks`];
//! 3. [`Comparison::from_separate_sessions`], which is §3.27's *it may be all
//!    that exists*: constructible, and unable to produce a paired difference
//!    at all, because there is no pairing to produce one from.
//!
//! **The reported quantity is the paired difference distribution.** Not the
//! difference of two medians — [`Comparison::paired_differences`] returns one
//! difference per pair, and the verdict is taken over those. Two summaries
//! have already destroyed the information that thirty paired differences
//! carry, which is why B53 names the distribution rather than the gap.
//!
//! **A comparison from separate sessions says so.** [`Finding`] carries
//! [`Strength`] beside the verdict and renders it, and
//! [`Comparison::paired_differences`] returns `None` for one — the weakness is
//! in the type rather than in a label somebody may forget to print.
//!
//! **Durations, not numbers.** The type is generic over the *clock* rather
//! than over the quantity, so that A11 still holds: a comparison of simulated
//! intervals and a comparison of monotonic ones are different types and cannot
//! be mixed. It also means the arithmetic a difference needs is available
//! without giving it to [`Quantity`], which is `Ord` and nothing more for the
//! reason stated in `mcf_core::measurement::quantity`.
//!
//! [`enough`]: super::enough
//! [`Quantity`]: mcf_core::measurement::Quantity

use core::fmt;

use mcf_core::measurement::{Conditions, Isolation, PartsPerMillion};
use mcf_core::time::{ClockKind, Duration};
use mcf_core::trial::{Arm, Position, SessionId, Trial, Trials};

use super::enough::{self, Verdict};

/// A million, as the ratios here are expressed.
const MILLION: i128 = 1_000_000;

/// Which arm of a comparison.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Side {
    /// The arm named first when the comparison was made.
    Left,
    /// The arm named second.
    Right,
}

impl Side {
    /// The other one.
    #[must_use]
    pub const fn other(self) -> Self {
        match self {
            Self::Left => Self::Right,
            Self::Right => Self::Left,
        }
    }
}

impl fmt::Display for Side {
    fn fmt(&self, form: &mut fmt::Formatter<'_>) -> fmt::Result {
        form.write_str(match self {
            Self::Left => "left",
            Self::Right => "right",
        })
    }
}

/// One arm of a comparison: a configuration, and the name it is called by.
///
/// The two travel together because A8's question is about the configuration
/// rather than about the name — *a comparison is only meaningful when one
/// thing differs*, and what differs is a condition. An arm that carried only a
/// name would leave [`Comparison::isolation`] with nothing to read, and a
/// comparison that cannot say what it isolated is one whose delta a reader
/// will over-read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnderTest {
    arm: Arm,
    conditions: Conditions,
}

impl UnderTest {
    /// An arm, and the conditions it is measured under.
    ///
    /// Both are arguments and neither has a default. `Conditions` built on
    /// `Floor::nothing_known()` is the honest answer where MCF cannot yet read
    /// them, and it produces [`Isolation::Undetermined`] rather than a claim.
    #[must_use]
    pub const fn new(arm: Arm, conditions: Conditions) -> Self {
        Self { arm, conditions }
    }

    /// What it is called.
    #[must_use]
    pub const fn arm(&self) -> &Arm {
        &self.arm
    }

    /// What it was measured under.
    #[must_use]
    pub const fn conditions(&self) -> &Conditions {
        &self.conditions
    }
}

/// Two runs of the two arms, adjacent in one session.
///
/// Adjacent is the whole content of the word *paired*: the two saw the same
/// thermal state, the same contention and the same second, so what differs
/// between them is the arm and whatever happened in the gap between two
/// consecutive runs — which is as small as this instrument can make it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pair<K: ClockKind> {
    left: Duration<K>,
    right: Duration<K>,
    first: Side,
    at: (Position, Position),
}

impl<K: ClockKind> Pair<K> {
    /// What the left arm took.
    #[must_use]
    pub const fn left(&self) -> Duration<K> {
        self.left
    }

    /// What the right arm took.
    #[must_use]
    pub const fn right(&self) -> Duration<K> {
        self.right
    }

    /// Which arm ran first in this pair.
    #[must_use]
    pub const fn first(&self) -> Side {
        self.first
    }

    /// Where each arm sat in the session's interleaving, left then right.
    #[must_use]
    pub const fn positions(&self) -> (Position, Position) {
        self.at
    }

    /// This pair's difference.
    #[must_use]
    pub fn difference(&self) -> Difference {
        Difference::between(self.left, self.right)
    }
}

/// What one pair showed: which arm was quicker, and by how much.
///
/// The magnitude is against the **quicker** of the two, which is the ratio a
/// reader means by *thirty percent faster*. It is a [`PartsPerMillion`]
/// because the shipped crates hold no floating-point number: a ratio of
/// integers cannot be a NaN, and a pair in which the quicker arm took no
/// measurable time is [`Difference::Unmeasurable`] rather than an infinity
/// (A7).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Difference {
    /// One arm was quicker, by this much against itself.
    Quicker {
        /// Which arm.
        side: Side,
        /// The gap, against the quicker arm.
        by: PartsPerMillion,
    },
    /// The two took exactly the same time.
    Level,
    /// The quicker arm took no measurable time, so there is no ratio to state.
    Unmeasurable,
}

impl Difference {
    /// The difference between two intervals from the same clock.
    #[must_use]
    fn between<K: ClockKind>(left: Duration<K>, right: Duration<K>) -> Self {
        let (a, b) = (left.as_nanos(), right.as_nanos());
        if a == b {
            return Self::Level;
        }
        let quicker = a.min(b);
        if quicker == 0 {
            return Self::Unmeasurable;
        }
        let gap = i128::from(a.abs_diff(b))
            .saturating_mul(MILLION)
            .checked_div(i128::from(quicker))
            .unwrap_or(0);
        Self::Quicker {
            side: if a < b { Side::Left } else { Side::Right },
            by: PartsPerMillion(u64::try_from(gap).unwrap_or(u64::MAX)),
        }
    }

    /// The difference as a signed ratio, positive where the left arm was
    /// quicker.
    ///
    /// This is the number the verdict is taken over. An unmeasurable pair
    /// contributes zero rather than being dropped, because dropping it would
    /// quietly shorten the pairing and A1 forbids losing the fact that a trial
    /// happened.
    #[must_use]
    fn signed(self) -> i64 {
        match self {
            Self::Quicker { side, by } => {
                let magnitude = i64::try_from(by.0).unwrap_or(i64::MAX);
                match side {
                    Side::Left => magnitude,
                    Side::Right => magnitude.saturating_neg(),
                }
            }
            Self::Level | Self::Unmeasurable => 0,
        }
    }
}

impl fmt::Display for Difference {
    fn fmt(&self, form: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Quicker { side, by } => write!(form, "{side} quicker by {}", percent(*by)),
            Self::Level => form.write_str("level"),
            Self::Unmeasurable => form.write_str("unmeasurable — the quicker arm took no time"),
        }
    }
}

/// How much a comparison's construction is worth.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Strength {
    /// Arms interleaved within one session, with the order randomized.
    ///
    /// The claim §3.27 calls durable: whatever drifted under the comparison
    /// landed on both arms and cancelled.
    Paired(SessionId),
    /// Arms measured in different sessions and put side by side afterwards.
    ///
    /// It may be all that exists, and it is not the same claim. Nothing
    /// cancels: a machine that was busier on one afternoon than the other
    /// reports that as a difference between the arms, and there is no pairing
    /// to reveal it.
    Assembled {
        /// The session the left arm came from.
        left: SessionId,
        /// The session the right arm came from.
        right: SessionId,
    },
}

impl Strength {
    /// Whether the arms are paired.
    #[must_use]
    pub const fn is_paired(&self) -> bool {
        matches!(*self, Self::Paired(_))
    }
}

impl fmt::Display for Strength {
    fn fmt(&self, form: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Paired(session) => {
                write!(form, "paired and interleaved within session {session}")
            }
            Self::Assembled { left, right } => write!(
                form,
                "assembled from separate sessions ({left} and {right}), which is a weaker claim: \
                 nothing that drifted between them cancels"
            ),
        }
    }
}

/// Why two sets of trials are not a comparison.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NotComparable {
    /// One of the arms has no trials in these sessions.
    ArmAbsent {
        /// The arm that is missing.
        arm: Arm,
    },
    /// Fewer than two pairs, which cannot separate anything from anything.
    TooFew {
        /// How many pairs there would have been.
        have: usize,
    },
    /// The arms have different numbers of trials.
    ///
    /// Not truncated to the shorter: a caller who ran one arm more than the
    /// other has not run a paired trial, and evening it up quietly would
    /// produce exactly the shape this module exists to refuse.
    Unbalanced {
        /// Trials of the left arm.
        left: usize,
        /// Trials of the right arm.
        right: usize,
    },
    /// The trials were run in blocks rather than interleaved.
    ///
    /// **This is the finding B-250 is about.** Two trials of the same arm sat
    /// next to each other in the session's ordering, so the arms did not
    /// alternate and any drift between the blocks is inside the difference.
    RanInBlocks {
        /// The arm that repeated.
        arm: Arm,
        /// Where the repeat was seen.
        at: Position,
    },
    /// Two trials claim the same place in the interleaving.
    PositionRepeated {
        /// The place claimed twice.
        at: Position,
    },
    /// The trials come from more than one session, so they are not paired.
    ///
    /// Use [`Comparison::from_separate_sessions`], which says so in the
    /// result.
    SeveralSessions {
        /// The sessions seen, sorted.
        seen: Vec<SessionId>,
    },
    /// The arms are from one session after all, so the weaker construction was
    /// asked for when the stronger one is available.
    OneSession {
        /// The session both arms came from.
        session: SessionId,
    },
}

impl fmt::Display for NotComparable {
    fn fmt(&self, form: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ArmAbsent { arm } => write!(form, "no trials of arm {arm}"),
            Self::TooFew { have } => write!(
                form,
                "{have} pair(s): two is the fewest that can say anything about a spread"
            ),
            Self::Unbalanced { left, right } => write!(
                form,
                "{left} trials of one arm and {right} of the other — an unpaired trial is not \
                 truncated away, it is reported"
            ),
            Self::RanInBlocks { arm, at } => write!(
                form,
                "the arms were run in blocks rather than interleaved: arm {arm} ran twice around \
                 {at}, so anything that drifted between the blocks is inside the difference (B53)"
            ),
            Self::PositionRepeated { at } => {
                write!(form, "two trials both claim position {at}")
            }
            Self::SeveralSessions { seen } => write!(
                form,
                "{} sessions among these trials — a comparison across sessions is built with \
                 `from_separate_sessions`, which labels it",
                seen.len()
            ),
            Self::OneSession { session } => write!(
                form,
                "both arms are from session {session}, so the paired construction is available \
                 and the weaker one is not needed"
            ),
        }
    }
}

/// What two arms were found to do, and how much the construction is worth.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    verdict: Option<Verdict>,
    strength: Strength,
    isolation: Isolation,
    declared: Option<String>,
    arms: (Arm, Arm),
}

impl Finding {
    /// The statistical outcome — **`None` where there is none to give.**
    ///
    /// A8: *when more than one thing differs, the honest output is "these are
    /// not comparable", not a delta.* This is that sentence with a type behind
    /// it. The arithmetic difference between two confounded arms exists and
    /// says nothing about which of the differences produced it, so it is not
    /// computed and not returned — a caller cannot print it by forgetting to
    /// check [`Finding::isolation`], because there is nothing to print.
    ///
    /// A confound the operator *declares* is science (A8), and comes back with
    /// its verdict, its variables and its declaration together.
    #[must_use]
    pub const fn verdict(&self) -> Option<&Verdict> {
        self.verdict.as_ref()
    }

    /// How the comparison was built.
    #[must_use]
    pub const fn strength(&self) -> &Strength {
        &self.strength
    }

    /// What the comparison isolated, if anything.
    #[must_use]
    pub const fn isolation(&self) -> &Isolation {
        &self.isolation
    }

    /// The operator's declaration of a confound, where one was made.
    #[must_use]
    pub fn declared(&self) -> Option<&str> {
        self.declared.as_deref()
    }
}

impl fmt::Display for Finding {
    fn fmt(&self, form: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (left, right) = &self.arms;
        write!(form, "{left} vs {right}: ")?;
        match &self.verdict {
            Some(verdict) => write!(form, "{verdict}")?,
            None => write!(form, "{}", self.isolation)?,
        }
        write!(form, " — {}", self.strength)?;
        if let Some(because) = &self.declared {
            write!(
                form,
                " — confound declared by the operator ({because}): {} differ, and MCF reports \
                 what was asked for rather than judging the declaration",
                self.isolation.differing().join(", ")
            )?;
        } else if self.verdict.is_some() && !self.isolation.isolates_a_variable() {
            write!(form, " — {}", self.isolation)?;
        }
        Ok(())
    }
}

/// Two arms, and the trials that compare them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Comparison<K: ClockKind> {
    left: UnderTest,
    right: UnderTest,
    declared: Option<String>,
    body: Body<K>,
}

/// What a comparison holds, which depends on how it was built.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Body<K: ClockKind> {
    /// Pairs, in interleaving order.
    Paired {
        session: SessionId,
        pairs: Vec<Pair<K>>,
    },
    /// Two arms that were never paired, kept apart because they are.
    Separate {
        left_session: SessionId,
        right_session: SessionId,
        left: Vec<Duration<K>>,
        right: Vec<Duration<K>>,
    },
}

impl<K: ClockKind> Comparison<K> {
    /// The arms, left then right.
    #[must_use]
    pub const fn arms(&self) -> (&UnderTest, &UnderTest) {
        (&self.left, &self.right)
    }

    /// What separates the two arms' configurations (A8, B-085).
    ///
    /// Computed from the conditions rather than remembered, and computed from
    /// [`Floor::entries`] rather than from a list written out here, so a
    /// condition added to the floor becomes a condition this comparison
    /// isolates on.
    ///
    /// [`Floor::entries`]: mcf_core::measurement::Floor::entries
    #[must_use]
    pub fn isolation(&self) -> Isolation {
        Isolation::between(self.left.conditions(), self.right.conditions())
    }

    /// Declares a confound, which A8 makes the difference between science and
    /// an error.
    ///
    /// *A confound the operator declares is science; a confound nobody
    /// declared is an error.* MCF does not judge the declaration — it cannot,
    /// since whether two variables may honestly move together is a statement
    /// about the question being asked — so it records the reason and prints it
    /// beside every variable that differs, wherever the finding is rendered.
    /// What it will not do is let the delta out without them.
    #[must_use]
    pub fn declaring(mut self, because: impl Into<String>) -> Self {
        self.declared = Some(because.into());
        self
    }

    /// How the arms were brought together.
    #[must_use]
    pub fn strength(&self) -> Strength {
        match &self.body {
            Body::Paired { session, .. } => Strength::Paired(session.clone()),
            Body::Separate {
                left_session,
                right_session,
                ..
            } => Strength::Assembled {
                left: left_session.clone(),
                right: right_session.clone(),
            },
        }
    }

    /// The pairs, in interleaving order.
    ///
    /// Empty for a comparison assembled from separate sessions, which has
    /// none.
    #[must_use]
    pub fn pairs(&self) -> &[Pair<K>] {
        match &self.body {
            Body::Paired { pairs, .. } => pairs,
            Body::Separate { .. } => &[],
        }
    }

    /// How many trials of each arm.
    #[must_use]
    pub fn trials_per_arm(&self) -> (usize, usize) {
        match &self.body {
            Body::Paired { pairs, .. } => (pairs.len(), pairs.len()),
            Body::Separate { left, right, .. } => (left.len(), right.len()),
        }
    }

    /// **The reported quantity** (B53): one difference per pair, in
    /// interleaving order.
    ///
    /// `None` where the arms were never paired, which is the whole of why that
    /// construction is weaker — there is no paired difference to report, and a
    /// difference of two summaries would be a different and smaller claim
    /// wearing this one's name.
    #[must_use]
    pub fn paired_differences(&self) -> Option<Vec<Difference>> {
        match &self.body {
            Body::Paired { pairs, .. } => Some(pairs.iter().map(Pair::difference).collect()),
            Body::Separate { .. } => None,
        }
    }

    /// How many pairs ran the left arm first, and how many the right.
    ///
    /// B53 randomizes the order so that going first is not an advantage; this
    /// is what makes that checkable rather than asserted. A comparison whose
    /// balance is far from even is one whose randomization did not happen.
    #[must_use]
    pub fn order_balance(&self) -> (usize, usize) {
        let first = self
            .pairs()
            .iter()
            .filter(|p| p.first == Side::Left)
            .count();
        (first, self.pairs().len().saturating_sub(first))
    }

    /// Whether the arms have separated, and how much the answer is worth.
    ///
    /// `resolving` is the difference the caller cares about — the size below
    /// which they are content to call two things the same. See
    /// [`enough`] for why there is no repeat count here.
    #[must_use]
    pub fn finding(&self, resolving: PartsPerMillion) -> Finding {
        let verdict = match &self.body {
            Body::Paired { pairs, .. } => {
                let differences: Vec<i64> = pairs
                    .iter()
                    .map(|pair| pair.difference().signed())
                    .collect();
                enough::over_paired_differences(&differences, resolving)
            }
            Body::Separate { left, right, .. } => enough::over_separate_arms(
                &left.iter().map(|d| d.as_nanos()).collect::<Vec<_>>(),
                &right.iter().map(|d| d.as_nanos()).collect::<Vec<_>>(),
                resolving,
            ),
        };
        let isolation = self.isolation();
        // A8's refusal, and the only place a delta is withheld. A declared
        // confound is not withheld: it is reported with its declaration.
        let withheld = isolation.is_confounded() && self.declared.is_none();
        Finding {
            verdict: if withheld { None } else { Some(verdict) },
            strength: self.strength(),
            isolation,
            declared: self.declared.clone(),
            arms: (self.left.arm().clone(), self.right.arm().clone()),
        }
    }

    /// A comparison read back out of one session's trials.
    ///
    /// The interleaving is **checked, not assumed**: the two arms' trials are
    /// merged in position order and taken two at a time, and every such couple
    /// must hold one trial of each arm. Thirty of A followed by thirty of B
    /// fails that at the first couple and comes back as
    /// [`NotComparable::RanInBlocks`], which is B-250's condition applied to
    /// the record rather than to the runner.
    ///
    /// # Errors
    ///
    /// Every way the trials are not a paired comparison, by name.
    pub fn from_trials(
        trials: &Trials<Duration<K>>,
        left_under_test: &UnderTest,
        right_under_test: &UnderTest,
    ) -> Result<Self, NotComparable> {
        let (left, right) = (left_under_test.arm(), right_under_test.arm());
        let mut merged: Vec<&Trial<Duration<K>>> = trials
            .all()
            .iter()
            .filter(|trial| trial.arm() == left || trial.arm() == right)
            .collect();
        merged.sort_by_key(|trial| trial.position());

        let mut sessions: Vec<SessionId> = merged.iter().map(|t| t.session().clone()).collect();
        sessions.sort();
        sessions.dedup();
        let Some(session) = sessions.first().cloned() else {
            return Err(NotComparable::ArmAbsent { arm: left.clone() });
        };
        if sessions.len() > 1 {
            return Err(NotComparable::SeveralSessions { seen: sessions });
        }

        for couple in merged.windows(2) {
            if let (Some(one), Some(other)) = (couple.first(), couple.last())
                && one.position() == other.position()
            {
                return Err(NotComparable::PositionRepeated { at: one.position() });
            }
        }

        let (of_left, of_right) = (count(&merged, left), count(&merged, right));
        if of_left == 0 {
            return Err(NotComparable::ArmAbsent { arm: left.clone() });
        }
        if of_right == 0 {
            return Err(NotComparable::ArmAbsent { arm: right.clone() });
        }
        if of_left != of_right {
            return Err(NotComparable::Unbalanced {
                left: of_left,
                right: of_right,
            });
        }

        let mut pairs = Vec::with_capacity(of_left);
        for couple in merged.as_chunks::<2>().0 {
            let (Some(one), Some(other)) = (couple.first(), couple.last()) else {
                continue;
            };
            if one.arm() == other.arm() {
                return Err(NotComparable::RanInBlocks {
                    arm: one.arm().clone(),
                    at: one.position(),
                });
            }
            let first = if one.arm() == left {
                Side::Left
            } else {
                Side::Right
            };
            let (l, r) = if first == Side::Left {
                (one, other)
            } else {
                (other, one)
            };
            pairs.push(Pair {
                left: l.value(),
                right: r.value(),
                first,
                at: (l.position(), r.position()),
            });
        }
        if pairs.len() < 2 {
            return Err(NotComparable::TooFew { have: pairs.len() });
        }
        Ok(Self {
            left: left_under_test.clone(),
            right: right_under_test.clone(),
            declared: None,
            body: Body::Paired { session, pairs },
        })
    }

    /// §3.27's *it may be all that exists*: two arms measured apart.
    ///
    /// Constructible, and honest about what it is. There is no pairing, so
    /// [`Comparison::paired_differences`] answers `None` and every [`Finding`]
    /// it produces carries [`Strength::Assembled`]. Refused where both arms
    /// turn out to be from one session, because the stronger construction is
    /// then available and choosing the weaker one would be discarding evidence.
    ///
    /// # Errors
    ///
    /// Where either arm is absent, has fewer than two trials, or spans several
    /// sessions of its own — and where both arms share a session.
    pub fn from_separate_sessions(
        left_trials: &Trials<Duration<K>>,
        left: &UnderTest,
        right_trials: &Trials<Duration<K>>,
        right: &UnderTest,
    ) -> Result<Self, NotComparable> {
        let (left_session, of_left) = one_arm(left_trials, left.arm())?;
        let (right_session, of_right) = one_arm(right_trials, right.arm())?;
        if left_session == right_session {
            return Err(NotComparable::OneSession {
                session: left_session,
            });
        }
        Ok(Self {
            left: left.clone(),
            right: right.clone(),
            declared: None,
            body: Body::Separate {
                left_session,
                right_session,
                left: of_left,
                right: of_right,
            },
        })
    }
}

/// One arm's session and values, or why it is not usable.
fn one_arm<K: ClockKind>(
    trials: &Trials<Duration<K>>,
    arm: &Arm,
) -> Result<(SessionId, Vec<Duration<K>>), NotComparable> {
    let found = trials.of_arm(arm);
    let mut sessions: Vec<SessionId> = found.iter().map(|t| t.session().clone()).collect();
    sessions.sort();
    sessions.dedup();
    let Some(session) = sessions.first().cloned() else {
        return Err(NotComparable::ArmAbsent { arm: arm.clone() });
    };
    if sessions.len() > 1 {
        return Err(NotComparable::SeveralSessions { seen: sessions });
    }
    if found.len() < 2 {
        return Err(NotComparable::TooFew { have: found.len() });
    }
    Ok((session, found.into_iter().map(Trial::value).collect()))
}

/// How many of these trials belong to an arm.
fn count<K: ClockKind>(merged: &[&Trial<Duration<K>>], arm: &Arm) -> usize {
    merged.iter().filter(|trial| trial.arm() == arm).count()
}

/// Runs two arms alternately, randomizing which goes first in each pair.
///
/// This is the constructor that makes B53 structural rather than advisory:
/// there is no way to hand it thirty timings of one arm. It takes one pair at
/// a time and the caller decides when to stop — normally by asking
/// [`Interleaving::finding`] after each round, which is how the count comes
/// out of the run rather than out of a policy (F53, F54).
#[derive(Debug)]
pub struct Interleaving<K: ClockKind> {
    comparison: Comparison<K>,
    next_position: u32,
    state: u64,
}

impl<K: ClockKind> Interleaving<K> {
    /// Begins a comparison of two arms in one session.
    ///
    /// `seed` decides the order within each pair. It is an argument rather
    /// than a reading of the clock so that a comparison replays: §3.12 wants
    /// the same inputs to give the same run, and an order drawn from the time
    /// of day is one more thing that differs between two sittings.
    #[must_use]
    pub fn new(left: UnderTest, right: UnderTest, session: SessionId, seed: u64) -> Self {
        Self {
            comparison: Comparison {
                left,
                right,
                declared: None,
                body: Body::Paired {
                    session,
                    pairs: Vec::new(),
                },
            },
            next_position: 0,
            // Zero is the one seed a xorshift cannot leave, so it is moved
            // rather than accepted: a caller passing a default must not get a
            // generator that returns nothing but zero.
            state: if seed == 0 {
                0x2545_F491_4F6C_DD1D
            } else {
                seed
            },
        }
    }

    /// Runs both arms once, in an order this pair draws for itself.
    ///
    /// `run` is called with the arm to run and returns what it took. It is
    /// called exactly twice, back to back, which is what makes the pair a
    /// pair.
    pub fn round(&mut self, mut run: impl FnMut(&Arm) -> Duration<K>) {
        let first = if self.next().is_multiple_of(2) {
            Side::Left
        } else {
            Side::Right
        };
        let (left_arm, right_arm) = (
            self.comparison.left.arm().clone(),
            self.comparison.right.arm().clone(),
        );
        let first_position = Position(self.next_position);
        let second_position = Position(self.next_position.saturating_add(1));
        self.next_position = self.next_position.saturating_add(2);

        let (left, right, at) = match first {
            Side::Left => {
                let l = run(&left_arm);
                let r = run(&right_arm);
                (l, r, (first_position, second_position))
            }
            Side::Right => {
                let r = run(&right_arm);
                let l = run(&left_arm);
                (l, r, (second_position, first_position))
            }
        };
        if let Body::Paired { pairs, .. } = &mut self.comparison.body {
            pairs.push(Pair {
                left,
                right,
                first,
                at,
            });
        }
    }

    /// What the comparison says so far.
    #[must_use]
    pub fn finding(&self, resolving: PartsPerMillion) -> Finding {
        self.comparison.finding(resolving)
    }

    /// The comparison built so far.
    #[must_use]
    pub const fn comparison(&self) -> &Comparison<K> {
        &self.comparison
    }

    /// The comparison, finished.
    #[must_use]
    pub fn finish(self) -> Comparison<K> {
        self.comparison
    }

    /// The next value of the order generator.
    fn next(&mut self) -> u64 {
        self.state ^= self.state << 13;
        self.state ^= self.state >> 7;
        self.state ^= self.state << 17;
        self.state
    }
}

/// A ratio, as a reader wants it.
fn percent(held: PartsPerMillion) -> String {
    let whole = held.0.wrapping_div(10_000);
    let tenths = held.0.wrapping_div(1_000).wrapping_rem(10);
    format!("{whole}.{tenths}%")
}

#[cfg(test)]
mod tests;
