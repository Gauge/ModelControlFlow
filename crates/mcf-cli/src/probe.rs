use std::io::{BufRead as _, BufReader, Write as _};
use std::os::unix::net::UnixStream;

use mcf_record::json::Value;
use mcf_serve::control::{Answer, Request};

use crate::Response;
use crate::run::{ambiguous, resolve};

pub(crate) fn run(
    model: &str,
    engine: Option<&str>,
    apply: bool,
    up_to: Option<usize>,
    only: Option<&str>,
) -> Response {
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
    let only: Vec<String> = only
        .unwrap_or_default()
        .split(',')
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .map(str::to_owned)
        .collect();
    let unknown: Vec<&str> = only
        .iter()
        .map(String::as_str)
        .filter(|name| !mcf_serve::probes::run::PROBES.contains(name))
        .collect();
    if !unknown.is_empty() {
        return Response {
            text: format!(
                "mcf: --only names a probe MCF does not have: {}\n  the probes are {}",
                unknown.join(", "),
                mcf_serve::probes::run::PROBES.join(", ")
            ),
            served: false,
        };
    }
    let Some(socket) = crate::serve::socket_path() else {
        return Response {
            text: "mcf: there is nowhere to look for a daemon, and a probe needs an engine to \
                   ask (D42)"
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
    let Ok(mut connection) = UnixStream::connect(&socket) else {
        return Response {
            text: format!("mcf: nothing is listening on {}", socket.display()),
            served: false,
        };
    };
    let request = Request::Probe {
        model: path.display().to_string(),
        engine: engine.map(str::to_owned),
        apply,
        up_to,
        only,
    };
    if writeln!(connection, "{}", request.to_line())
        .and_then(|()| connection.flush())
        .is_err()
    {
        return Response {
            text: "mcf: the request could not be sent".to_owned(),
            served: false,
        };
    }
    println!("probed {}", path.display());
    println!();
    let Some(answer) = the_findings_as_they_come(&connection) else {
        return Response {
            text: "mcf: MCF did not answer".to_owned(),
            served: false,
        };
    };
    if !answer.served {
        return Response {
            text: format!(
                "mcf: refused\n  {}",
                crate::say::refused_because(&answer.body)
            ),
            served: false,
        };
    }
    Response {
        text: lines_of(&answer.body).join("\n"),
        served: true,
    }
}

fn the_findings_as_they_come(connection: &UnixStream) -> Option<Answer> {
    for read in BufReader::new(connection).lines() {
        let read = read.ok()?;
        let Ok(answer) = Answer::read(read.trim_end()) else {
            continue;
        };
        if !answer.served || matches!(answer.body.get("done"), Some(Value::Bool(true))) {
            return Some(answer);
        }
        let lines = lines_of(&answer.body);
        if lines.is_empty() {
            if let Some(step) = step_said(&answer.body) {
                println!("{step}");
            }
        } else {
            for line in lines {
                println!("{line}");
            }
            println!();
        }
        let _flushed = std::io::stdout().flush();
    }
    None
}

fn lines_of(body: &Value) -> Vec<String> {
    body.get("lines")
        .and_then(Value::as_list)
        .unwrap_or(&[])
        .iter()
        .filter_map(Value::as_text)
        .map(str::to_owned)
        .collect()
}

fn step_said(body: &Value) -> Option<String> {
    let step = body.get("step")?;
    let figure = |key: &str| step.get(key).and_then(Value::as_integer);
    Some(format!(
        "probe {} of {}: {}",
        figure("count")?,
        figure("of")?,
        step.get("name").and_then(Value::as_text)?
    ))
}
