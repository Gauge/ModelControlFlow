use crate::dial::{Dial, Step};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Way {
    #[default]
    Halving,
    ByHand,
}

impl Way {
    pub const ALL: [Self; 2] = [Self::Halving, Self::ByHand];

    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Halving => "Automatic",
            Self::ByHand => "Manual",
        }
    }

    #[must_use]
    pub const fn short(self) -> &'static str {
        match self {
            Self::Halving => "automatic",
            Self::ByHand => "manual",
        }
    }
}

/// How a search is moving. It climbs first where the setting is one that climbs, and only
/// closes in once something has come back worse than what came before it: until that
/// happens there is nothing to close in on, and the best value seen so far is simply the
/// highest one tried.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    Climbing,
    Closing,
}

impl Phase {
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Climbing => "doubling",
            Self::Closing => "closing in",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hunt {
    dial: Dial,
    phase: Phase,
    gap: u32,
    round: u32,
    asked: Vec<u32>,
    settled: bool,
}

impl Hunt {
    #[must_use]
    pub fn started(dial: Dial) -> Self {
        let span = dial.span();
        Self {
            dial,
            phase: Phase::Climbing,
            gap: span.finest,
            round: 1,
            asked: vec![span.clamped(dial.climbs_from())],
            settled: false,
        }
    }

    #[must_use]
    pub const fn phase(&self) -> Phase {
        self.phase
    }

    #[must_use]
    pub const fn dial(&self) -> Dial {
        self.dial
    }

    #[must_use]
    pub const fn round(&self) -> u32 {
        self.round
    }

    #[must_use]
    pub const fn gap(&self) -> u32 {
        self.gap
    }

    #[must_use]
    pub const fn settled(&self) -> bool {
        self.settled
    }

    #[must_use]
    pub fn asked(&self) -> Vec<Step> {
        self.asked
            .iter()
            .map(|held| self.dial.step_of(*held))
            .collect()
    }

    #[must_use]
    pub fn tried(&self) -> &[u32] {
        &self.asked
    }

    /// What to run next, given everything measured so far. While it is climbing that is the
    /// last value doubled, until one comes back worse than the one below it; from then on it
    /// is the halfway point between the best and each of the two values beside it.
    pub fn stepped_on(
        &mut self,
        best: Step,
        scored: &[(Step, Option<f64>)],
        already: &[Step],
    ) -> Vec<Step> {
        if self.settled {
            return Vec::new();
        }
        if self.phase == Phase::Climbing {
            if !self.has_turned(scored) {
                return self.climbed();
            }
            self.phase = Phase::Closing;
        }
        self.closed_in_on(best, scored, already)
    }

    /// Whether the value it climbed to last came back worse than what is below it. Worse
    /// than the best of them rather than worse than whichever happened to be last: on a
    /// setting that improves and then falls away those are the same rung, and where they
    /// differ it is because one reading came back low, which is a reason to keep climbing
    /// rather than a reason to stop.
    ///
    /// And clearly worse, not worse by a hair. One take of a trial is not exact, so a value
    /// that reads within a whisker of the best has not told us anything has turned.
    ///
    /// A value that produced no reading to score is worse than one that did: there is
    /// nothing above a value that could not be measured worth climbing to.
    fn has_turned(&self, scored: &[(Step, Option<f64>)]) -> bool {
        let score_of = |wanted: u32| {
            scored
                .iter()
                .find(|(step, _)| one_value(*step) == wanted)
                .and_then(|(_, held)| *held)
        };
        let Some(highest) = self.asked.last().copied() else {
            return false;
        };
        let Some(best) = self
            .asked
            .iter()
            .filter(|held| **held != highest)
            .filter_map(|held| score_of(*held))
            .max_by(f64::total_cmp)
        else {
            return false;
        };
        let margin = best.abs() / f64::from(Self::AS_GOOD);
        score_of(highest).is_none_or(|now| best - now > margin)
    }

    /// The next rung up, or nothing left once the span runs out. A climb that reaches the
    /// top without anything getting worse is finished where it stands: bigger was better
    /// every time it was asked, so the biggest is the answer and there is no peak to close
    /// in on.
    fn climbed(&mut self) -> Vec<Step> {
        let highest = self.asked.last().copied().unwrap_or(self.dial.span().floor);
        let next = self.dial.climbs_to(highest);
        if next <= highest || self.asked.contains(&next) {
            self.settled = true;
            return Vec::new();
        }
        self.round = self.round.saturating_add(1);
        self.asked.push(next);
        vec![self.dial.step_of(next)]
    }

    /// How close a neighbour's reading has to be to the best before there is no point
    /// splitting the difference again: within one part in this many. One take of a trial is
    /// not exact, and two values whose readings are that close are two values this
    /// measurement cannot tell apart, so halving between them measures the noise rather than
    /// the setting.
    const AS_GOOD: u32 = 50;

    /// Halfway between the best and the value beside it, on each side. The two values
    /// beside the peak are the ones that bracket it: nothing outside them can be the answer
    /// once they have both come back worse, so each round cuts the bracket in half. A side
    /// falls away once its neighbour is closer than the setting can be set, or once that
    /// neighbour reads as well as the best does — and when both have, the search is done.
    fn closed_in_on(
        &mut self,
        best: Step,
        scored: &[(Step, Option<f64>)],
        already: &[Step],
    ) -> Vec<Step> {
        let span = self.dial.span();
        let peak = one_value(best);
        let mut seen: Vec<u32> = self
            .asked
            .iter()
            .copied()
            .chain(already.iter().copied().map(one_value))
            .collect();
        seen.sort_unstable();
        seen.dedup();
        let below = seen.iter().rev().find(|held| **held < peak).copied();
        let above = seen.iter().find(|held| **held > peak).copied();
        let mut next = Vec::new();
        let mut widest = 0;
        for (neighbour, away, halfway) in [
            below.map(|held| {
                let away = peak.saturating_sub(held);
                (held, away, peak.saturating_sub(away.div_euclid(2)))
            }),
            above.map(|held| {
                let away = held.saturating_sub(peak);
                (held, away, peak.saturating_add(away.div_euclid(2)))
            }),
        ]
        .into_iter()
        .flatten()
        {
            widest = widest.max(away);
            if away < span.finest || reads_as_well(scored, peak, neighbour) {
                continue;
            }
            let landed = span.rounded(halfway);
            if landed == peak || next.contains(&landed) || seen.contains(&landed) {
                continue;
            }
            next.push(landed);
        }
        if next.is_empty() {
            self.settled = true;
            self.gap = widest;
            return Vec::new();
        }
        self.gap = widest;
        self.round = self.round.saturating_add(1);
        next.sort_unstable();
        for held in &next {
            self.asked.push(*held);
        }
        self.asked.sort_unstable();
        next.iter().map(|held| self.dial.step_of(*held)).collect()
    }

    #[must_use]
    pub fn said(&self) -> String {
        if self.settled {
            if self.phase == Phase::Climbing {
                return format!(
                    "settled after {} round(s): nothing above {} came back worse, so the top \
                     of the span is the answer",
                    self.round,
                    self.dial
                        .step_of(self.asked.last().copied().unwrap_or(0))
                        .said()
                );
            }
            return format!(
                "settled after {} round(s): the values either side of the best are within {} \
                 of it, which is as fine as this setting goes",
                self.round,
                self.dial.step_of(self.dial.span().finest).said()
            );
        }
        if self.phase == Phase::Climbing {
            return format!(
                "round {}, doubling — {} value(s) tried so far",
                self.round,
                self.asked.len()
            );
        }
        format!(
            "round {}, halving a bracket {} wide — {} value(s) tried so far",
            self.round,
            self.dial.step_of(self.gap).said(),
            self.asked.len()
        )
    }
}

/// Whether a neighbour's reading is as good as the best one's, near enough. What "near
/// enough" is depends on how big the reading is, so it is a share of the best rather than a
/// number: eight hundred tokens a second and eight hundred and sixteen are the same reading,
/// where sixteen tasks passed and thirty-two are not.
///
/// A neighbour nothing could be measured at is not as good as anything. That is a gap worth
/// splitting, because somewhere in it there may be a value that runs.
fn reads_as_well(scored: &[(Step, Option<f64>)], peak: u32, neighbour: u32) -> bool {
    let score_of = |wanted: u32| {
        scored
            .iter()
            .find(|(step, _)| one_value(*step) == wanted)
            .and_then(|(_, held)| *held)
    };
    let (Some(best), Some(beside)) = (score_of(peak), score_of(neighbour)) else {
        return false;
    };
    let margin = best.abs() / f64::from(Hunt::AS_GOOD);
    (best - beside).abs() <= margin
}

const fn one_value(step: Step) -> u32 {
    match step {
        Step::Whole(held) | Step::Thousandths(held) => held,
    }
}

#[cfg(test)]
mod tests;
