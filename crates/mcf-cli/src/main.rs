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

mod doctor;

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
    /// Report the surface that exists.
    Usage,
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
fn parse<'a>(arguments: &[&'a str]) -> Request<'a> {
    match arguments {
        ["--version" | "-V"] => Request::Version,
        [] | ["--help" | "-h"] => Request::Usage,
        ["doctor", rest @ ..] => match doctor_options(rest) {
            Ok(request) => request,
            Err(argument) => Request::UnexpectedArgument {
                command: "doctor",
                argument,
            },
        },
        [
            command @ ("--version" | "-V" | "--help" | "-h"),
            argument,
            ..,
        ] => Request::UnexpectedArgument { command, argument },
        [first, ..] => Request::Unrecognized(first),
    }
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
fn respond(request: &Request<'_>, identity: BuildIdentity) -> Response {
    match request {
        Request::Version => Response {
            text: identity.to_string(),
            served: true,
        },
        Request::Usage => Response {
            text: format!(
                "{identity}\n\
                 \n\
                 usage:\n\
                 \x20 mcf doctor [--no-record] [--json]   what this machine is, what MCF\n\
                 \x20                                     costs here, and what it promises\n\
                 \x20 mcf --version                       what this binary is\n\
                 \n\
                 This is M0. Nothing here acquires, serves or measures a model."
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
        Request::Unrecognized(argument) => Response {
            text: format!(
                "mcf: no such command: {argument}\n\
                 The surface at this milestone is `mcf --version`; see `mcf --help`."
            ),
            served: false,
        },
        Request::UnexpectedArgument { command, argument } => Response {
            text: format!("mcf: {command} takes no arguments, and was given: {argument}"),
            served: false,
        },
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
        for unbuilt in ["mcf pull", "mcf serve", "mcf bench", "mcf lab"] {
            assert!(
                !text.contains(unbuilt),
                "usage advertises {unbuilt}, which M0 has not built"
            );
        }
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
