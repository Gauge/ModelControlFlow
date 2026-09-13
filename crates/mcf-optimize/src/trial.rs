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
pub const TOKENS_PREFILLED: u32 = 8192;

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

/// A timed run is short enough that one of them is mostly noise, so every value is timed
/// this many times and the readings are taken together.
pub const TIMES_TIMED: u8 = 5;

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
    pub ceiling: u32,
    pub named: Vec<String>,
    pub timing: bool,
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
                ("prompt", to_be_read(asked.ceiling)),
                ("n_predict", Value::Integer(1)),
                ("ignore_eos", Value::Bool(true)),
                ("stream", Value::Bool(true)),
                ("cache_prompt", Value::Bool(false)),
            ]);
        }
        return Value::map([
            ("prompt", Value::text(TO_BE_TIMED.to_owned())),
            ("n_predict", Value::Integer(i64::from(asked.ceiling))),
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
        ("max_tokens", Value::Integer(i64::from(asked.ceiling))),
        ("stream", Value::Bool(true)),
        ("cache_prompt", Value::Bool(false)),
    ];

    if let Some(field) = asked.dial.field() {
        if asked.dial.is_named_by_the_model() {
            let said = asked.dial.said_among(asked.step, &asked.named);
            fields.push((field, Value::text(said)));
        } else {
            fields.push((field, sent(asked.step)));
        }
    }
    Value::map(fields)
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

pub fn ready_within(port: u16, patience: Duration, mut along: impl FnMut(u64)) -> bool {
    let began = Instant::now();
    let mut told = 0;
    while began.elapsed() < patience {
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
        .set_read_timeout(Some(endpoint.patience))
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

pub fn ask(
    endpoint: &Endpoint,
    asked: &Asked,
    along: &mut dyn FnMut(u64),
) -> Result<Said, Failure> {
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
    let mut read_in: Option<u64> = None;
    let mut cut_short = None;
    'reading: loop {
        let read = match connection.read(&mut held) {
            Ok(0) => break,
            Ok(read) => read,
            // A signal arriving in this process interrupts a blocking read. The window
            // samples the machine once a second while a sweep runs, and sampling starts a
            // child, so this happens often. It is not the end of anything: read again.
            Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(error) => {
                cut_short = Some(error.to_string());
                break;
            }
        };
        let arrived = String::from_utf8_lossy(held.get(..read).unwrap_or_default()).into_owned();
        if whole.len() < KEPT_OF_THE_REPLY {
            whole.push_str(&arrived);
        }
        pending.push_str(&arrived);
        while let Some(at) = pending.find('\n') {
            let line = pending.get(..at).unwrap_or_default().trim().to_owned();
            pending = pending
                .get(at.saturating_add(1)..)
                .unwrap_or_default()
                .to_owned();
            let Some(data) = line.strip_prefix("data:") else {
                continue;
            };
            let data = data.trim();
            if data == "[DONE]" {
                break 'reading;
            }
            let Ok(value) = parse(data) else { continue };
            counted = what_it_wrote(&value).or(counted);
            read_in = what_it_read(&value).or(read_in);
            let Some(piece) = spoken(&value) else {
                continue;
            };
            produced = produced.saturating_add(1);
            answer.push_str(&piece);
            if produced
                .checked_rem(TOLD_EVERY)
                .is_some_and(|left| left == 0)
            {
                along(counted.unwrap_or(produced));
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
    let counted_now = if asked.dial.times_reading_the_prompt() && asked.timing {
        read_in.unwrap_or(0)
    } else {
        counted.unwrap_or(produced)
    };
    if let Some(broke) = cut_short {
        ending = Ending::Failed;
        why = Some(format!(
            "the reply stopped arriving after {counted_now} token(s): {broke}"
        ));
    } else {
        (ending, why) = how_it_ended(asked, counted_now, produced, ending, why, &whole);
    }
    let _elapsed = started.elapsed();
    Ok(Said {
        answer,
        produced,
        ending,
        why,
        counted,
        read_in,
    })
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

/// How many tokens of prompt the engine says it read. Only the chunk carrying the timings
/// says, and it says it once.
fn what_it_read(value: &Value) -> Option<u64> {
    value
        .get("timings")
        .and_then(|timings| whole_in(timings, "prompt_n"))
}

/// A timed run that fell well short of what it asked for is not the same measurement.
fn how_it_ended(
    asked: &Asked,
    counted: u64,
    produced: u64,
    ending: Ending,
    why: Option<String>,
    whole: &str,
) -> (Ending, Option<String>) {
    if produced == 0 {
        return (Ending::Failed, Some(what_came_back(whole)));
    }
    if asked.timing {
        if counted < enough_of(asked.ceiling) {
            let work = if asked.dial.times_reading_the_prompt() {
                "the engine read only"
            } else {
                "the model stopped after"
            };
            return (
                Ending::Failed,
                Some(format!(
                    "{work} {counted} tokens of the {} it was asked for, so this rate is over \
                     a shorter run than the others and is not theirs to compare with",
                    asked.ceiling
                )),
            );
        }
        return (ending, why);
    }
    if ending == Ending::Answered && produced >= u64::from(asked.ceiling).saturating_sub(4) {
        return (Ending::Filled, why);
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

fn spoken(value: &Value) -> Option<String> {
    let Some(choices) = value.get("choices") else {
        let said = value.get("content").and_then(Value::as_text)?;
        return (!said.is_empty()).then(|| said.to_owned());
    };
    let Value::List(choices) = choices else {
        return None;
    };
    let delta = choices.first()?.get("delta")?;
    let content = delta.get("content").and_then(Value::as_text).unwrap_or("");
    let thought = delta
        .get("reasoning_content")
        .and_then(Value::as_text)
        .unwrap_or("");
    let said = format!("{thought}{content}");
    (!said.is_empty()).then_some(said)
}

#[must_use]
pub fn blocks(said: &str) -> Vec<String> {
    let mut held = Vec::new();
    let mut rest = said;
    while let Some(open) = rest.find("```") {
        let after = rest.get(open.saturating_add(3)..).unwrap_or_default();
        let body = after
            .find('\n')
            .and_then(|at| after.get(at.saturating_add(1)..))
            .unwrap_or(after);
        let Some(close) = body.find("```") else { break };
        held.push(body.get(..close).unwrap_or_default().trim().to_owned());
        rest = body.get(close.saturating_add(3)..).unwrap_or_default();
    }
    held
}

#[must_use]
pub fn reading_of(
    asked: &Asked,
    said: &Said,
    milliseconds: u64,
    passed: &[(String, bool)],
) -> Reading {
    Reading {
        dial: asked.dial,
        step: asked.step,
        set: asked.set.number,
        repeat: asked.repeat,
        passed: u32::try_from(passed.iter().filter(|(_, ok)| *ok).count()).unwrap_or(u32::MAX),
        of: u32::try_from(asked.set.tasks.len()).unwrap_or(u32::MAX),
        produced: if asked.timing && asked.dial.times_reading_the_prompt() {
            said.read_in.unwrap_or(0)
        } else {
            said.counted.unwrap_or(said.produced)
        },
        milliseconds,
        ending: said.ending,
        per_task: passed.to_vec(),
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
