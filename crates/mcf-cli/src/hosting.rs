use crate::say::refused_because;
use std::io::{BufRead as _, BufReader, Write as _};
use std::os::unix::net::UnixStream;

use mcf_record::json::Value;
use mcf_serve::control::{Answer, Request};

use crate::Response;

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

pub(crate) fn host(model: &str, changes: &[(String, Value)]) -> Response {
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

pub(crate) fn loading_said(body: &Value) -> Option<String> {
    let loading = body.get("loading")?;
    let figure = |key: &str| loading.get(key).and_then(Value::as_integer);
    let seconds = figure("seconds")?;
    let of = figure("of_bytes");
    let (read, where_) = match figure("card_bytes") {
        Some(on_card) => (on_card, " onto the card"),
        None => (figure("resident_bytes")?, ""),
    };
    Some(load_line(read, where_, of, seconds, "  "))
}

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

pub(crate) fn last_held(body: &Value) -> String {
    let Some(last) = body.get("last").filter(|held| !matches!(held, Value::Null)) else {
        return String::new();
    };
    let text = |key: &str| last.get(key).and_then(Value::as_text).unwrap_or("?");
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

pub(crate) fn ask(request: &Request) -> Result<Value, String> {
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
        let said = if value == recommended {
            value.to_owned()
        } else {
            format!("{value}   (MCF recommends {recommended})")
        };
        lines.push(format!("  {name:<20} {said}"));
        lines.push(format!("  {:<20} {}", "", text("because")));
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
        let over = figure("rate_over_seconds")
            .map_or_else(String::new, |seconds| format!(" over the last {seconds} s"));
        lines.push(format!("  tokens/s       {rate} generating{over}"));
    }
    if let Some(live) = figure("generated_tokens_live") {
        lines.push(format!(
            "                 {live} tokens produced, counting the answer in hand"
        ));
    }
    if let Some(watts) = figure("card_power_watts") {
        let whose = figure("power_is").unwrap_or_else(|| "the graphics device".to_owned());
        lines.push(format!("  power          {watts} W now, drawn by {whose}"));
    }
    if let (Some(joules), Some(over)) = (
        figure("card_energy_joules"),
        figure("card_energy_over_seconds"),
    ) {
        lines.push(format!(
            "                 {joules} J over {over} s of holding it — not this model's alone"
        ));
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

#[allow(clippy::too_many_lines, reason = "one report, a line a fact")]
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
        match body.get("network_address").and_then(Value::as_text) {
            Some(network) => format!("  on the network {network} — with the key the hold was set"),
            None => "  on the network no: this computer only, unless the hold is opened with a key"
                .to_owned(),
        },
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
    match body.get("takes") {
        Some(takes) if !matches!(takes, Value::Null) => {
            let takes = mcf_serve::takes::Takes::from_value(takes);
            lines.push(format!("  takes          {}", takes.media()));
            lines.push(format!("  template       {}", takes.template()));
            lines.push(format!("  thinking       {}", takes.thinking_said()));
        }
        _ => lines.push("  takes          the engine did not say what it takes".to_owned()),
    }
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

fn to_the_second(at: &str) -> String {
    at.get(..19)
        .map_or_else(|| at.to_owned(), |head| format!("{head}Z"))
}
