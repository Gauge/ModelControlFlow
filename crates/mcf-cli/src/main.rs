//! `mcf` — the headless surface.
//!
//! A22 makes this the complete surface rather than a convenience wrapper: any
//! action reachable only through an interface is one the laboratory cannot
//! test, which A19 already forbids. Every command MCF grows appears here
//! first, and the interface of §XI (M4) becomes a client of the same API.
//!
//! At M0 the surface is `mcf doctor` and `mcf --version`. `doctor` is the
//! milestone's product: it reports what this machine is, what MCF costs on it,
//! and what MCF will and will not promise here, and writes the whole thing to
//! the record.

mod bench;
mod check;
mod crosscheck;
mod doctor;
mod embed;
mod explain;
mod licence;
mod log;
mod models;
mod probe;
mod provision;
mod pull;
mod run;
mod say;
mod serve;

use std::process::ExitCode;

use mcf_core::build_identity::BuildIdentity;

/// What the process was asked to do.
///
/// A2's habit at the smallest scale: an unrecognized argument is a named
/// outcome carrying what it saw, never an ignored one and never a silent
/// fallback to help text.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Request<'a> {
    /// Report what this binary is and what built it.
    Version,
    /// State the licence, and the obligations that come with conveying this
    /// binary (B-330, D28).
    Licence {
        /// Whether to print the whole text, which is compiled in.
        full: bool,
    },
    /// Report the surface that exists.
    Usage,
    /// Write the record to one portable file.
    Export {
        /// Where to write it.
        to: &'a str,
    },
    /// Report what this machine is and what MCF costs on it.
    Doctor {
        /// Whether to write the report to the record.
        record: bool,
        /// Whether to render the record's own JSON rather than the report.
        ///
        /// A22 makes the headless path complete, and "complete" includes being
        /// consumable by something other than a person: the interface of §XI
        /// is a client of the same surface (B22), and a surface a script cannot
        /// read is one only a person can drive.
        as_json: bool,
    },
    /// Bring a model onto this machine.
    Pull {
        /// The reference, as the operator wrote it.
        reference: &'a str,
        /// A hub other than the default.
        from: Option<&'a str>,
        /// Which store to put it in, where the operator named one.
        into: Option<&'a str>,
        /// Where MCF may read a credential from, if the operator named one.
        offered: pull::Offered<'a>,
    },
    /// Check what this machine holds: the bytes, and where they came from.
    Check {
        /// One artifact, by any part of its path; every one when absent.
        only: Option<&'a str>,
        /// How much of the question to ask.
        reach: check::Reach,
        /// A hub other than the default.
        from: Option<&'a str>,
        /// Where MCF may read a credential from, if the operator named one.
        offered: pull::Offered<'a>,
    },
    /// Start the daemon and stay there.
    Serve,
    /// Read the record back.
    Log {
        /// Only entries of this kind.
        kind: Option<&'a str>,
        /// How many of the most recent to show.
        last: Option<usize>,
        /// The record's own JSON rather than a summary.
        full: bool,
    },
    /// Say what a model declares and what MCF would do with it.
    Explain {
        /// The model: a path, or something `mcf list` names.
        model: &'a str,
    },
    /// Ask a model something, with MCF's own engine.
    Run {
        /// The model: a path, or something `mcf list` names.
        model: &'a str,
        /// What to ask it.
        prompt: &'a str,
        /// How many tokens to produce at most.
        limit: Option<usize>,
        /// The seed, which is a condition of the answer (D19).
        seed: u64,
        /// Which engine, where the operator says (B-032).
        engine: Option<&'a str>,
    },
    /// Compare two models on a timeable engine, with no pass condition.
    Bench {
        /// The left arm: a path, or something `mcf list` names.
        left: &'a str,
        /// The right arm.
        right: &'a str,
        /// What to ask both of them.
        prompt: &'a str,
        /// How many tokens to produce at most.
        limit: Option<usize>,
        /// The seed, which is a condition of the answer and of the order the
        /// arms were drawn in (D19, B53).
        seed: u64,
        /// Which engine, where the operator says (B-032).
        engine: Option<&'a str>,
        /// The difference the caller cares about, in parts per million.
        resolving: Option<u64>,
        /// Whether every trial must load the model for itself (§6.13, F65).
        cold: bool,
    },
    /// Install, build and pin a component in a controlled environment.
    Provision {
        /// Which component, from the table MCF carries.
        name: &'a str,
        /// A prefix root other than the default.
        into: Option<&'a str>,
    },
    /// Say what can be provisioned and what is.
    ProvisionList {
        /// A prefix root other than the default.
        into: Option<&'a str>,
    },
    /// Remove a provisioned component, with the reason.
    ProvisionRemove {
        /// Which component.
        name: &'a str,
        /// Why it is going.
        because: Option<&'a str>,
        /// A prefix root other than the default.
        into: Option<&'a str>,
    },
    /// Compare MCF's own engine against the one it provisioned.
    CrossCheck {
        /// The model both engines read.
        model: &'a str,
    },
    /// Ask a model to do the thing, and report what it did.
    Probe {
        /// The model: a path, or something `mcf list` names.
        model: &'a str,
        /// Which engine to ask through, if the caller named one.
        engine: Option<&'a str>,
        /// Whether to apply what was observed, which is an act (D43).
        apply: bool,
    },
    /// Ask an embedding model for a vector.
    Embed {
        /// The model: a path, or something `mcf list` names.
        model: &'a str,
        /// The text to embed.
        text: &'a str,
    },
    /// Ask a running daemon what it is.
    Status,
    /// Ask a running daemon to stop.
    Stop {
        /// Why, which the daemon records rather than being killed silently.
        because: Option<&'a str>,
    },
    /// What this machine is holding.
    List,
    /// Stop holding something.
    Remove {
        /// What to remove, as the operator named it.
        names: Vec<&'a str>,
        /// Why — which is the authorization. Without it this previews.
        because: Option<&'a str>,
        /// Whether to delete what the removal shelves.
        purge: bool,
    },
    /// A command MCF does not have. Carries what was asked for, so the outcome
    /// can say it back.
    Unrecognized(&'a str),
    /// A command MCF has, given an argument it does not take. Distinct from
    /// [`Request::Unrecognized`] because telling an operator that `--version`
    /// is not a command when it is would be the wrong answer stated
    /// confidently, which P1 puts below saying nothing.
    UnexpectedArgument {
        /// The command that was recognized.
        command: &'a str,
        /// The first argument it does not take.
        argument: &'a str,
    },
    /// A command MCF has, without something it needs.
    ///
    /// Distinct from an unrecognized argument for the reason
    /// [`Request::UnexpectedArgument`] is distinct from
    /// [`Request::Unrecognized`]: telling an operator that `export` takes no
    /// arguments when it requires one would be a confident wrong answer.
    MissingArgument {
        /// The command.
        command: &'a str,
        /// What it needs.
        needs: &'static str,
    },
}

/// What MCF is going to do about it.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Response {
    /// The text the operator sees.
    text: String,
    /// Whether the request was one MCF could serve.
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

/// Reads the command line. Total: every input reaches a named request.
#[allow(
    clippy::too_many_lines,
    reason = "one arm per command, and a table of them is more readable in one \
              place than split across functions by an arbitrary line count"
)]
fn parse<'a>(arguments: &[&'a str]) -> Request<'a> {
    match arguments {
        ["--version" | "-V"] => Request::Version,
        // Both spellings, because the SPDX identifier and half the world spell
        // it one way and this project's documents spell it the other. A
        // redistributor looking for their obligations should not have to guess.
        ["licence" | "license" | "--licence" | "--license"] => Request::Licence { full: false },
        ["licence" | "license" | "--licence" | "--license", "--full"] => {
            Request::Licence { full: true }
        }
        [] | ["--help" | "-h"] => Request::Usage,
        ["export", "--to", to] => Request::Export { to },
        // A2: an `export` with no destination is a named outcome carrying what
        // it saw, not a guess at where the operator wanted the file.
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
        ["serve", argument, ..] => Request::UnexpectedArgument {
            command: "serve",
            argument,
        },
        ["log", rest @ ..] => match log_options(rest) {
            Ok(request) => request,
            Err(argument) => Request::UnexpectedArgument {
                command: "log",
                argument,
            },
        },
        ["explain", model] => Request::Explain { model },
        ["explain"] => Request::MissingArgument {
            command: "explain",
            needs: "<model>",
        },
        ["explain", _, argument, ..] => Request::UnexpectedArgument {
            command: "explain",
            argument,
        },
        ["cross-check", model] => Request::CrossCheck { model },
        ["cross-check"] => Request::MissingArgument {
            command: "cross-check",
            needs: "<model>",
        },
        ["probe", model] => Request::Probe {
            model,
            engine: None,
            apply: false,
        },
        // The act D43 requires. It is a flag rather than a default because
        // that is the whole of the decision: MCF may learn better, and what it
        // does with that is say so until somebody asks for the change.
        ["probe", model, "--apply"] => Request::Probe {
            model,
            engine: None,
            apply: true,
        },
        ["probe", model, "--engine", engine, "--apply"] => Request::Probe {
            model,
            engine: Some(engine),
            apply: true,
        },
        // Naming the engine is the point rather than a convenience: a probe
        // result belongs to the engine it was taken through (D42), and until
        // the two are shown to agree, which one answered is part of the
        // result (B-376).
        ["probe", model, "--engine", engine] => Request::Probe {
            model,
            engine: Some(engine),
            apply: false,
        },
        ["probe"] => Request::MissingArgument {
            command: "probe",
            needs: "<model>",
        },
        ["probe", _, argument, ..] if argument != &"--engine" && argument != &"--apply" => {
            Request::UnexpectedArgument {
                command: "probe",
                argument,
            }
        }
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
        ["provision", name] => Request::Provision { name, into: None },
        ["provision", name, "--into", into] => Request::Provision {
            name,
            into: Some(into),
        },
        ["provision"] => Request::MissingArgument {
            command: "provision",
            needs: "<component>, or --list",
        },
        ["provision", _, argument, ..] => Request::UnexpectedArgument {
            command: "provision",
            argument,
        },
        ["embed", model, "--text", text] => Request::Embed { model, text },
        ["embed", _model, "--text"] => Request::MissingArgument {
            command: "embed",
            needs: "--text <text>",
        },
        ["embed", _model] => Request::MissingArgument {
            command: "embed",
            needs: "--text <text>",
        },
        ["embed"] => Request::MissingArgument {
            command: "embed",
            needs: "<model> --text <text>",
        },
        ["embed", _, argument, ..] => Request::UnexpectedArgument {
            command: "embed",
            argument,
        },
        ["run", rest @ ..] => match run_options(rest) {
            Ok(request) => request,
            Err(argument) => Request::UnexpectedArgument {
                command: "run",
                argument,
            },
        },
        ["bench", rest @ ..] => match bench_options(rest) {
            Ok(request) => request,
            Err(argument) => Request::UnexpectedArgument {
                command: "bench",
                argument,
            },
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
        [first, ..] => Request::Unrecognized(first),
    }
}

/// Writes the record to one portable file (B-302, D20).
///
/// Not a gated category. A24 gates *publication* — the irreversible, itemized
/// act of sending something off this machine — and writing a file to a path the
/// operator named is not that; the gate belongs to whatever later *sends* a
/// bundle (B-160). What this does state is what the bundle contains, because a
/// portable file whose contents nobody described is a file nobody should send.
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
                 \x20 no prompt or completion content, by construction: this reads the record \
                 and the\n\x20 record is not the content store (A25)\n\
                 \x20 nothing has left this machine — sending a bundle is a separate, \
                 itemized act (A24)",
                to.display(),
                manifest.entries,
                manifest.digest
            ),
            served: true,
        },
        Err(failure) => Response {
            text: format!("mcf: the record could not be exported\n  {failure}"),
            served: false,
        },
    }
}

/// Reads `log`'s own arguments.
///
/// Total: an option it does not have is named back, and a count that is not a
/// number is a refusal rather than a default quietly substituted (A7).
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

/// Reads `run`'s own arguments.
///
/// Total: an option it does not have is named back rather than ignored, and a
/// number that is not one is a refusal rather than a default quietly
/// substituted (A7).
fn run_options<'a>(arguments: &[&'a str]) -> Result<Request<'a>, &'a str> {
    let mut model = None;
    let mut prompt = None;
    let mut limit = None;
    let mut seed = 0_u64;
    let mut engine = None;
    let mut rest = arguments.iter();
    while let Some(argument) = rest.next() {
        match *argument {
            "--prompt" => match rest.next() {
                Some(asked) => prompt = Some(*asked),
                None => {
                    return Ok(Request::MissingArgument {
                        command: "run",
                        needs: "--prompt <text>",
                    });
                }
            },
            "--limit" => match rest.next().and_then(|value| value.parse().ok()) {
                Some(tokens) => limit = Some(tokens),
                None => {
                    return Ok(Request::MissingArgument {
                        command: "run",
                        needs: "--limit <tokens>, a number",
                    });
                }
            },
            "--engine" => match rest.next() {
                Some(named) => engine = Some(*named),
                None => {
                    return Ok(Request::MissingArgument {
                        command: "run",
                        needs: "--engine <stand-in|provisioned>",
                    });
                }
            },
            "--seed" => match rest.next().and_then(|value| value.parse().ok()) {
                Some(chosen) => seed = chosen,
                None => {
                    return Ok(Request::MissingArgument {
                        command: "run",
                        needs: "--seed <number>",
                    });
                }
            },
            other if other.starts_with("--") => return Err(other),
            other if model.is_none() => model = Some(other),
            other => return Err(other),
        }
    }

    match (model, prompt) {
        (Some(model), Some(prompt)) => Ok(Request::Run {
            model,
            prompt,
            limit,
            seed,
            engine,
        }),
        (None, _) => Ok(Request::MissingArgument {
            command: "run",
            needs: "<model>",
        }),
        (Some(_), None) => Ok(Request::MissingArgument {
            command: "run",
            needs: "--prompt <text>",
        }),
    }
}

/// Reads `bench`'s own arguments.
///
/// `--resolving` is a percentage to one decimal place, read into parts per
/// million, because *how much is a difference* is the caller's question and
/// this is where they answer it (F55).
fn bench_options<'a>(arguments: &[&'a str]) -> Result<Request<'a>, &'a str> {
    let mut left = None;
    let mut right = None;
    let mut prompt = None;
    let mut limit = None;
    let mut seed = 0_u64;
    let mut engine = None;
    let mut resolving = None;
    let mut cold = false;
    let mut rest = arguments.iter();
    while let Some(argument) = rest.next() {
        match *argument {
            "--against" => match rest.next() {
                Some(other) => right = Some(*other),
                None => {
                    return Ok(Request::MissingArgument {
                        command: "bench",
                        needs: "--against <model>",
                    });
                }
            },
            "--prompt" => match rest.next() {
                Some(asked) => prompt = Some(*asked),
                None => {
                    return Ok(Request::MissingArgument {
                        command: "bench",
                        needs: "--prompt <text>",
                    });
                }
            },
            "--limit" => match rest.next().and_then(|value| value.parse().ok()) {
                Some(tokens) => limit = Some(tokens),
                None => {
                    return Ok(Request::MissingArgument {
                        command: "bench",
                        needs: "--limit <tokens>, a number",
                    });
                }
            },
            "--engine" => match rest.next() {
                Some(named) => engine = Some(*named),
                None => {
                    return Ok(Request::MissingArgument {
                        command: "bench",
                        needs: "--engine <stand-in|provisioned>",
                    });
                }
            },
            "--seed" => match rest.next().and_then(|value| value.parse().ok()) {
                Some(chosen) => seed = chosen,
                None => {
                    return Ok(Request::MissingArgument {
                        command: "bench",
                        needs: "--seed <number>",
                    });
                }
            },
            "--cold" => cold = true,
            "--resolving" => match rest.next().and_then(|value| per_cent(value)) {
                Some(held) => resolving = Some(held),
                None => {
                    return Ok(Request::MissingArgument {
                        command: "bench",
                        needs: "--resolving <per-cent>, such as 5 or 2.5",
                    });
                }
            },
            other if other.starts_with("--") => return Err(other),
            other if left.is_none() => left = Some(other),
            other => return Err(other),
        }
    }

    match (left, right, prompt) {
        (Some(left), Some(right), Some(prompt)) => Ok(Request::Bench {
            left,
            right,
            prompt,
            limit,
            seed,
            engine,
            resolving,
            cold,
        }),
        (None, _, _) => Ok(Request::MissingArgument {
            command: "bench",
            needs: "<model>",
        }),
        (Some(_), None, _) => Ok(Request::MissingArgument {
            command: "bench",
            needs: "--against <model>",
        }),
        (Some(_), Some(_), None) => Ok(Request::MissingArgument {
            command: "bench",
            needs: "--prompt <text>",
        }),
    }
}

/// A percentage to one decimal place, as parts per million.
///
/// Parsed by hand rather than through a float: this crate holds no
/// floating-point number, and a percentage with one decimal place is two
/// integers (A6).
pub(crate) fn per_cent(written: &str) -> Option<u64> {
    let (whole, tenths) = match written.split_once('.') {
        Some((whole, rest)) => {
            let mut digits = rest.chars();
            let tenth = digits.next()?.to_digit(10)?;
            if digits.next().is_some() {
                // More precision than the unit admits, refused rather than
                // silently rounded (A7).
                return None;
            }
            (whole.parse::<u64>().ok()?, u64::from(tenth))
        }
        None => (written.parse::<u64>().ok()?, 0),
    };
    let held = whole
        .checked_mul(10_000)?
        .checked_add(tenths.checked_mul(1_000)?)?;
    (held > 0).then_some(held)
}

/// Reads `pull`'s own arguments.
///
/// Total: an option it does not have is named back, and a `--from` with
/// nothing after it is a missing argument rather than a silent default.
fn pull_options<'a>(arguments: &[&'a str]) -> Result<Request<'a>, &'a str> {
    let mut reference = None;
    let mut from = None;
    let mut into = None;
    let mut offered = pull::Offered::Nothing;
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
        }),
        None => Ok(Request::MissingArgument {
            command: "pull",
            needs: "<owner/name[:file]>",
        }),
    }
}

/// Reads `check`'s own arguments.
///
/// The same two options `pull` has, for the same reason: a check speaks to the
/// same hub, and a credential comes from where the operator named and nowhere
/// else (B-024).
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

/// Reads `rm`'s own arguments.
///
/// Total: an option it does not have is named back rather than ignored, and a
/// `--because` with nothing after it is a missing argument rather than a
/// removal with an empty reason.
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

/// Reads `doctor`'s own arguments.
///
/// Total: an option it does not have is named back rather than ignored.
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

/// Answers a request. Pure, so the laboratory can exercise every branch
/// without a process (B19).
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
            text: format!(
                "{identity}\n\
                 \n\
                 usage:\n\
                 \x20 mcf doctor [--no-record] [--json]   what this machine is, what MCF\n\
                 \x20                                     costs here, and what it promises\n\
                 \x20 mcf pull <owner/name[:file]>        bring a model here, with its\n\
                 \x20          [--into <directory>]       provenance; without a file it\n\
                 \x20          [--from <hub>]             says which variants would run\n\
                 \x20          [--token-from <file>]      here. MCF reads a credential\n\
                 \x20          [--token-from-env <VAR>]   only where you name one, and\n\
                 \x20                                     puts models where MCF_MODELS\n\
                 \x20                                     says unless --into names one\n\
                 \x20 mcf serve                           start the daemon: it stays up,\n\
                 \x20                                     recovers what is on the disk and\n\
                 \x20                                     costs nothing while idle\n\
                 \x20 mcf run <model> --prompt <text>     ask a model something, with MCF's\n\
                 \x20         [--limit <n>] [--seed <n>]  own engine — a behaviour answer,\n\
                 \x20                                     never a speed (D31, B65)\n\
                 \x20 mcf bench <model> --against <model> compare two models on an engine\n\
                 \x20       --prompt <text> [--limit <n>]  that can be timed. No pass\n\
                 \x20       [--seed <n>] [--resolving <%>] condition: every verdict is\n\
                 \x20       [--engine <name>] [--cold]     something the machine said (A18)\n\
                 \x20 mcf cross-check <model>              read one engine's tokens with the\n\
                 \x20                                       other, and say whether they agree\n\
                 \x20 mcf probe <model> [--engine <name>] [--apply]\n\
                 \x20                                       ask a model to do the thing, and\n\
                 \x20                                     report what it did — configuring\n\
                 \x20                                     nothing (§X, D42)\n\
                 \x20 mcf provision <component>           build a pinned component in a\n\
                 \x20     [--list] [--remove <c>          container, everything recorded,\n\
                 \x20      --because <why>] [--into <dir>] removable without residue (B-367)\n\
                 \x20 mcf embed <model> --text <text>     ask an embedding model for a\n\
                 \x20                                     vector: JSON first, conditions\n\
                 \x20                                     after (DEC-055)\n\
                 \x20 mcf log [--kind <kind>]             what happened on this machine,\n\
                 \x20         [--last <n>] [--full]       read back out of the record\n\
                 \x20 mcf explain <model>                 what it declares, what MCF read,\n\
                 \x20                                     what MCF would choose, and what\n\
                 \x20                                     it cannot tell you\n\
                 \x20 mcf status                          ask a running daemon what it is\n\
                 \x20                                     and what it is holding\n\
                 \x20 mcf stop [--because <why>]          ask it to stop, and say why\n\
                 \x20 mcf list                            what this machine is holding\n\
                 \x20 mcf check [<model>] [--here]        is what you hold still what it\n\
                 \x20           [--from <hub>]            should be? the bytes against the\n\
                 \x20                                     digest recorded for them, and the\n\
                 \x20                                     hub against what it published\n\
                 \x20 mcf rm <model> [--because <why>]    stop holding it: without a reason\n\
                 \x20            [--purge]                this previews and removes nothing\n\
                 \x20 mcf export --to <path>              the record, as one portable file\n\
                 \x20 mcf licence [--full]                the terms, and what conveying this\n\
                 \x20                                     binary obliges you to (GPL-3.0-only)\n\
                 \x20 mcf --version                       what this binary is\n\
                 \n\
                 Acquisition reaches an encrypted hub over MCF's own HTTP and a vendored\n\
                 TLS stack, or a plain one where you name it — a mirror, or the\n\
                 laboratory's own (B-322).\n\
                 \n\
                 What MCF cannot do yet is serve a model or time one. `mcf serve` runs\n\
                 the daemon and says so when asked; `mcf run` answers with MCF's own\n\
                 stand-in and marks every answer as one, because a timing taken from it\n\
                 would measure the stand-in rather than the model (D31, B65).\n\
                 \n\
                 `mcf-helper` is beside this binary and does three things that need\n\
                 rights this one does not have: the processor governor, a device's\n\
                 exclusive mode, and the processor's energy counter (D35)."
            ),
            served: true,
        },
        Request::Doctor { record, as_json } => {
            let report = doctor::run(*record);
            Response {
                text: if *as_json {
                    report.to_value().to_line()
                } else {
                    report.render()
                },
                // `doctor` reports; it does not fail because the machine did.
                served: true,
            }
        }
        Request::Pull {
            reference,
            from,
            into,
            offered,
        } => pull::run(reference, *from, *into, *offered),
        Request::Serve => serve::run(),
        Request::Log { kind, last, full } => log::run(*kind, *last, *full),
        Request::Check {
            only,
            reach,
            from,
            offered,
        } => check::run(*only, *reach, *from, *offered),
        Request::Explain { model } => explain::run(model),
        Request::Run {
            model,
            prompt,
            limit,
            seed,
            engine,
        } => run::run(model, prompt, *limit, *seed, *engine),
        Request::Bench {
            left,
            right,
            prompt,
            limit,
            seed,
            engine,
            resolving,
            cold,
        } => bench::bench(
            left,
            right,
            prompt,
            *limit,
            *seed,
            *engine,
            resolving.map(mcf_core::measurement::PartsPerMillion),
            *cold,
        ),
        Request::CrossCheck { model } => crosscheck::run(model),
        Request::Probe {
            model,
            engine,
            apply,
        } => probe::run(model, *engine, *apply),
        Request::Provision { name, into } => provision::run(name, *into),
        Request::ProvisionList { into } => provision::list(*into),
        Request::ProvisionRemove {
            name,
            because,
            into,
        } => provision::remove(name, *because, *into),
        Request::Embed { model, text } => embed::run(model, text),
        Request::Status => serve::status(),
        Request::Stop { because } => serve::stop(because.unwrap_or_default()),
        Request::List => models::list(),
        Request::Remove {
            names,
            because,
            purge,
        } => models::remove(names, *because, *purge),
        Request::Unrecognized(argument) => Response {
            text: format!(
                "mcf: no such command: {argument}\n\
                 The surface at this milestone is `mcf --version`; see `mcf --help`."
            ),
            served: false,
        },
        Request::UnexpectedArgument { command, argument } => Response {
            // Not "takes no arguments": some of them take one, and telling an
            // operator that `licence` takes none when it takes `--full` is a
            // confident wrong answer of the kind P1 puts below saying less.
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
    fn no_arguments_is_usage() {
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

    /// A2: an unrecognized command is a named outcome carrying what it saw,
    /// not a silent fall-through to help text.
    #[test]
    fn an_unknown_command_is_named_and_carries_its_input() {
        assert_eq!(parse(&["measure"]), Request::Unrecognized("measure"));
        let Response { text, served } = respond(&parse(&["measure"]), BuildIdentity::current());
        assert!(!served, "an absent command must not report success");
        assert!(
            text.contains("measure"),
            "{text:?} does not say what it saw"
        );
    }

    /// Usage advertises exactly what exists. A22 makes the headless surface
    /// complete, so a command missing from usage is a capability only somebody
    /// who read the source can reach — and one that appears there without
    /// existing is the fabricated report C7 and A20 are written against.
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
        assert!(text.contains("mcf run"), "{text}");
        assert!(text.contains("mcf explain"), "{text}");
        assert!(text.contains("mcf log"), "{text}");
        assert!(text.contains("mcf pull"), "{text}");
        assert!(text.contains("mcf check"), "{text}");
        // Built since this list was written: `mcf probe` asks a model to do
        // the thing (B-051, B-052) and `mcf provision` builds a component in a
        // controlled environment (B-367).
        assert!(text.contains("mcf probe"), "{text}");
        assert!(text.contains("mcf provision"), "{text}");
        // And `mcf bench`, which compares two models on an engine that can be
        // timed and has no pass condition (B-080, A18).
        assert!(text.contains("mcf bench"), "{text}");
        for unbuilt in ["mcf lab", "mcf recommend"] {
            assert!(
                !text.contains(unbuilt),
                "usage advertises {unbuilt}, which nothing has built"
            );
        }
        // And it does not claim what MCF stopped being unable to do. The TLS
        // sentence outlived the vendoring by several weeks; a usage text is
        // read by somebody deciding whether to try something (D7), so a stale
        // *cannot* is worse than a missing line.
        assert!(
            !text.contains("has not vendored"),
            "usage still says a vendored dependency is not vendored: {text}"
        );
        assert!(text.contains("mcf-helper"), "{text}");
    }

    /// `export` needs a destination, and says which rather than guessing at
    /// one. A2: a named outcome carrying what it saw.
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

    /// `doctor` reads its own options, and refuses anything else by name (A2).
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

    /// The version response is the build identity verbatim, so what a record
    /// says and what the operator is told cannot drift (§3.4).
    #[test]
    fn version_is_the_build_identity_verbatim() {
        let identity = BuildIdentity::current();
        let Response { text, served } = respond(&Request::Version, identity);
        assert!(served);
        assert_eq!(text, identity.to_string());
    }

    /// Total function: no input is unhandled, including the empty string and
    /// arguments that look like flags MCF does not have (B7's shape, at the
    /// smallest scale).
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

    /// A21's habit applied to arguments: a recognized command given an
    /// argument it does not take is its own outcome. Reporting it as "no such
    /// command: --version" would be a confident wrong answer.
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
}
