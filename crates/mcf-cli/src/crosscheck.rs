//! `mcf cross-check <model>` — MCF's engine against the one it provisioned.
//!
//! **The command the window's checkbox runs.** A22 asks that everything a
//! surface can do, the headless path can do; this is that path for the
//! cross-check, and both send the same control request to the same daemon.
//! Nothing is compared here — the daemon does it, because the daemon is
//! where the model and both engines are, and the sentences printed are the
//! daemon's, so that no surface can say the same figures in different words
//! (B-072, B-424).

use std::io::{BufRead as _, BufReader, Write as _};
use std::os::unix::net::UnixStream;

use mcf_record::json::Value;
use mcf_serve::control::{Answer, Request};

use crate::Response;
use crate::run::{ambiguous, resolve};
use crate::say::refused_because;

/// Compares the two engines on one model and says whether they agree.
pub(crate) fn run(model: &str) -> Response {
    let path = match resolve(model) {
        Ok(Some(path)) => path,
        Ok(None) => {
            return Response {
                text: format!(
                    "mcf: there is no model at {model}\n  `mcf list` says what this machine is \
                     holding; a path to a file works too"
                ),
                served: false,
            };
        }
        Err(found) => {
            return Response {
                text: ambiguous(model, &found),
                served: false,
            };
        }
    };
    let Some(socket) = crate::serve::socket_path() else {
        return Response {
            text: "mcf: MCF has nowhere to put a control socket on this machine\n  \
                   a cross-check is served by a running daemon, and there is no path to one"
                .to_owned(),
            served: false,
        };
    };
    if let Some(why) = crate::serve::ensure_running(&socket) {
        return Response {
            text: format!("mcf: MCF could not start\n  {why}"),
            served: false,
        };
    }
    let mut connection = match UnixStream::connect(&socket) {
        Ok(connection) => connection,
        Err(error) => {
            return Response {
                text: format!("mcf: MCF is not answering\n  {error}"),
                served: false,
            };
        }
    };
    // No read timeout: MCF's own engine pays a forward pass per position, and
    // a deadline here would turn a slow model into a lie about a broken
    // daemon.
    let line = Request::CrossCheck {
        model: path.display().to_string(),
    }
    .to_line();
    if let Err(error) = writeln!(connection, "{line}").and_then(|()| connection.flush()) {
        return Response {
            text: format!("mcf: the cross-check could not be asked for\n  {error}"),
            served: false,
        };
    }

    let mut lines: Vec<String> = Vec::new();
    let mut served = false;
    let reader = BufReader::new(&connection);
    for read in reader.lines() {
        let Ok(read) = read else { break };
        let Ok(answer) = Answer::read(read.trim_end()) else {
            continue;
        };
        if !answer.served {
            lines.push(format!(
                "mcf: the two engines were not compared on {}\n  {}",
                path.display(),
                refused_because(&answer.body)
            ));
            break;
        }
        lines.extend(said(&answer.body));
        if matches!(answer.body.get("done"), Some(Value::Bool(true))) {
            served = answer
                .body
                .get("agreement")
                .and_then(|agreement| agreement.get("within_arithmetic"))
                .is_some_and(|within| matches!(within, Value::Bool(true)));
            break;
        }
    }
    Response {
        text: lines.join("\n"),
        served,
    }
}

/// One answer, as a line or two of terminal output.
fn said(body: &Value) -> Vec<String> {
    let text = |key: &str| body.get(key).and_then(Value::as_text).map(str::to_owned);
    let number = |key: &str| body.get(key).and_then(Value::as_integer).unwrap_or(0);

    if let (Some(low), Some(high)) = (
        body.get("estimate_low_seconds").and_then(Value::as_integer),
        body.get("estimate_high_seconds")
            .and_then(Value::as_integer),
    ) {
        return vec![
            format!(
                "cross-checking {}",
                text("cross_checking").unwrap_or_default()
            ),
            format!(
                "  asking the provisioned engine for {} tokens of \"{}\" ({} tokens of prompt)",
                number("positions"),
                text("prompt").unwrap_or_default(),
                number("prompt_tokens")
            ),
            format!("  this will take somewhere between {low} and {high} seconds"),
            String::new(),
        ];
    }
    if matches!(body.get("reading"), Some(Value::Bool(true))) {
        return vec![format!(
            "  {} produced {} tokens; MCF's own engine is reading them",
            text("engine_ran").unwrap_or_else(|| "the provisioned engine".to_owned()),
            number("produced")
        )];
    }
    if let Some(sentences) = body.get("said").and_then(Value::as_list) {
        let mut lines = vec![String::new()];
        lines.extend(
            sentences
                .iter()
                .filter_map(Value::as_text)
                .map(|sentence| format!("  {sentence}")),
        );
        lines.push(String::new());
        return lines;
    }
    Vec::new()
}
