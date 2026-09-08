use core::fmt;

use mcf_core::attested::Attested;
use mcf_core::measurement::{ConditionValue, Conditions, Isolation, PartsPerMillion};
use mcf_core::time::{ClockKind, Duration};
use mcf_core::trial::{Arm, Draw, Position, SeedSet, SessionId, Trial, Trials};

use super::enough::{self, Verdict};
use super::warmth::{Reuse, Warmth};

const MILLION: i128 = 1_000_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Side {
    Left,
    Right,
}

impl Side {
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnderTest {
    arm: Arm,
    conditions: Conditions,
}

impl UnderTest {
    #[must_use]
    pub const fn new(arm: Arm, conditions: Conditions) -> Self {
        Self { arm, conditions }
    }

    #[must_use]
    pub const fn arm(&self) -> &Arm {
        &self.arm
    }

    #[must_use]
    pub const fn conditions(&self) -> &Conditions {
        &self.conditions
    }

    fn state_reuse(&mut self, held: ConditionValue) {
        let mut floor = self.conditions.floor().clone();
        floor.reuse = Attested::Known(held);
        self.conditions = Conditions::new(self.conditions.mcf(), floor);
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Discipline {
    Timing { seed: u64, tokens: u32 },
    Behaviour { seeds: SeedSet },
}

impl Discipline {
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

    #[must_use]
    pub const fn pinned_tokens(&self) -> Option<u32> {
        match self {
            Self::Timing { tokens, .. } => Some(*tokens),
            Self::Behaviour { .. } => None,
        }
    }

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
    #[must_use]
    pub const fn left(&self) -> Duration<K> {
        self.left
    }

    #[must_use]
    pub const fn right(&self) -> Duration<K> {
        self.right
    }

    #[must_use]
    pub const fn first(&self) -> Side {
        self.first
    }

    #[must_use]
    pub const fn positions(&self) -> (Position, Position) {
        self.at
    }

    #[must_use]
    pub const fn warmth(&self) -> (Warmth, Warmth) {
        self.warmth
    }

    #[must_use]
    pub const fn drew(&self) -> &Draw {
        &self.drew
    }

    #[must_use]
    pub fn difference(&self) -> Difference {
        Difference::between(self.left, self.right)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Difference {
    Quicker { side: Side, by: PartsPerMillion },
    Level,
    Unmeasurable,
}

impl Difference {
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Strength {
    Paired(SessionId),
    Assembled { left: SessionId, right: SessionId },
}

impl Strength {
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NotComparable {
    ArmAbsent { arm: Arm },
    TooFew { have: usize },
    Unbalanced { left: usize, right: usize },
    RanInBlocks { arm: Arm, at: Position },
    PositionRepeated { at: Position },
    SeveralSessions { seen: Vec<SessionId> },
    SeedSetsDiffer { left: String, right: String },
    DisciplinesDiffer,
    OneSession { session: SessionId },
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
    headroom: Option<mcf_core::hardware::headroom::Headroom>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Withheld {
    Confounded,
    MixedReuse,
}

impl fmt::Display for Withheld {
    fn fmt(&self, form: &mut fmt::Formatter<'_>) -> fmt::Result {
        form.write_str(match *self {
            Self::Confounded => "more than one condition differs",
            Self::MixedReuse => "the trials were not alike in what they reused",
        })
    }
}

impl Finding {
    #[must_use]
    pub const fn verdict(&self) -> Option<&Verdict> {
        self.verdict.as_ref()
    }

    #[must_use]
    pub const fn strength(&self) -> &Strength {
        &self.strength
    }

    #[must_use]
    pub const fn isolation(&self) -> &Isolation {
        &self.isolation
    }

    #[must_use]
    pub fn declared(&self) -> Option<&str> {
        self.declared.as_deref()
    }

    #[must_use]
    pub const fn reuse(&self) -> &Reuse {
        &self.reuse
    }

    #[must_use]
    pub fn cut_short(&self) -> Option<&str> {
        self.cut_short.as_deref()
    }

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Comparison<K: ClockKind> {
    left: UnderTest,
    right: UnderTest,
    declared: Option<String>,
    discipline: Discipline,
    cut_short: Option<String>,
    body: Body<K>,
    headroom: Option<mcf_core::hardware::headroom::Headroom>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Body<K: ClockKind> {
    Paired {
        session: SessionId,
        pairs: Vec<Pair<K>>,
    },
    Separate {
        left_session: SessionId,
        right_session: SessionId,
        left: Vec<Duration<K>>,
        right: Vec<Duration<K>>,
    },
}

impl<K: ClockKind> Comparison<K> {
    #[must_use]
    pub const fn arms(&self) -> (&UnderTest, &UnderTest) {
        (&self.left, &self.right)
    }

    #[must_use]
    pub const fn discipline(&self) -> &Discipline {
        &self.discipline
    }

    #[must_use]
    pub fn cut_short(&self) -> Option<&str> {
        self.cut_short.as_deref()
    }

    #[must_use]
    pub fn isolation(&self) -> Isolation {
        Isolation::between(self.left.conditions(), self.right.conditions())
    }

    #[must_use]
    pub fn declaring(mut self, because: impl Into<String>) -> Self {
        self.declared = Some(because.into());
        self
    }

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

    #[must_use]
    pub fn pairs(&self) -> &[Pair<K>] {
        match &self.body {
            Body::Paired { pairs, .. } => pairs,
            Body::Separate { .. } => &[],
        }
    }

    #[must_use]
    pub fn trials_per_arm(&self) -> (usize, usize) {
        match &self.body {
            Body::Paired { pairs, .. } => (pairs.len(), pairs.len()),
            Body::Separate { left, right, .. } => (left.len(), right.len()),
        }
    }

    #[must_use]
    pub fn paired_differences(&self) -> Option<Vec<Difference>> {
        match &self.body {
            Body::Paired { pairs, .. } => Some(pairs.iter().map(Pair::difference).collect()),
            Body::Separate { .. } => None,
        }
    }

    #[must_use]
    pub fn reuse(&self) -> Reuse {
        Reuse::over(
            self.pairs()
                .iter()
                .flat_map(|pair| [pair.warmth.0, pair.warmth.1]),
        )
    }

    fn state_reuse(&mut self) {
        let held = ConditionValue::text(self.reuse().condition());
        for arm in [&mut self.left, &mut self.right] {
            arm.state_reuse(held.clone());
        }
    }

    #[must_use]
    pub fn medians(&self) -> Option<(Duration<K>, Duration<K>)> {
        let middle = |mut held: Vec<u64>| -> u64 {
            held.sort_unstable();
            held.get(held.len().wrapping_div(2)).copied().unwrap_or(0)
        };
        if self.pairs().is_empty() {
            return None;
        }
        Some((
            Duration::from_nanos(middle(
                self.pairs().iter().map(|p| p.left.as_nanos()).collect(),
            )),
            Duration::from_nanos(middle(
                self.pairs().iter().map(|p| p.right.as_nanos()).collect(),
            )),
        ))
    }

    #[must_use]
    pub fn order_balance(&self) -> (usize, usize) {
        let first = self
            .pairs()
            .iter()
            .filter(|p| p.first == Side::Left)
            .count();
        (first, self.pairs().len().saturating_sub(first))
    }

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
            headroom: self.headroom,
        }
    }

    #[must_use]
    pub const fn on_a_machine_with(
        mut self,
        headroom: mcf_core::hardware::headroom::Headroom,
    ) -> Self {
        self.headroom = Some(headroom);
        self
    }

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
            if l.drew() != r.drew() {
                return Err(disagreement(l.drew(), r.drew()));
            }
            pairs.push(Pair {
                left: l.value(),
                right: r.value(),
                first,
                at: (l.position(), r.position()),
                drew: l.drew().clone(),
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
            headroom: None,
        })
    }

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
            discipline: Discipline::Timing { seed: 0, tokens: 0 },
            body: Body::Separate {
                left_session,
                right_session,
                left: of_left,
                right: of_right,
            },
            headroom: None,
        })
    }
}

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
        _ => NotComparable::DisciplinesDiffer,
    }
}

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
            Some(Discipline::Behaviour {
                seeds: SeedSet::declared(from.clone(), seen).ok()?,
            })
        }
    }
}

fn count<K: ClockKind>(merged: &[&Trial<Duration<K>>], arm: &Arm) -> usize {
    merged.iter().filter(|trial| trial.arm() == arm).count()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Method {
    pub prompt: String,
    pub resolving: PartsPerMillion,
    pub workload: mcf_core::contribution::Workload,
    pub ceiling: usize,
    pub engine: Option<String>,
    pub cold: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MachineHeld {
    pub before: u64,
    pub after: u64,
    pub steady_before: Option<u64>,
    pub steady_after: Option<u64>,
}

impl MachineHeld {
    #[must_use]
    pub fn moved(&self) -> Option<u64> {
        let smaller = self.before.min(self.after);
        if smaller == 0 {
            return None;
        }
        u64::try_from(
            u128::from(self.before.abs_diff(self.after))
                .saturating_mul(1_000_000)
                .wrapping_div(u128::from(smaller)),
        )
        .ok()
    }
}

impl fmt::Display for MachineHeld {
    #[allow(clippy::integer_division)]
    fn fmt(&self, form: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            form,
            "{}.{:02} core(s) competing before, {}.{:02} after",
            self.before / 1000,
            (self.before % 1000) / 10,
            self.after / 1000,
            (self.after % 1000) / 10
        )?;
        match self.moved() {
            Some(moved) => write!(
                form,
                " — the level moved {}.{}% across the run, which is a condition and not a \
                 verdict: what movement is too much is DEC-007's open band",
                moved / 10_000,
                (moved / 1_000) % 10
            ),
            None => form.write_str(
                " — one end had nothing measurable competing, so there is no ratio between them \
                 (A7)",
            ),
        }
    }
}

#[derive(Debug)]
pub struct Interleaving<K: ClockKind> {
    comparison: Comparison<K>,
    next_position: u32,
    state: u64,
}

impl<K: ClockKind> Interleaving<K> {
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
                headroom: None,
            },
            next_position: 0,
            state: {
                let scrambled = scramble(seed);
                if scrambled == 0 {
                    0x2545_F491_4F6C_DD1D
                } else {
                    scrambled
                }
            },
        }
    }

    pub fn round(
        &mut self,
        mut run: impl FnMut(&Arm, &Draw) -> Option<(Duration<K>, Warmth)>,
    ) -> bool {
        let Some(drew) = self
            .comparison
            .discipline
            .draw_for(self.comparison.pairs().len())
        else {
            return false;
        };
        let first = if self.draws_left_first() {
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
                let l = run(&left_arm, &drew);
                let r = l.as_ref().and_then(|_| run(&right_arm, &drew));
                (l, r, (first_position, second_position))
            }
            Side::Right => {
                let r = run(&right_arm, &drew);
                let l = r.as_ref().and_then(|_| run(&left_arm, &drew));
                (l, r, (second_position, first_position))
            }
        };
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

    #[must_use]
    pub fn finding(&self, resolving: PartsPerMillion) -> Finding {
        self.comparison.finding(resolving)
    }

    #[must_use]
    pub const fn comparison(&self) -> &Comparison<K> {
        &self.comparison
    }

    pub fn stopped_short(&mut self, because: impl Into<String>) {
        self.comparison.cut_short = Some(because.into());
    }

    #[must_use]
    pub fn finish(mut self) -> Comparison<K> {
        self.comparison.state_reuse();
        self.comparison
    }

    fn next(&mut self) -> u64 {
        self.state ^= self.state << 13;
        self.state ^= self.state >> 7;
        self.state ^= self.state << 17;
        self.state
    }

    fn draws_left_first(&mut self) -> bool {
        self.next() >> 63 == 0
    }
}

const fn scramble(seed: u64) -> u64 {
    let mut held = seed.wrapping_add(0x9E37_79B9_7F4A_7C15);
    held = (held ^ (held >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    held = (held ^ (held >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    held ^ (held >> 31)
}

fn percent(held: PartsPerMillion) -> String {
    let whole = held.0.wrapping_div(10_000);
    let tenths = held.0.wrapping_div(1_000).wrapping_rem(10);
    format!("{whole}.{tenths}%")
}

#[cfg(test)]
mod tests;

#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum NotFitToContribute {
    OutsideTheBand(mcf_core::hardware::headroom::Headroom),
    SizeNotEstablished,
    NoDelta(Withheld),
}

impl fmt::Display for NotFitToContribute {
    fn fmt(&self, form: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::OutsideTheBand(held) => write!(form, "{held}"),
            Self::SizeNotEstablished => form.write_str(
                "the order is established and the size is not, at the resolution asked about \
                 (F92)",
            ),
            Self::NoDelta(why) => write!(form, "there is no delta: {why}"),
        }
    }
}

impl Finding {
    #[must_use]
    pub fn not_fit_to_contribute(&self) -> Vec<NotFitToContribute> {
        let mut found = Vec::new();
        if let Some(held) = self.headroom
            && !held.within_band()
        {
            found.push(NotFitToContribute::OutsideTheBand(held));
        }
        if matches!(self.verdict, Some(crate::enough::Verdict::Ordered { .. })) {
            found.push(NotFitToContribute::SizeNotEstablished);
        }
        if let Some(why) = self.withheld {
            found.push(NotFitToContribute::NoDelta(why));
        }
        found
    }
}
