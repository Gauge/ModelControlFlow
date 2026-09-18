//! `mcf downloads` — the queue of files MCF is bringing here.
//!
//! The same queue the window shows, because there is one queue and it lives in the daemon.
//! Asking for a file here and pausing it in the window is one transfer, not two.

use mcf_record::json::Value;
use mcf_serve::control::Request;

use crate::Response;

/// What can be asked of the queue.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum About {
    Pause,
    Resume,
    Cancel,
}

impl About {
    pub(crate) const fn word(self) -> &'static str {
        match self {
            Self::Pause => "paused",
            Self::Resume => "carrying on",
            Self::Cancel => "given up",
        }
    }

    fn request(self, id: u64) -> Request {
        match self {
            Self::Pause => Request::PauseTransfer { id },
            Self::Resume => Request::ResumeTransfer { id },
            Self::Cancel => Request::GiveUpTransfer { id },
        }
    }
}

pub(crate) fn listed() -> Response {
    answered(&Request::Transfers, |body| queue_lines(body, None))
}

/// Ask for a file without waiting for it. `mcf pull` waits; this queues, which is what
/// somebody with four files to fetch and a model to run wants.
pub(crate) fn queue(reference: &str, file: &str, from: Option<&str>) -> Response {
    let asked = Request::Queue {
        reference: reference.to_owned(),
        file: file.to_owned(),
        from: from.map(str::to_owned),
    };
    answered(&asked, |body| {
        let id = body.get("id").and_then(Value::as_integer).unwrap_or(0);
        let mut lines = vec![format!("{file} is queued as transfer {id}")];
        lines.extend(queue_lines(body, None));
        lines.push(String::new());
        lines.push(
            "it keeps arriving whether or not anything is watching; `mcf downloads` says how \
             far it has got"
                .to_owned(),
        );
        lines
    })
}

pub(crate) fn about(id: u64, about: About) -> Response {
    answered(&about.request(id), |body| {
        let mut lines = vec![format!("transfer {id} is {}", about.word())];
        lines.extend(queue_lines(body, Some(id)));
        lines
    })
}

pub(crate) fn forget() -> Response {
    answered(&Request::ForgetTransfers, |body| {
        let forgotten = body
            .get("forgotten")
            .and_then(Value::as_integer)
            .unwrap_or(0);
        let mut lines = vec![match forgotten {
            0 => "nothing in the queue had finished".to_owned(),
            1 => "one finished transfer dropped from the queue".to_owned(),
            many => format!("{many} finished transfers dropped from the queue"),
        }];
        lines.extend(queue_lines(body, None));
        lines
    })
}

/// Every transfer, or just the one asked about.
fn queue_lines(body: &Value, only: Option<u64>) -> Vec<String> {
    let rows = body
        .get("transfers")
        .and_then(Value::as_list)
        .map(<[Value]>::to_vec)
        .unwrap_or_default();
    let shown: Vec<&Value> = rows
        .iter()
        .filter(|row| {
            only.is_none_or(|wanted| {
                row.get("id").and_then(Value::as_integer) == Some(as_whole(wanted))
            })
        })
        .collect();
    if shown.is_empty() {
        return vec![
            "nothing is queued — `mcf downloads add <owner/name> <file>` asks for one".to_owned(),
        ];
    }
    shown.iter().map(|row| one_line(row)).collect()
}

fn as_whole(held: u64) -> i64 {
    i64::try_from(held).unwrap_or(i64::MAX)
}

fn one_line(row: &Value) -> String {
    let text = |key: &str| row.get(key).and_then(Value::as_text).unwrap_or("?");
    let count = |key: &str| {
        row.get(key)
            .and_then(Value::as_integer)
            .and_then(|held| u64::try_from(held).ok())
            .unwrap_or(0)
    };
    let id = count("id");
    let state = text("state");
    let (arrived, whole) = (count("arrived_bytes"), count("whole_bytes"));
    let far = match arrived.saturating_mul(100).checked_div(whole) {
        Some(share) => format!(" — {share}% of {whole} bytes"),
        None => String::new(),
    };
    let parts = match (count("part"), count("parts")) {
        (at, of) if of > 1 => format!(" — part {} of {of}", at.max(1)),
        _ => String::new(),
    };
    // One line of it. The whole refusal, with every scrap of context it carries, is in
    // the record: `mcf failures` reads it back. A list where one row is twelve lines is a
    // list nobody can see the shape of.
    let why = row
        .get("why")
        .filter(|held| !matches!(held, Value::Null))
        .map(crate::say::refused_because)
        .and_then(|why| why.lines().next().map(str::to_owned))
        .map_or_else(String::new, |why| format!("\n      {why}"));
    format!(
        "  {id}  {state:<9} {}  {}{far}{parts}{why}",
        text("file"),
        text("reference")
    )
}

fn answered(request: &Request, render: impl Fn(&Value) -> Vec<String>) -> Response {
    match crate::serve::asked(request) {
        Ok(answer) if answer.served => Response {
            text: render(&answer.body).join("\n"),
            served: true,
        },
        Ok(answer) => Response {
            text: format!(
                "mcf: refused\n  {}",
                crate::say::refused_because(&answer.body)
            ),
            served: false,
        },
        Err(why) => Response {
            text: format!("mcf: {why}"),
            served: false,
        },
    }
}

#[cfg(test)]
mod tests;
