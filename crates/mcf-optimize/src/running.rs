use std::fmt::Write as _;
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

#[must_use]
pub fn as_a_clock(held: Duration) -> String {
    let all = held.as_secs();
    let hours = all.checked_div(3600).unwrap_or(0);
    let minutes = all
        .checked_div(60)
        .unwrap_or(0)
        .checked_rem(60)
        .unwrap_or(0);
    let seconds = all.checked_rem(60).unwrap_or(0);
    format!("{hours:02}:{minutes:02}:{seconds:02}")
}

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
    /// Which round of the search the sweep has reached. An automatic search opens another
    /// round whenever the last one found something, so this is the only honest measure of
    /// how far along it is.
    Round(u32),
    Holding(String),
    Producing(u64),
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
    named: Vec<String>,
    mark: bool,
    room: std::path::PathBuf,
    ready_within: Duration,
    host: Hosting,
    recorded: std::boxed::Box<dyn Fn() -> String + Send>,
    asked_to_stop: Arc<AtomicBool>,
}

enum Stopped {
    Gone,
    Refused(String),
}

fn held_again(
    send: &std::sync::mpsc::Sender<Heard>,
    host: &Hosting,
    step: Step,
    ready_within: Duration,
) -> Result<u16, Stopped> {
    let said = step.said();
    if send
        .send(Heard::Holding(format!("holding the model at {said}")))
        .is_err()
    {
        return Err(Stopped::Gone);
    }
    let telling = send.clone();
    let mut along = move |how: String| {
        let _sent = telling.send(Heard::Holding(format!(
            "holding the model at {said} — {how}"
        )));
    };
    let port = host(step, &mut along).map_err(Stopped::Refused)?;
    let ready = crate::trial::ready_within(port, ready_within, |seconds| {
        along(format!("waiting for the engine to answer, {seconds}s"));
    });
    if ready {
        return Ok(port);
    }
    Err(Stopped::Refused(format!(
        "the engine was held on port {port} but never started answering — a sweep will not \
         write down readings taken against a model that was still loading"
    )))
}

fn answered(doing: &mut Doing, asked: &Asked, step: Step) -> Result<crate::trial::Said, Stopped> {
    let telling = doing.send.clone();
    let mut along = move |held: u64| {
        let _sent = telling.send(Heard::Producing(held));
    };
    let first = match ask(&doing.endpoint, asked, &mut along) {
        Ok(said) => return Ok(said),
        Err(failure) => failure.to_string(),
    };
    doing
        .send
        .send(Heard::Holding(
            "the engine stopped answering — holding it again".to_owned(),
        ))
        .map_err(|_gone| Stopped::Gone)?;
    match held_again(&doing.send, &doing.host, step, doing.ready_within) {
        Ok(port) => doing.endpoint.port = port,
        Err(Stopped::Gone) => return Err(Stopped::Gone),
        Err(Stopped::Refused(why)) => {
            return Err(Stopped::Refused(format!(
                "{first}; holding it again did not work either: {why}"
            )));
        }
    }
    let telling = doing.send.clone();
    let mut along = move |held: u64| {
        let _sent = telling.send(Heard::Producing(held));
    };
    ask(&doing.endpoint, asked, &mut along).map_err(|again| {
        Stopped::Refused(format!("the engine stopped answering twice over: {again}"))
    })
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
        let round = doing.course.hunt().map_or(0, crate::hunt::Hunt::round);
        if doing.send.send(Heard::Round(round)).is_err() {
            return;
        }
        if doing.send.send(Heard::Started(spot)).is_err() {
            return;
        }
        if needs_a_fresh_hold(doing.dial, held_at, spot.step) {
            match held_again(&doing.send, &doing.host, spot.step, doing.ready_within) {
                Ok(port) => {
                    doing.endpoint.port = port;
                    held_at = Some(spot.step);
                }
                Err(Stopped::Gone) => return,
                Err(Stopped::Refused(why)) => {
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
            ceiling: if doing.dial.times_reading_the_prompt() {
                crate::trial::prompt_within(doing.under.context)
            } else if doing.dial.only_changes_speed() {
                crate::trial::TOKENS_TIMED
            } else {
                doing.ceiling
            },
            named: doing.named.clone(),
            timing: doing.dial.only_changes_speed(),
        };
        let began = Instant::now();
        let said = match answered(&mut doing, &asked, spot.step) {
            Ok(said) => said,
            Err(Stopped::Gone) => return,
            Err(Stopped::Refused(why)) => {
                let _sent = doing.send.send(Heard::Refused(why));
                break;
            }
        };
        held_at = Some(spot.step);
        let milliseconds = u64::try_from(began.elapsed().as_millis()).unwrap_or(u64::MAX);
        let marking = doing.mark && !doing.dial.only_changes_speed();
        let (judged, unmarked) = judged_by(marking, &doing.room, spot, &asked.set.tasks, &said);
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
    pub named: Vec<String>,
    pub mark: bool,
    pub room: std::path::PathBuf,
    pub ready_within: Duration,
}

pub type Hosting =
    std::boxed::Box<dyn Fn(Step, &mut dyn FnMut(String)) -> Result<u16, String> + Send>;

#[derive(Debug)]
pub struct Running {
    heard: Receiver<Heard>,
    stop: Arc<AtomicBool>,
    pub report: Report,
    pub doing: Option<At>,
    pub taken: usize,
    pub skipped: usize,
    pub refused: Option<String>,
    pub holding: Option<String>,
    pub produced: u64,
    pub stopped: Option<String>,
    pub finished: bool,
    pub round: u32,
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
            named,
            mark,
            room,
            ready_within,
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
                named,
                mark,
                room,
                ready_within,
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
            produced: 0,
            stopped: None,
            finished: false,
            round: 0,
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
                    self.produced = 0;
                    moved = true;
                }
                Ok(Heard::Holding(said)) => {
                    self.holding = Some(said);
                    moved = true;
                }
                Ok(Heard::Producing(held)) => {
                    self.produced = held;
                    moved = true;
                }
                Ok(Heard::Round(round)) => {
                    self.round = round;
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

    /// What a sweep is doing this second: the clock, the value it is on, and how far into
    /// that one trial it has got. Nothing that does not move — a take counter that is always
    /// one, or a token count on work that writes no tokens, is a number to read and discard.
    #[must_use]
    pub fn label(&self, named: &[String], dial: Dial, ceiling: u32) -> String {
        let clock = as_a_clock(self.running_for());
        if let Some(said) = &self.holding {
            return format!("{clock} · {said}");
        }
        let Some(at) = self.doing else {
            if self.finished {
                return clock;
            }
            return format!("{clock} · starting");
        };
        let mut said = format!("{clock} · {}", dial.said_among(at.step, named));
        if !dial.only_changes_speed() {
            let _wrote = write!(said, " · set {}", at.set);
        }
        if at.repeat > 1 {
            let _wrote = write!(said, " · take {}", at.repeat);
        }
        let doing = if dial.times_reading_the_prompt() {
            format!(" · read {} of {ceiling}", self.produced)
        } else {
            format!(" · wrote {} of {ceiling}", self.produced)
        };
        said.push_str(&doing);
        said
    }

    /// How far through the sweep it is, in the only terms an automatic search can honestly
    /// give: the round it is on and what it has measured. A search that opens another round
    /// whenever the last one found something has no total to count towards.
    #[must_use]
    pub fn far_along(&self) -> String {
        let mut said = match self.round {
            0 => String::new(),
            round => format!("round {round} · "),
        };
        let _wrote = write!(said, "{} measured", self.taken);
        if self.skipped > 0 {
            let _wrote = write!(said, ", {} already known", self.skipped);
        }
        said
    }

    /// What a sweep has come to, in one sentence. What it counted, and then why it stopped
    /// counting — each said once, because a line that says the same thing twice is a line
    /// that gets cut off before its end.
    #[must_use]
    pub fn said(&self) -> String {
        if let Some(why) = &self.refused {
            return format!("stopped: {why}");
        }
        let counted = self.far_along();
        if self.finished {
            return match &self.stopped {
                Some(why) => format!("{counted} · {why}"),
                None => counted,
            };
        }
        if let Some(said) = &self.holding {
            return format!("{counted} · {said}");
        }
        counted
    }
}

#[cfg(test)]
mod tests;
