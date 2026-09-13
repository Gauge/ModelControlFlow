use std::sync::mpsc::{Receiver, TryRecvError, channel};
use std::sync::{Arc, atomic::AtomicBool, atomic::Ordering};
use std::time::Instant;

use crate::corpus::Set;
use crate::course::{Course, Next};
use crate::dial::Dial;
use crate::ledger::{At, Ledger, Under};
use crate::reading::{Reading, Report};
use crate::trial::{Asked, Endpoint, ask, reading_of};

#[derive(Debug)]
pub enum Heard {
    Started(At),
    Took(Box<Reading>),
    Skipped(usize),
    Refused(String),
    Stopped(String),
    Ended,
}

#[derive(Debug, Clone)]
pub struct Orders {
    pub endpoint: Endpoint,
    pub under: Under,
    pub dial: Dial,
    pub ceiling: u32,
    pub thinking: Option<bool>,
    pub effort: Option<String>,
}

#[derive(Debug)]
pub struct Running {
    heard: Receiver<Heard>,
    stop: Arc<AtomicBool>,
    pub report: Report,
    pub doing: Option<At>,
    pub taken: usize,
    pub skipped: usize,
    pub refused: Option<String>,
    pub stopped: Option<String>,
    pub finished: bool,
    started: Instant,
}

impl Running {
    #[must_use]
    pub fn begun(
        orders: Orders,
        mut course: Course,
        ledger: Ledger,
        recorded: impl Fn() -> String + Send + 'static,
    ) -> Self {
        let Orders {
            endpoint,
            under,
            dial,
            ceiling,
            thinking,
            effort,
        } = orders;
        let (send, heard) = channel();
        let stop = Arc::new(AtomicBool::new(false));
        let asked_to_stop = Arc::clone(&stop);
        let _worker = std::thread::spawn(move || {
            let mut ledger = ledger;
            let mut report = Report::default();
            loop {
                if asked_to_stop.load(Ordering::Relaxed) {
                    break;
                }
                let before = course.skipped();
                let found = course.next(dial, &ledger, &mut report);
                let over = course.skipped().saturating_sub(before);
                if over > 0 && send.send(Heard::Skipped(over)).is_err() {
                    return;
                }
                let Next::Take(spot) = found else {
                    if let Some(why) = course.stopped()
                        && send.send(Heard::Stopped(why.to_owned())).is_err()
                    {
                        return;
                    }
                    break;
                };
                if send.send(Heard::Started(spot)).is_err() {
                    return;
                }
                let Some(set) = Set::numbered(spot.set) else {
                    let _sent = send.send(Heard::Refused(format!(
                        "there is no test set numbered {}",
                        spot.set
                    )));
                    continue;
                };
                let asked = Asked {
                    set,
                    dial,
                    step: spot.step,
                    repeat: spot.repeat,
                    thinking,
                    effort: effort.clone(),
                    ceiling,
                };
                let began = Instant::now();
                let said = match ask(&endpoint, &asked) {
                    Ok(said) => said,
                    Err(failure) => {
                        let _sent = send.send(Heard::Refused(failure.to_string()));
                        break;
                    }
                };
                let milliseconds = u64::try_from(began.elapsed().as_millis()).unwrap_or(u64::MAX);
                let unmarked: Vec<(String, bool)> = asked
                    .set
                    .tasks
                    .iter()
                    .map(|task| (task.name.clone(), false))
                    .collect();
                let reading = reading_of(&asked, &said, milliseconds, &unmarked);
                if let Err(failure) = ledger.record(&under, spot, &reading, &recorded()) {
                    let _sent = send.send(Heard::Refused(failure.to_string()));
                    break;
                }
                report.record(reading.clone());
                if send.send(Heard::Took(Box::new(reading))).is_err() {
                    return;
                }
            }
            let _sent = send.send(Heard::Ended);
        });
        Self {
            heard,
            stop,
            report: Report::default(),
            doing: None,
            taken: 0,
            skipped: 0,
            refused: None,
            stopped: None,
            finished: false,
            started: Instant::now(),
        }
    }

    pub fn stop(&self) {
        self.stop.store(true, Ordering::Relaxed);
    }

    #[must_use]
    pub fn stopping(&self) -> bool {
        self.stop.load(Ordering::Relaxed)
    }

    #[must_use]
    pub fn running_for(&self) -> std::time::Duration {
        self.started.elapsed()
    }

    pub fn hear(&mut self) -> bool {
        let mut moved = false;
        loop {
            match self.heard.try_recv() {
                Ok(Heard::Started(at)) => {
                    self.doing = Some(at);
                    moved = true;
                }
                Ok(Heard::Skipped(over)) => {
                    self.skipped = self.skipped.saturating_add(over);
                    moved = true;
                }
                Ok(Heard::Took(reading)) => {
                    self.taken = self.taken.saturating_add(1);
                    self.report.record(*reading);
                    self.doing = None;
                    moved = true;
                }
                Ok(Heard::Refused(why)) => {
                    self.refused = Some(why);
                    moved = true;
                }
                Ok(Heard::Stopped(why)) => {
                    self.stopped = Some(why);
                    moved = true;
                }
                Ok(Heard::Ended) => {
                    self.finished = true;
                    self.doing = None;
                    moved = true;
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    self.finished = true;
                    self.doing = None;
                    break;
                }
            }
        }
        moved
    }

    #[must_use]
    pub fn said(&self) -> String {
        if let Some(why) = &self.refused {
            return format!("stopped: {why}");
        }
        if self.finished {
            let said = format!(
                "finished — {} measured, {} already known",
                self.taken, self.skipped
            );
            return match &self.stopped {
                Some(why) => format!("{said}. {why}"),
                None => said,
            };
        }
        match self.doing {
            Some(at) => format!(
                "measuring {} on set {}, take {} — {} done, {} already known",
                at.step.said(),
                at.set,
                at.repeat,
                self.taken,
                self.skipped
            ),
            None => format!("{} done, {} already known", self.taken, self.skipped),
        }
    }
}

#[cfg(test)]
mod tests;
