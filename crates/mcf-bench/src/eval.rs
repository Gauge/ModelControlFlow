use mcf_core::graded::{Graded, LabId, Score};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Case {
    pub call: &'static str,
    pub expects: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Task {
    pub name: &'static str,
    pub function: &'static str,
    pub asks: &'static str,
    pub cases: &'static [Case],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Ran {
    Checked {
        passed: usize,
        of: usize,
    },
    Refused {
        because: String,
        wrote_something: bool,
    },
}

pub type Run<'a> = &'a mut dyn FnMut(&Task, &str) -> Ran;

pub type Ask<'a> = &'a mut dyn FnMut(&Task) -> Option<String>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Attempt {
    pub task: &'static str,
    pub ran: Ran,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Trials {
    pub task: &'static str,
    pub attempts: Vec<Ran>,
}

impl Trials {
    #[must_use]
    pub fn ran(&self) -> usize {
        self.attempts
            .iter()
            .filter(|held| matches!(held, Ran::Checked { .. }))
            .count()
    }

    #[must_use]
    pub fn whole(&self) -> usize {
        self.attempts
            .iter()
            .filter(|held| matches!(held, Ran::Checked { passed, of } if passed == of && *of > 0))
            .count()
    }

    #[must_use]
    pub fn graded(&self) -> Graded {
        let lab = LabId::new(self.task);
        if self.attempts.is_empty() {
            return Graded::Unknown {
                why: "no attempt was made".to_owned(),
            };
        }
        if self.ran() == 0 {
            let wrote = self.attempts.iter().any(
                |held| matches!(held, Ran::Refused { wrote_something, .. } if *wrote_something),
            );
            return Graded::Unknown {
                why: if wrote {
                    "every answer this model wrote failed to run, so what was measured is \
                     whether it compiles here and not whether it is right"
                        .to_owned()
                } else {
                    "this model wrote nothing that could be run, so nothing was checked".to_owned()
                },
            };
        }
        let attempts = u64::try_from(self.attempts.len()).unwrap_or(1).max(1);
        let whole = u64::try_from(self.whole()).unwrap_or(0);
        Graded::Measured(Score::new(
            lab,
            whole.saturating_mul(1_000_000).wrapping_div(attempts),
        ))
    }
}

#[must_use]
pub fn measure(task: &Task, attempts: usize, ask: Ask<'_>, run: Run<'_>) -> Trials {
    let mut made = Vec::with_capacity(attempts);
    for _ in 0..attempts {
        let ran = match ask(task) {
            Some(written) => run(task, &written),
            None => Ran::Refused {
                because: "the model said nothing".to_owned(),
                wrote_something: false,
            },
        };
        made.push(ran);
    }
    Trials {
        task: task.name,
        attempts: made,
    }
}

#[cfg(test)]
mod tests;
