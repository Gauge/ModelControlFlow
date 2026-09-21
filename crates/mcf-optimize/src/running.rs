use std::fmt::Write as _;
use std::sync::mpsc::{Receiver, TryRecvError, channel};
use std::sync::{Arc, atomic::AtomicBool, atomic::Ordering};
use std::time::{Duration, Instant};

use crate::corpus::Set;
use crate::course::{Course, Next};
use crate::dial::{Dial, Step};
use crate::ledger::{At, Ledger, Under};
use crate::reading::{Measure, Reading, Report};
use crate::trial::{Asked, Endpoint, ask, reading_of};

const MARKING_PATIENCE: Duration = Duration::from_secs(600);

mod together;
pub use together::{AT_ONCE, Question};

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

/// A length of time as a person would say it roughly: to the minute past an hour, to the
/// second under a minute.
#[must_use]
pub fn roughly(held: Duration) -> String {
    let all = held.as_secs();
    let hours = all.checked_div(3600).unwrap_or(0);
    let minutes = all
        .checked_div(60)
        .unwrap_or(0)
        .checked_rem(60)
        .unwrap_or(0);
    if hours > 0 {
        return format!("{hours} h {minutes} m");
    }
    if minutes > 0 {
        return format!("{minutes} m");
    }
    format!("{all} s")
}

fn judged_by(
    mark: bool,
    room: &std::path::Path,
    spot: At,
    set_kind: crate::corpus::Kind,
    tasks: &[crate::corpus::Task],
    said: &crate::trial::Said,
) -> (Vec<crate::marking::Checked>, Option<String>) {
    if !mark {
        return (crate::marking::nothing_held(tasks), None);
    }
    // A set whose answers are read needs no container, and nothing is written to disk to
    // mark it: the questions each have one right answer and the model wrote them on lines.
    if set_kind == crate::corpus::Kind::ShortAnswer {
        return (crate::marking::marked_by_reading(tasks, &said.answer), None);
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
    /// Whether the sweep is waiting to be told to carry on. Sent when it starts waiting
    /// and again when it stops, so that what the window shows is what the sweep is doing
    /// rather than what it was last asked to do.
    Paused(bool),
    /// Which round of the search the sweep has reached. An automatic search opens another
    /// round whenever the last one found something, so this is the only honest measure of
    /// how far along it is.
    Round(u32),
    Holding(String),
    /// The model is loaded and answering. Whatever was said about loading it is over.
    Held,
    /// The questions of the set about to be asked, in order: what each asks, and the answer
    /// it wants.
    Questions(Vec<Question>),
    /// A question, by where it stands in its set counting from nought, has been sent.
    Sent {
        at: usize,
    },
    /// The latest of a question's reply, as it arrives: the end of what the model has
    /// thought, the end of what it has answered, and how much it has written.
    Reply {
        at: usize,
        thought: String,
        answer: String,
        produced: u64,
    },
    /// A question's reply has come back, after this long.
    Answered {
        at: usize,
        milliseconds: u64,
    },
    /// Where this trial falls among the trials of its value, counting from one, and how
    /// many that value has.
    Place {
        at: usize,
        of: usize,
    },
    /// One question marked, as soon as it is. A set of them is minutes of work, and a
    /// window that only hears about the set sits still for all of it.
    Marked {
        at: usize,
        right: bool,
        produced: u64,
        /// The answer line the marker read, as the model wrote it.
        given: Option<String>,
        ending: crate::reading::Ending,
        why: Option<String>,
    },
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
    ceiling: Option<u32>,
    named: Vec<String>,
    switch: bool,
    mark: bool,
    at_once: usize,
    room: std::path::PathBuf,
    ready_within: Duration,
    host: Hosting,
    recorded: std::boxed::Box<dyn Fn() -> String + Send>,
    asked_to_stop: Arc<AtomicBool>,
    asked_to_wait: Arc<AtomicBool>,
}

enum Stopped {
    Gone,
    Refused(String),
    /// Asked to stop while this trial was still running. Nothing is written down for it.
    Cut,
}

/// What loading the model is for, in words. A setting that rides in each request is not
/// what the model is loaded with, so it is not named: "at 0" beside a load said the model
/// was being loaded with a temperature, which it never is.
fn loading_for(dial: Dial, step: Step, named: &[String]) -> String {
    if !dial.reloads_the_engine() {
        return "loading the model".to_owned();
    }
    format!(
        "loading the model with {} {}",
        dial.label().to_lowercase(),
        dial.said_among(step, named)
    )
}

fn held_again(
    send: &std::sync::mpsc::Sender<Heard>,
    host: &Hosting,
    (step, what): (Step, String),
    ready_within: Duration,
    give_up: &dyn Fn() -> bool,
) -> Result<u16, Stopped> {
    if send.send(Heard::Holding(what.clone())).is_err() {
        return Err(Stopped::Gone);
    }
    let telling = send.clone();
    let mut along = move |how: String| {
        let _sent = telling.send(Heard::Holding(format!("{what} — {how}")));
    };
    let port = host(step, &mut along).map_err(Stopped::Refused)?;
    let ready = crate::trial::ready_within(
        port,
        ready_within,
        |seconds| {
            along(format!("waiting for the engine to answer, {seconds}s"));
        },
        give_up,
    );
    if ready {
        // Said as soon as it is true, so the line under the bar goes back to saying what
        // the sweep is doing rather than what it was doing before it started.
        let _sent = send.send(Heard::Held);
        return Ok(port);
    }
    if give_up() {
        return Err(Stopped::Cut);
    }
    Err(Stopped::Refused(format!(
        "the engine was held on port {port} but never started answering — a sweep will not \
         write down readings taken against a model that was still loading"
    )))
}

fn answered(
    doing: &mut Doing,
    asked: &Asked,
    step: Step,
) -> Result<crate::trial::Outcome, Stopped> {
    let flag = Arc::clone(&doing.asked_to_stop);
    let give_up = move || flag.load(Ordering::Relaxed);
    let telling = doing.send.clone();
    let mut along = move |held: crate::trial::Along<'_>| {
        if let crate::trial::Along::Counted(held) = held {
            let _sent = telling.send(Heard::Producing(held));
        }
    };
    let first = match ask(&doing.endpoint, asked, &mut along, &give_up) {
        Ok(outcome) => return Ok(outcome),
        Err(failure) => failure.to_string(),
    };
    // The engine dropped the connection. If that happened because the sweep was stopping,
    // there is nothing to hold again for.
    if give_up() {
        return Err(Stopped::Cut);
    }
    doing
        .send
        .send(Heard::Holding(
            "the engine stopped answering — loading the model again".to_owned(),
        ))
        .map_err(|_gone| Stopped::Gone)?;
    let what = loading_for(doing.dial, step, &doing.named);
    match held_again(
        &doing.send,
        &doing.host,
        (step, what),
        doing.ready_within,
        &give_up,
    ) {
        Ok(port) => doing.endpoint.port = port,
        Err(Stopped::Gone) => return Err(Stopped::Gone),
        Err(Stopped::Cut) => return Err(Stopped::Cut),
        Err(Stopped::Refused(why)) => {
            return Err(Stopped::Refused(format!(
                "{first}; holding it again did not work either: {why}"
            )));
        }
    }
    let telling = doing.send.clone();
    let mut along = move |held: crate::trial::Along<'_>| {
        if let crate::trial::Along::Counted(held) = held {
            let _sent = telling.send(Heard::Producing(held));
        }
    };
    ask(&doing.endpoint, asked, &mut along, &give_up).map_err(|again| {
        if give_up() {
            return Stopped::Cut;
        }
        Stopped::Refused(format!("the engine stopped answering twice over: {again}"))
    })
}

/// Ask a short-answer set one question to a request, several requests at once, and mark
/// each answer as it comes back.
///
/// The set is still the unit written down — one reading, the sum of its questions — so a
/// ledger read a year from now still has one row per set. What changes is that a model
/// asked one sum no longer reasons about twenty-four others before it answers it.
fn answered_one_at_a_time(
    doing: &mut Doing,
    asked: &Asked,
    step: Step,
) -> Result<Option<(crate::trial::Said, Vec<crate::marking::Checked>)>, Stopped> {
    Ok(together::asked_together(doing, asked, step)?.map(together::summed))
}

/// Take one trial: a short-answer set a question at a time, marked as it goes, and anything
/// else in one request, left to be marked once it is back. Nothing means it was cut short.
fn taken(
    doing: &mut Doing,
    asked: &Asked,
    step: Step,
    one_at_a_time: bool,
) -> Result<Option<(crate::trial::Said, Vec<crate::marking::Checked>)>, Stopped> {
    if one_at_a_time {
        return answered_one_at_a_time(doing, asked, step);
    }
    answered(doing, asked, step).map(|outcome| match outcome {
        crate::trial::Outcome::Said(said) => Some((said, Vec::new())),
        crate::trial::Outcome::Cut => None,
    })
}

/// What one trial asks for: how much room it gets, and whether it is timed or marked.
fn trial_for(doing: &Doing, spot: At, set: Set, timed: bool) -> Asked {
    Asked {
        set,
        dial: doing.dial,
        step: spot.step,
        repeat: spot.repeat,
        ceiling: if !timed {
            doing.ceiling
        } else if doing.dial.times_reading_the_prompt() {
            Some(crate::trial::prompt_within(doing.under.context))
        } else {
            Some(crate::trial::TOKENS_TIMED)
        },
        named: doing.named.clone(),
        timing: timed,
        switch: doing.switch,
    }
}

/// How often a paused sweep looks to see whether it has been told to carry on.
const WHILE_PAUSED: Duration = Duration::from_millis(200);

/// Wait here for as long as the sweep is paused, and say whether it should carry on.
///
/// A sweep is paused between trials, never inside one. A trial part way through is a
/// reading part way through, and a reading that was taken across a pause is a reading
/// of nothing — the engine sat idle for however long the pause lasted, and the rate that
/// came out of it would be written into the ledger as if it meant something.
fn waited_out(doing: &Doing) -> bool {
    if !doing.asked_to_wait.load(Ordering::Relaxed) {
        return !doing.asked_to_stop.load(Ordering::Relaxed);
    }
    if doing.send.send(Heard::Paused(true)).is_err() {
        return false;
    }
    while doing.asked_to_wait.load(Ordering::Relaxed) {
        if doing.asked_to_stop.load(Ordering::Relaxed) {
            return false;
        }
        std::thread::sleep(WHILE_PAUSED);
    }
    doing.send.send(Heard::Paused(false)).is_ok() && !doing.asked_to_stop.load(Ordering::Relaxed)
}

enum Held {
    Ready,
    /// The window has gone, so there is nobody left to report to.
    Gone,
    /// Stopped, or refused — either way this sweep is done.
    Over,
}

/// Hold the model again at a value, for the dials that are launch flags.
fn hold_for(doing: &mut Doing, step: Step) -> Held {
    let flag = Arc::clone(&doing.asked_to_stop);
    let give_up = move || flag.load(Ordering::Relaxed);
    let what = loading_for(doing.dial, step, &doing.named);
    match held_again(
        &doing.send,
        &doing.host,
        (step, what),
        doing.ready_within,
        &give_up,
    ) {
        Ok(port) => {
            doing.endpoint.port = port;
            Held::Ready
        }
        Err(Stopped::Gone) => Held::Gone,
        Err(Stopped::Cut) => Held::Over,
        Err(Stopped::Refused(why)) => {
            let _sent = doing.send.send(Heard::Refused(why));
            Held::Over
        }
    }
}

/// Say which round the search is in, which trial is starting, and where it falls among
/// its value's trials. False when nobody is listening any more.
fn told_where(doing: &Doing, spot: At) -> bool {
    let round = doing.course.hunt().map_or(0, crate::hunt::Hunt::round);
    let place = doing
        .course
        .place_of(&spot)
        .map(|(at, of)| Heard::Place { at, of });
    [Some(Heard::Round(round)), Some(Heard::Started(spot)), place]
        .into_iter()
        .flatten()
        .all(|heard| doing.send.send(heard).is_ok())
}

fn sweeping(mut doing: Doing) {
    let mut held_at: Option<Step> = None;
    loop {
        if doing.asked_to_stop.load(Ordering::Relaxed) {
            break;
        }
        if !waited_out(&doing) {
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
        if !told_where(&doing, spot) {
            return;
        }
        if needs_a_fresh_hold(doing.dial, held_at, spot.step) {
            match hold_for(&mut doing, spot.step) {
                Held::Ready => held_at = Some(spot.step),
                Held::Gone => return,
                Held::Over => break,
            }
        }
        let timed = !doing.course.measure().needs_the_answers_run();
        let Some(set) = Set::numbered(spot.set) else {
            let _sent = doing.send.send(Heard::Refused(format!(
                "there is no test set numbered {}",
                spot.set
            )));
            continue;
        };
        let asked = trial_for(&doing, spot, set, timed);
        let marking = doing.mark && !timed;
        let one_at_a_time = marking && asked.set.kind == crate::corpus::Kind::ShortAnswer;
        let began = Instant::now();
        let (said, marked_already) = match taken(&mut doing, &asked, spot.step, one_at_a_time) {
            Ok(Some(taken)) => taken,
            // Cut off part way through. Nothing below this line runs, so nothing about
            // this trial reaches the ledger, the report or the window.
            Ok(None) | Err(Stopped::Cut) => break,
            Err(Stopped::Gone) => return,
            Err(Stopped::Refused(why)) => {
                let _sent = doing.send.send(Heard::Refused(why));
                break;
            }
        };
        held_at = Some(spot.step);
        let milliseconds = u64::try_from(began.elapsed().as_millis()).unwrap_or(u64::MAX);
        if doing.asked_to_stop.load(Ordering::Relaxed) {
            break;
        }
        let (judged, unmarked) = if one_at_a_time {
            (marked_already, None)
        } else {
            judged_by(
                marking,
                &doing.room,
                spot,
                asked.set.kind,
                &asked.set.tasks,
                &said,
            )
        };
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
    /// How many tokens a marked trial may write. Nothing, as the desk asks it: a question
    /// is answered for as long as the model needs, up to the window it is held at.
    pub ceiling: Option<u32>,
    pub named: Vec<String>,
    /// Whether this model's template reads `enable_thinking`, which decides how a trial
    /// asks for no thinking at all.
    pub switch: bool,
    pub mark: bool,
    /// How many of a set's questions are asked at once. The engine the sweep holds is held
    /// with at least this many slots, so they are answered together rather than queued.
    pub at_once: usize,
    pub room: std::path::PathBuf,
    pub ready_within: Duration,
}

pub type Hosting =
    std::boxed::Box<dyn Fn(Step, &mut dyn FnMut(String)) -> Result<u16, String> + Send>;

/// What came of one question.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Verdict {
    Right,
    /// Answered wrongly, with the answer the marker read, if it found one at all.
    Wrong(Option<String>),
    /// Looped, or filled the window it is held at, and so marked wrong — said apart from a
    /// wrong answer so that a runaway is never mistaken for one.
    RanAway(String),
}

/// One question of the set being asked, as far as it has got.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Seen {
    pub asked: String,
    pub wanted: String,
    pub sent: bool,
    /// The end of what the model has thought, and of what it has answered, so far.
    pub thought: String,
    pub answer: String,
    pub produced: u64,
    pub milliseconds: Option<u64>,
    pub verdict: Option<Verdict>,
}

impl Seen {
    /// Sent, and not come back marked.
    #[must_use]
    pub const fn open(&self) -> bool {
        self.sent && self.verdict.is_none()
    }
}

/// What one value has come to so far, counted a question at a time. Started again when
/// the sweep moves to another value, because a score is only worth watching against the
/// value it belongs to.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Tally {
    pub value: Option<Step>,
    pub asked: u64,
    pub right: u64,
    pub produced: u64,
    /// How long the sweep has spent on this value, by the clock, since its first question
    /// was sent. The clock rather than the questions' own times added up: four questions
    /// in flight at once each take longer than one would, and adding them up would count
    /// the same minute four times over.
    pub milliseconds: u64,
    /// When, on the sweep's clock, this value's first question was sent.
    pub since: Option<Duration>,
}

impl Tally {
    /// How long a question has taken on this value, on average.
    #[must_use]
    pub fn a_question(&self) -> Option<Duration> {
        let each = self.milliseconds.checked_div(self.asked)?;
        Some(Duration::from_millis(each))
    }

    /// One more question marked.
    pub const fn count(&mut self, right: bool, produced: u64) {
        self.asked = self.asked.saturating_add(1);
        if right {
            self.right = self.right.saturating_add(1);
        }
        self.produced = self.produced.saturating_add(produced);
    }

    /// How fast this value's answers are being written, over every question so far.
    #[must_use]
    pub fn tokens_a_second(&self) -> Option<u64> {
        self.produced
            .saturating_mul(1000)
            .checked_div(self.milliseconds)
    }

    /// The share of this value's questions answered rightly so far, in whole percent.
    #[must_use]
    pub fn share(&self) -> Option<u64> {
        self.right.saturating_mul(100).checked_div(self.asked)
    }
}

#[derive(Debug)]
pub struct Running {
    heard: Receiver<Heard>,
    stop: Arc<AtomicBool>,
    wait: Arc<AtomicBool>,
    pub report: Report,
    pub doing: Option<At>,
    pub taken: usize,
    pub skipped: usize,
    pub refused: Option<String>,
    pub holding: Option<String>,
    /// Which question of a short-answer set is being asked, and of how many.
    pub question: Option<(usize, usize)>,
    /// Where the trial being taken falls among the trials of its value, and of how many.
    pub place: Option<(usize, usize)>,
    /// What the value on screen has come to so far in this sweep, question by question.
    pub tally: Tally,
    /// Every question of the set being asked: what it asks, what it wants, and what has come
    /// of it so far. Kept until the next set's questions arrive, so the last set can still
    /// be looked back over in the moment between the two.
    pub questions: Vec<Seen>,
    pub produced: u64,
    pub stopped: Option<String>,
    pub finished: bool,
    pub round: u32,
    /// Whether the sweep has actually stopped where it stands, as opposed to having been
    /// asked to. A sweep asked to wait finishes the trial it is in first.
    pub waiting: bool,
    started: Instant,
    /// How long the sweep has spent waiting to be told to carry on, so that the clock
    /// beside it counts work rather than counting a pause as though it were work.
    paused_for: Duration,
    paused_at: Option<Instant>,
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
            switch,
            ceiling,
            named,
            mark,
            at_once,
            room,
            ready_within,
        } = orders;
        let (send, heard) = channel();
        let stop = Arc::new(AtomicBool::new(false));
        let wait = Arc::new(AtomicBool::new(false));
        let asked_to_stop = Arc::clone(&stop);
        let asked_to_wait = Arc::clone(&wait);
        let _worker = std::thread::spawn(move || {
            sweeping(Doing {
                switch,
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
                at_once,
                room,
                ready_within,
                host,
                recorded: std::boxed::Box::new(recorded),
                asked_to_stop,
                asked_to_wait,
            });
        });
        Self {
            heard,
            stop,
            wait,
            report: Report::default(),
            doing: None,
            taken: 0,
            skipped: 0,
            refused: None,
            holding: None,
            question: None,
            place: None,
            tally: Tally::default(),
            questions: Vec::new(),
            produced: 0,
            stopped: None,
            finished: false,
            round: 0,
            waiting: false,
            started: Instant::now(),
            paused_for: Duration::ZERO,
            paused_at: None,
        }
    }

    /// Stop where it stands, inside the trial it is in.
    ///
    /// The trial under way is abandoned, not finished: its connection is closed and
    /// nothing about it is written to the ledger, the report or the window. Trials that
    /// had already finished are measurements and stay where they are.
    pub fn stop(&self) {
        // Cleared, so that a sweep paused and then stopped is not left asleep waiting for
        // a carry-on that is never coming.
        self.wait.store(false, Ordering::Relaxed);
        self.stop.store(true, Ordering::Relaxed);
    }

    #[must_use]
    pub fn stopping(&self) -> bool {
        self.stop.load(Ordering::Relaxed)
    }

    /// Ask the sweep to wait after the trial it is in. Unlike a stop, a pause lets the
    /// trial under way finish, because a reading cut in half is not a reading and the
    /// point of pausing is to come back to where you were.
    pub fn pause(&self) {
        self.wait.store(true, Ordering::Relaxed);
    }

    /// Tell a paused sweep to carry on.
    pub fn resume(&self) {
        self.wait.store(false, Ordering::Relaxed);
    }

    /// Whether the sweep has been asked to wait. It may still be finishing a trial.
    #[must_use]
    pub fn asked_to_wait(&self) -> bool {
        self.wait.load(Ordering::Relaxed)
    }

    /// How long the sweep has been working, not counting time spent paused. A clock that
    /// counted a pause would read as though the sweep were slower than it is.
    #[must_use]
    pub fn running_for(&self) -> std::time::Duration {
        let paused = match self.paused_at {
            Some(since) => self.paused_for.saturating_add(since.elapsed()),
            None => self.paused_for,
        };
        self.started.elapsed().saturating_sub(paused)
    }

    pub fn hear(&mut self) -> bool {
        let mut moved = false;
        loop {
            match self.heard.try_recv() {
                Ok(Heard::Paused(waiting)) => {
                    self.waiting = waiting;
                    if waiting {
                        self.paused_at = Some(Instant::now());
                    } else if let Some(since) = self.paused_at.take() {
                        self.paused_for = self.paused_for.saturating_add(since.elapsed());
                    }
                    moved = true;
                }
                Ok(Heard::Started(at)) => {
                    if self.tally.value != Some(at.step) {
                        self.tally = Tally {
                            value: Some(at.step),
                            ..Tally::default()
                        };
                    }
                    self.doing = Some(at);
                    self.question = None;
                    self.place = None;
                    self.produced = 0;
                    moved = true;
                }
                Ok(Heard::Holding(said)) => {
                    self.holding = Some(said);
                    moved = true;
                }
                Ok(Heard::Held) => {
                    self.holding = None;
                    moved = true;
                }
                Ok(
                    heard @ (Heard::Questions(_)
                    | Heard::Sent { .. }
                    | Heard::Reply { .. }
                    | Heard::Answered { .. }
                    | Heard::Marked { .. }),
                ) => {
                    self.heard_of_a_question(heard);
                    moved = true;
                }
                Ok(Heard::Place { at, of }) => {
                    self.place = Some((at, of));
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
                    self.question = None;
                    self.place = None;
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
                    self.waiting = false;
                    if let Some(since) = self.paused_at.take() {
                        self.paused_for = self.paused_for.saturating_add(since.elapsed());
                    }
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

    /// Take in word of one question: the set's questions arriving, one being sent, its reply
    /// arriving, and its verdict.
    fn heard_of_a_question(&mut self, heard: Heard) {
        match heard {
            Heard::Questions(listed) => {
                self.questions = listed
                    .into_iter()
                    .map(|question| Seen {
                        asked: question.asked,
                        wanted: question.wanted,
                        ..Seen::default()
                    })
                    .collect();
            }
            Heard::Sent { at } => {
                if self.tally.since.is_none() {
                    self.tally.since = Some(self.running_for());
                }
                if let Some(seen) = self.questions.get_mut(at) {
                    seen.sent = true;
                }
            }
            Heard::Reply {
                at,
                thought,
                answer,
                produced,
            } => {
                if let Some(seen) = self.questions.get_mut(at) {
                    seen.thought = thought;
                    seen.answer = answer;
                    seen.produced = produced;
                }
            }
            Heard::Answered { at, milliseconds } => {
                if let Some(seen) = self.questions.get_mut(at) {
                    seen.milliseconds = Some(milliseconds);
                }
            }
            Heard::Marked {
                at,
                right,
                produced,
                given,
                ending,
                why,
            } => {
                let verdict = if ending.is_a_runaway() {
                    Verdict::RanAway(why.unwrap_or_else(|| ending.label().to_owned()))
                } else if right {
                    Verdict::Right
                } else {
                    Verdict::Wrong(given)
                };
                if let Some(seen) = self.questions.get_mut(at) {
                    seen.verdict = Some(verdict);
                }
                self.tally.count(right && !ending.is_a_runaway(), produced);
                if let Some(since) = self.tally.since {
                    let on_it = self.running_for().saturating_sub(since);
                    self.tally.milliseconds = u64::try_from(on_it.as_millis()).unwrap_or(u64::MAX);
                }
            }
            _ => {}
        }
        let of = self.questions.len();
        let done = self
            .questions
            .iter()
            .filter(|seen| seen.verdict.is_some())
            .count();
        self.question = (of > 0).then(|| (done.saturating_add(1).min(of), of));
        self.produced = self.following().map_or(0, |at| {
            self.questions.get(at).map_or(0, |seen| seen.produced)
        });
    }

    /// The question the window follows: the oldest one still being answered.
    #[must_use]
    pub fn following(&self) -> Option<usize> {
        self.questions.iter().position(Seen::open)
    }

    /// Which of the set's questions are being answered now, counting from one: the first
    /// and the last of them.
    #[must_use]
    pub fn in_flight(&self) -> Option<(usize, usize)> {
        let first = self.questions.iter().position(Seen::open)?;
        let last = self.questions.iter().rposition(Seen::open)?;
        Some((first.saturating_add(1), last.saturating_add(1)))
    }

    /// What a sweep is doing this second: the clock, the value it is on, and how far into
    /// that one trial it has got. Nothing that does not move — a take counter that is always
    /// one, or a token count on work that writes no tokens, is a number to read and discard.
    #[must_use]
    pub fn label(
        &self,
        named: &[String],
        dial: Dial,
        measure: Measure,
        ceiling: Option<u32>,
    ) -> String {
        let clock = as_a_clock(self.running_for());
        if self.waiting {
            return format!("{clock} · paused");
        }
        if self.asked_to_wait() {
            return format!("{clock} · pausing after this trial");
        }
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
        let timed = !measure.needs_the_answers_run();
        if !timed {
            let _wrote = write!(said, " · set {}", at.set);
            if let Some((place, of)) = self.place.filter(|(_, of)| *of > 1) {
                let _wrote = write!(said, " ({place} of {of})");
            }
        }
        if at.repeat > 1 {
            let _wrote = write!(said, " · take {}", at.repeat);
        }
        if let Some((_, of)) = self.question.filter(|_| !timed) {
            let _wrote = match self.in_flight() {
                Some((first, last)) if last > first => {
                    write!(said, " · questions {first}–{last} of {of}")
                }
                Some((first, _)) => write!(said, " · question {first} of {of}"),
                None => write!(
                    said,
                    " · question {} of {of}",
                    self.question.map_or(1, |(at, _)| at)
                ),
            };
        }
        let work = if timed && dial.times_reading_the_prompt() {
            "read"
        } else {
            "wrote"
        };
        // A trial asked for no number of tokens has nothing to count towards, so it counts
        // what it has done and nothing else.
        let doing = ceiling.map_or_else(
            || format!(" · {work} {}", self.produced),
            |ceiling| format!(" · {work} {} of {ceiling}", self.produced),
        );
        said.push_str(&doing);
        said
    }

    /// How far through the value on screen the sweep is, a question at a time: the sets of
    /// this value already done, and the questions of this set already marked. Nothing for a
    /// trial that is not asked a question at a time, which has nothing finer to count.
    #[must_use]
    pub fn through_the_value(&self) -> Option<f32> {
        let (place, sets) = self.place?;
        let (question, of) = self.question?;
        let whole = u16::try_from(sets.saturating_mul(of)).ok()?;
        let done = place
            .saturating_sub(1)
            .saturating_mul(of)
            .saturating_add(question.saturating_sub(1));
        let done = u16::try_from(done).ok()?;
        (whole > 0).then(|| f32::from(done) / f32::from(whole))
    }

    /// What the value on screen has scored so far and how fast it is going, said while it
    /// is still being measured rather than once a set of twenty-five has finished.
    #[must_use]
    pub fn so_far(&self, named: &[String], dial: Dial) -> Option<String> {
        let tally = &self.tally;
        if tally.asked == 0 {
            return None;
        }
        let value = dial.said_among(tally.value?, named);
        let share = tally.share().unwrap_or(0);
        let mut said = format!(
            "{value} so far: {} of {} right ({share}%)",
            tally.right, tally.asked
        );
        if let Some(each) = tally.a_question() {
            let _wrote = write!(said, " · {:.1} s a question", each.as_secs_f64());
            if let Some(rate) = tally.tokens_a_second() {
                let _wrote = write!(said, " ({rate} tok/s)");
            }
            if let (Some((place, sets)), Some((question, of))) = (self.place, self.question) {
                let left = sets
                    .saturating_sub(place)
                    .saturating_mul(of)
                    .saturating_add(of.saturating_sub(question.saturating_sub(1)));
                let left = u32::try_from(left).unwrap_or(u32::MAX);
                let _wrote = write!(
                    said,
                    " · about {} left on {value}",
                    roughly(each.saturating_mul(left))
                );
            }
        }
        Some(said)
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
        if self.waiting {
            return format!("{counted} · paused where it stood — carry on to take the rest");
        }
        if self.asked_to_wait() {
            return format!("{counted} · pausing once this trial is done");
        }
        if let Some(said) = &self.holding {
            return format!("{counted} · {said}");
        }
        counted
    }
}

#[cfg(test)]
mod tests;
