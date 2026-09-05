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
    // `--on` names a placement; the daemon lists them with the build that
    // fits each, and the choice becomes the engine, the device and the
    // layers of that placement rather than the recommended build with its
    // layers moved (F176).
    let mut changes: Vec<(String, Value)> = changes.to_vec();
    if let Some(at) = changes.iter().position(|(name, _)| name == "on") {
        let (_, wanted) = changes.remove(at);
        let wanted = wanted.as_text().unwrap_or_default().to_owned();
        let listed = match ask(&Request::Settings {
            model: model.to_owned(),
        }) {
            Ok(body) => body,
            Err(text) => {
                return Response {
                    text,
                    served: false,
                };
            }
        };
        let Some(placement) =
            listed
                .get("placements")
                .and_then(Value::as_list)
                .and_then(|placements| {
                    placements
                        .iter()
                        .find(|held| held.get("on").and_then(Value::as_text) == Some(&wanted))
                })
        else {
            return Response {
                text: format!(
                    "mcf: nothing here puts this model on the {wanted}\n  `mcf settings {model}` \
                     lists where it can go"
                ),
                served: false,
            };
        };
        for key in ["engine", "device", "gpu_layers"] {
            if let Some(value) = placement.get(key) {
                changes.push((key.to_owned(), value.clone()));
            }
        }
    }
    let settings = if changes.is_empty() {
        Value::Null
    } else {
        Value::map(
            changes
                .iter()
                .map(|(name, value)| (name.as_str(), value.clone())),
        )
    };
    // Printed as it loads: the daemon says once a second how much of the
    // model the engine has read, and a terminal that showed nothing for the
    // minutes a large model takes showed a load that looked stopped (A7).
    ask_as_it_comes(
        &Request::Host {
            model: model.to_owned(),
            settings,
        },
        &mut |body| {
            if let Some(said) = loading_said(body) {
                println!("  {said}");
                let _flushed = std::io::stdout().flush();
            }
        },
    )
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

/// One line of a load's progress, where the answer is one: how much of the
/// model the engine holds so far, of how much, after how long — and, once
/// there is a rate to read it off, about how long is left. The estimate is
/// arithmetic on what was read so far and says so with *about* (A6).
pub(crate) fn loading_said(body: &Value) -> Option<String> {
    let loading = body.get("loading")?;
    let figure = |key: &str| loading.get(key).and_then(Value::as_integer);
    let seconds = figure("seconds")?;
    let of = figure("of_bytes");
    // Onto a card, the card's memory is the figure that grows; the engine's
    // own does not show weights that went there.
    let (read, where_) = match figure("card_bytes") {
        Some(on_card) => (on_card, " onto the card"),
        None => (figure("resident_bytes")?, ""),
    };
    Some(load_line(read, where_, of, seconds, "  "))
}

/// The sentence for how far a load has got, shared with the window's words.
///
/// No estimate off the first crumbs: a rate read from under a twentieth of
/// the weights said *about 200 s* two seconds into a ten-second load. And
/// none past the weights: what follows them is the cache and the engine's
/// buffers, whose size the file does not say (A6, A7).
pub(crate) fn load_line(
    read: i64,
    where_: &str,
    of: Option<i64>,
    seconds: i64,
    gap: &str,
) -> String {
    match of {
        Some(of) if read >= of => format!(
            "loading{where_}{gap}{} so far — the {} of weights are on, and the cache and the \
             engine's buffers follow; {seconds} s so far",
            in_gigabytes(read),
            in_gigabytes(of)
        ),
        Some(of) => {
            let left = if read.saturating_mul(20) >= of && seconds > 0 && read > 0 {
                #[expect(
                    clippy::integer_division,
                    reason = "whole seconds left at the rate so far; the remainder is under a second"
                )]
                let eta = (of - read).saturating_mul(seconds) / read;
                if eta == 0 {
                    ", nearly there".to_owned()
                } else {
                    format!(", about {eta} s to go")
                }
            } else {
                String::new()
            };
            format!(
                "loading{where_}{gap}{} of {} of weights, {seconds} s so far{left}",
                in_gigabytes(read),
                in_gigabytes(of)
            )
        }
        None => format!(
            "loading{where_}{gap}{}, {seconds} s so far",
            in_gigabytes(read)
        ),
    }
}

/// What was last held, where the record says: which model, on what, since
/// when and until when, and how long ago that was.
pub(crate) fn last_held(body: &Value) -> String {
    let Some(last) = body.get("last").filter(|held| !matches!(held, Value::Null)) else {
        return String::new();
    };
    let text = |key: &str| last.get(key).and_then(Value::as_text).unwrap_or("?");
    // The times are the record's, in its shape; shown as the record shows
    // them elsewhere.
    let when = |key: &str| {
        last.get(key)
            .and_then(|at| mcf_record::decode::timestamp(at).ok())
            .map_or_else(|| "?".to_owned(), |at| at.to_string())
    };
    let ago = last
        .get("ago_seconds")
        .and_then(Value::as_integer)
        .map_or_else(String::new, |seconds| {
            format!(", {} ago", ago_said(seconds))
        });
    format!(
        "\n  last held      {}\n  on             {} through {}\n  {:<14} {}{ago}\n  `mcf host {}` holds it again",
        text("model"),
        text("device"),
        text("engine"),
        if matches!(last.get("until"), None | Some(Value::Null)) {
            "held from"
        } else {
            "stopped"
        },
        if matches!(last.get("until"), None | Some(Value::Null)) {
            when("since")
        } else {
            when("until")
        },
        text("model")
    )
}

/// Seconds as a span a person says: *40 s*, *12 min*, *2 h 5 min*.
pub(crate) fn ago_said(seconds: i64) -> String {
    #[expect(
        clippy::integer_division,
        reason = "whole minutes and hours, and the rest"
    )]
    let (hours, minutes, rest) = (seconds / 3600, (seconds % 3600) / 60, seconds % 60);
    match (hours, minutes) {
        (0, 0) => format!("{rest} s"),
        (0, minutes) => format!("{minutes} min"),
        (hours, minutes) => format!("{hours} h {minutes} min"),
    }
}

/// What a stop gave back, where the daemon measured it: the engine's own
/// memory, and the card's where the model was on one.
pub(crate) fn freed_said(body: &Value) -> String {
    let figure = |key: &str| body.get(key).and_then(Value::as_integer);
    match (figure("freed_bytes"), figure("freed_card_bytes")) {
        (Some(memory), Some(card)) => format!(
            " — freed {} of memory and {} on the card",
            in_gigabytes(memory),
            in_gigabytes(card)
        ),
        (Some(memory), None) => format!(" — freed {}", in_gigabytes(memory)),
        (None, Some(card)) => format!(" — freed {} on the card", in_gigabytes(card)),
        (None, None) => String::new(),
    }
}

/// Sends one request that answers in many lines, handing each line short
/// of the last to `heard`, and returns the last.
pub(crate) fn ask_as_it_comes(
    request: &Request,
    heard: &mut dyn FnMut(&Value),
) -> Result<Value, String> {
    let Some(socket) = crate::serve::socket_path() else {
        return Err("mcf: MCF has nowhere to put a control socket on this machine".to_owned());
    };
    if let Some(why) = crate::serve::ensure_running(&socket) {
        return Err(format!("mcf: MCF could not start\n  {why}"));
    }
    let mut connection = UnixStream::connect(&socket)
        .map_err(|error| format!("mcf: MCF is not answering\n  {error}"))?;
    writeln!(connection, "{}", request.to_line())
        .and_then(|()| connection.flush())
        .map_err(|error| format!("mcf: the request could not be sent\n  {error}"))?;
    let reader = BufReader::new(&connection);
    for read in reader.lines() {
        let read = read.map_err(|error| format!("mcf: MCF stopped answering\n  {error}"))?;
        let Ok(answer) = Answer::read(read.trim_end()) else {
            continue;
        };
        if !answer.served {
            return Err(format!("mcf: refused\n  {}", refused_because(&answer.body)));
        }
        if matches!(answer.body.get("done"), Some(Value::Bool(true))) {
            return Ok(answer.body);
        }
        heard(&answer.body);
    }
    Err("mcf: MCF stopped answering before it said it had finished".to_owned())
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
                format!(
                    "mcf: nothing is being hosted\n  `mcf host <model>` holds one{}",
                    last_held(&body)
                )
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
                Some(was) => format!("stopped hosting {was}{}", freed_said(&body)),
                None => "nothing was being hosted".to_owned(),
            },
            served: true,
        },
    )
}

/// Sends one request and returns what it answered, or a sentence.
pub(crate) fn ask(request: &Request) -> Result<Value, String> {
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
    // Where it can go, each with the build that fits and what the device
    // has free — the choice `--on` makes, listed before the settings it
    // changes (§3.15).
    if let Some(placements) = body.get("placements").and_then(Value::as_list) {
        for placement in placements {
            let text = |key: &str| placement.get(key).and_then(Value::as_text).unwrap_or("?");
            let free = placement
                .get("free_bytes")
                .and_then(Value::as_integer)
                .map_or_else(String::new, |free| format!(", {} free", in_gigabytes(free)));
            lines.push(format!(
                "  {:<20} {} — {} on {}{free}",
                if text("on") == "resolved" {
                    "where it can go"
                } else {
                    ""
                },
                match text("on") {
                    "resolved" => "as resolved (--on not given)",
                    "processor" => "--on cpu",
                    "card" => "--on gpu",
                    other => other,
                },
                text("engine"),
                text("device")
            ));
        }
        lines.push(String::new());
    }
    if let Some(said) = last_held_under(body) {
        lines.push(said);
        lines.push(String::new());
    }
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
        // What the file carries that this setting leaves in it. A model has
        // no other way of saying it, and the plain load is the setting a
        // person will most often keep (B-456, A7).
        if name == "started with"
            && let Some(left) = body
                .get("declares")
                .map(mcf_serve::declared::Declared::from_value)
                .and_then(|declared| {
                    declared.not_started(mcf_serve::declared::Started::from_value(
                        body.get("settings").unwrap_or(&Value::Null),
                    ))
                })
        {
            lines.push(format!("  {:<20} this file declares {left}", ""));
        }
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
/// What a held model is doing, from the engine's own counters where it
/// published them: the same figures the window's Running page draws (A22).
fn in_use_lines(body: &Value) -> Vec<String> {
    let mut lines = Vec::new();
    let Some(in_use) = body.get("use") else {
        return lines;
    };
    let figure = |key: &str| {
        in_use.get(key).and_then(|held| match held {
            Value::Text(text) => Some(text.clone()),
            Value::Integer(number) => Some(number.to_string()),
            _ => None,
        })
    };
    if let (Some(generated), Some(prompted)) =
        (figure("generated_tokens"), figure("prompted_tokens"))
    {
        lines.push(format!(
            "  tokens         {generated} generated, {prompted} prompted"
        ));
    }
    if let Some(rate) = figure("generated_tokens_per_second") {
        lines.push(format!("  tokens/s       {rate} generating now"));
    }
    if let (Some(processing), Some(queued)) =
        (figure("requests_processing"), figure("requests_queued"))
    {
        lines.push(format!(
            "  requests       {processing} in hand, {queued} queued"
        ));
    }
    if let Some(ratio) = figure("cache_used_ratio") {
        lines.push(format!("  cache          {ratio} of the window in use"));
    }
    if let Some(up) = in_use.get("uptime_seconds").and_then(Value::as_integer) {
        lines.push(format!("  up             {}", ago_said(up)));
    }
    lines
}

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
        format!(
            "  projector      {}",
            settings
                .as_ref()
                .and_then(|settings| settings.get("projector"))
                .and_then(Value::as_text)
                .map_or_else(
                    || "none — text only".to_owned(),
                    |path| format!(
                        "{} (beside the model)",
                        path.rsplit('/').next().unwrap_or(path)
                    )
                )
        ),
        format!("  since          {}", text("since")),
    ];
    lines.extend(in_use_lines(body));
    // What reaches the model through the port, as the engine reported it
    // after it came up. Absent where the engine did not answer, which is
    // said rather than shown as nothing taken (A7).
    match body.get("takes") {
        Some(takes) if !matches!(takes, Value::Null) => {
            let takes = mcf_serve::takes::Takes::from_value(takes);
            lines.push(format!("  takes          {}", takes.media()));
            lines.push(format!("  template       {}", takes.template()));
            lines.push(format!("  thinking       {}", takes.thinking_said()));
        }
        _ => lines.push("  takes          the engine did not say what it takes".to_owned()),
    }
    // What the engine was started with beyond the plain load, and what the
    // file declares that it was not: a model hosted without a feature its
    // own file carries says so, here, where a person reads it (B-456).
    let started = settings
        .as_ref()
        .map(|settings| mcf_serve::declared::Started::from_value(settings))
        .unwrap_or_default();
    if started.asks_anything() {
        lines.push(format!("  started        {}", started.said()));
    }
    if let Some(left) = body
        .get("declares")
        .map(mcf_serve::declared::Declared::from_value)
        .and_then(|declared| declared.not_started(started))
    {
        lines.push(format!("  declares       {left}"));
    }
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

/// What the model was last held under, where it has been: what moved off
/// the recommendation, or that nothing did, and since when (B-475).
fn last_held_under(body: &Value) -> Option<String> {
    let last = body
        .get("last")
        .filter(|held| !matches!(held, Value::Null))?;
    let bare = mcf_serve::hosting::Hosting::recommended("", "", false, 0, None, false, None);
    let recommended = mcf_serve::hosting::Hosting::from_value(body.get("recommended")?, &bare);
    let held = mcf_serve::hosting::Hosting::from_value(last.get("settings")?, &recommended);
    let moved = held.differs_from(&recommended);
    let since = last
        .get("since")
        .and_then(|at| mcf_record::decode::timestamp(at).ok())
        .map_or_else(|| "?".to_owned(), |at| to_the_second(&at.to_string()));
    Some(format!(
        "  {:<20} {} — since {since}",
        "last held under",
        if moved.is_empty() {
            "as recommended".to_owned()
        } else {
            moved.join(", ")
        }
    ))
}

/// A timestamp to the second: what a person reads *since* by.
fn to_the_second(at: &str) -> String {
    at.get(..19)
        .map_or_else(|| at.to_owned(), |head| format!("{head}Z"))
}
