mod acquire;
mod ask;
mod check;
mod desk;
mod doctor;
mod downloads;
mod explain;
mod failures;
mod hosting;
mod licence;
mod log;
mod models;
mod provision;
mod pull;
mod say;
mod serve;
mod share;
mod support;
mod tui;

use std::process::ExitCode;

use mcf_core::build_identity::BuildIdentity;

#[derive(Debug, Clone, PartialEq, Eq)]
enum Request<'a> {
    Version,
    Share {
        into: Option<&'a str>,
    },
    Licence {
        full: bool,
    },
    Usage,
    CommandUsage {
        command: &'a str,
    },
    NameExpected {
        command: &'a str,
        argument: &'a str,
        needs: &'static str,
    },
    Export {
        to: &'a str,
    },
    Doctor {
        record: bool,
        as_json: bool,
    },
    Pull {
        reference: &'a str,
        from: Option<&'a str>,
        into: Option<&'a str>,
        offered: pull::Offered<'a>,
        fresh: bool,
    },
    Check {
        only: Option<&'a str>,
        reach: check::Reach,
        from: Option<&'a str>,
        offered: pull::Offered<'a>,
    },
    Serve,
    Desk,
    Tui,
    Ask {
        model: Option<&'a str>,
        prompt: &'a str,
        limit: Option<usize>,
        seed: u64,
        engine: Option<&'a str>,
    },
    Failures {
        last: Option<usize>,
    },
    Log {
        kind: Option<&'a str>,
        last: Option<usize>,
        full: bool,
    },
    Explain {
        model: &'a str,
        json: bool,
    },
    Support {
        into: Option<&'a str>,
    },
    Provision {
        name: Option<&'a str>,
        into: Option<&'a str>,
    },
    ProvisionList {
        into: Option<&'a str>,
    },
    ProvisionRemove {
        name: &'a str,
        because: Option<&'a str>,
        into: Option<&'a str>,
    },
    Offered {
        reference: &'a str,
    },
    Acquire {
        reference: &'a str,
        file: &'a str,
    },
    Downloads,
    Queue {
        reference: &'a str,
        file: &'a str,
    },
    AboutDownload {
        id: u64,
        about: downloads::About,
    },
    ForgetDownloads,
    Settings {
        at: Option<u64>,
        held_as: Option<mcf_core::configuration::CacheType>,
        model: &'a str,
    },
    Host {
        model: &'a str,
        changes: Vec<(String, mcf_record::json::Value)>,
        remember: bool,
    },
    Hosted,
    Unhost,
    Status,
    Stop {
        because: Option<&'a str>,
    },
    List,
    Remove {
        names: Vec<&'a str>,
        because: Option<&'a str>,
        purge: bool,
    },
    Unrecognized(&'a str),
    UnexpectedArgument {
        command: &'a str,
        argument: &'a str,
    },
    MissingArgument {
        command: &'a str,
        needs: &'static str,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Response {
    text: String,
    served: bool,
}

fn main() -> ExitCode {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    let borrowed: Vec<&str> = arguments.iter().map(String::as_str).collect();
    let response = respond(&parse(&borrowed), BuildIdentity::current());
    if response.served {
        println!("{}", response.text);
        ExitCode::SUCCESS
    } else {
        eprintln!("{}", response.text);
        ExitCode::FAILURE
    }
}

#[allow(
    clippy::too_many_lines,
    reason = "one arm per command, and a table of them is more readable in one \
              place than split across functions by an arbitrary line count"
)]
fn parse<'a>(arguments: &[&'a str]) -> Request<'a> {
    if let [command, argument, ..] = arguments
        && (!command.starts_with('-') || usage_of(command).is_some())
    {
        if matches!(*argument, "--help" | "-h") {
            return Request::CommandUsage { command };
        }
        if argument.starts_with('-')
            && let Some(needs) = name_wanted_first(command)
        {
            return Request::NameExpected {
                command,
                argument,
                needs,
            };
        }
    }
    match arguments {
        ["--version" | "-V"] => Request::Version,
        ["licence" | "license" | "--licence" | "--license"] => Request::Licence { full: false },
        ["licence" | "license" | "--licence" | "--license", "--full"] => {
            Request::Licence { full: true }
        }
        [] if on_a_terminal() => Request::Tui,
        [] | ["--help" | "-h"] => Request::Usage,
        ["export", "--to", to] => Request::Export { to },
        ["export", rest @ ..] => match rest.first() {
            Some(argument) => Request::UnexpectedArgument {
                command: "export",
                argument,
            },
            None => Request::MissingArgument {
                command: "export",
                needs: "--to <path>",
            },
        },
        ["check", rest @ ..] => match check_options(rest) {
            Ok(request) => request,
            Err(argument) => Request::UnexpectedArgument {
                command: "check",
                argument,
            },
        },
        ["pull", rest @ ..] => match pull_options(rest) {
            Ok(request) => request,
            Err(argument) => Request::UnexpectedArgument {
                command: "pull",
                argument,
            },
        },
        ["serve"] => Request::Serve,
        ["tui"] => Request::Tui,
        ["desk"] => Request::Desk,
        ["desk", argument, ..] => Request::UnexpectedArgument {
            command: "desk",
            argument,
        },
        ["tui", argument, ..] => Request::UnexpectedArgument {
            command: "tui",
            argument,
        },
        ["serve", argument, ..] => Request::UnexpectedArgument {
            command: "serve",
            argument,
        },
        ["failures"] => Request::Failures { last: None },
        ["failures", "--last", count] => match count.parse() {
            Ok(count) => Request::Failures { last: Some(count) },
            Err(_) => Request::MissingArgument {
                command: "failures",
                needs: "--last <n>, a number",
            },
        },
        ["failures", argument, ..] => Request::UnexpectedArgument {
            command: "failures",
            argument,
        },
        ["log", rest @ ..] => match log_options(rest) {
            Ok(request) => request,
            Err(argument) => Request::UnexpectedArgument {
                command: "log",
                argument,
            },
        },
        ["explain", model] => Request::Explain { model, json: false },
        ["explain", model, "--json"] => Request::Explain { model, json: true },
        ["explain"] => Request::MissingArgument {
            command: "explain",
            needs: "<model>",
        },
        ["explain", _, argument, ..] => Request::UnexpectedArgument {
            command: "explain",
            argument,
        },
        ["settings", model] => Request::Settings {
            model,
            at: None,
            held_as: None,
        },
        ["settings", model, "--cache", held_as] => {
            match mcf_core::configuration::CacheType::parse(held_as) {
                Some(width) => Request::Settings {
                    model,
                    at: None,
                    held_as: Some(width),
                },
                None => Request::UnexpectedArgument {
                    command: "settings",
                    argument: held_as,
                },
            }
        }
        ["settings", model, "--context", at, "--cache", held_as]
        | ["settings", model, "--cache", held_as, "--context", at] => {
            match (
                at.parse::<u64>(),
                mcf_core::configuration::CacheType::parse(held_as),
            ) {
                (Ok(at), Some(width)) => Request::Settings {
                    model,
                    at: Some(at),
                    held_as: Some(width),
                },
                (Err(_), _) => Request::UnexpectedArgument {
                    command: "settings",
                    argument: at,
                },
                (_, None) => Request::UnexpectedArgument {
                    command: "settings",
                    argument: held_as,
                },
            }
        }
        ["settings", model, "--context", at] => match at.parse::<u64>() {
            Ok(at) => Request::Settings {
                model,
                at: Some(at),
                held_as: None,
            },
            Err(_) => Request::UnexpectedArgument {
                command: "settings",
                argument: at,
            },
        },
        ["settings"] => Request::MissingArgument {
            command: "settings",
            needs: "<model>",
        },
        ["ask", rest @ ..] => match ask_options(rest) {
            Ok(request) => request,
            Err(argument) => Request::UnexpectedArgument {
                command: "ask",
                argument,
            },
        },
        ["hosted"] => Request::Hosted,
        ["unhost"] => Request::Unhost,
        ["host"] => Request::MissingArgument {
            command: "host",
            needs: "<model>",
        },
        ["host", model, rest @ ..] => match host_options(rest) {
            Ok((changes, remember)) => Request::Host {
                model,
                changes,
                remember,
            },
            Err(argument) => Request::UnexpectedArgument {
                command: "host",
                argument,
            },
        },
        ["offered", reference] => Request::Offered { reference },
        ["offered"] => Request::MissingArgument {
            command: "offered",
            needs: "<owner/name>",
        },
        ["acquire", reference, file] => Request::Acquire { reference, file },
        ["acquire"] | ["acquire", _] => Request::MissingArgument {
            command: "acquire",
            needs: "<owner/name> <file>",
        },
        ["provision", "--list"] => Request::ProvisionList { into: None },
        ["provision", "--list", "--into", into] => Request::ProvisionList { into: Some(into) },
        ["provision", "--remove", name] => Request::ProvisionRemove {
            name,
            because: None,
            into: None,
        },
        ["provision", "--remove", name, "--because", because] => Request::ProvisionRemove {
            name,
            because: Some(because),
            into: None,
        },
        [
            "provision",
            "--remove",
            name,
            "--because",
            because,
            "--into",
            into,
        ] => Request::ProvisionRemove {
            name,
            because: Some(because),
            into: Some(into),
        },
        ["provision", "--into", into] => Request::Provision {
            name: None,
            into: Some(into),
        },
        ["provision", name] => Request::Provision {
            name: Some(name),
            into: None,
        },
        ["provision", name, "--into", into] => Request::Provision {
            name: Some(name),
            into: Some(into),
        },
        ["provision"] => Request::Provision {
            name: None,
            into: None,
        },
        ["provision", _, argument, ..] => Request::UnexpectedArgument {
            command: "provision",
            argument,
        },
        ["share"] => Request::Share { into: None },
        ["share", "--into", into] => Request::Share { into: Some(into) },
        ["share", argument, ..] => Request::UnexpectedArgument {
            command: "share",
            argument,
        },
        ["downloads"] => Request::Downloads,
        ["downloads", "add", reference, file] => Request::Queue { reference, file },
        ["downloads", "clear"] => Request::ForgetDownloads,
        ["downloads", told @ ("pause" | "resume" | "cancel"), id] => match id.parse::<u64>() {
            Ok(id) => Request::AboutDownload {
                id,
                about: match *told {
                    "pause" => downloads::About::Pause,
                    "resume" => downloads::About::Resume,
                    _ => downloads::About::Cancel,
                },
            },
            Err(_) => Request::NameExpected {
                command: "downloads",
                argument: id,
                needs: "<id>",
            },
        },
        ["downloads", "add", ..] => Request::MissingArgument {
            command: "downloads",
            needs: "add <owner/name> <file>",
        },
        ["downloads", "pause" | "resume" | "cancel"] => Request::MissingArgument {
            command: "downloads",
            needs: "<id>",
        },
        ["downloads", argument, ..] => Request::UnexpectedArgument {
            command: "downloads",
            argument,
        },
        ["status"] => Request::Status,
        ["status", argument, ..] => Request::UnexpectedArgument {
            command: "status",
            argument,
        },
        ["stop"] => Request::Stop { because: None },
        ["stop", "--because", reason] => Request::Stop {
            because: Some(reason),
        },
        ["stop", "--because"] => Request::MissingArgument {
            command: "stop",
            needs: "--because <why>",
        },
        ["stop", argument, ..] => Request::UnexpectedArgument {
            command: "stop",
            argument,
        },
        ["support"] => Request::Support { into: None },
        ["support", "--into", path] => Request::Support { into: Some(path) },
        ["support", argument, ..] => Request::UnexpectedArgument {
            command: "support",
            argument,
        },
        ["list"] => Request::List,
        ["list", argument, ..] => Request::UnexpectedArgument {
            command: "list",
            argument,
        },
        ["rm", rest @ ..] => match remove_options(rest) {
            Ok(request) => request,
            Err(argument) => Request::UnexpectedArgument {
                command: "rm",
                argument,
            },
        },
        ["doctor", rest @ ..] => match doctor_options(rest) {
            Ok(request) => request,
            Err(argument) => Request::UnexpectedArgument {
                command: "doctor",
                argument,
            },
        },
        [
            command @ ("--version" | "-V" | "--help" | "-h" | "licence" | "license" | "--licence"
            | "--license"),
            argument,
            ..,
        ] => Request::UnexpectedArgument { command, argument },
        [command, _, argument, ..] if usage_of(command).is_some() => {
            Request::UnexpectedArgument { command, argument }
        }
        [first, ..] => Request::Unrecognized(first),
    }
}

fn usage_of(command: &str) -> Option<&'static str> {
    let mut at = 0;
    let mut start = None;
    for line in COMMANDS.lines() {
        if let Some(introduced) = introduces(line) {
            match start {
                Some(from) => return COMMANDS.get(from..at),
                None if introduced == command => start = Some(at),
                None => {}
            }
        }
        at += line.len() + 1;
    }
    COMMANDS.get(start?..)
}

fn introduces(line: &str) -> Option<&str> {
    line.strip_prefix("  mcf ")?.split_whitespace().next()
}

fn name_wanted_first(command: &str) -> Option<&'static str> {
    let line = COMMANDS
        .lines()
        .find(|line| introduces(line) == Some(command))?;
    let after = line
        .strip_prefix("  mcf ")?
        .strip_prefix(command)?
        .trim_start();
    if !after.starts_with('<') {
        return None;
    }
    let close = after.find('>')?;
    after.get(..=close)
}

fn export(to: &std::path::Path) -> Response {
    let Some(journal) = mcf_record::journal::default_path() else {
        return Response {
            text: "mcf: there is no record to export — neither XDG_DATA_HOME nor HOME is set"
                .to_owned(),
            served: false,
        };
    };
    if !journal.exists() {
        return Response {
            text: format!(
                "mcf: there is no record at {} yet — run `mcf doctor` first",
                journal.display()
            ),
            served: false,
        };
    }

    match mcf_record::export::write(&journal, to, mcf_record::export::Kind::Export) {
        Ok(manifest) => Response {
            text: format!(
                "wrote {}\n\
                 \x20 {} entries · sha256:{}\n\
                 {}\
                 \x20 nothing has left this machine — sending a bundle is a separate, \
                 itemized act (A24)",
                to.display(),
                manifest.entries,
                manifest.digest,
                if manifest.content_entries == 0 {
                    "\x20 no prompt or completion content: the record keeps a length and a \
                     digest, and\n\x20 the text is in the content store, which this does not \
                     read (A25)\n"
                        .to_owned()
                } else {
                    format!(
                        "\x20 {} of those entries hold a prompt or a completion, recorded \
                         before MCF\n\x20 kept content out of its record. They are in this \
                         file. Nothing written\n\x20 since does that, and the record is not \
                         edited to look better (A25, A1, F105)\n",
                        manifest.content_entries
                    )
                }
            ),
            served: true,
        },
        Err(failure) => Response {
            text: format!("mcf: the record could not be exported\n  {failure}"),
            served: false,
        },
    }
}

type Change = (String, mcf_record::json::Value);

fn thousandths(said: Option<&str>) -> Result<mcf_record::json::Value, &'static str> {
    let held: mcf_core::configuration::Thousandths = said
        .ok_or("a sampling setting with no value")?
        .parse()
        .map_err(|()| "a sampling setting whose value is not a number like 0.2")?;
    Ok(mcf_record::json::Value::Integer(i64::from(held.0)))
}

fn host_options(rest: &[&str]) -> Result<(Vec<Change>, bool), &'static str> {
    use mcf_record::json::Value;
    let mut changes = Vec::new();
    let mut remember = false;
    let mut at = 0;
    while at < rest.len() {
        let Some(flag) = rest.get(at) else { break };
        let said = rest.get(at + 1).copied();
        let number = |name: &str| -> Result<Change, &'static str> {
            let held: i64 = said
                .ok_or("a setting with no value")?
                .parse()
                .map_err(|_| "a setting whose value is not a number")?;
            Ok((name.to_owned(), Value::Integer(held)))
        };
        if *flag == "--remember" {
            remember = true;
            at += 1;
            continue;
        }
        let Some(change) = one_setting(flag, said, &number)? else {
            return Err("a setting mcf host does not take");
        };
        changes.push(change);
        at += 2;
    }
    Ok((changes, remember))
}

#[allow(
    clippy::too_many_lines,
    reason = "one flag a line, which is the readable shape for a list of them"
)]
fn one_setting(
    flag: &str,
    said: Option<&str>,
    number: &dyn Fn(&str) -> Result<Change, &'static str>,
) -> Result<Option<Change>, &'static str> {
    use mcf_record::json::Value;
    Ok(Some(match flag {
        "--context" => number("context")?,
        "--gpu-layers" => number("gpu_layers")?,
        "--on" => (
            "on".to_owned(),
            Value::text(
                mcf_serve::control::On::parse(said.ok_or("--on with no value")?)
                    .ok_or("--on wants cpu or gpu")?
                    .as_str()
                    .to_owned(),
            ),
        ),
        "--threads" => number("threads")?,
        "--batch" => number("batch")?,
        "--ubatch" => number("ubatch")?,
        "--answers" => (
            "answers".to_owned(),
            Value::text(
                mcf_serve::hosting::Answers::parse(said.ok_or("--answers with no value")?)
                    .ok_or("--answers wants chat, embeddings or reranking")?
                    .as_str(),
            ),
        ),
        "--pooling" => (
            "pooling".to_owned(),
            Value::text(
                mcf_serve::hosting::Pooling::parse(said.ok_or("--pooling with no value")?)
                    .ok_or("--pooling wants none, mean, cls, last or rank")?
                    .as_str(),
            ),
        ),
        "--alias" => (
            "alias".to_owned(),
            Value::text(said.ok_or("--alias with no value")?),
        ),
        "--adapter" => (
            "adapters".to_owned(),
            Value::List(vec![Value::text(said.ok_or("--adapter with no value")?)]),
        ),
        "--threads-batch" => number("threads_batch")?,
        "--loading" => (
            "loading".to_owned(),
            Value::text(
                mcf_serve::hosting::Loading::parse(said.ok_or("--loading with no value")?)
                    .ok_or("--loading wants auto, none, mmap, mlock, mmap+mlock or dio")?
                    .as_str(),
            ),
        ),
        "--large-tensors" => (
            "lazily".to_owned(),
            Value::text(
                mcf_serve::hosting::Lazily::parse(said.ok_or("--large-tensors with no value")?)
                    .ok_or("--large-tensors wants auto, on or off")?
                    .as_str(),
            ),
        ),
        "--slots" => number("slots")?,
        "--cache-reuse" => number("cache_reuse")?,
        "--prompt-cache-memory" => number("prompt_cache_mib")?,
        "--checkpoints" => number("checkpoints")?,
        "--checkpoint-spacing" => number("checkpoint_min_step")?,
        "--keep" => number("keep")?,
        "--cache-on" => (
            "cache_on_processor".to_owned(),
            Value::Bool(said == Some("cpu")),
        ),
        "--split-mode" => (
            "split_mode".to_owned(),
            Value::text(
                mcf_serve::hosting::Split::parse(said.ok_or("--split-mode with no value")?)
                    .ok_or("--split-mode wants layer, none, row or tensor")?
                    .as_str(),
            ),
        ),
        "--experts-on" => {
            let said = said.ok_or("--experts-on with no value")?;
            (
                "experts".to_owned(),
                match said {
                    "cpu" => Value::text("all"),
                    "model" => Value::Null,
                    layers => Value::Integer(
                        layers
                            .parse()
                            .map_err(|_| "--experts-on wants cpu, model, or a layer count")?,
                    ),
                },
            )
        }
        "--dense-layers-on-cpu" => number("ffn_layers_on_processor")?,
        "--main-device" => number("main_device")?,
        "--devices" => (
            "devices".to_owned(),
            Value::text(said.ok_or("--devices with no value")?),
        ),
        "--override-tensor" => (
            "override_tensors".to_owned(),
            Value::text(said.ok_or("--override-tensor with no value")?),
        ),
        "--prompt-cache" => ("prompt_cache".to_owned(), Value::Bool(said == Some("on"))),
        "--idle-slots" => ("idle_slots".to_owned(), Value::Bool(said == Some("on"))),
        "--context-shift" => ("context_shift".to_owned(), Value::Bool(said == Some("on"))),
        "--port" => number("port")?,
        "--engine" => (
            "engine".to_owned(),
            Value::text(said.ok_or("--engine with no value")?),
        ),
        "--api-key" => (
            "api_key".to_owned(),
            Value::text(said.ok_or("--api-key with no value")?),
        ),
        "--flash-attention" => (
            "flash_attention".to_owned(),
            Value::Bool(said == Some("on")),
        ),
        "--keep-resident" => ("keep_resident".to_owned(), Value::Bool(said == Some("on"))),
        "--cache" => (
            "cache".to_owned(),
            Value::text(
                mcf_core::configuration::CacheType::parse(said.ok_or("--cache with no value")?)
                    .ok_or("--cache wants f32, f16, bf16, q8_0, q5_1, q5_0, q4_1, q4_0 or iq4_nl")?
                    .as_str(),
            ),
        ),
        "--open" => ("open".to_owned(), Value::Bool(said == Some("on"))),
        "--draft-head" => ("draft_head".to_owned(), Value::Bool(said == Some("on"))),
        "--rope-scaling" => (
            "rope_scaling".to_owned(),
            Value::text(said.ok_or("--rope-scaling with no value")?),
        ),
        "--rope-scale" => number("rope_scale")?,
        "--draft-depth" => number("drafted")?,
        "--trained-window" => number("trained")?,
        "--lift-ceiling" => number("lift")?,
        "--thinking-budget" => number("thinking")?,
        "--thinking-level" => (
            "effort".to_owned(),
            Value::text(said.ok_or("--thinking-level with no level")?),
        ),
        "--temperature" => ("temperature".to_owned(), thousandths(said)?),
        "--top-p" => ("top_p".to_owned(), thousandths(said)?),
        "--top-k" => number("top_k")?,
        _ => return Ok(None),
    }))
}

fn on_a_terminal() -> bool {
    #[allow(
        unsafe_code,
        reason = "asking the C library whether stdout is a terminal"
    )]
    unsafe {
        unsafe extern "C" {
            fn isatty(fd: i32) -> i32;
        }
        isatty(1) == 1
    }
}

fn ask_options<'a>(arguments: &[&'a str]) -> Result<Request<'a>, &'a str> {
    let mut model = None;
    let mut prompt = None;
    let mut engine = None;
    let mut limit = None;
    let mut seed = 0;
    let mut rest = arguments;
    while let Some((flag, after)) = rest.split_first() {
        match *flag {
            "--prompt" => match after.split_first() {
                Some((text, tail)) => {
                    prompt = Some(*text);
                    rest = tail;
                }
                None => return Err("--prompt"),
            },
            "--limit" => match after.split_first() {
                Some((held, tail)) => {
                    limit = Some(held.parse::<usize>().map_err(|_| *held)?);
                    rest = tail;
                }
                None => return Err("--limit"),
            },
            "--engine" => match after.split_first() {
                Some((named, tail)) => {
                    engine = Some(*named);
                    rest = tail;
                }
                None => return Err("--engine"),
            },
            "--seed" => match after.split_first() {
                Some((held, tail)) => {
                    seed = held.parse::<u64>().map_err(|_| *held)?;
                    rest = tail;
                }
                None => return Err("--seed"),
            },
            other if other.starts_with('-') => return Err(other),
            named if model.is_none() => {
                model = Some(named);
                rest = after;
            }
            other => return Err(other),
        }
    }
    match prompt {
        Some(prompt) => Ok(Request::Ask {
            model,
            prompt,
            limit,
            seed,
            engine,
        }),
        None => Ok(Request::MissingArgument {
            command: "ask",
            needs: "--prompt <text>",
        }),
    }
}

fn log_options<'a>(arguments: &[&'a str]) -> Result<Request<'a>, &'a str> {
    let mut kind = None;
    let mut last = None;
    let mut full = false;
    let mut rest = arguments.iter();
    while let Some(argument) = rest.next() {
        match *argument {
            "--kind" => match rest.next() {
                Some(named) => kind = Some(*named),
                None => {
                    return Ok(Request::MissingArgument {
                        command: "log",
                        needs: "--kind <kind>",
                    });
                }
            },
            "--last" => match rest.next().and_then(|value| value.parse().ok()) {
                Some(count) => last = Some(count),
                None => {
                    return Ok(Request::MissingArgument {
                        command: "log",
                        needs: "--last <n>, a number",
                    });
                }
            },
            "--full" => full = true,
            other => return Err(other),
        }
    }
    Ok(Request::Log { kind, last, full })
}

fn pull_options<'a>(arguments: &[&'a str]) -> Result<Request<'a>, &'a str> {
    let mut reference = None;
    let mut from = None;
    let mut into = None;
    let mut offered = pull::Offered::Nothing;
    let mut fresh = false;
    let mut rest = arguments.iter();
    while let Some(argument) = rest.next() {
        match *argument {
            "--from" => match rest.next() {
                Some(hub) => from = Some(*hub),
                None => {
                    return Ok(Request::MissingArgument {
                        command: "pull",
                        needs: "--from <hub>",
                    });
                }
            },
            "--into" => match rest.next() {
                Some(path) => into = Some(*path),
                None => {
                    return Ok(Request::MissingArgument {
                        command: "pull",
                        needs: "--into <directory>",
                    });
                }
            },
            "--token-from" => match rest.next() {
                Some(path) => offered = pull::Offered::File(path),
                None => {
                    return Ok(Request::MissingArgument {
                        command: "pull",
                        needs: "--token-from <file>",
                    });
                }
            },
            "--token-from-env" => match rest.next() {
                Some(variable) => offered = pull::Offered::Variable(variable),
                None => {
                    return Ok(Request::MissingArgument {
                        command: "pull",
                        needs: "--token-from-env <VARIABLE>",
                    });
                }
            },
            "--fresh" => fresh = true,
            other if other.starts_with("--") => return Err(other),
            other if reference.is_none() => reference = Some(other),
            other => return Err(other),
        }
    }
    match reference {
        Some(reference) => Ok(Request::Pull {
            reference,
            from,
            into,
            offered,
            fresh,
        }),
        None => Ok(Request::MissingArgument {
            command: "pull",
            needs: "<owner/name[:file]>",
        }),
    }
}

fn check_options<'a>(arguments: &[&'a str]) -> Result<Request<'a>, &'a str> {
    let mut only = None;
    let mut reach = check::Reach::Everything;
    let mut from = None;
    let mut offered = pull::Offered::Nothing;
    let mut rest = arguments.iter();
    while let Some(argument) = rest.next() {
        match *argument {
            "--from" => match rest.next() {
                Some(hub) => from = Some(*hub),
                None => {
                    return Ok(Request::MissingArgument {
                        command: "check",
                        needs: "--from <hub>",
                    });
                }
            },
            "--token-from" => match rest.next() {
                Some(path) => offered = pull::Offered::File(path),
                None => {
                    return Ok(Request::MissingArgument {
                        command: "check",
                        needs: "--token-from <file>",
                    });
                }
            },
            "--token-from-env" => match rest.next() {
                Some(variable) => offered = pull::Offered::Variable(variable),
                None => {
                    return Ok(Request::MissingArgument {
                        command: "check",
                        needs: "--token-from-env <VARIABLE>",
                    });
                }
            },
            "--here" => reach = check::Reach::HereOnly,
            other if other.starts_with("--") => return Err(other),
            other if only.is_none() => only = Some(other),
            other => return Err(other),
        }
    }
    Ok(Request::Check {
        only,
        reach,
        from,
        offered,
    })
}

fn remove_options<'a>(arguments: &[&'a str]) -> Result<Request<'a>, &'a str> {
    let mut names = Vec::new();
    let mut because = None;
    let mut purge = false;
    let mut rest = arguments.iter();
    while let Some(argument) = rest.next() {
        match *argument {
            "--because" => match rest.next() {
                Some(reason) => because = Some(*reason),
                None => {
                    return Ok(Request::MissingArgument {
                        command: "rm",
                        needs: "--because <why>",
                    });
                }
            },
            "--purge" => purge = true,
            other if other.starts_with("--") => return Err(other),
            other => names.push(other),
        }
    }
    if names.is_empty() {
        return Ok(Request::MissingArgument {
            command: "rm",
            needs: "<model file>",
        });
    }
    Ok(Request::Remove {
        names,
        because,
        purge,
    })
}

fn doctor_options<'a>(arguments: &[&'a str]) -> Result<Request<'a>, &'a str> {
    let mut record = true;
    let mut as_json = false;
    for argument in arguments {
        match *argument {
            "--no-record" => record = false,
            "--json" => as_json = true,
            other => return Err(other),
        }
    }
    Ok(Request::Doctor { record, as_json })
}

const COMMANDS: &str = "\
    \x20 mcf desk                            MCF in a window: every screen a\n\
    \x20                                     client of the same daemon. Needs\n\
    \x20                                     SDL3 provisioned before MCF is\n\
    \x20                                     built, and says so if it is not\n\
    \x20 mcf tui                             the same screens with no display\n\
    \x20                                     attached; `mcf` with no arguments\n\
    \x20                                     opens it where there is a terminal\n\
    \x20 mcf ask --prompt <text>             ask whatever is being held, through\n\
    \x20         [--limit <n>] [--seed <n>]  the endpoint it is served on\n\
    \x20 mcf doctor [--no-record] [--json]   what this machine is, what MCF\n\
    \x20                                     costs here, and what it promises\n\
    \x20 mcf pull <owner/name[:file]>        bring a model here, with its\n\
    \x20          [--into <directory>]       provenance; without a file it\n\
    \x20          [--from <hub>]             says which variants would run\n\
    \x20          [--token-from <file>]      here. MCF reads a credential\n\
    \x20          [--token-from-env <VAR>]   only where you name one, and\n\
    \x20          [--fresh]                  puts models where MCF_MODELS\n\
    \x20                                     says unless --into names one;\n\
    \x20                                     a word searches the hub, kept\n\
    \x20                                     a day unless --fresh\n\
    \x20 mcf serve                           start the daemon: it stays up,\n\
    \x20                                     recovers what is on the disk and\n\
    \x20                                     costs nothing while idle\n\
    \x20 mcf provision [<component>]         build a pinned component in a\n\
    \x20     [--list] [--remove <c>          container, everything recorded,\n\
    \x20      --because <why>] [--into <dir>] removable without residue; unnamed,\n\
    \x20                                     the engine a model here needs (B-367)\n\
    \x20 mcf failures [--last <n>]          what went wrong, classified: the\n\
    \x20                                     record's newest failures, each\n\
    \x20                                     with its context\n\
    \x20 mcf log [--kind <kind>]             what happened on this machine,\n\
    \x20         [--last <n>] [--full]       read back out of the record\n\
    \x20 mcf explain <model> [--json]        what it declares, what MCF read,\n\
    \x20                                     what MCF would choose, and what\n\
    \x20                                     it cannot tell you; --json is\n\
    \x20                                     what the file holds, counted by\n\
    \x20                                     the daemon as the window reads it\n\
    \x20 mcf support [--into <path>]         what a maintainer would need to\n\
    \x20                                     read this machine's sensors, as a\n\
    \x20                                     file you read before you send it\n\
    \x20 mcf downloads                       the queue of files MCF is\n\
    \x20                                     bringing here: one line each,\n\
    \x20                                     how far along and what state\n\
    \x20 mcf downloads add                   ask for one without waiting for\n\
    \x20     <owner/name> <file>             it. Several can be on their way\n\
    \x20                                     at once, and they keep arriving\n\
    \x20                                     with nothing watching — unlike\n\
    \x20                                     `mcf pull`, which waits\n\
    \x20 mcf downloads pause <id>            stop one where it stands; what\n\
    \x20                                     arrived stays on the disk\n\
    \x20 mcf downloads resume <id>           carry on from wherever it got to\n\
    \x20 mcf downloads cancel <id>           give one up, and sweep what had\n\
    \x20                                     arrived\n\
    \x20 mcf downloads clear                 drop the finished ones from the\n\
    \x20                                     list\n\
    \x20 mcf status                          ask a running daemon what it is\n\
    \x20                                     and what it is holding\n\
    \x20 mcf stop [--because <why>]          ask it to stop, and say why\n\
    \x20 mcf host <model> [--context <n>]    hold a model on a port where\n\
    \x20      [--port <n>] [--engine <name>] another program can reach it;\n\
    \x20      [--on cpu|gpu] [--gpu-layers <n>] it prints the settings it\n\
    \x20      [--threads <n>]                chose and what they cost\n\
    \x20      [--batch <n>] [--api-key <key>] — including the model's own\n\
    \x20      [--flash-attention]            draft head, which a file can\n\
    \x20      [--draft-head on|off]          carry and the engine leaves\n\
    \x20      [--rope-scaling <kind>]        in it unless it is asked for;\n\
    \x20      [--rope-scale <n>]             --on puts it where you say;\n\
    \x20      [--trained-window <n>]         --trained-window is what a rope\n\
    \x20      [--lift-ceiling <n>]           scaling counts from, and\n\
    \x20      [--draft-depth <n>]            --lift-ceiling raises the window\n\
    \x20      [--thinking-budget <n>]        the file itself declares;\n\
    \x20                                     --thinking-budget stops reasoning\n\
    \x20                                     after n tokens, 0 at once;\n\
    \x20      [--cache <type>] [--slots <n>] --cache holds each cached token\n\
    \x20      [--cache-reuse <n>]            narrower, so the same memory\n\
    \x20      [--prompt-cache on|off]        holds a longer conversation;\n\
    \x20      [--prompt-cache-memory <mib>]  the prompt cache settings decide\n\
    \x20      [--idle-slots on|off]          what a second message reuses of\n\
    \x20      [--context-shift on|off]       the first, which is what a long\n\
    \x20      [--checkpoints <n>]            pause after a gap is made of\n\
    \x20      [--checkpoint-spacing <n>]\n\
    \x20      [--keep <n>]\n\
    \x20      [--cache-on cpu|gpu]           --cache-on cpu leaves the card's\n\
    \x20      [--split-mode <kind>]          whole pool to the weights;\n\
    \x20      [--experts-on cpu|model|<n>]   --experts-on cpu fits a mixture\n\
    \x20      [--dense-layers-on-cpu <n>]    of experts that would not fit\n\
    \x20      [--main-device <n>]            otherwise\n\
    \x20      [--devices <list>]\n\
    \x20      [--override-tensor <pattern>]\n\
    \x20      [--ubatch <n>]                 --ubatch is what the compute\n\
    \x20      [--threads-batch <n>]          buffers are built for, so it\n\
    \x20      [--loading <mode>]             is what to lower when a hold\n\
    \x20      [--large-tensors auto|on|off]  is a little short of fitting;\n\
    \x20      [--answers <kind>]             --answers holds an embedding or\n\
    \x20      [--pooling <kind>]             reranking model as itself, and\n\
    \x20      [--alias <name>]               --alias is the name callers ask\n\
    \x20      [--adapter <file>]             for instead of a path\n\
    \x20                                     narrower, so the same memory\n\
    \x20                                     holds a longer conversation\n\
    \x20 mcf hosted                          what is being held, and where\n\
    \x20 mcf settings <model>                every setting a model would run\n\
    \x20              [--context <n>]        under, and where each came from;\n\
    \x20                                     with a context, what that window\n\
    \x20                                     would reserve in memory\n\
    \x20 mcf unhost                          stop holding it, and give the\n\
    \x20                                     memory back\n\
    \x20 mcf list                            what this machine is holding\n\
    \x20 mcf check [<model>] [--here]        is what you hold still what it\n\
    \x20           [--from <hub>]            should be? the bytes against the\n\
    \x20                                     digest recorded for them, and the\n\
    \x20                                     hub against what it published\n\
    \x20 mcf rm <model> [--because <why>]    stop holding it: without a reason\n\
    \x20            [--purge]                this previews and removes nothing\n\
    \x20 mcf export --to <path>              the record, as one portable file\n\
    \x20 mcf share [--into <path>]           what would leave this machine,\n\
    \x20                                     row by row, before it does\n\
    \x20                                     (B-160, A24)\n\
    \x20 mcf offered <owner/name>            what a repository publishes, and\n\
    \x20                                     which of it will run here\n\
    \x20 mcf acquire <owner/name> <file>     fetch one published file through\n\
    \x20                                     the daemon\n\
    \x20 mcf licence [--full]                the terms, and what conveying this\n\
    \x20                                     binary obliges you to (GPL-3.0-only)\n\
    \x20 mcf --version                       what this binary is\n";

const NOTES: &str = "\
    Acquisition reaches an encrypted hub over MCF's own HTTP and a vendored\n\
    TLS stack, or a plain one where you name it — a mirror of your own.\n\
    \n\
    MCF holds one model at a time. `mcf host` puts it on a port where\n\
    another program can reach it, and `mcf unhost` gives the memory back.\n\
    \n\
    `mcf-helper` is beside this binary and does three things that need\n\
    rights this one does not have: the processor governor, a device's\n\
    exclusive mode, and the processor's energy counter";

#[allow(
    clippy::too_many_lines,
    reason = "one arm per request, for the same reason `parse` has one per command"
)]
fn respond(request: &Request<'_>, identity: BuildIdentity) -> Response {
    match request {
        Request::Version => Response {
            text: identity.to_string(),
            served: true,
        },
        Request::Licence { full } => Response {
            text: licence::render(identity, *full),
            served: true,
        },
        Request::Usage => Response {
            text: format!("{identity}\n\nusage:\n{COMMANDS}\n{NOTES}"),
            served: true,
        },
        Request::CommandUsage { command } => match usage_of(command) {
            Some(block) => Response {
                text: format!("usage:\n{block}"),
                served: true,
            },
            None => respond(&Request::Unrecognized(command), identity),
        },
        Request::NameExpected {
            command,
            argument,
            needs,
        } => Response {
            text: format!("mcf: {command} needs {needs} where it got {argument}"),
            served: false,
        },
        Request::Doctor { record, as_json } => {
            let report = doctor::run(*record);
            Response {
                text: if *as_json {
                    report.to_value().to_line()
                } else {
                    report.render()
                },
                served: true,
            }
        }
        Request::Pull {
            reference,
            from,
            into,
            offered,
            fresh,
        } => pull::run(reference, *from, *into, *offered, *fresh),
        Request::Serve => serve::run(),
        Request::Tui => tui::run(),
        Request::Desk => desk::run(),
        Request::Failures { last } => failures::run(*last),
        Request::Log { kind, last, full } => log::run(*kind, *last, *full),
        Request::Check {
            only,
            reach,
            from,
            offered,
        } => check::run(*only, *reach, *from, *offered),
        Request::Explain { model, json: false } => explain::run(model),
        Request::Explain { model, json: true } => explain::json(model),
        Request::Settings { model, at, held_as } => hosting::settings(model, *at, *held_as),
        Request::Host {
            model,
            changes,
            remember,
        } => hosting::host(model, changes, *remember),
        Request::Ask {
            model,
            prompt,
            limit,
            seed,
            engine,
        } => ask::ask(*model, prompt, *limit, *seed, *engine),
        Request::Hosted => hosting::held(),
        Request::Unhost => hosting::unhost(),
        Request::Offered { reference } => acquire::offered(reference, None, false),
        Request::Acquire { reference, file } => acquire::acquire(reference, file, None),
        Request::Downloads => downloads::listed(),
        Request::Queue { reference, file } => downloads::queue(reference, file, None),
        Request::AboutDownload { id, about } => downloads::about(*id, *about),
        Request::ForgetDownloads => downloads::forget(),
        Request::Provision { name, into } => provision::run(*name, *into),
        Request::ProvisionList { into } => provision::list(*into),
        Request::ProvisionRemove {
            name,
            because,
            into,
        } => provision::remove(name, *because, *into),
        Request::Share { into } => share::run(*into),
        Request::Status => serve::status(),
        Request::Stop { because } => serve::stop(because.unwrap_or_default()),
        Request::Support { into } => support::run(*into),
        Request::List => models::list(),
        Request::Remove {
            names,
            because,
            purge,
        } => models::remove(names, *because, *purge),
        Request::Unrecognized(argument) => Response {
            text: format!(
                "mcf: no such command: {argument}\n\
                 `mcf --help` lists the commands there are."
            ),
            served: false,
        },
        Request::UnexpectedArgument { command, argument } => Response {
            text: format!("mcf: {command} does not take: {argument}"),
            served: false,
        },
        Request::MissingArgument { command, needs } => Response {
            text: format!("mcf: {command} needs {needs}"),
            served: false,
        },
        Request::Export { to } => export(std::path::Path::new(to)),
    }
}

#[cfg(test)]
mod tests {
    use super::{Request, Response, parse, respond};
    use mcf_core::build_identity::BuildIdentity;

    #[test]
    fn no_arguments_is_usage_when_nothing_is_watching() {
        assert_eq!(parse(&[]), Request::Usage);
    }

    #[test]
    fn both_version_spellings_are_the_version() {
        assert_eq!(parse(&["--version"]), Request::Version);
        assert_eq!(parse(&["-V"]), Request::Version);
    }

    #[test]
    fn both_help_spellings_are_usage() {
        assert_eq!(parse(&["--help"]), Request::Usage);
        assert_eq!(parse(&["-h"]), Request::Usage);
    }

    #[test]
    fn an_unknown_command_is_named_and_carries_its_input() {
        let absent = "quinquagesima";
        assert!(
            !respond(&Request::Usage, BuildIdentity::current())
                .text
                .contains(absent),
            "the example command is one MCF offers, so this proves nothing"
        );
        assert_eq!(parse(&[absent]), Request::Unrecognized(absent));
        let Response { text, served } = respond(&parse(&[absent]), BuildIdentity::current());
        assert!(!served, "an absent command must not report success");
        assert!(text.contains(absent), "{text:?} does not say what it saw");
    }

    #[test]
    fn usage_advertises_what_exists_and_nothing_else() {
        let Response { text, .. } = respond(&Request::Usage, BuildIdentity::current());
        assert!(text.contains("mcf --version"), "{text}");
        assert!(text.contains("mcf doctor"), "{text}");
        assert!(text.contains("mcf export"), "{text}");
        assert!(text.contains("mcf licence"), "{text}");
        assert!(text.contains("mcf list"), "{text}");
        assert!(text.contains("mcf rm"), "{text}");
        assert!(text.contains("mcf serve"), "{text}");
        assert!(text.contains("mcf stop"), "{text}");
        assert!(text.contains("mcf status"), "{text}");
        assert!(text.contains("mcf ask"), "{text}");
        assert!(text.contains("mcf explain"), "{text}");
        assert!(text.contains("mcf log"), "{text}");
        assert!(text.contains("mcf failures"), "{text}");
        assert!(text.contains("mcf pull"), "{text}");
        assert!(text.contains("mcf check"), "{text}");
        assert!(text.contains("mcf provision"), "{text}");
        assert!(text.contains("mcf support"), "{text}");
        for unbuilt in ["mcf lab", "mcf recommend"] {
            assert!(
                !text.contains(unbuilt),
                "usage advertises {unbuilt}, which nothing has built"
            );
        }
        assert!(
            !text.contains("has not vendored"),
            "usage still says a vendored dependency is not vendored: {text}"
        );
        assert!(text.contains("mcf-helper"), "{text}");
    }

    #[test]
    fn export_needs_a_destination_and_says_so() {
        assert_eq!(
            parse(&["export", "--to", "/tmp/a.mcf"]),
            Request::Export { to: "/tmp/a.mcf" }
        );
        assert_eq!(
            parse(&["export"]),
            Request::MissingArgument {
                command: "export",
                needs: "--to <path>"
            }
        );
        assert_eq!(
            parse(&["export", "--somewhere"]),
            Request::UnexpectedArgument {
                command: "export",
                argument: "--somewhere"
            }
        );

        let Response { text, served } = respond(&parse(&["export"]), BuildIdentity::current());
        assert!(!served);
        assert!(text.contains("--to <path>"), "{text}");
    }

    #[test]
    fn doctor_reads_its_own_options() {
        assert_eq!(
            parse(&["doctor"]),
            Request::Doctor {
                record: true,
                as_json: false
            }
        );
        assert_eq!(
            parse(&["doctor", "--no-record", "--json"]),
            Request::Doctor {
                record: false,
                as_json: true
            }
        );
        assert_eq!(
            parse(&["doctor", "--quiet"]),
            Request::UnexpectedArgument {
                command: "doctor",
                argument: "--quiet"
            }
        );
    }

    #[test]
    fn explain_reads_its_own_options() {
        assert_eq!(
            parse(&["explain", "a-model"]),
            Request::Explain {
                model: "a-model",
                json: false
            }
        );
        assert_eq!(
            parse(&["explain", "a-model", "--json"]),
            Request::Explain {
                model: "a-model",
                json: true
            }
        );
        assert_eq!(
            parse(&["explain", "a-model", "--yaml"]),
            Request::UnexpectedArgument {
                command: "explain",
                argument: "--yaml"
            }
        );
    }

    #[test]
    fn version_is_the_build_identity_verbatim() {
        let identity = BuildIdentity::current();
        let Response { text, served } = respond(&Request::Version, identity);
        assert!(served);
        assert_eq!(text, identity.to_string());
    }

    #[test]
    fn every_input_reaches_a_named_request() {
        for argument in ["", "-", "--", "--verbose", "-x", "🙂", "--version=1"] {
            let request = parse(&[argument]);
            assert!(
                matches!(request, Request::Unrecognized(seen) if seen == argument),
                "{argument:?} produced {request:?}"
            );
        }
    }

    #[test]
    fn a_trailing_argument_is_its_own_outcome() {
        assert_eq!(
            parse(&["--version", "extra"]),
            Request::UnexpectedArgument {
                command: "--version",
                argument: "extra",
            },
        );
        let Response { text, served } =
            respond(&parse(&["--version", "extra"]), BuildIdentity::current());
        assert!(!served);
        assert!(
            text.contains("--version") && text.contains("extra"),
            "{text:?}"
        );
        assert!(
            !text.contains("no such command"),
            "{text:?} denies a command that exists"
        );
    }

    #[test]
    fn help_asked_of_a_command_is_its_usage_and_not_a_run() {
        for command in commands() {
            for help in ["--help", "-h"] {
                let request = parse(&[command, help]);
                assert_eq!(
                    request,
                    Request::CommandUsage { command },
                    "mcf {command} {help}"
                );
                let Response { text, served } = respond(&request, BuildIdentity::current());
                assert!(served, "mcf {command} {help}: {text}");
                assert!(
                    text.starts_with("usage:\n  mcf ") && text.contains(command),
                    "mcf {command} {help}: {text}"
                );
                assert_eq!(
                    text.lines()
                        .filter(|line| line.starts_with("  mcf "))
                        .count(),
                    1,
                    "mcf {command} {help}: {text}"
                );
            }
        }
        assert_eq!(
            parse(&["quinquagesima", "--help"]),
            Request::CommandUsage {
                command: "quinquagesima"
            }
        );
        let Response { text, served } = respond(
            &parse(&["quinquagesima", "--help"]),
            BuildIdentity::current(),
        );
        assert!(!served && text.contains("no such command"), "{text}");
    }

    #[test]
    fn a_command_the_table_has_is_never_denied() {
        for command in commands() {
            let Response { text, .. } = respond(
                &parse(&[command, "foo", "--bogus"]),
                BuildIdentity::current(),
            );
            assert!(
                !text.contains("no such command"),
                "mcf {command} foo --bogus: {text:?} denies a command that exists"
            );
        }
        assert_eq!(
            parse(&["settings", "foo", "--bogus"]),
            Request::UnexpectedArgument {
                command: "settings",
                argument: "--bogus",
            }
        );
    }

    fn commands() -> Vec<&'static str> {
        super::COMMANDS
            .lines()
            .filter_map(super::introduces)
            .collect()
    }
}
