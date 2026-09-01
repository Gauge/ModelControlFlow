//! `mcf settings`, `mcf host`, `mcf hosted` and `mcf unhost`: holding a model
//! where other programs can reach it.
//!
//! **The API is the engine's and MCF says so.** MCF does not implement an
//! inference API. It provisions an engine that has one, starts it under
//! settings that are written down, and supervises it — so what a caller talks
//! to is `llama-server`'s OpenAI-compatible interface, and calling it MCF's
//! own would be claiming authorship of something MCF did not write (A19).
//!
//! **Every setting is shown with what MCF recommended beside it**, and the
//! ones somebody moved are listed separately, because a run under a changed
//! setting is not a run under the recommended one (§3.15, A6).

use crate::say::refused_because;
use std::io::{BufRead as _, BufReader, Write as _};
use std::os::unix::net::UnixStream;

use mcf_record::json::Value;
use mcf_serve::control::{Answer, Request};

use crate::Response;

/// What MCF would run a model under, and why.
pub(crate) fn settings(model: &str, at: Option<u64>) -> Response {
    ask(&Request::Settings {
        model: model.to_owned(),
    })
    .map_or_else(
        |text| Response {
            text,
            served: false,
        },
        |body| Response {
            text: explained(&body, at),
            served: true,
        },
    )
}

/// Holds a model and answers on a port.
pub(crate) fn host(model: &str, changes: &[(String, Value)]) -> Response {
    let settings = if changes.is_empty() {
        Value::Null
    } else {
        Value::map(
            changes
                .iter()
                .map(|(name, value)| (name.as_str(), value.clone())),
        )
    };
    ask(&Request::Host {
        model: model.to_owned(),
        settings,
    })
    .map_or_else(
        |text| Response {
            text,
            served: false,
        },
        |body| Response {
            text: hosting(&body),
            served: true,
        },
    )
}

/// What is being held, if anything.
pub(crate) fn held() -> Response {
    ask(&Request::Hosted).map_or_else(
        |text| Response {
            text,
            served: false,
        },
        |body| {
            let text = if matches!(body.get("hosting"), None | Some(Value::Null)) {
                "mcf: nothing is being hosted\n  `mcf host <model>` holds one".to_owned()
            } else {
                hosting(&body)
            };
            Response { text, served: true }
        },
    )
}

/// Stops holding it.
pub(crate) fn unhost() -> Response {
    ask(&Request::Unhost).map_or_else(
        |text| Response {
            text,
            served: false,
        },
        |body| Response {
            text: match body.get("was").and_then(Value::as_text) {
                Some(was) => format!("stopped hosting {was}"),
                None => "nothing was being hosted".to_owned(),
            },
            served: true,
        },
    )
}

/// Sends one request and returns what it answered, or a sentence.
fn ask(request: &Request) -> Result<Value, String> {
    let Some(socket) = crate::serve::socket_path() else {
        return Err("mcf: MCF has nowhere to put a control socket on this machine".to_owned());
    };
    if let Some(why) = crate::serve::ensure_running(&socket) {
        return Err(format!("mcf: MCF could not start\n  {why}"));
    }
    let mut connection = UnixStream::connect(&socket)
        .map_err(|error| format!("mcf: MCF is not answering\n  {error}"))?;
    // Loading a large model onto a card is tens of seconds, so no deadline
    // here: a timeout would report a working load as a broken daemon.
    writeln!(connection, "{}", request.to_line())
        .and_then(|()| connection.flush())
        .map_err(|error| format!("mcf: the request could not be sent\n  {error}"))?;
    let mut line = String::new();
    BufReader::new(&connection)
        .read_line(&mut line)
        .map_err(|error| format!("mcf: MCF did not answer\n  {error}"))?;
    let answer = Answer::read(line.trim_end()).map_err(|failure| failure.to_string())?;
    if answer.served {
        Ok(answer.body)
    } else {
        Err(format!("mcf: refused\n  {}", refused_because(&answer.body)))
    }
}

/// Every setting, with what it does and what MCF advised.
/// Bytes as a figure somebody weighs a machine against.
#[allow(
    clippy::integer_division,
    reason = "gibibytes to one decimal is the resolution shown; the rest is not"
)]
fn in_gigabytes(bytes: i64) -> String {
    let whole = bytes / (1 << 30);
    let tenth = (bytes % (1 << 30)) * 10 / (1 << 30);
    format!("{whole}.{tenth} GiB")
}

fn explained(body: &Value, at: Option<u64>) -> String {
    let mut lines = vec![format!(
        "{}\n",
        body.get("model").and_then(Value::as_text).unwrap_or("?")
    )];
    for setting in body.get("explains").and_then(Value::as_list).unwrap_or(&[]) {
        let text = |key: &str| setting.get(key).and_then(Value::as_text).unwrap_or("?");
        let (name, value, recommended) = (text("name"), text("value"), text("recommended"));
        // Where the two differ, both are shown: what is set and what was
        // advised are two facts (§3.15).
        let said = if value == recommended {
            value.to_owned()
        } else {
            format!("{value}   (MCF recommends {recommended})")
        };
        lines.push(format!("  {name:<20} {said}"));
        lines.push(format!("  {:<20} {}", "", text("because")));
        // The window's cost, beside the window. A recommendation of *the
        // largest that fits* reserved 54.6 GiB for a 17.6 GB model here, and
        // nothing said so until the memory was gone (§3.15, §3.4).
        if name == "context window"
            && let Some(bytes) = body.get("cache_bytes").and_then(Value::as_integer)
            && bytes > 0
        {
            lines.push(format!(
                "  {:<20} at that size the cache reserves {}",
                "",
                in_gigabytes(bytes)
            ));
        }
        // **And at a window somebody is considering rather than the one MCF
        // chose.** Deciding how much of the machine to give a model means
        // comparing windows, and the window MCF picked is only one of them.
        // The rate comes from the daemon and the multiplication happens here,
        // so asking about six sizes is one question rather than six (A22).
        if name == "context window"
            && let Some(wanted) = at
            && let Some(per) = body
                .get("cache_bytes_per_token")
                .and_then(Value::as_integer)
                .and_then(|held| u64::try_from(held).ok())
            && per > 0
        {
            lines.push(format!(
                "  {:<20} at {} tokens it reserves {}",
                "",
                wanted,
                in_gigabytes(i64::try_from(per.saturating_mul(wanted)).unwrap_or(i64::MAX))
            ));
        }
        lines.push(String::new());
    }
    lines.push("`mcf host <model>` starts it under these".to_owned());
    lines.join("\n")
}

/// What is being hosted and where.
fn hosting(body: &Value) -> String {
    let text = |key: &str| body.get(key).and_then(Value::as_text).unwrap_or("?");
    let settings = body.get("settings");
    let held = |key: &str| {
        settings
            .as_ref()
            .and_then(|settings| settings.get(key))
            .and_then(Value::as_integer)
    };
    let engine = settings
        .as_ref()
        .and_then(|settings| settings.get("engine"))
        .and_then(Value::as_text)
        .unwrap_or("?")
        .to_owned();

    let mut lines = vec![
        format!("hosting {}", text("hosting")),
        String::new(),
        format!("  reachable at   {}", text("address")),
        format!("  engine         {engine}"),
        format!(
            "  layers on card {}",
            held("gpu_layers").map_or_else(
                || "?".to_owned(),
                |held| if held == 0 {
                    "none — the processor".to_owned()
                } else {
                    "all of them".to_owned()
                }
            )
        ),
        format!("  context        {} tokens", held("context").unwrap_or(0)),
        format!("  since          {}", text("since")),
    ];
    let changed = body.get("changed").and_then(Value::as_list).unwrap_or(&[]);
    if changed.is_empty() {
        lines.push("  settings       as MCF recommended".to_owned());
    } else {
        lines.push("  settings       moved off what MCF recommended:".to_owned());
        for one in changed {
            if let Some(said) = one.as_text() {
                lines.push(format!("                   {said}"));
            }
        }
    }
    lines.push(String::new());
    // Whose API it is. MCF started it and supervises it; it did not write it.
    lines.push(
        "  the interface is the provisioned engine's own, which speaks the OpenAI shape:"
            .to_owned(),
    );
    lines.push(format!(
        "    curl {}/v1/chat/completions \\",
        text("address")
    ));
    lines.push(
        "      -H 'Content-Type: application/json' \\\n      -d '{\"messages\":[{\"role\":\"user\",\
         \"content\":\"hello\"}]}'"
            .to_owned(),
    );
    lines.join("\n")
}
