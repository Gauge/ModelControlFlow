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
    pub fn columns_of(dial: Dial) -> Vec<&'static str> {
        if dial.times_reading_the_prompt() {
            return vec!["Value", "Take", "Prompt", "Read tok/s", "Seconds", "Ending"];
        }
        if dial.only_changes_speed() {
            return vec!["Value", "Take", "Tokens", "Tok/s", "Seconds", "Ending"];
        }
        vec![
            "Value", "Set", "Take", "Score", "Tokens", "Tok/s", "Tok/✓", "Seconds", "Ending",
        ]
    }

    #[must_use]
    pub fn cells_of(reading: &Reading, dial: Dial, named: &[String]) -> Vec<String> {
        let rate = reading
            .tokens_a_second()
            .map_or_else(|| "—".to_owned(), |rate| format!("{rate:.1}"));
        let seconds = reading
            .milliseconds
            .checked_div(1000)
            .unwrap_or(0)
            .to_string();
        if dial.only_changes_speed() {
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
