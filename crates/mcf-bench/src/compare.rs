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

use mcf_core::attested::Attested;
use mcf_core::measurement::{ConditionValue, Conditions, Isolation, PartsPerMillion};
use mcf_core::time::{ClockKind, Duration};
use mcf_core::trial::{Arm, Draw, Position, SeedSet, SessionId, Trial, Trials};

use super::enough::{self, Verdict};
use super::warmth::{Reuse, Warmth};

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

    /// Records what this arm's trials reused (§6.13, B-081).
    ///
    /// The one condition a benchmark can only fill in afterwards, and the
    /// reason `UnderTest` is not otherwise mutable: everything else about a
    /// configuration is known before it runs.
    fn state_reuse(&mut self, held: ConditionValue) {
        let mut floor = self.conditions.floor().clone();
        floor.reuse = Attested::Known(held);
        self.conditions = Conditions::new(self.conditions.mcf(), floor);
    }
}

/// Which discipline a comparison's trials are taken under (B61, D19, B-290).
///
/// D19 splits laboratories in two and gives them opposite rules, and the
/// division is here rather than in a comment because a run that got it wrong
/// would report an artefact as a spread.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Discipline {
    /// A timing run: the seed is held still and the generation length is
    /// **pinned**.
    ///
    /// D19's own words: *a seed changes which tokens are produced and
    /// therefore possibly how many, and a timing that varies because one run
    /// stopped earlier is measuring the stop, not the speed.* A timing
    /// comparison that let its arms stop where they liked would be reporting
    /// the models' verbosity as the machine's throughput.
    Timing {
        /// The seed held still, which is a condition and not a choice about
        /// quality — D19: there is no systematically better seed.
        seed: u64,
        /// The generation length pinned, in tokens.
        tokens: u32,
    },
    /// A behaviour run: trial *i* draws seed *i* from a declared set.
    ///
    /// Both arms of a pair draw the **same** seed, because what must differ
    /// between them is the arm and not the trajectory — the same reasoning
    /// that makes the pairing worth having at all (§3.27).
    Behaviour {
        /// The set, which travels as a condition and is checked before two
        /// comparisons are put side by side (A8, D19).
        seeds: SeedSet,
    },
}

impl Discipline {
    /// What trial `round` draws under this discipline.
    ///
    /// `None` only where a declared behaviour set has run out, which is a fact
    /// to report rather than to wrap around: repeating the list would repeat a
    /// trajectory, which is the whole failure B61 names.
    #[must_use]
    pub fn draw_for(&self, round: usize) -> Option<Draw> {
        match self {
            Self::Timing { seed, tokens } => Some(Draw::LengthPinned {
                seed: *seed,
                tokens: *tokens,
            }),
            Self::Behaviour { seeds } => Some(Draw::Seeded {
                seed: seeds.seed_for(round)?,
                from: seeds.identifier(),
            }),
        }
    }

    /// How the seed set is recorded as a condition.
    ///
    /// **A timing run answers this, and the answer is *none*.** A7 governs
    /// values MCF *could not read*; a run that held its seed still knows
    /// perfectly well what it did, and recording that as `Unknown` would put a
    /// deliberate discipline in the same box as a failure to look — and would
    /// make every timing comparison's isolation undetermined for ever, which
    /// is a wrong answer rather than a cautious one.
    #[must_use]
    pub fn seed_set(&self) -> String {
        match self {
            Self::Timing { seed, tokens } => {
                format!("none: seed {seed} held still, {tokens} token(s) pinned")
            }
            Self::Behaviour { seeds } => seeds.identifier(),
        }
    }
}

impl fmt::Display for Discipline {
    fn fmt(&self, form: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Timing { seed, tokens } => write!(
                form,
                "a timing run: seed {seed} held still and {tokens} token(s) pinned, because a \
                 timing that varies because one run stopped earlier is measuring the stop (D19)"
            ),
            Self::Behaviour { seeds } => {
                write!(form, "a behaviour run: trial i draws seed i from {seeds}")
            }
        }
    }
}

/// Two runs of the two arms, adjacent in one session.
///
/// Adjacent is the whole content of the word *paired*: the two saw the same
/// thermal state, the same contention and the same second, so what differs
/// between them is the arm and whatever happened in the gap between two
/// consecutive runs — which is as small as this instrument can make it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pair<K: ClockKind> {
    left: Duration<K>,
    right: Duration<K>,
    first: Side,
    at: (Position, Position),
    drew: Draw,
    warmth: (Warmth, Warmth),
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

    /// What each run of this pair found already loaded, left then right.
    ///
    /// Per run rather than per pair, because the two are not alike: only one
    /// model is resident at a time (DEC-001), so a pair that alternates arms
    /// may have one warm run and one cold — which is precisely the hidden
    /// state §6.13 requires be visible.
    #[must_use]
    pub const fn warmth(&self) -> (Warmth, Warmth) {
        self.warmth
    }

    /// What both runs of this pair drew.
    ///
    /// One draw for the pair rather than two, because both arms must draw the
    /// same thing: what differs between them has to be the arm and not the
    /// trajectory (§3.27, D19).
    #[must_use]
    pub const fn drew(&self) -> &Draw {
        &self.drew
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
    /// The two arms drew from different seed sets.
    ///
    /// **D19's own requirement:** *comparisons require matching seed sets the
    /// way they require matching hardware — recorded, checked, and refused
    /// when they differ.* Two arms on different sets took different
    /// trajectories, so a difference between them is a difference between the
    /// draws as much as between the arms, and no repeat count separates the
    /// two (A8).
    SeedSetsDiffer {
        /// What the left arm drew from.
        left: String,
        /// What the right arm drew from.
        right: String,
    },
    /// One arm took its trials under a discipline the other did not.
    ///
    /// A timing trial holds its seed still and pins its length; a behaviour
    /// trial draws seed *i* at trial *i* (D19). An arm of each is two runs of
    /// two different experiments put side by side.
    DisciplinesDiffer,
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
            Self::SeedSetsDiffer { left, right } => write!(
                form,
                "the arms drew from different seed sets ({left} and {right}): a difference \
                 between them is a difference between the draws as much as between the arms, and \
                 no repeat count separates the two (D19, A8)"
            ),
            Self::DisciplinesDiffer => form.write_str(
                "one arm's trials pinned their generation length and the other's drew seeds, \
                 which are two different experiments put side by side (D19)",
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
    reuse: Reuse,
    cut_short: Option<String>,
    withheld: Option<Withheld>,
    declared: Option<String>,
    arms: (Arm, Arm),
}

/// Why a comparison has no delta to give.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Withheld {
    /// More than one condition differs (A8).
    Confounded,
    /// The trials were not alike in what they reused (§6.13).
    ///
    /// A run that loaded the model for some trials and not for others has
    /// measured two things and would be reporting one. The delta exists
    /// arithmetically and is not a delta between the arms: part of it is the
    /// difference between a trial that paid the load and one that did not,
    /// and which trials those were is a property of the order the run drew.
    MixedReuse,
}

impl fmt::Display for Withheld {
    fn fmt(&self, form: &mut fmt::Formatter<'_>) -> fmt::Result {
        form.write_str(match *self {
            Self::Confounded => "more than one condition differs (A8)",
            Self::MixedReuse => "the trials were not alike in what they reused (§6.13)",
        })
    }
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

    /// What the run reused (§6.13, B-081).
    #[must_use]
    pub const fn reuse(&self) -> &Reuse {
        &self.reuse
    }

    /// Why the run stopped before it was done, where it did (A4, B-087).
    #[must_use]
    pub fn cut_short(&self) -> Option<&str> {
        self.cut_short.as_deref()
    }

    /// Why there is no delta, where there is none.
    #[must_use]
    pub const fn withheld(&self) -> Option<Withheld> {
        self.withheld
    }
}

impl fmt::Display for Finding {
    fn fmt(&self, form: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (left, right) = &self.arms;
        write!(form, "{left} vs {right}: ")?;
        match (&self.verdict, self.withheld) {
            (Some(verdict), _) => write!(form, "{verdict}")?,
            (None, Some(Withheld::MixedReuse)) => write!(
                form,
                "no delta: {} — a run whose trials were not alike is not one measurement, and \
                 part of any difference between the arms would be the difference between a trial \
                 that loaded the model and one that did not",
                self.reuse
            )?,
            (None, _) => write!(form, "{}", self.isolation)?,
        }
        write!(form, " — {}", self.strength)?;
        if let Some(because) = &self.cut_short {
            // A4: what was produced is reported, and what was lost is said
            // rather than implied by a smaller number.
            write!(form, " — CUT SHORT: {because}")?;
        }
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
    discipline: Discipline,
    cut_short: Option<String>,
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

    /// Which discipline the trials were taken under (B61, D19).
    #[must_use]
    pub const fn discipline(&self) -> &Discipline {
        &self.discipline
    }

    /// Why the run stopped before it was done, where it did (A4, B-087).
    ///
    /// **Nine of ten trials completing is nine data points.** A4 is absolute
    /// and its violation is *an all-or-nothing return type on anything that
    /// can partially succeed* — so a run that was interrupted keeps every pair
    /// it completed, the verdict over them stands, and what was lost is said
    /// rather than thrown away with the evidence.
    ///
    /// The pairs are not diminished by it: each one is two runs of two arms
    /// taken back to back under the same conditions, and an interruption
    /// afterwards does not reach back and unmake them.
    #[must_use]
    pub fn cut_short(&self) -> Option<&str> {
        self.cut_short.as_deref()
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

    /// What the whole run reused, over both arms (§6.13, B-081).
    ///
    /// A run that mixed warm and cold trials is not one measurement, and this
    /// is where that stops being invisible. It goes into the arms' conditions,
    /// so `Isolation` sees it: two arms that differ in warmth *and* in the
    /// thing under test are confounded, and A8 withholds the delta.
    #[must_use]
    pub fn reuse(&self) -> Reuse {
        Reuse::over(
            self.pairs()
                .iter()
                .flat_map(|pair| [pair.warmth.0, pair.warmth.1]),
        )
    }

    /// Writes what the run reused into both arms' conditions (§6.13, B-081).
    ///
    /// Done at the end rather than at the start, because it is a fact about
    /// what happened: a floor filled in before the first trial would state what
    /// MCF intended. Both arms get the same value because reuse is a property
    /// of the *run* — one model is resident at a time, so what one arm found
    /// depends on what the other did.
    ///
    /// From here it flows into `Isolation` for nothing: two arms that differ in
    /// warmth and in the thing under test are confounded, and A8 withholds the
    /// delta.
    ///
    /// Called by [`Interleaving::finish`] rather than by the caller: the runner
    /// is what learns the warmth, so the runner is what records it, and a
    /// caller cannot forget.
    fn state_reuse(&mut self) {
        let held = ConditionValue::text(self.reuse().condition());
        for arm in [&mut self.left, &mut self.right] {
            arm.state_reuse(held.clone());
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
        let reuse = self.reuse();
        // The two refusals, and the only places a delta is withheld. A8's is a
        // confound nobody declared; §6.13's is a run whose trials were not
        // alike, which is not one measurement whatever else was equal.
        //
        // A declared confound is not withheld — it is reported with its
        // declaration — and a mixed run is not declarable: an operator can say
        // *I know these two variables moved together*, and cannot say *I know
        // some of my trials loaded the model*, because that is not a statement
        // about the question, it is a statement about the instrument.
        let withheld = if reuse.is_uniform() {
            (isolation.is_confounded() && self.declared.is_none()).then_some(Withheld::Confounded)
        } else {
            Some(Withheld::MixedReuse)
        };
        Finding {
            verdict: if withheld.is_some() {
                None
            } else {
                Some(verdict)
            },
            strength: self.strength(),
            isolation,
            reuse,
            cut_short: self.cut_short.clone(),
            withheld,
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
            // D19, checked pair by pair rather than once at the end: the two
            // runs of a pair must have drawn the same thing, since what differs
            // between them has to be the arm and not the trajectory.
            if l.drew() != r.drew() {
                return Err(disagreement(l.drew(), r.drew()));
            }
            pairs.push(Pair {
                left: l.value(),
                right: r.value(),
                first,
                at: (l.position(), r.position()),
                drew: l.drew().clone(),
                // A trial read back out of the record does not say what it
                // reused: the runner learns that from the engine as it goes,
                // and a `Trial` carries the value rather than the engine's
                // account of it. Unstated is the honest answer and is not a
                // guess in either direction (A7).
                warmth: (Warmth::Unstated, Warmth::Unstated),
            });
        }
        if pairs.len() < 2 {
            return Err(NotComparable::TooFew { have: pairs.len() });
        }
        let Some(discipline) = discipline_of(&pairs) else {
            return Err(NotComparable::DisciplinesDiffer);
        };
        Ok(Self {
            left: left_under_test.clone(),
            right: right_under_test.clone(),
            declared: None,
            discipline,
            cut_short: None,
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
            cut_short: None,
            // Two arms that were never paired have no pairs to read a
            // discipline off, and §3.27 already calls the construction weaker.
            // A timing run with no length pinned is what it is: a set of
            // durations whose stop nobody controlled.
            discipline: Discipline::Timing { seed: 0, tokens: 0 },
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

/// Why two draws in one pair disagree.
///
/// Named apart so that *the arms drew from different sets* and *the arms were
/// under different disciplines* are two answers rather than one vague one.
fn disagreement(left: &Draw, right: &Draw) -> NotComparable {
    match (left, right) {
        (Draw::Seeded { from: one, .. }, Draw::Seeded { from: other, .. }) if one != other => {
            NotComparable::SeedSetsDiffer {
                left: one.clone(),
                right: other.clone(),
            }
        }
        (Draw::Seeded { from, .. }, Draw::LengthPinned { .. }) => NotComparable::SeedSetsDiffer {
            left: from.clone(),
            right: "no set: the other arm pinned its length instead".to_owned(),
        },
        (Draw::LengthPinned { .. }, Draw::Seeded { from, .. }) => NotComparable::SeedSetsDiffer {
            left: "no set: this arm pinned its length instead".to_owned(),
            right: from.clone(),
        },
        // Same set, different seed within the pair; or two timing draws that
        // pinned different lengths. Both are one experiment run two ways.
        _ => NotComparable::DisciplinesDiffer,
    }
}

/// The discipline every pair agrees on, or `None` where they do not.
fn discipline_of<K: ClockKind>(pairs: &[Pair<K>]) -> Option<Discipline> {
    let first = pairs.first()?;
    match first.drew() {
        Draw::LengthPinned { seed, tokens } => {
            let same = pairs.iter().all(|pair| {
                matches!(pair.drew(), Draw::LengthPinned { tokens: held, .. } if held == tokens)
            });
            same.then_some(Discipline::Timing {
                seed: *seed,
                tokens: *tokens,
            })
        }
        Draw::Seeded { from, .. } => {
            // Every pair from one set, and — B61's own violation — no two
            // pairs on the same seed, which would be one trajectory counted
            // twice.
            let mut seen: Vec<u64> = Vec::new();
            for pair in pairs {
                let Draw::Seeded { seed, from: held } = pair.drew() else {
                    return None;
                };
                if held != from || seen.contains(seed) {
                    return None;
                }
                seen.push(*seed);
            }
            // The set is named rather than reconstructed: a comparison read
            // back out of the record knows which set was drawn from and not
            // what else was in it.
            Some(Discipline::Behaviour {
                seeds: SeedSet::declared(from.clone(), seen).ok()?,
            })
        }
    }
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
    pub fn new(
        left: UnderTest,
        right: UnderTest,
        session: SessionId,
        seed: u64,
        discipline: Discipline,
    ) -> Self {
        Self {
            comparison: Comparison {
                left,
                right,
                declared: None,
                discipline,
                cut_short: None,
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
    /// `run` is called with the arm to run and what this pair drew, and returns
    /// what it took **and what it found already loaded** (§6.13, B-081). It is called exactly twice, back to back, which is what
    /// makes the pair a pair — and with the *same* draw both times, because
    /// what must differ between the two runs is the arm and not the trajectory
    /// (§3.27, D19).
    ///
    /// `run` answers `None` where the run did not happen, and then **no pair
    /// is recorded**: a run that did not happen is not a trial, and a
    /// zero-duration stand-in for it would put a number nobody measured into
    /// the distribution (A4, A1). The pairs already taken are kept.
    ///
    /// Returns `false` where a declared behaviour set has run out — repeating
    /// the list would repeat a trajectory (B61) — or where a run did not
    /// happen. Either way the caller records what stopped it.
    pub fn round(
        &mut self,
        mut run: impl FnMut(&Arm, &Draw) -> Option<(Duration<K>, Warmth)>,
    ) -> bool {
        let Some(drew) = self
            .comparison
            .discipline
            .draw_for(self.comparison.pairs().len())
        else {
            // A declared behaviour set has run out. Reported rather than
            // wrapped around, because repeating the list would repeat a
            // trajectory and that is the failure B61 names.
            return false;
        };
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

        // Both runs of the pair are handed the *same* draw: what must differ
        // between them is the arm and not the trajectory (§3.27, D19).
        let (left, right, at) = match first {
            Side::Left => {
                let l = run(&left_arm, &drew);
                // The second run is not attempted where the first did not
                // happen: a pair is two runs taken back to back, and one of
                // them alone is not half a pair.
                let r = l.as_ref().and_then(|_| run(&right_arm, &drew));
                (l, r, (first_position, second_position))
            }
            Side::Right => {
                let r = run(&right_arm, &drew);
                let l = r.as_ref().and_then(|_| run(&left_arm, &drew));
                (l, r, (second_position, first_position))
            }
        };
        // **A run that did not happen is not a trial** (A4, A1). Pushing a
        // zero-duration pair here would put a number nobody measured into the
        // distribution, which is worse than losing the pair — the pairs
        // already taken are kept, and the caller records what stopped it.
        let (Some(left), Some(right)) = (left, right) else {
            self.next_position = first_position.0;
            return false;
        };
        let warmth = (left.1, right.1);
        let (left, right) = (left.0, right.0);
        if let Body::Paired { pairs, .. } = &mut self.comparison.body {
            pairs.push(Pair {
                left,
                right,
                first,
                at,
                drew,
                warmth,
            });
        }
        true
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

    /// Records why the run stopped before it was done (A4, B-087).
    ///
    /// Every pair already taken is kept: an interruption afterwards does not
    /// reach back and unmake trials that happened, and A4 forbids the
    /// all-or-nothing return that would discard them.
    pub fn stopped_short(&mut self, because: impl Into<String>) {
        self.comparison.cut_short = Some(because.into());
    }

    /// The comparison, finished.
    ///
    /// Finishing is where what the run reused becomes a condition (§6.13,
    /// B-081): the runner is what saw each trial's warmth, so the runner is
    /// what writes it down, and a caller cannot forget to.
    #[must_use]
    pub fn finish(mut self) -> Comparison<K> {
        self.comparison.state_reuse();
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
