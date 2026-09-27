//! Asking the questions of a set several at a time.
//!
//! An engine given one request at a time streams the model's weights out of memory once
//! for every token of that one answer. Given several, it writes a token of each in the same
//! pass, and the pass costs little more. Measured on this machine's kind of hardware, four
//! at once answered half again as many questions a minute as one, and eight hardly more
//! than four — so four is what a marked sweep asks, and each question is still asked on
//! its own, in a request of its own, marked on its own.

use std::collections::VecDeque;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use super::{Doing, Heard, MARKING_PATIENCE, Stopped, answered, held_again, loading_for};
use crate::corpus::{Kind, Set};
use crate::dial::Step;
use crate::marking::Checked;
use crate::reading::Ending;
use crate::trial::{Along, Asked, Endpoint, Outcome, Said, ask};

/// How many of a set's questions are asked at once, unless a sweep is told otherwise.
pub const AT_ONCE: usize = 4;

/// How much of a reply the window is sent as it arrives: the latest part, which is the part
/// being written. A reply that thinks for ten thousand tokens is not sent whole every time a
/// piece of it arrives.
const TAIL: usize = 800;

/// How many pieces arrive between one word to the window and the next.
const TOLD_EVERY: usize = 12;

/// What one question came to: the reply, and the verdict on it.
pub(super) type Answered = (Said, Vec<Checked>);

/// What a question is, for the window: what was asked, and the answer wanted — which for a
/// program is nothing to read, since what it is wanted to do is what the question says.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Question {
    pub asked: String,
    pub wanted: String,
}

/// What came of asking one question.
enum Came {
    Marked(Said, Vec<Checked>),
    /// The sweep was told to stop while it was being answered.
    Cut,
    /// The engine went away part way through. Asked again once the model is held again.
    Dropped(String),
    /// Answered, but the answer could not be marked — the container could not be run. A
    /// sweep does not write down a score that was never taken.
    Unmarked(String),
}

/// Ask every question of the set, `at_once` at a time, and hand back what each came to in
/// the order the questions stand in. Nothing means the sweep was told to stop part way.
pub(super) fn asked_together(
    doing: &mut Doing,
    asked: &Asked,
    step: Step,
) -> Result<Option<Vec<Answered>>, Stopped> {
    let questions = asked.set.one_at_a_time();
    let listed = questions
        .iter()
        .filter_map(|held| held.tasks.first())
        .map(|task| Question {
            asked: task.asked.clone(),
            wanted: if asked.set.kind == Kind::ShortAnswer {
                task.checked.clone()
            } else {
                String::new()
            },
        })
        .collect();
    doing
        .send
        .send(Heard::Questions {
            kind: asked.set.kind,
            listed,
        })
        .map_err(|_gone| Stopped::Gone)?;
    let mut came: Vec<Option<Came>> = {
        let queue = Mutex::new((0..questions.len()).collect::<VecDeque<usize>>());
        let results = Mutex::new((0..questions.len()).map(|_| None).collect::<Vec<_>>());
        let shared = Shared {
            room: &doing.room,
            endpoint: &doing.endpoint,
            asked,
            questions: &questions,
            queue: &queue,
            results: &results,
            stop: &doing.asked_to_stop,
        };
        std::thread::scope(|scope| {
            for _ in 0..doing.at_once.clamp(1, questions.len().max(1)) {
                let send = doing.send.clone();
                let shared = &shared;
                let _worker = scope.spawn(move || shared.work(&send));
            }
        });
        results.into_inner().unwrap_or_default()
    };
    if let Some(why) = came.iter().find_map(|held| match held {
        Some(Came::Unmarked(why)) => Some(why.clone()),
        _ => None,
    }) {
        return Err(Stopped::Refused(format!("answers were not marked: {why}")));
    }
    if doing.asked_to_stop.load(Ordering::Relaxed)
        || came
            .iter()
            .any(|held| matches!(held, Some(Came::Cut) | None))
    {
        return Ok(None);
    }
    again_where_it_failed(doing, asked, step, &questions, &mut came)?;
    Ok(Some(
        came.into_iter()
            .filter_map(|held| match held {
                Some(Came::Marked(said, marked)) => Some((said, marked)),
                _ => None,
            })
            .collect(),
    ))
}

/// What the workers share. Everything in it is read, or taken under a lock.
struct Shared<'held> {
    room: &'held Path,
    endpoint: &'held Endpoint,
    asked: &'held Asked,
    questions: &'held [Set],
    queue: &'held Mutex<VecDeque<usize>>,
    results: &'held Mutex<Vec<Option<Came>>>,
    stop: &'held Arc<AtomicBool>,
}

impl Shared<'_> {
    /// Take questions off the queue until there are none, asking each, marking it and
    /// saying how it went. A failure is kept rather than acted on: holding the model again
    /// is the sweep's to do, once, after every question in flight has come back.
    fn work(&self, send: &Sender<Heard>) {
        let give_up = || self.stop.load(Ordering::Relaxed);
        loop {
            if give_up() {
                return;
            }
            let Some(at) = self
                .queue
                .lock()
                .ok()
                .and_then(|mut queue| queue.pop_front())
            else {
                return;
            };
            let Some(set) = self.questions.get(at) else {
                return;
            };
            let one = Asked {
                set: set.clone(),
                ..self.asked.clone()
            };
            let came = match asked_one(self.endpoint, &one, at, send, &give_up) {
                Ok(Outcome::Said(said)) => match marked(self.room, at, &one, &said) {
                    Ok(marked) => {
                        told_the_verdict(send, at, &one, &said, &marked);
                        Came::Marked(said, marked)
                    }
                    Err(why) => Came::Unmarked(why),
                },
                Ok(Outcome::Cut) => Came::Cut,
                Err(why) => Came::Dropped(why),
            };
            if let Ok(mut results) = self.results.lock()
                && let Some(slot) = results.get_mut(at)
            {
                *slot = Some(came);
            }
        }
    }
}

/// Mark one answer the way its kind is marked: a short answer or a focus run by reading
/// it, and a program by running it against its check in a container, in a room of its own
/// so that four being marked at once never read each other's files.
fn marked(room: &Path, at: usize, one: &Asked, said: &Said) -> Result<Vec<Checked>, String> {
    if one.set.kind == Kind::ShortAnswer {
        return Ok(crate::marking::marked_by_reading(
            &one.set.tasks,
            &said.answer,
        ));
    }
    if one.set.kind == Kind::Focus {
        return Ok(crate::focus::marked(&one.set.tasks, &said.answer));
    }
    let here = room.join(format!(
        "set-{}-{}-{}-question-{at}",
        one.set.number,
        one.step.said().replace('.', "-"),
        one.repeat
    ));
    let held = crate::marking::marked(&here, &one.set.tasks, &said.answer, MARKING_PATIENCE);
    let _swept = std::fs::remove_dir_all(&here);
    match held.why() {
        Some(why) => Err(why.to_owned()),
        None => Ok(held.or_unmarked(&one.set.tasks)),
    }
}

/// Ask one question, sending its reply to the window as it arrives.
fn asked_one(
    endpoint: &Endpoint,
    one: &Asked,
    at: usize,
    send: &Sender<Heard>,
    give_up: &dyn Fn() -> bool,
) -> Result<Outcome, String> {
    let _sent = send.send(Heard::Sent { at });
    let began = Instant::now();
    let mut thought = String::new();
    let mut answer = String::new();
    let mut pieces = 0_usize;
    let mut produced = 0_u64;
    let came = {
        let mut along = |held: Along<'_>| match held {
            Along::Counted(counted) => produced = counted,
            Along::Piece(thinking, answering) => {
                thought.push_str(thinking);
                answer.push_str(answering);
                pieces = pieces.saturating_add(1);
                if pieces.checked_rem(TOLD_EVERY) == Some(0) {
                    let written = produced.max(u64::try_from(pieces).unwrap_or(u64::MAX));
                    let _sent = send.send(reply(at, &thought, &answer, written));
                }
            }
        };
        ask(endpoint, one, &mut along, give_up).map_err(|failure| failure.to_string())
    };
    if let Ok(Outcome::Said(said)) = &came {
        let written = said.counted.unwrap_or(said.produced);
        let _sent = send.send(reply(at, &thought, &answer, written));
        let _sent = send.send(Heard::Answered {
            at,
            milliseconds: u64::try_from(began.elapsed().as_millis()).unwrap_or(u64::MAX),
        });
    }
    came
}

fn reply(at: usize, thought: &str, answer: &str, produced: u64) -> Heard {
    Heard::Reply {
        at,
        thought: tail_of(thought),
        answer: tail_of(answer),
        produced,
    }
}

/// The last of a piece of text, cut on a character rather than through one.
fn tail_of(said: &str) -> String {
    let mut from = said.len().saturating_sub(TAIL);
    while !said.is_char_boundary(from) {
        from = from.saturating_add(1);
    }
    said.get(from..).unwrap_or_default().to_owned()
}

/// Say how a question was marked: how many of its claims held, and — for a short answer —
/// the line the marker read, or for a focus run where it first slipped.
fn told_the_verdict(send: &Sender<Heard>, at: usize, one: &Asked, said: &Said, marked: &[Checked]) {
    let _sent = send.send(Heard::Marked {
        at,
        passed: marked.iter().map(|held| held.passed).sum(),
        of: marked.iter().map(|held| held.of).sum(),
        produced: said.counted.unwrap_or(said.produced),
        given: match one.set.kind {
            Kind::ShortAnswer => crate::marking::given(&said.answer, 1),
            Kind::Focus => crate::focus::first_slip(&one.set.tasks, &said.answer),
            Kind::Code | Kind::LongScript => None,
        },
        ending: said.ending,
        why: said.why.clone(),
    });
}

/// Hold the model again once and ask again whatever the engine dropped. A question it dropped
/// has no verdict: nothing is written down for it until it has been asked and answered.
fn again_where_it_failed(
    doing: &mut Doing,
    asked: &Asked,
    step: Step,
    questions: &[Set],
    came: &mut [Option<Came>],
) -> Result<(), Stopped> {
    let dropped: Vec<usize> = came
        .iter()
        .enumerate()
        .filter(|(_, held)| matches!(held, Some(Came::Dropped(_))))
        .map(|(at, _)| at)
        .collect();
    let Some(first) = dropped.first() else {
        return Ok(());
    };
    let why = match came.get(*first) {
        Some(Some(Came::Dropped(why))) => why.clone(),
        _ => String::new(),
    };
    doing
        .send
        .send(Heard::Holding(
            "the engine stopped answering — loading the model again".to_owned(),
        ))
        .map_err(|_gone| Stopped::Gone)?;
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
        Ok(port) => doing.endpoint.port = port,
        Err(Stopped::Refused(again)) => {
            return Err(Stopped::Refused(format!(
                "{why}; holding it again did not work either: {again}"
            )));
        }
        Err(other) => return Err(other),
    }
    for at in dropped {
        let Some(set) = questions.get(at) else {
            continue;
        };
        let one = Asked {
            set: set.clone(),
            ..asked.clone()
        };
        let _sent = doing.send.send(Heard::Sent { at });
        let Outcome::Said(said) = answered(doing, &one, step)? else {
            return Err(Stopped::Cut);
        };
        let marked = marked(&doing.room, at, &one, &said)
            .map_err(|why| Stopped::Refused(format!("answers were not marked: {why}")))?;
        told_the_verdict(&doing.send, at, &one, &said, &marked);
        if let Some(slot) = came.get_mut(at) {
            *slot = Some(Came::Marked(said, marked));
        }
    }
    Ok(())
}

/// What a set of answers came to as one reading: the sum of them, and how it ended.
///
/// A question that ran out of room or looped is a question answered wrongly, not a set that
/// never ran: the reading is a runaway only if every question in it was.
pub(super) fn summed(answers: Vec<Answered>) -> Answered {
    let of = answers.len();
    let mut judged = Vec::with_capacity(of);
    let mut replies = Vec::with_capacity(of);
    let (mut produced, mut counted, mut runaways) = (0_u64, 0_u64, 0_usize);
    let (mut failed, mut looped, mut filled) = (None, None, None);
    for (at, (said, marked)) in answers.into_iter().enumerate() {
        let number = at.saturating_add(1);
        let why = || {
            format!(
                "question {number}: {}",
                said.why.as_deref().unwrap_or(said.ending.label())
            )
        };
        match said.ending {
            Ending::Answered => {}
            Ending::Failed => failed = failed.or_else(|| Some(why())),
            Ending::Looped => {
                runaways = runaways.saturating_add(1);
                looped = looped.or_else(|| Some(why()));
            }
            Ending::Filled => {
                runaways = runaways.saturating_add(1);
                filled = filled.or_else(|| Some(why()));
            }
        }
        produced = produced.saturating_add(said.produced);
        counted = counted.saturating_add(said.counted.unwrap_or(said.produced));
        judged.extend(marked);
        replies.push(said.answer);
    }
    let (ending, why) = if let Some(why) = failed {
        (Ending::Failed, Some(why))
    } else if runaways == of && of > 0 {
        match looped {
            Some(why) => (Ending::Looped, Some(why)),
            None => (Ending::Filled, filled),
        }
    } else if runaways > 0 {
        (
            Ending::Answered,
            Some(format!(
                "{runaways} of {of} questions ran away and were marked wrong — {}",
                looped.or(filled).unwrap_or_default()
            )),
        )
    } else {
        (Ending::Answered, None)
    };
    (
        Said {
            answer: replies.join("\n"),
            produced,
            ending,
            why,
            counted: Some(counted),
            read_in: None,
        },
        judged,
    )
}
