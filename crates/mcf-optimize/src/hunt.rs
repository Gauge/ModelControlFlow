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
        if let Some(from) = dial.climbs_from() {
            return Self {
                dial,
                phase: Phase::Climbing,
                gap: dial.span().finest,
                round: 1,
                asked: vec![dial.span().clamped(from)],
                settled: false,
            };
        }
        let coarse = values_of(&dial.coarse());
        let gap = widest_gap(&coarse).max(dial.span().finest);
        Self {
            dial,
            phase: Phase::Closing,
            gap,
            round: 1,
            asked: coarse,
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
    /// is whatever sits half a gap either side of the best.
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
            self.gap = self
                .asked
                .last()
                .copied()
                .unwrap_or(0)
                .saturating_sub(one_value(best))
                .max(self.dial.span().finest);
        }
        self.closed_in_on(best, already)
    }

    /// Whether the value it climbed to last came back worse than the one below it. A value
    /// that produced no reading to score is worse than one that did: there is nothing above
    /// it worth climbing to if it could not be measured here.
    fn has_turned(&self, scored: &[(Step, Option<f64>)]) -> bool {
        let score_of = |wanted: u32| {
            scored
                .iter()
                .find(|(step, _)| one_value(*step) == wanted)
                .and_then(|(_, held)| *held)
        };
        let mut climbed = self.asked.iter().rev();
        let Some(highest) = climbed.next().copied() else {
            return false;
        };
        let Some(below) = climbed.next().copied() else {
            return false;
        };
        let Some(before) = score_of(below) else {
            return false;
        };
        score_of(highest).is_none_or(|now| now < before)
    }

    /// The next rung: the last value doubled, or nothing left once the span runs out.
    fn climbed(&mut self) -> Vec<Step> {
        let span = self.dial.span();
        let highest = self.asked.last().copied().unwrap_or(span.floor);
        if highest >= span.ceiling {
            self.settled = true;
            return Vec::new();
        }
        let next = span.rounded(highest.saturating_mul(2));
        if next <= highest || self.asked.contains(&next) {
            self.settled = true;
            return Vec::new();
        }
        self.round = self.round.saturating_add(1);
        self.asked.push(next);
        vec![self.dial.step_of(next)]
    }

    fn closed_in_on(&mut self, best: Step, already: &[Step]) -> Vec<Step> {
        if self.settled {
            return Vec::new();
        }
        let span = self.dial.span();
        let middle = one_value(best);
        let seen: Vec<u32> = already.iter().copied().map(one_value).collect();
        let mut next = Vec::new();
        while next.is_empty() {
            let halved = self.gap.checked_div(2).unwrap_or(0);
            if halved < span.finest {
                self.settled = true;
                return Vec::new();
            }
            self.gap = halved;
            self.round = self.round.saturating_add(1);
            for beside in [
                middle.saturating_sub(self.gap),
                middle.saturating_add(self.gap),
            ] {
                let landed = span.rounded(beside);
                if landed == middle || next.contains(&landed) {
                    continue;
                }
                if seen.contains(&landed) || self.asked.contains(&landed) {
                    continue;
                }
                next.push(landed);
            }
        }
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
                    "settled after {} round(s): nothing above {} got worse, so the top of the \
                     span is the answer",
                    self.round,
                    self.dial
                        .step_of(self.asked.last().copied().unwrap_or(0))
                        .said()
                );
            }
            return format!(
                "settled after {} round(s): no gap left to halve above {}",
                self.round,
                self.dial.span().finest
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
            "round {}, closing in {} at a time — {} value(s) tried so far",
            self.round,
            self.gap,
            self.asked.len()
        )
    }
}

const fn one_value(step: Step) -> u32 {
    match step {
        Step::Whole(held) | Step::Thousandths(held) => held,
    }
}

fn values_of(steps: &[Step]) -> Vec<u32> {
    let mut held: Vec<u32> = steps.iter().copied().map(one_value).collect();
    held.sort_unstable();
    held.dedup();
    held
}

fn widest_gap(sorted: &[u32]) -> u32 {
    sorted
        .windows(2)
        .filter_map(|pair| {
            let (one, two) = (pair.first()?, pair.get(1)?);
            Some(two.saturating_sub(*one))
        })
        .max()
        .unwrap_or(0)
}

#[cfg(test)]
mod tests;
