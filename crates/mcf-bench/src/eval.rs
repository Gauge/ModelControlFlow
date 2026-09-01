//! A laboratory that asks a model to do the work, and checks what it did
//! (B-110, §IX, B40, B41, D2, A18, A19).
//!
//! **What separates this from a benchmark.** [`crate::compare`] times two
//! models doing the same thing; this asks whether either of them can do it at
//! all. A timing needs no opinion about the answer — a laboratory does, and the
//! whole difficulty is holding that opinion mechanically.
//!
//! **So the answer is executed, not read.** A task states a function to write
//! and the cases it must satisfy; the model writes it; MCF runs it and counts
//! the cases that held. Nobody grades prose, nobody scores style, and there is
//! no rubric — which is what lets this produce a [`Score`] at all without the
//! rater §XIII says a judgement needs. What it measures is narrow and it is
//! stated: *this function, these cases, on this machine*.
//!
//! **Four outcomes and no total** (B40, B41). A model that could not be asked,
//! or whose run broke, has not scored badly — it has not been measured, and
//! [`mcf_core::graded::Graded`] is the type that refuses to let those become a
//! number. A laboratory that returned zero for *the container would not start*
//! would put MCF's own failure into a column about a model.
//!
//! **Cross-checked by test:** `a_laboratory_reading_is_the_share_of_attempts_that_were_whole`
//! in the instrument tier, which pins the reading against arithmetic a reader
//! can do by hand — three attempts, one of them entirely right, is one in
//! three. There is no second instrument to ask here: everything else in that
//! tier cross-checks against the kernel or a vendor tool because it measures
//! the machine, and a laboratory measures a model. So what is checked is that
//! the reading is the one the definition gives, over cases whose answer is
//! stated rather than computed by the code under test (A19).
//!
//! The cases that must never reach the division are pinned beside the module:
//! a run that broke is `Unknown` and not zero, a model that wrote nothing is
//! told apart from one whose code did not compile, a function satisfying two
//! cases of three is not two thirds correct, and two tasks' scores have no
//! ordering between them (B40, B41).
//!
//! **The code MCF runs is never trusted.** Executing what a model wrote is a
//! different act from running an engine MCF built, and the boundary is the
//! caller's: this module produces the source and the cases and never starts
//! anything. What runs it is [`Run`], which the command line supplies with a
//! container around it (B-025, §6.4).

use mcf_core::graded::{Graded, LabId, Score};

/// One case a written function must satisfy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Case {
    /// The call, as the checker will make it.
    pub call: &'static str,
    /// What it must produce, compared as text.
    ///
    /// Text rather than a value because the checker is a program in the task's
    /// own language and its notion of equality is that language's, not MCF's.
    pub expects: &'static str,
}

/// One task: what to write, and what it must do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Task {
    /// What this task is called, which is also its laboratory's name.
    pub name: &'static str,
    /// The function the model is asked for.
    pub function: &'static str,
    /// What the model is asked, in words.
    pub asks: &'static str,
    /// The cases the written function must satisfy, hidden from the model.
    ///
    /// Hidden because a task whose cases are in the prompt measures whether a
    /// model can copy them (§IX).
    pub cases: &'static [Case],
}

/// What running one written answer produced.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Ran {
    /// It ran, and this many of the cases held.
    Checked {
        /// How many cases held.
        passed: usize,
        /// How many there were.
        of: usize,
    },
    /// It could not be run, and why — a compile error, a timeout, a container
    /// that would not start.
    ///
    /// **Kept apart from zero passes.** *The code was wrong* and *MCF could not
    /// run it* are different facts, and a laboratory that scored the second as
    /// the first would be reporting its own failure as the model's (B40).
    Refused {
        /// What stopped it, in the checker's own words.
        because: String,
        /// Whether the model produced anything to run at all.
        wrote_something: bool,
    },
}

/// How a caller runs one written answer against a task's cases.
///
/// The source and the cases in; what happened out. MCF does not start anything
/// here — running code a model wrote is an act with a container around it, and
/// the container is the command line's (B-025).
pub type Run<'a> = &'a mut dyn FnMut(&Task, &str) -> Ran;

/// How a caller asks a model for one answer.
///
/// `None` where the model said nothing, which is a different outcome from
/// saying something that does not run.
pub type Ask<'a> = &'a mut dyn FnMut(&Task) -> Option<String>;

/// One attempt at one task.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Attempt {
    /// Which task.
    pub task: &'static str,
    /// What happened.
    pub ran: Ran,
}

/// Every attempt at one task, and what they came to.
///
/// **A distribution rather than a figure.** One attempt at a task is one
/// sample of a model that is not deterministic, and a laboratory reporting the
/// first one would be reporting a draw. What is kept is every attempt, so a
/// model that passes three times in five is legible as exactly that rather
/// than as sixty percent (A4, §3.23).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Trials {
    /// Which task these are attempts at.
    pub task: &'static str,
    /// What each attempt did, in the order they were made.
    pub attempts: Vec<Ran>,
}

impl Trials {
    /// How many attempts ran at all.
    #[must_use]
    pub fn ran(&self) -> usize {
        self.attempts
            .iter()
            .filter(|held| matches!(held, Ran::Checked { .. }))
            .count()
    }

    /// How many attempts satisfied every case.
    ///
    /// Every case rather than most: a function that handles two inputs of
    /// three is a function that is wrong, and a laboratory that gave it two
    /// thirds of a mark would be inventing partial credit nobody defined.
    #[must_use]
    pub fn whole(&self) -> usize {
        self.attempts
            .iter()
            .filter(|held| matches!(held, Ran::Checked { passed, of } if passed == of && *of > 0))
            .count()
    }

    /// What this task measured, or why it measured nothing.
    ///
    /// **The three cases that are not a score.** No attempt ran at all — MCF
    /// could not put the question, which is `Unknown` and not zero. The model
    /// wrote nothing — it declined, which is a reading and scores zero. Some
    /// ran: the score is how many were whole, in parts per million of the
    /// attempts made.
    #[must_use]
    pub fn graded(&self) -> Graded {
        let lab = LabId::new(self.task);
        if self.attempts.is_empty() {
            return Graded::Unknown {
                why: "no attempt was made".to_owned(),
            };
        }
        if self.ran() == 0 {
            // Nothing ran. Whether the model can do this is not a question that
            // was put, and a zero here would be MCF's failure in the model's
            // column (B40, A7).
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

/// Runs one task against one model, `attempts` times.
///
/// Every attempt is kept, including the ones that did not run: what a
/// laboratory reports is the distribution it saw and not the best of it.
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
