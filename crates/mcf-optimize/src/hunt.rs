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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hunt {
    dial: Dial,
    gap: u32,
    round: u32,
    asked: Vec<u32>,
    settled: bool,
}

impl Hunt {
    #[must_use]
    pub fn started(dial: Dial) -> Self {
        let coarse = values_of(&dial.coarse());
        let gap = widest_gap(&coarse).max(dial.span().finest);
        Self {
            dial,
            gap,
            round: 1,
            asked: coarse,
            settled: false,
        }
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

    pub fn closed_in_on(&mut self, best: Step, already: &[Step]) -> Vec<Step> {
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
            return format!(
                "settled after {} round(s): no gap left to halve above {}",
                self.round,
                self.dial.span().finest
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
