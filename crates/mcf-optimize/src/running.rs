use std::sync::mpsc::{Receiver, TryRecvError, channel};
use std::sync::{Arc, atomic::AtomicBool, atomic::Ordering};
use std::time::{Duration, Instant};

use crate::corpus::Set;
use crate::course::{Course, Next};
use crate::dial::{Dial, Step};
use crate::ledger::{At, Ledger, Under};
use crate::reading::{Reading, Report};
use crate::trial::{Asked, Endpoint, ask, reading_of};

const MARKING_PATIENCE: Duration = Duration::from_secs(600);

fn judged_by(
    mark: bool,
    room: &std::path::Path,
    spot: At,
    tasks: &[crate::corpus::Task],
    said: &crate::trial::Said,
) -> (Vec<(String, bool)>, Option<String>) {
    if !mark {
        return (
            tasks
                .iter()
                .map(|task| (task.name.clone(), false))
                .collect(),
            None,
        );
    }
    let here = room.join(format!(
        "set-{}-{}-{}",
        spot.set,
        spot.step.said().replace('.', "-"),
        spot.repeat
    ));
    let held = crate::marking::marked(&here, tasks, &said.answer, MARKING_PATIENCE);
    let _swept = std::fs::remove_dir_all(&here);
    let why = held
        .why()
        .map(|why| format!("answers were not marked: {why}"));
    (held.or_unmarked(tasks), why)
}

#[must_use]
pub fn needs_a_fresh_hold(dial: Dial, held_at: Option<Step>, wanted: Step) -> bool {
    match held_at {
        None => true,
        Some(held) => dial.reloads_the_engine() && held != wanted,
    }
}

#[derive(Debug)]
pub enum Heard {
    Started(At),
    Holding(Step),
    Took(Box<Reading>),
    Skipped(usize),
    Refused(String),
    Stopped(String),
    Ended,
}

struct Doing {
    send: std::sync::mpsc::Sender<Heard>,
    course: Course,
    ledger: Ledger,
    report: Report,
    endpoint: Endpoint,
    under: Under,
    dial: Dial,
    ceiling: u32,
    thinking: Option<bool>,
    effort: Option<String>,
    mark: bool,
    room: std::path::PathBuf,
    host: Hosting,
    recorded: std::boxed::Box<dyn Fn() -> String + Send>,
    asked_to_stop: Arc<AtomicBool>,
}

fn sweeping(mut doing: Doing) {
    let mut held_at: Option<Step> = None;
    loop {
        if doing.asked_to_stop.load(Ordering::Relaxed) {
            break;
        }
        let before = doing.course.skipped();
        let found = doing
            .course
            .next(doing.dial, &doing.ledger, &mut doing.report);
        let over = doing.course.skipped().saturating_sub(before);
        if over > 0 && doing.send.send(Heard::Skipped(over)).is_err() {
            return;
        }
        let Next::Take(spot) = found else {
            if let Some(why) = doing.course.stopped()
                && doing.send.send(Heard::Stopped(why.to_owned())).is_err()
            {
                return;
            }
            break;
        };
        if doing.send.send(Heard::Started(spot)).is_err() {
            return;
        }
        if needs_a_fresh_hold(doing.dial, held_at, spot.step) {
            if doing.send.send(Heard::Holding(spot.step)).is_err() {
                return;
            }
            match (doing.host)(spot.step) {
                Ok(port) => {
                    doing.endpoint.port = port;
                    held_at = Some(spot.step);
                }
                Err(why) => {
                    let _sent = doing.send.send(Heard::Refused(why));
                    break;
                }
            }
        }
        let Some(set) = Set::numbered(spot.set) else {
            let _sent = doing.send.send(Heard::Refused(format!(
                "there is no test set numbered {}",
                spot.set
            )));
            continue;
        };
        let asked = Asked {
            set,
            dial: doing.dial,
            step: spot.step,
            repeat: spot.repeat,
            thinking: doing.thinking,
            effort: doing.effort.clone(),
            ceiling: doing.ceiling,
        };
        let began = Instant::now();
        let said = match ask(&doing.endpoint, &asked) {
            Ok(said) => said,
            Err(failure) => {
                let _sent = doing.send.send(Heard::Refused(failure.to_string()));
                break;
            }
        };
        let milliseconds = u64::try_from(began.elapsed().as_millis()).unwrap_or(u64::MAX);
        let (judged, unmarked) = judged_by(doing.mark, &doing.room, spot, &asked.set.tasks, &said);
        if let Some(why) = unmarked
            && doing.send.send(Heard::Stopped(why)).is_err()
        {
            return;
        }
        let reading = reading_of(&asked, &said, milliseconds, &judged);
        if let Err(failure) = doing
            .ledger
            .record(&doing.under, spot, &reading, &(doing.recorded)())
        {
            let _sent = doing.send.send(Heard::Refused(failure.to_string()));
            break;
        }
        doing.report.record(reading.clone());
        if doing
            .send
            .send(Heard::Took(std::boxed::Box::new(reading)))
            .is_err()
        {
            return;
        }
    }
    let _sent = doing.send.send(Heard::Ended);
}

#[derive(Debug, Clone)]
pub struct Orders {
    pub endpoint: Endpoint,
    pub under: Under,
    pub dial: Dial,
    pub ceiling: u32,
    pub thinking: Option<bool>,
    pub effort: Option<String>,
    pub mark: bool,
    pub room: std::path::PathBuf,
}

pub type Hosting = std::boxed::Box<dyn Fn(Step) -> Result<u16, String> + Send>;

#[derive(Debug)]
pub struct Running {
    heard: Receiver<Heard>,
    stop: Arc<AtomicBool>,
    pub report: Report,
    pub doing: Option<At>,
    pub taken: usize,
    pub skipped: usize,
    pub refused: Option<String>,
    pub holding: Option<Step>,
    pub stopped: Option<String>,
    pub finished: bool,
    started: Instant,
}

impl Running {
    #[must_use]
    pub fn begun(
        orders: Orders,
        course: Course,
        ledger: Ledger,
        host: Hosting,
        recorded: impl Fn() -> String + Send + 'static,
    ) -> Self {
        let Orders {
            endpoint,
            under,
            dial,
            ceiling,
            thinking,
            effort,
            mark,
            room,
        } = orders;
        let (send, heard) = channel();
        let stop = Arc::new(AtomicBool::new(false));
        let asked_to_stop = Arc::clone(&stop);
        let _worker = std::thread::spawn(move || {
            sweeping(Doing {
                send,
                course,
                ledger,
                report: Report::default(),
                endpoint,
                under,
                dial,
                ceiling,
                thinking,
                effort,
                mark,
                room,
                host,
                recorded: std::boxed::Box::new(recorded),
                asked_to_stop,
            });
        });
        Self {
            heard,
            stop,
            report: Report::default(),
            doing: None,
            taken: 0,
            skipped: 0,
            refused: None,
            holding: None,
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
                Ok(Heard::Holding(step)) => {
                    self.holding = Some(step);
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
                    self.holding = None;
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
        if let Some(step) = self.holding {
            return format!(
                "holding the model again at {} — {} done, {} already known",
                step.said(),
                self.taken,
                self.skipped
            );
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
