use mcf_record::json::Value;
use mcf_serve::control::Request;

use crate::Response;

const TOKENS: usize = 512;

pub(crate) fn ask(
    model: Option<&str>,
    prompt: &str,
    limit: Option<usize>,
    seed: u64,
    engine: Option<&str>,
) -> Response {
    let named = match model {
        Some(named) => match crate::models::resolve_named(named) {
            Ok(Some(path)) => path.display().to_string(),
            Ok(None) => {
                return Response {
                    text: format!("mcf: {named} is not a model on this machine"),
                    served: false,
                };
            }
            Err(found) => {
                return Response {
                    text: crate::models::ambiguous(named, &found),
                    served: false,
                };
            }
        },
        None => match held_model() {
            Ok(named) => named,
            Err(response) => return response,
        },
    };
    asking(&named, prompt, limit, seed, engine)
}

fn held_model() -> Result<String, Response> {
    let held = match crate::hosting::ask(&Request::Hosted) {
        Ok(body) => body,
        Err(text) => {
            return Err(Response {
                text,
                served: false,
            });
        }
    };
    held.get("hosting")
        .and_then(Value::as_text)
        .map(str::to_owned)
        .ok_or_else(|| Response {
            text: "mcf: nothing is being held, so name a model or hold one\n  `mcf host \
                   <model>` holds one"
                .to_owned(),
            served: false,
        })
}

fn asking(
    model: &str,
    prompt: &str,
    limit: Option<usize>,
    seed: u64,
    engine: Option<&str>,
) -> Response {
    use std::io::{BufRead as _, BufReader, Write as _};

    let (socket, mut connection) = match connected() {
        Ok(held) => held,
        Err(response) => return response,
    };
    let request = Request::Generate {
        model: model.to_owned(),
        prompt: prompt.to_owned(),
        limit: Some(limit.unwrap_or(TOKENS)),
        seed,
        tokens: None,
        pieces: None,
        engine: engine.map(str::to_owned),
        whose: mcf_record::content::Whose::User,
        pinned: false,
        turn: None,
        image: None,
        started: std::boxed::Box::new(mcf_serve::declared::Started::default()),
    };
    if let Err(error) =
        writeln!(connection, "{}", request.to_line()).and_then(|()| connection.flush())
    {
        return Response {
            text: format!("mcf: the request could not be sent\n  {error}"),
            served: false,
        };
    }

    let mut out = std::io::stdout();
    let mut produced = 0_usize;
    let mut refused = None;
    let mut account: Option<Value> = None;
    let reader = BufReader::new(&connection);
    for line in reader.lines() {
        let Ok(line) = line else { break };
        let trimmed = line.trim_end();
        if let Ok(answer) = mcf_serve::control::Answer::read(trimmed)
            && !answer.served
        {
            refused = Some(crate::say::refused_because(&answer.body));
            continue;
        }
        match mcf_serve::control::Streamed::read(trimmed) {
            Ok(mcf_serve::control::Streamed::Token { text, .. }) => {
                produced = produced.saturating_add(1);
                let _printed = write!(out, "{text}");
                let _flushed = out.flush();
            }
            Ok(mcf_serve::control::Streamed::Done(body)) => account = Some(body),
            Ok(_) | Err(_) => {}
        }
    }
    if let Some(why) = refused {
        return Response {
            text: format!("mcf: refused\n  {why}"),
            served: false,
        };
    }
    let Some(account) = account else {
        return Response {
            text: format!(
                "mcf: the stream ended before its account; {produced} token(s) were received"
            ),
            served: false,
        };
    };
    let failure = account.get("failure");
    if let Some(failure) = failure
        && account.get("tokens").and_then(Value::as_integer) == Some(0)
    {
        return Response {
            text: format!("mcf: {model} did not run\n  {}", why_not(failure, &socket)),
            served: false,
        };
    }
    Response {
        served: failure.is_none(),
        text: what_produced_it(&account, model, &socket),
    }
}

fn what_produced_it(account: &Value, model: &str, socket: &std::path::Path) -> String {
    let counted = |key: &str| {
        account
            .get(key)
            .map_or_else(|| "?".to_owned(), Value::to_line)
    };
    let condition = |key: &str| {
        account
            .get("conditions")
            .and_then(|held| held.get(key))
            .map_or_else(
                || "?".to_owned(),
                |held| held.as_text().map_or_else(|| held.to_line(), str::to_owned),
            )
    };
    let degraded = account.get("degraded").and_then(Value::as_text);
    let died = account.get("failure").map(Value::to_line);
    format!(
        "\n\n── what produced it ─────────────────────────────────────────\n  \
         model    {model}\n  \
         prompt   {} token(s)\n  \
         produced {} token(s); stopped: {}\n  \
         sampler  {}, seed {}\n  \
         engine   {}\n  \
         served   by the daemon at {}, model loaded {}\n{}{}\n",
        counted("prompt_tokens"),
        counted("tokens"),
        counted("stopped").trim_matches('"'),
        condition("sampler"),
        condition("seed"),
        condition("engine"),
        socket.display(),
        condition("loaded"),
        match degraded {
            Some(mark) => format!("  MARKED   {mark}"),
            None => "  a real engine: nothing here is marked degraded".to_owned(),
        },
        match died {
            Some(failure) =>
                format!("\n  THE ENGINE DIED mid-answer; what arrived is above:\n  {failure}"),
            None => String::new(),
        },
    )
}

fn connected() -> Result<(std::path::PathBuf, std::os::unix::net::UnixStream), Response> {
    let Some(socket) = crate::serve::socket_path() else {
        return Err(Response {
            text: "mcf: MCF has nowhere to put a control socket on this machine".to_owned(),
            served: false,
        });
    };
    if let Some(why) = crate::serve::ensure_running(&socket) {
        return Err(Response {
            text: format!("mcf: MCF could not start\n  {why}"),
            served: false,
        });
    }
    match std::os::unix::net::UnixStream::connect(&socket) {
        Ok(connection) => Ok((socket, connection)),
        Err(error) => Err(Response {
            text: format!("mcf: MCF is not answering\n  {error}"),
            served: false,
        }),
    }
}

fn why_not(failure: &Value, socket: &std::path::Path) -> String {
    let category = failure
        .get("category")
        .and_then(Value::as_text)
        .unwrap_or_default();
    if !category.starts_with("artifact.format") {
        return format!(
            "the daemon at {} refused it:\n  {}",
            socket.display(),
            failure.to_line()
        );
    }
    let detail = failure
        .get("detail")
        .and_then(Value::as_text)
        .map_or_else(|| failure.to_line(), str::to_owned);
    format!(
        "{detail}\n  MCF's own reader handles {}. A model it refuses may still run on a \
         provisioned engine — `mcf provision llama.cpp` builds one",
        mcf_standin::llama::FAMILIES.join(", ")
    )
}
