use crate::dial::{Dial, Step};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ending {
    Answered,
    Looped,
    Filled,
    Failed,
}

impl Ending {
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Answered => "answered",
            Self::Looped => "looped",
            Self::Filled => "filled the budget",
            Self::Failed => "failed",
        }
    }

    #[must_use]
    pub const fn is_a_runaway(self) -> bool {
        matches!(self, Self::Looped | Self::Filled)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reading {
    pub dial: Dial,
    pub step: Step,
    pub set: usize,
    pub repeat: u8,
    pub passed: u32,
    pub of: u32,
    pub produced: u64,
    pub milliseconds: u64,
    pub ending: Ending,
    /// Why it ended that way, where that is not obvious from the ending alone. A reading
    /// that failed and does not say what it hit is a reading nobody can act on: it is the
    /// difference between "the engine said nothing at all" and "the model ran out of room",
    /// and between either of those and a value that is genuinely no good.
    pub why: Option<String>,
    pub per_task: Vec<(String, bool)>,
}

impl Reading {
    #[must_use]
    pub fn tokens_a_second(&self) -> Option<f64> {
        if self.milliseconds == 0 {
            return None;
        }
        let produced = u32::try_from(self.produced).ok()?;
        Some(f64::from(produced) * 1000.0 / f64::from(u32::try_from(self.milliseconds).ok()?))
    }

    #[must_use]
    pub fn tokens_an_answer(&self) -> Option<u64> {
        if self.passed == 0 {
            return None;
        }
        self.produced.checked_div(u64::from(self.passed))
    }

    #[must_use]
    pub fn seconds_an_answer(&self) -> Option<u64> {
        if self.passed == 0 {
            return None;
        }
        self.milliseconds
            .checked_div(1000)?
            .checked_div(u64::from(self.passed))
    }
}

/// One part in this many is what a timed reading repeats to. Measured with nothing else
/// running: the same micro-batch read 742.3, 747.3 and 753.2 tokens a second.
const A_RATE_REPEATS_WITHIN: u32 = 50;

/// What a value scored, and how far that could be out.
#[derive(Debug, Clone, PartialEq)]
pub struct Scored {
    pub step: Step,
    /// Nothing where the value produced no reading to score.
    pub score: Option<f64>,
    /// One standard error of it, in the same units.
    pub error: f64,
}

impl Scored {
    /// Whether this reading is worse than another by more than the pair of them can tell
    /// apart. A value that could not be measured at all is worse than one that could.
    #[must_use]
    pub fn clearly_worse_than(&self, better: &Self) -> bool {
        let (Some(now), Some(before)) = (self.score, better.score) else {
            return self.score.is_none() && better.score.is_some();
        };
        before - now > Self::apart(self.error, better.error)
    }

    /// How far apart two readings have to be before the difference is the setting rather
    /// than the measurement: twice their errors taken together, the ordinary two-sigma bar.
    #[must_use]
    pub fn apart(one: f64, two: f64) -> f64 {
        2.0 * one.mul_add(one, two * two).sqrt()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Measure {
    #[default]
    Correctness,
    Speed,
}

impl Measure {
    pub const ALL: [Self; 2] = [Self::Correctness, Self::Speed];

    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Correctness => "Correctness",
            Self::Speed => "Speed",
        }
    }

    #[must_use]
    pub const fn short(self) -> &'static str {
        match self {
            Self::Speed => "tok/s",
            Self::Correctness => "passed",
        }
    }

    #[must_use]
    pub const fn needs_the_answers_run(self) -> bool {
        matches!(self, Self::Correctness)
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Report {
    pub readings: Vec<Reading>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Summary {
    pub step: Step,
    pub trials: usize,
    pub passed: u32,
    pub of: u32,
    pub runaways: usize,
    pub produced: u64,
    pub milliseconds: u64,
}

impl Summary {
    #[must_use]
    pub fn share(&self) -> Option<f64> {
        if self.of == 0 {
            return None;
        }
        Some(f64::from(self.passed) * 100.0 / f64::from(self.of))
    }

    #[must_use]
    pub fn tokens_an_answer(&self) -> Option<u64> {
        if self.passed == 0 {
            return None;
        }
        self.produced.checked_div(u64::from(self.passed))
    }

    #[must_use]
    pub fn tokens_a_second(&self) -> Option<f64> {
        if self.milliseconds == 0 {
            return None;
        }
        let produced = u32::try_from(self.produced).ok()?;
        let millis = u32::try_from(self.milliseconds).ok()?;
        Some(f64::from(produced) * 1000.0 / f64::from(millis))
    }

    /// How far this reading could be out: one standard error of it.
    ///
    /// A share of tasks passed is a count of coin flips. Sixty-four of them are worth about
    /// six points either way and eight of them about eighteen, and that is what a reading of
    /// this kind can tell apart — nothing like the couple of per cent a rate is good to.
    ///
    /// A rate timed once has no spread of its own to go on, so what stands in for it is what
    /// one take was measured to repeat within: three readings of the same micro-batch on this
    /// machine gave 742.3, 747.3 and 753.2 tokens a second.
    #[must_use]
    pub fn error_of(&self, measure: Measure) -> f64 {
        let Some(score) = self.scored(measure) else {
            return 0.0;
        };
        match measure {
            Measure::Speed => score.abs() / f64::from(A_RATE_REPEATS_WITHIN),
            Measure::Correctness => {
                if self.of == 0 {
                    return 0.0;
                }
                let share = score / 100.0;
                (share * (1.0 - share) / f64::from(self.of)).sqrt() * 100.0
            }
        }
    }

    #[must_use]
    pub fn scored(&self, measure: Measure) -> Option<f64> {
        if self.runaways > 0 && self.trials == self.runaways {
            return None;
        }
        match measure {
            Measure::Speed => self.tokens_a_second(),
            Measure::Correctness => self.share(),
        }
    }
}

impl Report {
    pub fn record(&mut self, reading: Reading) {
        self.readings.push(reading);
    }

    #[must_use]
    pub fn by_step(&self) -> Vec<Summary> {
        let mut held: Vec<Summary> = Vec::new();
        for reading in &self.readings {
            if let Some(found) = held.iter_mut().find(|summary| summary.step == reading.step) {
                found.trials = found.trials.saturating_add(1);
                found.passed = found.passed.saturating_add(reading.passed);
                found.of = found.of.saturating_add(reading.of);
                found.produced = found.produced.saturating_add(reading.produced);
                found.milliseconds = found.milliseconds.saturating_add(reading.milliseconds);
                if reading.ending.is_a_runaway() {
                    found.runaways = found.runaways.saturating_add(1);
                }
                continue;
            }
            held.push(Summary {
                step: reading.step,
                trials: 1,
                passed: reading.passed,
                of: reading.of,
                runaways: usize::from(reading.ending.is_a_runaway()),
                produced: reading.produced,
                milliseconds: reading.milliseconds,
            });
        }
        held
    }

    /// What each value scored, or nothing for one that produced no reading to score. A
    /// search that climbs until something gets worse needs the values beside each other
    /// rather than only the best of them.
    #[must_use]
    pub fn scored_by(&self, measure: Measure) -> Vec<Scored> {
        self.by_step()
            .into_iter()
            .map(|summary| Scored {
                step: summary.step,
                score: summary.scored(measure),
                error: summary.error_of(measure),
            })
            .collect()
    }

    #[must_use]
    pub fn best(&self) -> Option<Summary> {
        self.best_by(Measure::Correctness)
    }

    #[must_use]
    pub fn best_by(&self, measure: Measure) -> Option<Summary> {
        let mut held: Option<Summary> = None;
        for summary in self.by_step() {
            let Some(score) = summary.scored(measure) else {
                continue;
            };
            let better = held.as_ref().and_then(|best| best.scored(measure));
            if better.is_none_or(|best| score > best) {
                held = Some(summary);
            }
        }
        held
    }

    /// The columns a reading of this dial is worth showing. A timed run marks no answers
    /// and runs no set, so those columns would be a row of dashes.
    #[must_use]
    pub fn columns_of(dial: Dial, measure: Measure) -> Vec<&'static str> {
        if !measure.needs_the_answers_run() {
            if dial.times_reading_the_prompt() {
                return vec!["Value", "Take", "Prompt", "Read tok/s", "Seconds", "Ending"];
            }
            return vec!["Value", "Take", "Tokens", "Tok/s", "Seconds", "Ending"];
        }
        vec![
            "Value", "Set", "Take", "Score", "Tokens", "Tok/s", "Tok/✓", "Seconds", "Ending",
        ]
    }

    #[must_use]
    pub fn cells_of(
        reading: &Reading,
        dial: Dial,
        measure: Measure,
        named: &[String],
    ) -> Vec<String> {
        let rate = reading
            .tokens_a_second()
            .map_or_else(|| "—".to_owned(), |rate| format!("{rate:.1}"));
        let seconds = reading
            .milliseconds
            .checked_div(1000)
            .unwrap_or(0)
            .to_string();
        if !measure.needs_the_answers_run() {
            return vec![
                dial.said_among(reading.step, named),
                reading.repeat.to_string(),
                reading.produced.to_string(),
                rate,
                seconds,
                reading.ending.label().to_owned(),
            ];
        }
        vec![
            dial.said_among(reading.step, named),
            reading.set.to_string(),
            reading.repeat.to_string(),
            format!("{}/{}", reading.passed, reading.of),
            reading.produced.to_string(),
            rate,
            reading
                .tokens_an_answer()
                .map_or_else(|| "—".to_owned(), |held| held.to_string()),
            seconds,
            reading.ending.label().to_owned(),
        ]
    }

    #[must_use]
    pub fn to_rows(&self) -> Vec<Vec<String>> {
        self.readings
            .iter()
            .map(|reading| {
                vec![
                    reading.dial.said(reading.step),
                    reading.set.to_string(),
                    reading.repeat.to_string(),
                    format!("{}/{}", reading.passed, reading.of),
                    reading.produced.to_string(),
                    reading
                        .tokens_a_second()
                        .map_or_else(|| "—".to_owned(), |rate| format!("{rate:.1}")),
                    reading
                        .tokens_an_answer()
                        .map_or_else(|| "—".to_owned(), |held| held.to_string()),
                    reading
                        .milliseconds
                        .checked_div(1000)
                        .unwrap_or(0)
                        .to_string(),
                    reading.ending.label().to_owned(),
                ]
            })
            .collect()
    }

    pub const COLUMNS: [&'static str; 9] = [
        "Value", "Set", "Rep", "Score", "Tokens", "Tok/s", "Tok/✓", "Sec", "Ending",
    ];
}

#[cfg(test)]
mod tests;
