use crate::dial::{Dial, Step};
use crate::hunt::{Hunt, Way};
use crate::ledger::{At, Ledger, Row, Under};
use crate::reading::{Measure, Report};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Next {
    Take(At),
    Finished,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Course {
    under: Under,
    way: Way,
    hunt: Option<Hunt>,
    steps: Vec<Step>,
    sets: Vec<usize>,
    repeats: u8,
    measure: Measure,
    stopped: Option<String>,
    at: usize,
    handed: Vec<At>,
    taken: usize,
    skipped: usize,
}

impl Course {
    #[must_use]
    pub fn laid_out(
        under: Under,
        way: Way,
        dial: Dial,
        chosen: &[Step],
        sets: &[usize],
        repeats: u8,
        measure: Measure,
    ) -> Self {
        let (hunt, steps) = match way {
            Way::Halving => {
                let hunt = Hunt::started(dial);
                let steps = hunt.asked();
                (Some(hunt), steps)
            }
            Way::ByHand => (None, chosen.to_vec()),
        };
        Self {
            under,
            way,
            hunt,
            steps,
            sets: sets.to_vec(),
            repeats: repeats.max(1),
            measure,
            stopped: None,
            at: 0,
            handed: Vec::new(),
            taken: 0,
            skipped: 0,
        }
    }

    #[must_use]
    pub const fn way(&self) -> Way {
        self.way
    }

    #[must_use]
    pub const fn measure(&self) -> Measure {
        self.measure
    }

    #[must_use]
    pub fn stopped(&self) -> Option<&str> {
        self.stopped.as_deref()
    }

    #[must_use]
    pub fn dial(&self) -> Dial {
        self.hunt.as_ref().map_or_else(
            || {
                self.steps
                    .first()
                    .map_or(Dial::default(), |_| Dial::default())
            },
            Hunt::dial,
        )
    }

    #[must_use]
    pub const fn hunt(&self) -> Option<&Hunt> {
        self.hunt.as_ref()
    }

    #[must_use]
    pub fn steps(&self) -> &[Step] {
        &self.steps
    }

    #[must_use]
    pub const fn taken(&self) -> usize {
        self.taken
    }

    #[must_use]
    pub const fn skipped(&self) -> usize {
        self.skipped
    }

    #[must_use]
    pub const fn under(&self) -> &Under {
        &self.under
    }

    #[must_use]
    pub fn laid(&self) -> usize {
        self.steps
            .len()
            .saturating_mul(self.sets.len())
            .saturating_mul(usize::from(self.repeats))
    }

    #[must_use]
    pub fn left(&self) -> usize {
        self.laid().saturating_sub(self.handed.len())
    }

    #[must_use]
    pub fn handed(&self) -> &[At] {
        &self.handed
    }

    fn spot(&self, at: usize, dial: Dial) -> Option<At> {
        let per_step = self.sets.len().saturating_mul(usize::from(self.repeats));
        if per_step == 0 {
            return None;
        }
        let step = *self.steps.get(at.checked_div(per_step)?)?;
        let within = at.checked_rem(per_step)?;
        let set = *self
            .sets
            .get(within.checked_div(usize::from(self.repeats))?)?;
        let repeat = u8::try_from(within.checked_rem(usize::from(self.repeats))?)
            .unwrap_or(0)
            .saturating_add(1);
        Some(At {
            dial,
            step,
            set,
            repeat,
        })
    }

    pub fn next(&mut self, dial: Dial, ledger: &Ledger, report: &mut Report) -> Next {
        loop {
            while let Some(spot) = self.spot(self.at, dial) {
                self.at = self.at.saturating_add(1);
                if self.handed.contains(&spot) {
                    continue;
                }
                self.handed.push(spot);
                if let Some(row) = ledger.already(&self.under, &spot) {
                    self.skipped = self.skipped.saturating_add(1);
                    report.record(row.reading.clone());
                    continue;
                }
                self.taken = self.taken.saturating_add(1);
                return Next::Take(spot);
            }
            if !self.open_another_round(dial, ledger, report) {
                return Next::Finished;
            }
        }
    }

    fn open_another_round(&mut self, dial: Dial, ledger: &Ledger, report: &Report) -> bool {
        let measure = self.measure;
        let Some(hunt) = self.hunt.as_mut() else {
            return false;
        };
        if hunt.settled() {
            self.stopped = Some(format!("the search settled: {}", hunt.said()));
            return false;
        }
        let Some(best) = report.best_by(measure).map(|summary| summary.step) else {
            self.stopped = Some(
                "no value could be called best, so there was nowhere to close in on. Every \
                 one of them ran away — looped, or filled the token budget without \
                 finishing — and a value that never produced an answer is not a value to \
                 search around."
                    .to_owned(),
            );
            return false;
        };
        let already: Vec<Step> = ledger
            .against(&self.under, dial)
            .iter()
            .map(|row| row.at.step)
            .collect();
        let opened = hunt.closed_in_on(best, &already);
        if opened.is_empty() {
            self.stopped = Some(format!("the search settled: {}", hunt.said()));
            return false;
        }
        for step in opened {
            self.steps.push(step);
        }
        self.steps.sort_by_key(|step| match *step {
            Step::Whole(held) | Step::Thousandths(held) => held,
        });
        self.at = 0;
        true
    }

    #[must_use]
    pub fn said(&self) -> String {
        let way = self.way.short();
        match self.hunt.as_ref() {
            Some(hunt) => format!(
                "{way}: {} — {} taken, {} already known",
                hunt.said(),
                self.taken,
                self.skipped
            ),
            None => format!(
                "{way}: {} value(s) over {} set(s) — {} taken, {} already known",
                self.steps.len(),
                self.sets.len(),
                self.taken,
                self.skipped
            ),
        }
    }
}

#[must_use]
pub fn already_known(ledger: &Ledger, under: &Under, dial: Dial) -> Vec<Row> {
    ledger.against(under, dial).into_iter().cloned().collect()
}

#[cfg(test)]
mod tests;
