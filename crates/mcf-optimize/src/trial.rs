use std::io::{Read as _, Write as _};
use std::net::TcpStream;
use std::time::{Duration, Instant};

use mcf_core::failure::{Attribution, Category, Disposition, Failure, Subsystem};
use mcf_record::json::{Value, parse};

use crate::corpus::Set;
use crate::dial::{Dial, Step};
use crate::looping::looping;
use crate::reading::{Ending, Reading};

const LOOPBACK: &str = "127.0.0.1";
const READ_AT_A_TIME: usize = 8192;

/// How long a read waits before coming back empty-handed so the stop flag can be looked
/// at. It is not how long a trial may take — `Endpoint::patience` is still the whole
/// trial's deadline, timed here rather than left to the socket. A socket timeout of two
/// hours meant a sweep asked to stop could sit in one `read` for two hours.
const STOP_TICK: Duration = Duration::from_millis(200);
const CHECKED_EVERY: usize = 200;

/// How often a trial says how far it has got. Often enough to look alive, seldom enough
/// that saying so is not the work.
pub(crate) const TOLD_EVERY: u64 = 32;
const KEPT_OF_THE_REPLY: usize = 4096;

/// What a speed trial starts from. What comes back is never read: only how long it took.
/// The engine is told to ignore the model's own ending, so this only has to be something
/// to continue from.
pub const TO_BE_TIMED: &str = "1\n2\n3\n";

pub const TOKENS_TIMED: u32 = 1024;

/// What a trial that times reading rather than writing sends: a prompt of exactly this many
/// tokens, with one token asked for back. A micro-batch is how many prompt tokens go through
/// the device in one pass, so reading a prompt is the work it changes and writing an answer
/// is not.
///
/// It has to be at least as long as the largest micro-batch a search will try. A pass takes
/// as much of the prompt as it can hold, so a micro-batch above the length of the prompt is
/// the same one pass as a micro-batch equal to it, and a search that cannot tell those two
/// apart climbs to the top of its span and calls that the answer.
pub const TOKENS_PREFILLED: u32 = 32_768;

/// The most a search will hand to thinking.
///
/// A marked trial is not given a number of tokens to stop at: it writes until it is done,
/// and the only room it runs out of is the window the model is held at. This is the top of
/// the thinking budget's own span, not a limit on the answer. Measured on a set of eight
/// tasks: eleven thousand tokens of thinking and three thousand of answer — a question
/// asked on its own thinks far less than that, so eight thousand is room to spare.
pub const TOKENS_THOUGHT: u32 = 8_192;

/// Room left at the end of the window so a prompt this long still has somewhere to answer.
const KEPT_FOR_AN_ANSWER: u64 = 512;

/// The prompt of a reading trial is given as token numbers rather than as prose, so its
/// length is exact on every model rather than whatever that model's tokenizer makes of a
/// piece of text. The numbers stay low enough to be inside the smallest vocabulary, and what
/// they mean does not matter: reading a prompt costs the same whatever it says.
const HIGHEST_TOKEN_NUMBER: u32 = 4000;

/// A prompt long enough to time, and short enough to fit the window the model is held at.
#[must_use]
pub fn prompt_within(context: u64) -> u32 {
    let room = u32::try_from(context.saturating_sub(KEPT_FOR_AN_ANSWER)).unwrap_or(u32::MAX);
    if room < TOKENS_PREFILLED {
        return room;
    }
    TOKENS_PREFILLED
}

#[must_use]
pub fn to_be_read(tokens: u32) -> Value {
    Value::List(
        (0..tokens)
            .map(|at| {
                let held = at
                    .wrapping_mul(7919)
                    .checked_rem(HIGHEST_TOKEN_NUMBER)
                    .unwrap_or(0);
                Value::Integer(i64::from(held.saturating_add(100)))
            })
            .collect(),
    )
}

/// How many times a value is timed. One: a sweep is for telling values apart, not for
/// settling the last percent of any one of them, and the gap a sweep is looking for is
/// wider than the spread between takes. Ask for a take again by hand when one reading
/// looks wrong.
pub const TIMES_TIMED: u8 = 1;

#[derive(Debug, Clone)]
pub struct Endpoint {
    pub port: u16,
    pub key: Option<String>,
    pub patience: Duration,
}

impl Default for Endpoint {
    fn default() -> Self {
        Self {
            port: 8000,
            key: None,
            patience: Duration::from_secs(7200),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Asked {
    pub set: Set,
    pub dial: Dial,
    pub step: Step,
    pub repeat: u8,
    /// How many tokens this trial asks for. A timed trial asks for exactly this many, since
    /// that is the work being timed. A marked one asks for none: it writes until it is done.
    pub ceiling: Option<u32>,
    pub named: Vec<String>,
    pub timing: bool,
    /// Whether this model's template reads `enable_thinking`. It decides how thinking is
    /// turned off, and the two ways are not interchangeable.
    pub switch: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Said {
    pub answer: String,
    pub produced: u64,
    pub ending: Ending,
    pub why: Option<String>,
    /// What the engine itself said it generated. The engine counts every token; a reader of
    /// the stream counts only the ones that carried text, and the two differ by a few.
    pub counted: Option<u64>,
    /// How many prompt tokens the engine said it read. A trial that times the micro-batch is
    /// timing exactly this, and nothing it wrote afterwards.
    pub read_in: Option<u64>,
}

/// Where a trial is sent. A timed run goes to the plain completion endpoint: it wants
/// tokens rather than an answer, and that endpoint applies no chat template and parses no
/// reply, so a model told to ignore its own ending cannot walk off the end of a parser.
#[must_use]
pub const fn the_way_in(asked: &Asked) -> &'static str {
    if asked.timing {
        "/completion"
    } else {
        "/v1/chat/completions"
    }
}

#[must_use]
pub fn body(asked: &Asked) -> Value {
    if asked.timing {
        if asked.dial.times_reading_the_prompt() {
            return Value::map([
                (
                    "prompt",
                    to_be_read(asked.ceiling.unwrap_or(TOKENS_PREFILLED)),
                ),
                ("n_predict", Value::Integer(1)),
                ("ignore_eos", Value::Bool(true)),
                ("stream", Value::Bool(true)),
                ("cache_prompt", Value::Bool(false)),
                // Nothing is written while a prompt is read, so without this a reading trial
                // would show a still clock for as long as it takes. With it the engine says
                // how far through the prompt it is as it goes.
                ("return_progress", Value::Bool(true)),
            ]);
        }
        return Value::map([
            ("prompt", Value::text(TO_BE_TIMED.to_owned())),
            (
                "n_predict",
                Value::Integer(i64::from(asked.ceiling.unwrap_or(TOKENS_TIMED))),
            ),
            ("ignore_eos", Value::Bool(true)),
            ("stream", Value::Bool(true)),
            ("cache_prompt", Value::Bool(false)),
        ]);
    }
    let mut fields: Vec<(&str, Value)> = vec![
        (
            "messages",
            Value::List(vec![Value::map([
                ("role", Value::text("user")),
                ("content", Value::text(asked.set.asked())),
            ])]),
        ),
        ("stream", Value::Bool(true)),
        ("cache_prompt", Value::Bool(false)),
    ];
    // Asked for no number of tokens, the engine writes until the model stops or the window
    // is full. A limit here was a place a right answer could be cut off half written.
    if let Some(ceiling) = asked.ceiling {
        fields.push(("max_tokens", Value::Integer(i64::from(ceiling))));
    }

    if let Some(field) = asked.dial.field() {
        if asked.dial.is_named_by_the_model() {
            let said = asked.dial.said_among(asked.step, &asked.named);
            if said == Dial::OFF {
                fields.push(turning_thinking_off(asked.switch));
            } else {
                fields.push((field, Value::text(said)));
            }
        } else {
            fields.push((field, sent(asked.step)));
        }
    }
    Value::map(fields)
}

/// How to ask for no thinking at all. Two ways, and which one works depends on the model.
///
/// A template that reads `enable_thinking` is told false, and stops. Measured on a model
/// whose template reads it: nothing, against four thousand three hundred characters of
/// thinking when asked the other way.
///
/// A template that does not read it — gpt-oss does not — is stopped by the engine instead,
/// which watches for the tag the thinking section opens with and closes it. That costs a
/// token of thinking rather than none, because a budget of nothing is not a budget of
/// nothing: the engine reads it as no budget at all and lets the model think until it is
/// finished. Measured the same way: a budget of nought left the thinking running, and a
/// budget above nought cut it where it said it would.
///
/// Asking for a level of "none" does neither. The engine takes the word away and the
/// template falls back to whatever it does by default.
fn turning_thinking_off(switch: bool) -> (&'static str, Value) {
    if switch {
        return (
            "chat_template_kwargs",
            Value::map([("enable_thinking", Value::Bool(false))]),
        );
    }
    ("reasoning_budget_tokens", Value::Integer(1))
}

#[must_use]
pub fn is_ready(port: u16) -> bool {
    let Ok(mut connection) = TcpStream::connect((LOOPBACK, port)) else {
        return false;
    };
    if connection
        .set_read_timeout(Some(Duration::from_secs(5)))
        .is_err()
    {
        return false;
    }
    if write!(
        connection,
        "GET /health HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n"
    )
    .and_then(|()| connection.flush())
    .is_err()
    {
        return false;
    }
    let mut said = String::new();
    let mut held = [0_u8; 1024];
    while let Ok(read) = connection.read(&mut held) {
        if read == 0 {
            break;
        }
        said.push_str(&String::from_utf8_lossy(
            held.get(..read).unwrap_or_default(),
        ));
        if said.len() > 4096 {
            break;
        }
    }
    said.contains("\"status\":\"ok\"")
}

pub fn ready_within(
    port: u16,
    patience: Duration,
    mut along: impl FnMut(u64),
    stop: &dyn Fn() -> bool,
) -> bool {
    let began = Instant::now();
    let mut told = 0;
    while began.elapsed() < patience {
        // Waiting out a re-hold is the other place a stop used to go unheard: holding a
        // large model again can take minutes.
        if stop() {
            return false;
        }
        if is_ready(port) {
            return true;
        }
        let seconds = began.elapsed().as_secs();
        if seconds != told {
            told = seconds;
            along(seconds);
        }
        std::thread::sleep(Duration::from_millis(500));
    }
    is_ready(port)
}

fn sent_to(endpoint: &Endpoint, asked: &Asked) -> Result<TcpStream, Failure> {
    let payload = body(asked).to_string();
    let mut connection = TcpStream::connect((LOOPBACK, endpoint.port))
        .map_err(|error| unreachable(endpoint.port, &error.to_string()))?;
    connection
        .set_read_timeout(Some(STOP_TICK))
        .map_err(|error| unreachable(endpoint.port, &error.to_string()))?;
    let bearer = endpoint.key.as_ref().map_or_else(String::new, |key| {
        format!("Authorization: Bearer {key}\r\n")
    });
    write!(
        connection,
        "POST {} HTTP/1.1\r\nHost: localhost\r\nContent-Type: \
         application/json\r\n{bearer}Content-Length: {}\r\nConnection: close\r\n\r\n{payload}",
        the_way_in(asked),
        payload.len()
    )
    .and_then(|()| connection.flush())
    .map_err(|error| unreachable(endpoint.port, &error.to_string()))?;
    Ok(connection)
}

/// What a trial says as it goes: how much it has written or read, every so often, and each
/// piece of text as it arrives — what the model thought, and what it answered.
#[derive(Debug, Clone, Copy)]
pub enum Along<'piece> {
    Counted(u64),
    Piece(&'piece str, &'piece str),
}

/// What a trial came to.
///
/// `Cut` is not a failure and not a reading: the sweep was asked to stop while this trial
/// was still running, so there is nothing to write down. It is its own case precisely so
/// that no caller can mistake a half-finished trial for a measurement.
#[derive(Debug)]
pub enum Outcome {
    Said(Said),
    Cut,
}

/// What one turn of the read loop got.
enum Got {
    Bytes(usize),
    /// The tick ran out with nothing on the wire, which is what a model thinking looks
    /// like. Read again.
    Again,
    /// Asked to stop, or out of patience.
    Done(Option<String>),
}

/// One read, with the stop flag looked at whenever the wire goes quiet.
///
/// The socket's own timeout is a short tick rather than the trial's whole patience, so a
/// sweep asked to stop is never more than that tick away from noticing.
fn read_once(
    connection: &mut TcpStream,
    into: &mut [u8],
    since: Instant,
    patience: Duration,
    give_up: &dyn Fn() -> bool,
) -> Got {
    if give_up() {
        return Got::Done(None);
    }
    match connection.read(into) {
        Ok(0) => Got::Done(None),
        Ok(read) => Got::Bytes(read),
        // A signal arriving in this process interrupts a blocking read. The window samples
        // the machine once a second while a sweep runs, and sampling starts a child, so
        // this happens often. It is not the end of anything: read again.
        Err(error) if error.kind() == std::io::ErrorKind::Interrupted => Got::Again,
        Err(error)
            if matches!(
                error.kind(),
                std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
            ) =>
        {
            if give_up() {
                return Got::Done(None);
            }
            if since.elapsed() >= patience {
                return Got::Done(Some(format!(
                    "the engine said nothing for {} seconds",
                    patience.as_secs()
                )));
            }
            Got::Again
        }
        Err(error) => Got::Done(Some(error.to_string())),
    }
}

/// Everything one read loop gathered, before it is turned into a reading.
struct Gathered {
    answer: String,
    produced: u64,
    thinking: usize,
    counted: Option<u64>,
    read_in: Option<u64>,
    whole: String,
    ending: Ending,
    why: Option<String>,
    cut_short: Option<String>,
    /// Whether the engine said it stopped because there was no more room, rather than
    /// because the model was done.
    ran_out: bool,
}

/// What the trial came to, from what the loop gathered: which figure is the one being
/// timed, and how it ended.
fn said_of(asked: &Asked, held: Gathered) -> Said {
    let counted_now = if asked.dial.times_reading_the_prompt() && asked.timing {
        held.read_in.unwrap_or(0)
    } else {
        held.counted.unwrap_or(held.produced)
    };
    let (ending, why) = came_to(
        asked,
        Ended {
            counted: counted_now,
            produced: held.produced,
            thinking: held.thinking,
            answered: !held.answer.trim().is_empty(),
            ran_out: held.ran_out,
        },
        (held.ending, held.why, held.cut_short),
        &held.whole,
    );
    Said {
        answer: held.answer,
        produced: held.produced,
        ending,
        why,
        counted: held.counted,
        read_in: held.read_in,
    }
}

pub fn ask(
    endpoint: &Endpoint,
    asked: &Asked,
    along: &mut dyn FnMut(Along<'_>),
    give_up: &dyn Fn() -> bool,
) -> Result<Outcome, Failure> {
    let mut connection = sent_to(endpoint, asked)?;
    let started = Instant::now();
    let mut held = [0_u8; READ_AT_A_TIME];
    let mut pending = String::new();
    let mut whole = String::new();
    let mut answer = String::new();
    let mut produced: u64 = 0;
    let mut ending = Ending::Answered;
    let mut why = None;
    let mut counted: Option<u64> = None;
    let mut thinking: usize = 0;
    let mut read_in: Option<u64> = None;
    let mut cut_short = None;
    let mut ran_out = false;
    'reading: loop {
        let read = match read_once(
            &mut connection,
            &mut held,
            started,
            endpoint.patience,
            give_up,
        ) {
            Got::Bytes(read) => read,
            Got::Again => continue,
            Got::Done(why) => {
                // Asked to stop part way through: close the connection so the engine is
                // not left producing tokens nobody will read, and say nothing was measured.
                if why.is_none() && give_up() {
                    let _closed = connection.shutdown(std::net::Shutdown::Both);
                    return Ok(Outcome::Cut);
                }
                cut_short = why;
                break;
            }
        };
        let arrived = String::from_utf8_lossy(held.get(..read).unwrap_or_default()).into_owned();
        if whole.len() < KEPT_OF_THE_REPLY {
            whole.push_str(&arrived);
        }
        pending.push_str(&arrived);
        while let Some(line) = next_line(&mut pending) {
            let Some(data) = line.strip_prefix("data:") else {
                continue;
            };
            let data = data.trim();
            if data == "[DONE]" {
                break 'reading;
            }
            let Ok(value) = parse(data) else { continue };
            counted = what_it_wrote(&value).or(counted);
            ran_out = ran_out || stopped_for_room(&value);
            if let Some(read) = what_it_read(&value) {
                read_in = Some(read);
                if asked.dial.times_reading_the_prompt() {
                    along(Along::Counted(read));
                }
            }
            let Some(piece) = spoken(&value) else {
                continue;
            };
            produced = produced.saturating_add(1);
            along(Along::Piece(&piece.thought, &piece.answer));
            answer.push_str(&piece.answer);
            thinking = thinking.saturating_add(piece.thought.len());
            if produced
                .checked_rem(TOLD_EVERY)
                .is_some_and(|left| left == 0)
            {
                along(Along::Counted(counted.unwrap_or(produced)));
            }
            let time_to_look = !asked.timing
                && produced
                    .checked_rem(u64::try_from(CHECKED_EVERY).unwrap_or(1))
                    .is_some_and(|left| left == 0);
            if let Some(found) = (time_to_look && answer.len() > 1500)
                .then(|| looping(&answer))
                .flatten()
            {
                ending = Ending::Looped;
                why = Some(found);
                break 'reading;
            }
        }
    }
    Ok(Outcome::Said(said_of(
        asked,
        Gathered {
            answer,
            produced,
            thinking,
            counted,
            read_in,
            whole,
            ending,
            why,
            cut_short,
            ran_out,
        },
    )))
}

#[must_use]
pub fn what_came_back(whole: &str) -> String {
    if whole.trim().is_empty() {
        return "the endpoint accepted the request and then said nothing at all".to_owned();
    }
    let status = whole.lines().next().unwrap_or("").trim().to_owned();
    let (head, body) = match whole.find("\r\n\r\n") {
        Some(at) => (
            status.clone(),
            whole.get(at.saturating_add(4)..).unwrap_or("").trim(),
        ),
        None => (status.clone(), whole.trim()),
    };
    if !head.starts_with("HTTP/") {
        return format!(
            "the endpoint answered something that is not HTTP: {}",
            shortened(whole.trim())
        );
    }
    let ok = head.contains(" 200");
    if !ok {
        return format!("the endpoint refused it — {head}: {}", shortened(body));
    }
    if body.contains("\"finish_reason\":\"length\"") {
        return "the model stopped on its token limit before writing anything — at a thinking \
                budget this small there is no room left to answer in"
            .to_owned();
    }
    format!(
        "the endpoint answered {head} and sent no content: {}",
        shortened(body)
    )
}

fn shortened(said: &str) -> String {
    let kept: String = said.chars().take(300).collect();
    let one_line = kept.replace(['\n', '\r'], " ");
    one_line.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn whole_in(value: &Value, key: &str) -> Option<u64> {
    value
        .get(key)
        .and_then(Value::as_integer)
        .and_then(|held| u64::try_from(held).ok())
}

/// How many tokens the engine says it has written so far. A chunk carries the running count
/// and the last one carries the engine's own final figure, which is the one to keep.
fn what_it_wrote(value: &Value) -> Option<u64> {
    value
        .get("timings")
        .and_then(|timings| whole_in(timings, "predicted_n"))
        .or_else(|| whole_in(value, "tokens_predicted"))
}

/// How many tokens of prompt the engine says it read. The chunk carrying the timings says
/// so at the end; while it is still reading, the progress it reports says how far it has got.
fn what_it_read(value: &Value) -> Option<u64> {
    value
        .get("timings")
        .and_then(|timings| whole_in(timings, "prompt_n"))
        .or_else(|| {
            value
                .get("prompt_progress")
                .and_then(|held| whole_in(held, "processed"))
        })
}

/// A timed run that fell well short of what it asked for is not the same measurement.
/// What a trial came to, counted up.
#[derive(Clone, Copy)]
struct Ended {
    /// What the engine says it generated, thinking included.
    counted: u64,
    /// How many chunks carried anything.
    produced: u64,
    /// How much of what came back was the model thinking rather than answering.
    thinking: usize,
    /// Whether there is an answer here at all, as opposed to a budget spent getting ready
    /// to write one.
    answered: bool,
    /// Whether the engine stopped it for want of room.
    ran_out: bool,
}

/// What a trial came to, whether it ran out of connection or ran to its end.
fn came_to(
    asked: &Asked,
    held: Ended,
    (ending, why, cut_short): (Ending, Option<String>, Option<String>),
    whole: &str,
) -> (Ending, Option<String>) {
    if let Some(broke) = cut_short {
        return (
            Ending::Failed,
            Some(format!(
                "the reply stopped arriving after {} token(s): {broke}",
                held.counted
            )),
        );
    }
    how_it_ended(asked, held, ending, why, whole)
}

fn how_it_ended(
    asked: &Asked,
    held: Ended,
    ending: Ending,
    why: Option<String>,
    whole: &str,
) -> (Ending, Option<String>) {
    let Ended {
        counted,
        produced,
        thinking,
        answered,
        ..
    } = held;
    if produced == 0 {
        return (Ending::Failed, Some(what_came_back(whole)));
    }
    if asked.timing {
        let asked_for = asked.ceiling.unwrap_or(0);
        if counted < enough_of(asked_for) {
            let work = if asked.dial.times_reading_the_prompt() {
                "the engine read only"
            } else {
                "the model stopped after"
            };
            return (
                Ending::Failed,
                Some(format!(
                    "{work} {counted} tokens of the {asked_for} it was asked for, so this rate \
                     is over a shorter run than the others and is not theirs to compare with"
                )),
            );
        }
        return (ending, why);
    }
    // Where the room ran out: the tokens it was asked for, if it was asked for any, and
    // otherwise the window the model is held at, which the engine says it stopped on.
    let ran_out = held.ran_out
        || asked
            .ceiling
            .is_some_and(|ceiling| produced >= u64::from(ceiling).saturating_sub(4));
    let room = asked.ceiling.map_or_else(
        || "the window it is held at".to_owned(),
        |ceiling| format!("its {ceiling} tokens"),
    );
    // A model that thought until the room ran out has not answered wrongly; it has not
    // answered. Marking that as wrong says the value was tried and found wanting, when
    // what happened is that the trial never reached the part being marked.
    if !answered {
        if !ran_out {
            return (
                ending,
                Some(format!(
                    "the model finished without writing an answer — {thinking} characters of \
                     thinking and nothing after them"
                )),
            );
        }
        return (
            Ending::Filled,
            Some(format!(
                "the model was still thinking when {room} ran out — {thinking} characters of \
                 it and not a word of answer. There is nothing here to mark: hold it with a \
                 wider window, or give it less thinking to do"
            )),
        );
    }
    if ending == Ending::Answered && ran_out {
        return (
            Ending::Filled,
            why.or_else(|| {
                Some(format!(
                    "the answer was still being written when {room} ran out"
                ))
            }),
        );
    }
    (ending, why)
}

fn enough_of(ceiling: u32) -> u64 {
    u64::from(ceiling)
        .saturating_mul(9)
        .checked_div(10)
        .unwrap_or(0)
}

fn sent(step: Step) -> Value {
    if let Some(exact) = step.thousandths() {
        return Value::exact_thousandths(exact);
    }
    Value::Integer(i64::from(step.whole().unwrap_or(0)))
}

/// What arrived in one chunk: what the model answered, and what it was thinking. They are
/// kept apart because only one of them is the answer. A model that thinks in code writes
/// fenced blocks while it is working out what to write, and marking those instead of the
/// solutions marks a draft the model itself went on to throw away.
struct Piece {
    answer: String,
    thought: String,
}

impl Piece {
    fn is_empty(&self) -> bool {
        self.answer.is_empty() && self.thought.is_empty()
    }
}

/// The next whole line of what has arrived, taken off the front of it, trimmed. Nothing
/// while the last line is still arriving.
fn next_line(pending: &mut String) -> Option<String> {
    let at = pending.find('\n')?;
    let line = pending.get(..at).unwrap_or_default().trim().to_owned();
    pending.replace_range(..=at, "");
    Some(line)
}

/// Whether this chunk says the engine stopped because there was no more room — the window
/// full, or the tokens asked for spent — rather than because the model had finished.
fn stopped_for_room(value: &Value) -> bool {
    let from_a_choice = value
        .get("choices")
        .and_then(Value::as_list)
        .and_then(|choices| choices.first())
        .and_then(|choice| choice.get("finish_reason"))
        .and_then(Value::as_text)
        .is_some_and(|reason| reason == "length");
    let from_the_engine = matches!(value.get("stopped_limit"), Some(Value::Bool(true)));
    from_a_choice || from_the_engine
}

fn spoken(value: &Value) -> Option<Piece> {
    let Some(choices) = value.get("choices") else {
        let said = value.get("content").and_then(Value::as_text)?;
        return (!said.is_empty()).then(|| Piece {
            answer: said.to_owned(),
            thought: String::new(),
        });
    };
    let Value::List(choices) = choices else {
        return None;
    };
    let delta = choices.first()?.get("delta")?;
    let held = Piece {
        answer: delta
            .get("content")
            .and_then(Value::as_text)
            .unwrap_or("")
            .to_owned(),
        thought: delta
            .get("reasoning_content")
            .and_then(Value::as_text)
            .unwrap_or("")
            .to_owned(),
    };
    (!held.is_empty()).then_some(held)
}

/// The fenced code blocks in an answer, in order.
///
/// A fence is a line, as it is in Markdown: a block opens on a line that starts with three
/// backticks and closes on a line that is nothing but three. Three backticks inside a line
/// are code. A program that renders Markdown has `'```'` in its own source, and reading that
/// as the end of the block cut every correct answer to such a task in half.
#[must_use]
pub fn blocks(said: &str) -> Vec<String> {
    let mut held = Vec::new();
    let mut open: Option<Vec<&str>> = None;
    for line in said.lines() {
        match open.as_mut() {
            None if line.trim_start().starts_with("```") => open = Some(Vec::new()),
            None => {}
            Some(body) if line.trim() == "```" => {
                held.push(body.join("\n").trim().to_owned());
                open = None;
            }
            Some(body) => body.push(line),
        }
    }
    held
}

#[must_use]
pub fn reading_of(
    asked: &Asked,
    said: &Said,
    milliseconds: u64,
    passed: &[crate::marking::Checked],
) -> Reading {
    Reading {
        dial: asked.dial,
        step: asked.step,
        set: asked.set.number,
        repeat: asked.repeat,
        // The claims a check makes, not the tasks it covers. A solution most of the way there
        // and one that did nothing both score nought out of eight when the task is the unit,
        // and a set of eight is worth eighteen points either way as a reading — far too coarse
        // to tell two settings apart. The same run, counted by claim, is worth about six.
        passed: passed.iter().map(|held| held.passed).sum(),
        of: passed.iter().map(|held| held.of).sum(),
        produced: if asked.timing && asked.dial.times_reading_the_prompt() {
            said.read_in.unwrap_or(0)
        } else {
            said.counted.unwrap_or(said.produced)
        },
        milliseconds,
        ending: said.ending,
        why: said.why.clone(),
        per_task: passed
            .iter()
            .map(|held| (held.name.clone(), held.whole()))
            .collect(),
    }
}

fn unreachable(port: u16, why: &str) -> Failure {
    Failure::new(
        Category::ConfigInvalid,
        Attribution::User,
        Disposition::Refused,
        Subsystem::new("mcf-optimize::trial"),
        format!("the held model could not be reached on port {port}: {why}"),
    )
    .with_context("port", port.to_string())
}

#[cfg(test)]
mod tests;
