//! `mcf` — the headless surface.
//!
//! A22 makes this the complete surface rather than a convenience wrapper: any
//! action reachable only through an interface is one the laboratory cannot
//! test, which A19 already forbids. Every command MCF grows appears here
//! first, and the interface of §XI (M4) becomes a client of the same API.
//!
//! At M0 the workspace has one command, `--version`, because that is all the
//! milestone has built. `mcf doctor` — the M0 product — arrives with B-014,
//! once the machine profiler (B-013) and the record store (B-004) it reports
//! from exist. The alternative, a `doctor` that prints a plausible-looking
//! report from nothing, is the exact failure A20 and C7 are written against.

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
        [
            command @ ("--version" | "-V" | "--help" | "-h"),
            argument,
            ..,
        ] => Request::UnexpectedArgument { command, argument },
        [first, ..] => Request::Unrecognized(first),
    }
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
                 usage: mcf --version\n\
                 \n\
                 This is the M0 workspace (B-001). It builds, records what \
                 built it, and\nclaims nothing else. `mcf doctor` — the M0 \
                 product — arrives with B-014."
            ),
            served: true,
        },
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
        assert_eq!(parse(&["doctor"]), Request::Unrecognized("doctor"));
        let Response { text, served } = respond(&parse(&["doctor"]), BuildIdentity::current());
        assert!(!served, "an absent command must not report success");
        assert!(text.contains("doctor"), "{text:?} does not say what it saw");
    }

    /// The M0 surface promises exactly one thing. A surface that claimed
    /// `doctor` before B-014 built it would be the fabricated report C7 and
    /// A20 are written against.
    #[test]
    fn usage_does_not_advertise_a_command_that_does_not_exist() {
        let Response { text, .. } = respond(&Request::Usage, BuildIdentity::current());
        assert!(text.contains("mcf --version"));
        assert!(
            !text.contains("usage: mcf doctor"),
            "usage advertises an unbuilt command"
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
