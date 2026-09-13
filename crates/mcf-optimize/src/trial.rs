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
const KEPT_OF_THE_REPLY: usize = 4096;

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
    pub thinking: Option<bool>,
    pub effort: Option<String>,
    pub ceiling: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Said {
    pub answer: String,
    pub produced: u64,
    pub ending: Ending,
    pub why: Option<String>,
}

#[must_use]
pub fn body(asked: &Asked) -> Value {
    let mut switches: Vec<(&str, Value)> = Vec::new();
    if let Some(on) = asked.thinking {
        switches.push(("enable_thinking", Value::Bool(on)));
    }
    let dialled = asked
        .dial
        .template_kwarg()
        .map(|kwarg| (kwarg, asked.dial.said(asked.step)));
    match (&dialled, &asked.effort) {
        (Some((kwarg, said)), _) => switches.push((kwarg, Value::text(said.clone()))),
        (None, Some(effort)) => switches.push(("reasoning_effort", Value::text(effort.clone()))),
        (None, None) => {}
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
        fields.push((field, sent(asked.step)));
    }
    if !switches.is_empty() {
        fields.push(("chat_template_kwargs", Value::map(switches)));
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

pub fn ask(endpoint: &Endpoint, asked: &Asked) -> Result<Said, Failure> {
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
        "POST /v1/chat/completions HTTP/1.1\r\nHost: localhost\r\nContent-Type: \
         application/json\r\n{bearer}Content-Length: {}\r\nConnection: close\r\n\r\n{payload}",
        payload.len()
    )
    .and_then(|()| connection.flush())
    .map_err(|error| unreachable(endpoint.port, &error.to_string()))?;

    let started = Instant::now();
    let mut held = [0_u8; READ_AT_A_TIME];
    let mut pending = String::new();
    let mut whole = String::new();
    let mut answer = String::new();
    let mut produced: u64 = 0;
    let mut ending = Ending::Answered;
    let mut why = None;
    'reading: while let Ok(read) = connection.read(&mut held) {
        if read == 0 {
            break;
        }
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
            let Some(piece) = spoken(&value) else {
                continue;
            };
            produced = produced.saturating_add(1);
            answer.push_str(&piece);
            let time_to_look = produced
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
    if ending == Ending::Answered && produced >= u64::from(asked.ceiling).saturating_sub(4) {
        ending = Ending::Filled;
    }
    if produced == 0 {
        ending = Ending::Failed;
        why = Some(what_came_back(&whole));
    }
    let _elapsed = started.elapsed();
    Ok(Said {
        answer,
        produced,
        ending,
        why,
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

fn sent(step: Step) -> Value {
    if let Some(exact) = step.thousandths() {
        return Value::exact_thousandths(exact);
    }
    Value::Integer(i64::from(step.whole().unwrap_or(0)))
}

fn spoken(value: &Value) -> Option<String> {
    let Value::List(choices) = value.get("choices")? else {
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
        produced: said.produced,
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
