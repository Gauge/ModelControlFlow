//! `mcf measure`: how quickly a model produces text, and how that changes as
//! the conversation gets longer.
//!
//! **The command the window's button presses.** A22 asks that everything a
//! surface can do, the headless path can do; this is that path for the
//! measurement, and both send the same control request to the same daemon.
//! Nothing is measured here — the daemon does it, because the daemon is where
//! the model and the engine are (B-072).
//!
//! **The estimate comes before the work**, as a range, and it is printed
//! rather than swallowed: a person who is told a run will take four minutes
//! can decide not to start it, and one who is told nothing cannot.

use crate::say::refused_because;
use std::io::{BufRead as _, BufReader, Write as _};
use std::os::unix::net::UnixStream;

use mcf_record::json::Value;
use mcf_serve::control::{Answer, Request};

use crate::Response;

/// Times a model at doubling depths and prints what came back.
pub(crate) fn run(
    model: &str,
    deepest: u64,
    engine: Option<&str>,
    on: Option<mcf_serve::control::On>,
    started: mcf_serve::declared::Started,
) -> Response {
    let Some(socket) = crate::serve::socket_path() else {
        return Response {
            text: "mcf: MCF has nowhere to put a control socket on this machine\n  \
                   a measurement is served by a running daemon, and there is no path to one"
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
    // No read timeout: a measurement is minutes by design, and a deadline
    // here would turn a long run into a lie about a broken daemon.
    let line = Request::Measure {
        model: model.to_owned(),
        engine: engine.map(str::to_owned),
        on,
        deepest,
        started,
    }
    .to_line();
    if let Err(error) = writeln!(connection, "{line}").and_then(|()| connection.flush()) {
        return Response {
            text: format!("mcf: the measurement could not be asked for\n  {error}"),
            served: false,
        };
    }

    // **Printed as it comes, not at the end.** A measurement is minutes and
    // says where it is at every generation; a terminal that showed nothing
    // until the last line showed a run that looked stopped, and gave nobody
    // a moment to cut it short (A7). What ends the run — the conditions, or
    // a refusal — is the response; the way there is printed on the way.
    let mut lines: Vec<String> = Vec::new();
    let mut served = true;
    let reader = BufReader::new(&connection);
    for read in reader.lines() {
        let Ok(read) = read else { break };
        let Ok(answer) = Answer::read(read.trim_end()) else {
            continue;
        };
        if !answer.served {
            served = false;
            lines.push(format!(
                "mcf: nothing was measured\n  {}",
                refused_because(&answer.body)
            ));
            break;
        }
        if matches!(answer.body.get("done"), Some(Value::Bool(true))) {
            lines.extend(said(&answer.body));
            break;
        }
        for line in said(&answer.body) {
            println!("{line}");
        }
        let _flushed = std::io::stdout().flush();
    }
    Response {
        text: lines.join("\n"),
        served,
    }
}

/// One answer, as a line or two of terminal output.
fn said(body: &Value) -> Vec<String> {
    let text = |key: &str| body.get(key).and_then(Value::as_text).map(str::to_owned);
    let number = |held: &Value, key: &str| held.get(key).and_then(Value::as_integer);

    if let (Some(low), Some(high)) = (
        number(body, "estimate_low_seconds"),
        number(body, "estimate_high_seconds"),
    ) {
        let depths = body
            .get("depths")
            .and_then(Value::as_list)
            .map(|depths| {
                depths
                    .iter()
                    .filter_map(Value::as_integer)
                    .map(|depth| depth.to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            })
            .unwrap_or_default();
        return vec![
            format!("measuring {}", text("measuring").unwrap_or_default()),
            format!("  at depths {depths}"),
            format!("  this will take somewhere between {low} and {high} seconds"),
            String::new(),
        ];
    }

    // Where the run is, as the daemon announces it: the same words the
    // console and the window use for the same line (A22, B-072).
    if let Some(step) = mcf_tui::screens::diagnostics::step_line(body) {
        return vec![format!("  {step}")];
    }

    if let Some(reading) = body.get("reading") {
        let depth = number(reading, "depth").unwrap_or(0);
        return vec![
            if matches!(reading.get("measured"), Some(Value::Bool(true))) {
                format!(
                    "  {depth:>7} tokens deep — {} ms a token (spread {}, over {} samples)",
                    reading
                        .get("ms_per_token")
                        .and_then(Value::as_text)
                        .unwrap_or("?"),
                    reading
                        .get("spread_ms")
                        .and_then(Value::as_text)
                        .unwrap_or("?"),
                    number(reading, "samples").unwrap_or(0),
                )
            } else {
                // A9: a depth that would not separate is a result, and it is
                // printed as one rather than left out of the list.
                format!(
                    "  {depth:>7} tokens deep — not measured: {}",
                    reading.get("why").and_then(Value::as_text).unwrap_or("?")
                )
            },
        ];
    }

    if matches!(body.get("done"), Some(Value::Bool(true))) {
        return under_what(body);
    }
    Vec::new()
}

/// The conditions the measurement was taken under, which is the half of it
/// that makes the figures usable (§3.4, A6).
fn under_what(body: &Value) -> Vec<String> {
    let number = |held: &Value, key: &str| held.get(key).and_then(Value::as_integer);
    {
        let Some(conditions) = body.get("conditions") else {
            return vec![String::new(), "measured".to_owned()];
        };
        let ran = conditions
            .get("engine_ran")
            .and_then(Value::as_text)
            .unwrap_or("MCF did not say");
        let device = conditions
            .get("device")
            .and_then(Value::as_text)
            .map_or_else(
                || "MCF did not say".to_owned(),
                |device| match number(conditions, "gpu_layers") {
                    Some(layers) if layers > 0 => format!("{device}, {layers} layers on the card"),
                    Some(_) => format!("{device}, nothing on a card"),
                    None => device.to_owned(),
                },
            );
        let mut out = vec![
            String::new(),
            "under these conditions:".to_owned(),
            format!("  engine        {ran}"),
            format!("  device        {device}"),
            format!(
                "  method        {}",
                conditions
                    .get("method")
                    .and_then(Value::as_text)
                    .unwrap_or("?")
            ),
            format!(
                "  repeats       {} a depth",
                number(conditions, "repeats").unwrap_or(0)
            ),
            format!(
                "  residency     {}",
                conditions
                    .get("loaded")
                    .and_then(Value::as_text)
                    .unwrap_or("?")
            ),
            // What the engine was started with beyond the plain load: two
            // runs are only comparable if each says which it was, and this
            // is the line that makes a switch's cost a measurement rather
            // than a claim (A6, B-463).
            format!(
                "  started with  {}",
                conditions
                    .get("started_with")
                    .map(mcf_serve::declared::Started::from_value)
                    .unwrap_or_default()
                    .said()
            ),
        ];
        out.extend(read_off_the_rungs(body));
        // B65 and D31: a timing taken from MCF's own reference implementation
        // measures the reference implementation, which is written to be read
        // rather than to be fast. Saying so is not a footnote.
        if matches!(conditions.get("is_the_stand_in"), Some(Value::Bool(true))) {
            out.push(String::new());
            out.push(
                "  these are timings of MCF's own reference engine, which is written to be \
                 read rather than to be fast — they are not what this model does on a real \
                 engine"
                    .to_owned(),
            );
        }
        out
    }
}

/// The figures the run reads off its rungs and derives on the daemon's side,
/// printed in the daemon's words (B-072); where it could not read one, its
/// reason is the line (A7, A9).
fn read_off_the_rungs(body: &Value) -> Vec<String> {
    let mut out = vec![String::new(), "read off the rungs:".to_owned()];
    for (what, lines) in [
        (
            "prompt reading",
            mcf_serve::ladder::prompt_reading_said(body.get("prompt_reading")),
        ),
        (
            "start-up",
            mcf_serve::ladder::first_token_said(body.get("first_token")),
        ),
        ("memory", mcf_serve::ladder::memory_said(body.get("memory"))),
        (
            "fall-off",
            mcf_serve::ladder::fall_off_said(body.get("fall_off")),
        ),
    ] {
        let mut lines = lines.into_iter();
        out.push(format!("  {what:<15} {}", lines.next().unwrap_or_default()));
        out.extend(lines.map(|line| format!("  {:<15} {line}", "")));
    }
    out
}
