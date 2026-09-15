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

/// Why a search stopped. Three different things, and which one it was says how much to
/// trust the answer: a search that ran out of span has not seen a peak at all, one that ran
/// out of grain has found the best value the setting can be set to, and one that stopped
/// because the values either side read the same has found a peak this measurement cannot
/// place any more exactly than that.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Settled {
    NothingGotWorse,
    AsFineAsItGoes,
    TooCloseToTell,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hunt {
    dial: Dial,
    phase: Phase,
    gap: u32,
    round: u32,
    asked: Vec<u32>,
    settled: Option<Settled>,
    /// The best value the round before this one ended on. A round that finds nothing better
    /// than the one before it is a round that found nothing.
    best_was: Option<u32>,
    /// How many of those have come one after another. One is not enough to stop on: halving
    /// a wide bracket can land both probes on the flat of a curve and the round after it
    /// land on the peak, so a flat round is confirmed the way a turn is.
    flat_rounds: u32,
    /// Whether the climb ran out of span rather than turning. It changes what the end of a
    /// search means: a peak found between two rungs is a peak, and a climb that merely hit
    /// the ceiling has not been shown one.
    topped_out: bool,
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
            settled: None,
            best_was: None,
            flat_rounds: 0,
            topped_out: false,
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
        self.settled.is_some()
    }

    #[must_use]
    pub const fn why_it_settled(&self) -> Option<Settled> {
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
        if self.settled.is_some() {
            return Vec::new();
        }
        if self.phase == Phase::Climbing {
            if !self.has_turned(scored) {
                let next = self.climbed();
                if !next.is_empty() {
                    return next;
                }
                // Out of span rather than turned, so the question is whether there is a peak
                // between the last two rungs. There is only somewhere to look if the curve
                // has flattened there: two rungs that read alike may have a better value
                // between them, where a ceiling that still clearly beats the rung below it
                // is a curve going up with nowhere left to go. A top-p sits between nine
                // tenths and all of them more often than it sits on either.
                // Out of span. Whether that is the end depends on where the best sits. Below
                // the ceiling means the reading fell away at the top and the climb would have
                // turned given one more rung — so there is a peak to close in on after all,
                // and a temperature whose best is 0.7 is otherwise answered 0.8 without ever
                // looking between its rungs.
                if one_value(best) < self.dial.span().ceiling {
                    self.phase = Phase::Closing;
                    return self.closed_in_on(best, scored, already);
                }
                // The best is the ceiling. That may be because the reading is still rising
                // and the span is too short, or because the peak sits inside the last gap —
                // which on a climb that doubles is the widest gap of all. The two look the
                // same from here, so the gap gets looked at: one probe, and the flat-round
                // rule ends it quickly where there was nothing in it.
                self.topped_out = true;
                self.phase = Phase::Closing;
                // One probe between them, taken rather than reasoned about. Closing in would
                // throw this side away on the very test that said to look here — two rungs
                // reading alike is what makes a plateau worth a look and what makes a
                // settled peak not worth another, and only one of those has been shown yet.
                if let Some(between) = self.between_the_top_two() {
                    self.round = self.round.saturating_add(1);
                    self.asked.push(between);
                    self.asked.sort_unstable();
                    return vec![self.dial.step_of(between)];
                }
            }
            self.phase = Phase::Closing;
        }
        self.closed_in_on(best, scored, already)
    }

    /// Whether the climb is over. Three things have to hold, and each was put there by a
    /// measurement rather than by taste.
    ///
    /// Worse than the best below it, not worse than whichever rung happened to be last. On a
    /// setting that improves and then falls away those are the same rung; where they differ
    /// it is because one reading came back low.
    ///
    /// And clearly worse, not worse by a hair — one take of a trial is not exact, so a value
    /// reading within a whisker of the best has said nothing about a turn.
    ///
    /// And confirmed, rather than taken on the first bad reading. A micro-batch of 512 reads
    /// nine per cent below both of its neighbours on the machine this was written on: going
    /// up, 712, 652, 718, 757, 780, 783. A climb that ended at the first value worse than
    /// the one before it answered 256 and stopped, five rungs below the best there was. So
    /// the rung above a bad one is tried before the climb is called over, and one dip costs
    /// a trial instead of the answer.
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
        let mut top = self.asked.iter().rev();
        let (Some(highest), Some(under_it)) = (top.next().copied(), top.next().copied()) else {
            return false;
        };
        let Some(best) = self
            .asked
            .iter()
            .filter(|held| **held != highest && **held != under_it)
            .filter_map(|held| score_of(*held))
            .max_by(f64::total_cmp)
        else {
            return false;
        };
        let clearly_worse =
            |value: u32| score_of(value).is_none_or(|now| best - now > within_a_part_of(best));
        clearly_worse(highest) && clearly_worse(under_it)
    }

    /// Why a search that has stopped improving is over. A search whose best is still the top
    /// of the span it was given has not been shown a peak, however it came to stop: what it
    /// needs is a longer span, and saying anything else sends somebody looking for precision
    /// that was never the problem.
    fn why_it_stops(&self, peak: u32) -> Settled {
        if self.topped_out && peak >= self.dial.span().ceiling {
            return Settled::NothingGotWorse;
        }
        Settled::TooCloseToTell
    }

    /// Halfway between the last two rungs, if that is somewhere it has not already been.
    fn between_the_top_two(&self) -> Option<u32> {
        let span = self.dial.span();
        let mut top = self.asked.iter().rev();
        let (highest, below) = (top.next().copied()?, top.next().copied()?);
        let away = highest.saturating_sub(below);
        if away < span.finest {
            return None;
        }
        let between = span.rounded(below.saturating_add(away.div_euclid(2)));
        (!self.asked.contains(&between)).then_some(between)
    }

    /// The next rung up, or nothing left once the span runs out. A climb that reaches the
    /// top without anything getting worse is finished where it stands: bigger was better
    /// every time it was asked, so the biggest is the answer and there is no peak to close
    /// in on.
    fn climbed(&mut self) -> Vec<Step> {
        let highest = self.asked.last().copied().unwrap_or(self.dial.span().floor);
        let next = self.dial.climbs_to(highest);
        if next <= highest || self.asked.contains(&next) {
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

    /// How many rounds in a row have to come back with nothing better before a search is
    /// over. Two, for the same reason a climb turns on two: halving a wide bracket can put
    /// both probes on the flat of a curve while the round after them lands on the peak.
    const FLAT_ROUNDS: u32 = 2;

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
        // Stop when a round stops paying. Judging a side by whether its neighbour reads like
        // the best does not work: two values the same distance either side of a peak read
        // exactly alike, and the peak between them can be plainly better than both. What
        // says the search has converged is a round that came back with nothing better than
        // the round before it.
        if let Some(was) = self.best_was {
            let score_of = |wanted: u32| {
                scored
                    .iter()
                    .find(|(step, _)| one_value(*step) == wanted)
                    .and_then(|(_, held)| *held)
            };
            if let (Some(now), Some(before)) = (score_of(peak), score_of(was)) {
                let margin = now.abs() / f64::from(Self::AS_GOOD);
                if now - before <= margin {
                    self.flat_rounds = self.flat_rounds.saturating_add(1);
                    if self.flat_rounds >= Self::FLAT_ROUNDS {
                        self.settled = Some(self.why_it_stops(peak));
                        return Vec::new();
                    }
                } else {
                    self.flat_rounds = 0;
                }
            }
        }
        self.best_was = Some(peak);
        let below = seen.iter().rev().find(|held| **held < peak).copied();
        let above = seen.iter().find(|held| **held > peak).copied();
        let mut next = Vec::new();
        let mut widest = 0;
        for (away, halfway) in [
            below.map(|held| {
                let away = peak.saturating_sub(held);
                (away, peak.saturating_sub(away.div_euclid(2)))
            }),
            above.map(|held| {
                let away = held.saturating_sub(peak);
                (away, peak.saturating_add(away.div_euclid(2)))
            }),
        ]
        .into_iter()
        .flatten()
        {
            widest = widest.max(away);
            if away < span.finest {
                continue;
            }
            let landed = span.rounded(halfway);
            if landed == peak || next.contains(&landed) || seen.contains(&landed) {
                continue;
            }
            next.push(landed);
        }
        if next.is_empty() {
            self.settled = Some(if self.topped_out && peak >= span.ceiling {
                Settled::NothingGotWorse
            } else {
                Settled::AsFineAsItGoes
            });
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
        match self.settled {
            Some(Settled::NothingGotWorse) => format!(
                "nothing above {} came back worse, so the top of the span is the answer — \
                 raise the span to look further",
                self.dial
                    .step_of(self.asked.last().copied().unwrap_or(0))
                    .said()
            ),
            Some(Settled::TooCloseToTell) => {
                "the values either side read as well as the best does, which is as close as \
                 one take can place it"
                    .to_owned()
            }
            Some(Settled::AsFineAsItGoes) => format!(
                "the values either side are within {} of the best, which is as fine as this \
                 setting goes",
                self.dial.step_of(self.dial.span().finest).said()
            ),
            None if self.phase == Phase::Climbing => "doubling".to_owned(),
            None => format!(
                "halving a bracket {} wide",
                self.dial.step_of(self.gap).said()
            ),
        }
    }
}

/// What counts as the same reading, for a reading this big. A share of it rather than a
/// fixed amount, because eight hundred tokens a second and eight hundred and sixteen are
/// the same reading, where sixteen tasks passed and thirty-two are not.
fn within_a_part_of(best: f64) -> f64 {
    best.abs() / f64::from(Hunt::AS_GOOD)
}

const fn one_value(step: Step) -> u32 {
    match step {
        Step::Whole(held) | Step::Thousandths(held) => held,
    }
}

#[cfg(test)]
mod tests;
