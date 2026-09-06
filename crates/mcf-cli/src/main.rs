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

mod acquire;
mod bench;
mod bundle;
mod check;
mod crosscheck;
mod data;
mod desk;
mod doctor;
mod edits;
mod embed;
mod eval;
mod examine;
mod explain;
mod history;
mod hosting;
mod languages;
mod licence;
mod log;
mod measure;
mod models;
mod probe;
mod prompt;
mod provision;
mod pull;
mod queries;
mod run;
mod say;
mod segment;
mod serve;
mod share;
mod show;
mod support;
mod testing;
mod tui;
mod verify;

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
    /// Show what would leave this machine, row by row, and write it (B-160,
    /// A24).
    Share {
        /// Where the file goes.
        into: Option<&'a str>,
    },
    /// State the licence, and the obligations that come with conveying this
    /// binary (B-330, D28).
    Licence {
        /// Whether to print the whole text, which is compiled in.
        full: bool,
    },
    /// Report the surface that exists.
    Usage,
    /// Report one command's line of it, because `--help` was asked of the
    /// command rather than of `mcf` (B-426).
    CommandUsage {
        /// The command.
        command: &'a str,
    },
    /// A command that wants a name first was given a flag there.
    ///
    /// Distinct from [`Request::UnexpectedArgument`]: `mcf measure --deepest`
    /// used to measure a model named `--deepest`, and *measure does not take
    /// --deepest* would be false — it does, after the model (B-426).
    NameExpected {
        /// The command.
        command: &'a str,
        /// What was found where the name goes.
        argument: &'a str,
        /// The name the usage table says goes there.
        needs: &'static str,
    },
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
        /// Whether to ask the hub again rather than answer a word from what
        /// was kept of its last answer within the day (B-488).
        fresh: bool,
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
    /// Open MCF in a window and stay there.
    ///
    /// A third surface and a client of the same control plane (A22, B-072),
    /// drawing the console's own screens at another scale.
    Desk,
    /// Open MCF as a terminal application and stay there.
    ///
    /// A second surface and a client of the same control plane (A22, B-072).
    /// It adds no capability: every action it offers is a request a command
    /// here already sends.
    Tui,
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
        /// The daemon's count of what the file holds, as the one line every
        /// client reads — the window included — rather than the pages.
        json: bool,
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
        /// How the turn is framed, where the person asked the engine to
        /// frame it from the model's own template (D47). Boxed for the
        /// size of the request, not for any sharing.
        turn: Box<mcf_serve::turn::Turn>,
        /// A picture to show the model, where the person named one (B-452).
        image: Option<&'a str>,
        /// What the engine is started with beyond the plain load, where the
        /// person asked for either (B-456).
        started: mcf_serve::declared::Started,
    },
    /// Check a bundle against this machine.
    Verify {
        /// The bundle to check.
        bundle: &'a str,
    },
    /// Write one file that reproduces one claim.
    Bundle {
        /// The claim's identifier, as `mcf log` prints it.
        id: &'a str,
        /// Where to write it.
        into: Option<&'a str>,
    },
    /// Expand one recorded entry into the evidence behind it.
    Show {
        /// The entry's identifier, as `mcf log` prints it.
        id: &'a str,
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
        /// A wall-clock budget, in seconds, from the operator.
        ///
        /// **The operator's, not the laboratory's** (B-224, B-226). A lab may
        /// not declare its work in minutes; a person may certainly say how
        /// many they have. What MCF owes in return is a proposal naming what
        /// fits and what does not, rather than a quietly smaller run (§3.1).
        within: Option<u64>,
        /// What the engine is started with beyond the plain load, the same
        /// for both arms (B-463).
        started: mcf_serve::declared::Started,
    },
    /// Write what a maintainer would need to read this machine's hardware.
    Support {
        /// Where to write it.
        into: Option<&'a str>,
    },
    /// Show how a model's vocabulary segments a prompt.
    Segment {
        /// The model whose vocabulary does the segmenting.
        model: &'a str,
        /// The text to segment.
        prompt: &'a str,
    },
    /// Install, build and pin a component in a controlled environment.
    Provision {
        /// Which component, from the table MCF carries — or none, for the
        /// one a model on this machine would run on.
        name: Option<&'a str>,
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
    /// What a repository publishes, and which of it will run here.
    Offered {
        /// The repository.
        reference: &'a str,
    },
    /// Fetch one published file, through the daemon.
    Acquire {
        /// The repository.
        reference: &'a str,
        /// Which published file.
        file: &'a str,
    },
    /// What MCF would run a model under.
    Settings {
        /// A context to price besides the one MCF recommends, where the
        /// operator named one.
        at: Option<u64>,
        /// The model.
        model: &'a str,
    },
    /// Hold a model and answer on a port.
    Host {
        /// The model.
        model: &'a str,
        /// Settings to move off what MCF recommends.
        changes: Vec<(String, mcf_record::json::Value)>,
    },
    /// What is being hosted.
    Hosted,
    /// Stop hosting.
    Unhost,
    /// Time a model at doubling depths.
    Measure {
        /// The model: a path, or something `mcf list` names.
        model: &'a str,
        /// The deepest context to sample, which implies every power of two
        /// below it.
        deepest: u64,
        /// Which engine to ask through, if the caller named one.
        engine: Option<&'a str>,
        /// Where the model goes, if the caller said: the processor or the
        /// card. Absent is where MCF resolves it to.
        on: Option<mcf_serve::control::On>,
        /// What the engine is started with beyond the plain load (B-463).
        started: mcf_serve::declared::Started,
    },
    /// What a prompt does to a model: which of its parts reach the answer.
    PromptReport {
        /// The model: a path, or something `mcf list` names.
        model: &'a str,
        /// The document to take apart, as given on the command line.
        prompt: Option<&'a str>,
        /// A file holding the document, `-` for standard input: a persona is
        /// a page, not a shell argument (B-430).
        file: Option<&'a str>,
        /// What to take it apart into, where the caller says.
        by: Option<mcf_serve::prompt::Unit>,
        /// The most parts to remove, where the caller says.
        most: Option<usize>,
        /// The temperature to draw the settledness seeds at, where the
        /// caller states one; none spends nothing on the question (B-431).
        temperature: Option<mcf_core::configuration::Thousandths>,
        /// The further readings asked for, each costing generations
        /// (B-434, B-435).
        extras: mcf_serve::prompt::Extras,
        /// How the turn is framed, where the person asked for one of the
        /// template's switches (B-455). Boxed for the size of the request,
        /// not for any sharing.
        turn: Box<mcf_serve::turn::Turn>,
        /// Whether to answer as data rather than as prose.
        as_json: bool,
    },
    /// Ask a model to do the work, and check what it did (B-110).
    Eval {
        /// The model: a path, or something `mcf list` names.
        model: &'a str,
        /// One suite to run rather than all of them.
        only: Option<&'a str>,
    },
    /// A model's readings as a table (D54).
    Data {
        /// The model: a path, or something `mcf list` names.
        model: &'a str,
        /// One method's runs only, where the caller named one.
        method: Option<&'a str>,
        /// JSON lines rather than a comma-separated table.
        as_json: bool,
    },
    /// Measure a model's parts by count and clock (D52).
    Examine {
        /// The model: a path, or something `mcf list` names.
        model: &'a str,
        /// Which engine to measure through, if the caller named one.
        engine: Option<&'a str>,
        /// Which measurements, by name and separated by commas; every one
        /// when absent.
        only: Option<&'a str>,
    },
    Probe {
        /// The model: a path, or something `mcf list` names.
        model: &'a str,
        /// Which engine to ask through, if the caller named one.
        engine: Option<&'a str>,
        /// Whether to apply what was observed, which is an act (D43).
        apply: bool,
        /// The longest prompt the usable-context probe may ask for, where
        /// the caller wants less than the file declares (B-461).
        up_to: Option<usize>,
        /// Which probes, by name and separated by commas; every one when
        /// absent (B-478).
        only: Option<&'a str>,
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
    // Before the table: `--help` asked of a command, and a flag where the
    // command wants a name. Each used to be read as the name — `mcf measure
    // --help` measured a model called `--help`, and started a daemon to do
    // it (B-426).
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
        ["cross-check", model] => Request::CrossCheck { model },
        ["cross-check"] => Request::MissingArgument {
            command: "cross-check",
            needs: "<model>",
        },
        ["settings", model] => Request::Settings { model, at: None },
        ["settings", model, "--context", at] => match at.parse::<u64>() {
            Ok(at) => Request::Settings {
                model,
                at: Some(at),
            },
            Err(_) => Request::UnexpectedArgument {
                command: "settings",
                argument: at,
            },
        },
        // Each of these wanted its argument, matched nothing without it,
        // and answered *no such command* — a command MCF has, denying
        // itself. `mcf explain` sends the operator to `mcf settings`
        // directly (F139).
        ["settings"] => Request::MissingArgument {
            command: "settings",
            needs: "<model>",
        },
        ["hosted"] => Request::Hosted,
        ["unhost"] => Request::Unhost,
        // Without this the pattern below wants a model, nothing matched,
        // and `mcf host` answered *no such command: host* — denying a
        // command MCF has, in the same breath as `mcf status` telling the
        // operator to run it (F139).
        ["host"] => Request::MissingArgument {
            command: "host",
            needs: "<model>",
        },
        ["host", model, rest @ ..] => match host_options(rest) {
            Ok(changes) => Request::Host { model, changes },
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
        ["measure"] => Request::MissingArgument {
            command: "measure",
            needs: "<model>",
        },
        ["measure", model, rest @ ..] => match measure_options(model, rest) {
            Ok(request) => request,
            Err(argument) => Request::UnexpectedArgument {
                command: "measure",
                argument,
            },
        },
        ["prompt"] => Request::MissingArgument {
            command: "prompt",
            needs: "<model> --prompt <text> or --file <path>",
        },
        ["prompt", model, rest @ ..] => match prompt_options(model, rest) {
            Ok(request) => request,
            Err(argument) => Request::UnexpectedArgument {
                command: "prompt",
                argument,
            },
        },
        ["eval", model] => Request::Eval { model, only: None },
        ["eval", model, "--only", suite] => Request::Eval {
            model,
            only: Some(suite),
        },
        ["eval", _, "--only"] => Request::MissingArgument {
            command: "eval --only",
            needs: "a suite's name: coding, languages, editing, tests or queries",
        },
        ["eval"] => Request::MissingArgument {
            command: "eval",
            needs: "<model>",
        },
        ["data"] => Request::MissingArgument {
            command: "data",
            needs: "<model>",
        },
        ["data", model, rest @ ..] => match data_options(model, rest) {
            Ok(request) => request,
            Err(argument) => Request::UnexpectedArgument {
                command: "data",
                argument,
            },
        },
        ["examine"] => Request::MissingArgument {
            command: "examine",
            needs: "<model>",
        },
        ["examine", model, rest @ ..] => match examine_options(model, rest) {
            Ok(request) => request,
            Err(argument) => Request::UnexpectedArgument {
                command: "examine",
                argument,
            },
        },
        ["probe"] => Request::MissingArgument {
            command: "probe",
            needs: "<model>",
        },
        ["probe", model, rest @ ..] => match probe_options(model, rest) {
            Ok(request) => request,
            Err(argument) => Request::UnexpectedArgument {
                command: "probe",
                argument,
            },
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
        ["verify", bundle] => Request::Verify { bundle },
        ["verify"] => Request::MissingArgument {
            command: "verify",
            needs: "<bundle>, which `mcf bundle` writes",
        },
        ["verify", _, argument, ..] => Request::UnexpectedArgument {
            command: "verify",
            argument,
        },
        ["bundle", id] => Request::Bundle { id, into: None },
        ["bundle", id, "--into", path] => Request::Bundle {
            id,
            into: Some(path),
        },
        ["bundle"] => Request::MissingArgument {
            command: "bundle",
            needs: "<entry-id>, which `mcf log --kind comparison` prints first on each line",
        },
        ["bundle", _, argument, ..] => Request::UnexpectedArgument {
            command: "bundle",
            argument,
        },
        ["show", id] => Request::Show { id },
        ["show"] => Request::MissingArgument {
            command: "show",
            needs: "<entry-id>, which `mcf log` prints first on each line",
        },
        ["show", _, argument, ..] => Request::UnexpectedArgument {
            command: "show",
            argument,
        },
        ["share"] => Request::Share { into: None },
        ["share", "--into", into] => Request::Share { into: Some(into) },
        ["share", argument, ..] => Request::UnexpectedArgument {
            command: "share",
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
        ["segment", model, "--prompt", prompt] => Request::Segment { model, prompt },
        ["segment", _model] | ["segment", _model, "--prompt"] => Request::MissingArgument {
            command: "segment",
            needs: "--prompt <text>",
        },
        ["segment"] => Request::MissingArgument {
            command: "segment",
            needs: "<model> --prompt <text>",
        },
        ["segment", _, argument, ..] => Request::UnexpectedArgument {
            command: "segment",
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
        // A command the table has, with arguments no arm above took. Before
        // this arm it fell through to *no such command: settings* — MCF
        // denying a command it has, in the same breath as `mcf --help`
        // listing it (F139, B-426).
        [command, _, argument, ..] if usage_of(command).is_some() => {
            Request::UnexpectedArgument { command, argument }
        }
        [first, ..] => Request::Unrecognized(first),
    }
}

/// The flags `mcf measure <model>` takes, in any order.
///
/// `--deepest` wants a power of two, 512 or larger: a context window is
/// asked for in powers of two, and a ladder that ended anywhere else would
/// have a top rung nobody could ask a model to run at; below 512 the cost a
/// token is the same to within the noise.
fn measure_options<'a>(model: &'a str, arguments: &[&'a str]) -> Result<Request<'a>, &'a str> {
    let mut deepest = 8192;
    let mut engine = None;
    let mut on = None;
    let mut started = mcf_serve::declared::Started::default();
    let mut rest = arguments.iter();
    while let Some(argument) = rest.next() {
        match *argument {
            "--deepest" => {
                let Some(value) = rest.next() else {
                    return Ok(Request::MissingArgument {
                        command: "measure --deepest",
                        needs: "a power of two, 512 or larger",
                    });
                };
                deepest = match value.parse::<u64>() {
                    Ok(deepest) if deepest.is_power_of_two() && deepest >= 512 => deepest,
                    _ => {
                        return Ok(Request::UnexpectedArgument {
                            command: "measure --deepest (wants a power of two, 512 or larger)",
                            argument: value,
                        });
                    }
                };
            }
            "--engine" => {
                let Some(value) = rest.next() else {
                    return Ok(Request::MissingArgument {
                        command: "measure --engine",
                        needs: "an engine's name",
                    });
                };
                engine = Some(*value);
            }
            "--on" => {
                let Some(value) = rest.next() else {
                    return Ok(Request::MissingArgument {
                        command: "measure --on",
                        needs: "cpu or gpu",
                    });
                };
                on = match mcf_serve::control::On::parse(value) {
                    Some(on) => Some(on),
                    None => {
                        return Ok(Request::UnexpectedArgument {
                            command: "measure --on (wants cpu or gpu)",
                            argument: value,
                        });
                    }
                };
            }
            // What the engine is started with beyond the plain load: a
            // timing under one of these is a timing of that condition, and
            // the two conditions are only comparable if each says which it
            // was (B-463, B-456).
            "--draft-head" => started.draft_head = true,
            "--rope-scaling" | "--rope-scale" => {
                if let Some(needs) = started_switch(&mut started, argument, rest.next().copied()) {
                    return Ok(Request::MissingArgument {
                        command: "measure",
                        needs,
                    });
                }
            }
            other => return Err(other),
        }
    }
    Ok(Request::Measure {
        model,
        deepest,
        engine,
        on,
        started,
    })
}

/// The flags `mcf data <model>` takes, in any order: `--method` names one
/// method's runs, `--json` asks for JSON lines (D54).
fn data_options<'a>(model: &'a str, arguments: &[&'a str]) -> Result<Request<'a>, &'a str> {
    let mut method = None;
    let mut as_json = false;
    let mut rest = arguments.iter();
    while let Some(argument) = rest.next() {
        match *argument {
            "--json" => as_json = true,
            "--method" => {
                let Some(value) = rest.next() else {
                    return Ok(Request::MissingArgument {
                        command: "data --method",
                        needs: "a method's name",
                    });
                };
                method = Some(*value);
            }
            other => return Err(other),
        }
    }
    Ok(Request::Data {
        model,
        method,
        as_json,
    })
}

/// The flags `mcf examine <model>` takes, in any order: `--only` names the
/// measurements, `--engine` the engine (D52).
fn examine_options<'a>(model: &'a str, arguments: &[&'a str]) -> Result<Request<'a>, &'a str> {
    let mut engine = None;
    let mut only = None;
    let mut rest = arguments.iter();
    while let Some(argument) = rest.next() {
        match *argument {
            "--only" => {
                let Some(value) = rest.next() else {
                    return Ok(Request::MissingArgument {
                        command: "examine --only",
                        needs: "measurement names, separated by commas",
                    });
                };
                only = Some(*value);
            }
            "--engine" => {
                let Some(value) = rest.next() else {
                    return Ok(Request::MissingArgument {
                        command: "examine --engine",
                        needs: "an engine's name",
                    });
                };
                engine = Some(*value);
            }
            other => return Err(other),
        }
    }
    Ok(Request::Examine {
        model,
        engine,
        only,
    })
}

/// The flags `mcf probe <model>` takes, in any order.
///
/// `--apply` is the act D43 requires, a flag rather than a default because
/// that is the whole of the decision: MCF may learn better, and what it does
/// with that is say so until somebody asks for the change. `--engine` names
/// the engine because a probe result belongs to the engine it was taken
/// through (D42), and until the two are shown to agree, which one answered
/// is part of the result (B-376). `--up-to` caps the usable-context probe's
/// question below the file's declared length: a trial the file's claim
/// would make six hours long can be asked for less, and the report says the
/// claim itself was not asked (B-461).
fn probe_options<'a>(model: &'a str, arguments: &[&'a str]) -> Result<Request<'a>, &'a str> {
    let mut engine = None;
    let mut apply = false;
    let mut up_to = None;
    let mut only = None;
    let mut rest = arguments.iter();
    while let Some(argument) = rest.next() {
        match *argument {
            "--apply" => apply = true,
            "--only" => {
                let Some(value) = rest.next() else {
                    return Ok(Request::MissingArgument {
                        command: "probe --only",
                        needs: "probe names, separated by commas",
                    });
                };
                only = Some(*value);
            }
            "--engine" => {
                let Some(value) = rest.next() else {
                    return Ok(Request::MissingArgument {
                        command: "probe --engine",
                        needs: "an engine's name",
                    });
                };
                engine = Some(*value);
            }
            "--up-to" => {
                let Some(value) = rest.next() else {
                    return Ok(Request::MissingArgument {
                        command: "probe --up-to",
                        needs: "a number of identifiers, 2 or more",
                    });
                };
                up_to = match value.parse::<usize>() {
                    Ok(tokens) if tokens >= 2 => Some(tokens),
                    _ => {
                        return Ok(Request::UnexpectedArgument {
                            command: "probe --up-to (wants a number of identifiers, 2 or more)",
                            argument: value,
                        });
                    }
                };
            }
            other => return Err(other),
        }
    }
    Ok(Request::Probe {
        model,
        engine,
        apply,
        up_to,
        only,
    })
}

/// The flags `mcf prompt <model>` takes, in any order.
///
/// The document comes from `--prompt` or `--file`, one or the other: a
/// persona is a page and a page is a file, and `-` reads the standard input
/// so that one can be piped in. `--by` and `--most` are choices the report
/// would otherwise make and say it made (§3.15, B-430).
/// A report reads one document: given inline or in a file, and never both.
fn one_document<'a>(prompt: Option<&'a str>, file: Option<&'a str>) -> Option<Request<'a>> {
    match (prompt, file) {
        (None, None) => Some(Request::MissingArgument {
            command: "prompt",
            needs: "--prompt <text> or --file <path>",
        }),
        (Some(_), Some(_)) => Some(Request::UnexpectedArgument {
            command: "prompt (takes --prompt or --file, not both)",
            argument: "--file",
        }),
        _ => None,
    }
}

/// A switch that needed a value it did not get.
const fn needs_for<'a>(command: &'static str, needs: &'static str) -> Request<'a> {
    Request::MissingArgument { command, needs }
}

fn prompt_options<'a>(model: &'a str, arguments: &[&'a str]) -> Result<Request<'a>, &'a str> {
    let mut prompt = None;
    let mut file = None;
    let mut by = None;
    let mut most = None;
    let mut temperature = None;
    let mut extras = mcf_serve::prompt::Extras::NONE;
    let mut turn = mcf_serve::turn::Turn::default();
    let mut as_json = false;
    let mut rest = arguments.iter();
    while let Some(argument) = rest.next() {
        // The value a switch takes, or what it needed: the name rather than
        // the whole request, which is a large thing to carry as an error.
        let value = |needs: &'static str, rest: &mut std::slice::Iter<'_, &'a str>| {
            rest.next().copied().ok_or(needs)
        };
        match *argument {
            "--prompt" => match value("--prompt <text>", &mut rest) {
                Ok(text) => prompt = Some(text),
                Err(needs) => return Ok(needs_for("prompt", needs)),
            },
            // The template's own switches, as `mcf run` takes them: what is
            // read is the prompt inside the turn it will be used in (B-455).
            "--thinking" | "--effort" | "--system" => {
                if let Some(needs) = turn_switch(&mut turn, argument, rest.next().copied()) {
                    return Ok(Request::MissingArgument {
                        command: "prompt",
                        needs,
                    });
                }
            }
            "--file" => match value("--file <path>, or - for the standard input", &mut rest) {
                Ok(path) => file = Some(path),
                Err(needs) => return Ok(needs_for("prompt", needs)),
            },
            "--by" => match value("--by word, phrase, sentence or paragraph", &mut rest) {
                Ok(word) => match mcf_serve::prompt::Unit::named(word) {
                    Some(unit) => by = Some(unit),
                    None => {
                        return Ok(Request::UnexpectedArgument {
                            command: "prompt --by (wants word, phrase, sentence or paragraph)",
                            argument: word,
                        });
                    }
                },
                Err(needs) => return Ok(needs_for("prompt", needs)),
            },
            "--most" => match value("--most <n>, how many parts to remove at most", &mut rest) {
                Ok(count) => match count.parse::<usize>() {
                    Ok(count) if count > 0 => most = Some(count),
                    _ => {
                        return Ok(Request::UnexpectedArgument {
                            command: "prompt --most (wants a number, 1 or more)",
                            argument: count,
                        });
                    }
                },
                Err(needs) => return Ok(needs_for("prompt", needs)),
            },
            "--temperature" => match value(
                "--temperature <decimal>, to draw the seeds at, above 0",
                &mut rest,
            ) {
                Ok(written) => match written.parse::<mcf_core::configuration::Thousandths>() {
                    Ok(held) if held.0 > 0 => temperature = Some(held),
                    _ => {
                        return Ok(Request::UnexpectedArgument {
                            command: "prompt --temperature (wants a decimal above 0, to three \
                                      places)",
                            argument: written,
                        });
                    }
                },
                Err(needs) => return Ok(needs_for("prompt", needs)),
            },
            "--json" => as_json = true,
            // Every further reading is a flag of its own name: --floors,
            // --alone, --prefixes, --swaps, --forms (B-072).
            other => match other
                .strip_prefix("--")
                .and_then(mcf_serve::prompt::Extra::named)
            {
                Some(extra) => extras = extras.with(extra, true),
                None => return Err(other),
            },
        }
    }
    if let Some(wrong) = one_document(prompt, file) {
        return Ok(wrong);
    }
    Ok(Request::PromptReport {
        model,
        prompt,
        file,
        by,
        most,
        temperature,
        extras,
        turn: Box::new(turn),
        as_json,
    })
}

/// The line of the usage table that introduces a command, with the lines
/// that continue it. `None` for a command the table does not have.
fn usage_of(command: &str) -> Option<&'static str> {
    let mut at = 0;
    let mut start = None;
    for line in COMMANDS.lines() {
        if let Some(introduced) = introduces(line) {
            match start {
                // The block ends where the next command's line begins.
                Some(from) => return COMMANDS.get(from..at),
                None if introduced == command => start = Some(at),
                None => {}
            }
        }
        at += line.len() + 1;
    }
    COMMANDS.get(start?..)
}

/// The command a line of the usage table introduces, if it introduces one:
/// the word after `mcf` on a line that begins with it.
fn introduces(line: &str) -> Option<&str> {
    line.strip_prefix("  mcf ")?.split_whitespace().next()
}

/// The name a command wants before any flag, as the usage table writes it —
/// `<model>`, `<owner/name>` — or `None` for one that takes a flag or nothing
/// first. An optional name, `[<model>]`, is not wanted: a flag may stand
/// there.
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
                 {}\
                 \x20 nothing has left this machine — sending a bundle is a separate, \
                 itemized act (A24)",
                to.display(),
                manifest.entries,
                manifest.digest,
                // Read from the file that was written rather than asserted
                // about the mechanism. The sentence this replaces said *no
                // prompt or completion content, by construction* over an
                // export carrying four thousand completions, because the
                // record held them and nothing here looked (F105, A1).
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

/// Reads `log`'s own arguments.
///
/// Total: an option it does not have is named back, and a count that is not a
/// number is a refusal rather than a default quietly substituted (A7).
/// The settings a `host` command moves off MCF's recommendation.
///
/// Every one is optional and anything unnamed keeps what MCF advised — a
/// caller who said nothing has not asked for a setting's lowest value (A7,
/// D43).
fn host_options(rest: &[&str]) -> Result<Vec<(String, mcf_record::json::Value)>, &'static str> {
    use mcf_record::json::Value;
    let mut changes = Vec::new();
    let mut at = 0;
    while at < rest.len() {
        let Some(flag) = rest.get(at) else { break };
        let said = rest.get(at + 1).copied();
        let number = |name: &str| -> Result<(String, Value), &'static str> {
            let held: i64 = said
                .ok_or("a setting with no value")?
                .parse()
                .map_err(|_| "a setting whose value is not a number")?;
            Ok((name.to_owned(), Value::Integer(held)))
        };
        let change = match *flag {
            "--context" => number("context")?,
            "--gpu-layers" => number("gpu_layers")?,
            // Where the model goes, in the daemon's own list of placements,
            // with the build that fits: resolved into engine, device and
            // layers before the hold is asked for.
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
            // The engine's own start, beyond the plain load (B-456).
            "--draft-head" => ("draft_head".to_owned(), Value::Bool(said == Some("on"))),
            "--rope-scaling" => (
                "rope_scaling".to_owned(),
                Value::text(said.ok_or("--rope-scaling with no value")?),
            ),
            "--rope-scale" => number("rope_scale")?,
            _ => return Err("a setting mcf host does not take"),
        };
        changes.push(change);
        at += 2;
    }
    Ok(changes)
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

/// Throws one of the engine's own start switches that takes a value, or
/// names what it needed.
///
/// `--draft-head` is not here: it takes no value, and a switch that reads
/// the next argument would swallow the one after it.
fn started_switch(
    started: &mut mcf_serve::declared::Started,
    switch: &str,
    value: Option<&str>,
) -> Option<&'static str> {
    if switch == "--rope-scaling" {
        match value.and_then(mcf_serve::declared::Scaling::from_word) {
            Some(scaling) => started.rope = Some(scaling),
            None => return Some("--rope-scaling <none|linear|yarn>"),
        }
        return None;
    }
    match value.and_then(|value| value.parse().ok()) {
        Some(factor) => started.factor = Some(factor),
        None => return Some("--rope-scale <n>, a whole number"),
    }
    None
}

/// Throws one of the template's switches as said, or names what it needed.
fn turn_switch(
    turn: &mut mcf_serve::turn::Turn,
    switch: &str,
    value: Option<&str>,
) -> Option<&'static str> {
    match (switch, value) {
        ("--thinking", Some("on")) => turn.thinking = Some(true),
        ("--thinking", Some("off")) => turn.thinking = Some(false),
        ("--thinking", _) => return Some("--thinking <on|off>"),
        ("--effort", Some(effort)) => turn.effort = Some(effort.to_owned()),
        ("--effort", None) => {
            return Some("--effort <word>, in the model's own vocabulary (low, medium, high…)");
        }
        ("--system", Some(system)) => turn.system = Some(system.to_owned()),
        (_, None) => return Some("--system <text>"),
        (_, Some(_)) => {}
    }
    None
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
    let mut turn = mcf_serve::turn::Turn::default();
    let mut image = None;
    let mut started = mcf_serve::declared::Started::default();
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
            "--image" => match rest.next() {
                Some(file) => image = Some(*file),
                None => {
                    return Ok(Request::MissingArgument {
                        command: "run",
                        needs: "--image <file>",
                    });
                }
            },
            // The engine's own start, where the person asked for more of
            // the file than the plain load reads (B-456).
            "--draft-head" => started.draft_head = true,
            "--rope-scaling" | "--rope-scale" => {
                if let Some(needs) = started_switch(&mut started, argument, rest.next().copied()) {
                    return Ok(Request::MissingArgument {
                        command: "run",
                        needs,
                    });
                }
            }
            // The template's own switches, each passed through as said.
            "--thinking" | "--effort" | "--system" => {
                if let Some(needs) = turn_switch(&mut turn, argument, rest.next().copied()) {
                    return Ok(Request::MissingArgument {
                        command: "run",
                        needs,
                    });
                }
            }
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
            turn: Box::new(turn),
            image,
            started,
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
/// What a benchmark is allowed to spend, and what difference it is asked to
/// resolve: both are numbers with their own shapes, read in one place.
fn bench_budget(
    switch: &str,
    value: Option<&str>,
) -> Result<(Option<u64>, Option<u64>), &'static str> {
    if switch == "--within" {
        let seconds = value
            .and_then(|value| value.parse().ok())
            .ok_or("--within <seconds>, a number")?;
        return Ok((Some(seconds), None));
    }
    let held = value
        .and_then(per_cent)
        .ok_or("--resolving <per-cent>, such as 5 or 2.5")?;
    Ok((None, Some(held)))
}

fn bench_options<'a>(arguments: &[&'a str]) -> Result<Request<'a>, &'a str> {
    let mut left = None;
    let mut right = None;
    let mut prompt = None;
    let mut limit = None;
    let mut seed = 0_u64;
    let mut engine = None;
    let mut resolving = None;
    let mut cold = false;
    let mut within = None;
    let mut started = mcf_serve::declared::Started::default();
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
            // Held the same across both arms: a comparison under a draft
            // head is a comparison of two models each with one (B-463).
            "--draft-head" => started.draft_head = true,
            "--rope-scaling" | "--rope-scale" => {
                if let Some(needs) = started_switch(&mut started, argument, rest.next().copied()) {
                    return Ok(Request::MissingArgument {
                        command: "bench",
                        needs,
                    });
                }
            }
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
            "--within" | "--resolving" => match bench_budget(argument, rest.next().copied()) {
                Ok((seconds, held)) => {
                    within = seconds.or(within);
                    resolving = held.or(resolving);
                }
                Err(needs) => return Ok(needs_for("bench", needs)),
            },
            other if other.starts_with("--") => return Err(other),
            other if left.is_none() => left = Some(other),
            other => return Err(other),
        }
    }

    Ok(assembled(&Asked {
        left,
        right,
        prompt,
        limit,
        seed,
        engine,
        resolving,
        cold,
        within,
        started,
    }))
}

/// What `bench` was asked for, before it is known to be a complete request.
struct Asked<'a> {
    left: Option<&'a str>,
    right: Option<&'a str>,
    prompt: Option<&'a str>,
    limit: Option<usize>,
    seed: u64,
    engine: Option<&'a str>,
    resolving: Option<u64>,
    cold: bool,
    within: Option<u64>,
    /// What the engine is started with beyond the plain load (B-463).
    started: mcf_serve::declared::Started,
}

/// A benchmark request, or the first thing missing from one.
///
/// Which thing is missing is named rather than counted: *bench needs an
/// argument* sends the reader back to the manual, and `--against <model>`
/// sends them back to the shell.
fn assembled<'a>(asked: &Asked<'a>) -> Request<'a> {
    let &Asked {
        left,
        right,
        prompt,
        limit,
        seed,
        engine,
        resolving,
        cold,
        within,
        started,
    } = asked;
    match (left, right) {
        (Some(left), Some(right)) => Request::Bench {
            left,
            right,
            // **The standard question when nobody names one** (B-160, B42).
            // A benchmark needs something to time, and who chose it decides
            // whether the result can be shared: MCF ships one so that the
            // ordinary run produces something that travels, and an operator
            // who wants their own text says so and gets a result that stays
            // here. Before this, `--prompt` was required and every result was
            // therefore the operator's own (F115).
            prompt: prompt.unwrap_or(mcf_bench::STANDARD_QUESTION),
            limit,
            seed,
            engine,
            resolving,
            cold,
            within,
            started,
        },
        (None, _) => Request::MissingArgument {
            command: "bench",
            needs: "<model>",
        },
        (Some(_), None) => Request::MissingArgument {
            command: "bench",
            needs: "--against <model>",
        },
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

/// The commands there are, one line each, as `mcf --help` prints them.
///
/// The parser reads this table too: which commands exist, and which of them
/// want a name before any flag, are read off it rather than listed a second
/// time (B-072, B-426). A line begins `  mcf <command>`; what follows is the
/// command's arguments, then its description, and a continuation line begins
/// with more spaces.
const COMMANDS: &str = "\
    \x20 mcf desk                            MCF in a window: every screen a\n\
    \x20                                     client of the same daemon. Needs\n\
    \x20                                     SDL3 provisioned before MCF is\n\
    \x20                                     built, and says so if it is not\n\
    \x20 mcf tui                             the same screens with no display\n\
    \x20                                     attached\n\
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
    \x20 mcf run <model> --prompt <text>     ask a model something — a\n\
    \x20         [--limit <n>] [--seed <n>]  behaviour answer, never a speed\n\
    \x20         [--thinking on|off]         (D31, B65). The switches go to\n\
    \x20         [--effort <word>]           the model's own template, which\n\
    \x20         [--system <text>]           the engine renders; a picture\n\
    \x20         [--image <file>]            goes through the model's own\n\
    \x20         [--draft-head]              projector, where it has one.\n\
    \x20         [--rope-scaling <kind>]     A draft head and a scaling are\n\
    \x20         [--rope-scale <n>]          the engine's to start, never\n\
    \x20                                     on unless they are asked for\n\
    \x20 mcf bench <model> --against <model> compare two models on an engine\n\
    \x20       --prompt <text> [--limit <n>]  that can be timed. No pass\n\
    \x20       [--seed <n>] [--resolving <%>] condition: every verdict is\n\
    \x20       [--engine <name>] [--cold]     something the machine said (A18).\n\
    \x20       [--draft-head]                 A draft head or a scaling is\n\
    \x20       [--rope-scaling <kind>]        held the same across both arms\n\
    \x20       [--rope-scale <n>]             and named in the report\n\
    \x20 mcf eval <model>                    ask a model to do the work and\n\
    \x20       [--only <suite>]              check what it did: each answer run\n\
    \x20                                     in a container, four outcomes and\n\
    \x20                                     no total; a suite is coding,\n\
    \x20                                     languages, editing, tests or\n\
    \x20                                     queries\n\
    \x20 mcf prompt <model> --prompt <text>   what a prompt does: how the model\n\
    \x20             or --file <path>         receives each word, and how much\n\
    \x20       [--by word|phrase|sentence|   the answer moves without each\n\
    \x20             paragraph] [--most <n>]  part. An ordering, never\n\
    \x20       [--json]                      relevance. A\n\
    \x20       [--temperature <t>] [--floors] persona goes in --file, whole;\n\
    \x20       [--alone] [--prefixes]        --temperature draws three seeds\n\
    \x20       [--swaps] [--forms]           at t to see whether it settles;\n\
    \x20       [--system <text>]             --system, --thinking and --effort\n\
    \x20       [--thinking on|off]           read the prompt inside the turn\n\
    \x20       [--effort <word>]             it will be used in;\n\
    \x20                                     --floors puts the control at\n\
    \x20                                     every position, one each;\n\
    \x20                                     --alone asks each part as the\n\
    \x20                                     whole prompt in turn; --prefixes\n\
    \x20                                     grows the prompt a part at a time;\n\
    \x20                                     --swaps changes each pair of\n\
    \x20                                     neighbours' places; --forms asks\n\
    \x20                                     the same parts as one line,\n\
    \x20                                     bullets, a numbered list, under\n\
    \x20                                     headings, in tags and in capitals\n\
    \x20 mcf cross-check <model>              read one engine's tokens with the\n\
    \x20                                       other, and say whether they agree\n\
    \x20 mcf probe <model> [--engine <name>] [--apply]\n\
    \x20           [--up-to <tokens>]        ask a model to do the thing, and\n\
    \x20           [--only <names>]          report what it did — configuring\n\
    \x20                                     nothing (§X, D42); the context\n\
    \x20                                     trial is projected before it is\n\
    \x20                                     spent, --up-to asks for less, and\n\
    \x20                                     --only names the probes to run\n\
    \x20 mcf data <model> [--method <name>]  a model's readings as a table:\n\
    \x20           [--json]                  every figure a diagnostic read,\n\
    \x20                                     one row each with its dimensions\n\
    \x20                                     and unit, comma-separated by\n\
    \x20                                     default, JSON lines with --json\n\
    \x20 mcf examine <model> [--engine <name>]\n\
    \x20           [--only <names>]          measure a model's parts by count\n\
    \x20                                     and clock — the offload curve,\n\
    \x20                                     prefill, prefix reuse, memory,\n\
    \x20                                     concurrency, cold start, fidelity\n\
    \x20                                     to a reference file, bits a byte,\n\
    \x20                                     determinism, tokenizer round trip,\n\
    \x20                                     retrieval, degeneration, grammar\n\
    \x20                                     and image cost (D52); --only names\n\
    \x20                                     the measurements to take\n\
    \x20 mcf provision [<component>]         build a pinned component in a\n\
    \x20     [--list] [--remove <c>          container, everything recorded,\n\
    \x20      --because <why>] [--into <dir>] removable without residue; unnamed,\n\
    \x20                                     the engine a model here needs (B-367)\n\
    \x20 mcf embed <model> --text <text>     ask an embedding model for a\n\
    \x20                                     vector: JSON first, conditions\n\
    \x20                                     after (DEC-055)\n\
    \x20 mcf verify <bundle>                 does this machine agree, and if\n\
    \x20                                     not, which conditions differ — MCF\n\
    \x20                                     will not say which caused it (A8)\n\
    \x20 mcf bundle <entry-id>               one file that reproduces one\n\
    \x20        [--into <path>]              claim: the method, the conditions,\n\
    \x20                                     every trial and the provenance (PR2)\n\
    \x20 mcf show <entry-id>                 one recorded entry, expanded into\n\
    \x20                                     the measurements and conditions it\n\
    \x20                                     rests on (B55)\n\
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
    \x20 mcf segment <model>                 the prompt as the model actually\n\
    \x20             --prompt <text>         receives it, fragment by fragment:\n\
    \x20                                     where text breaks, and where this\n\
    \x20                                     vocabulary has no word for it\n\
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
    \x20      [--rope-scale <n>]             --on puts it where you say\n\
    \x20 mcf hosted                          what is being held, and where\n\
    \x20 mcf measure <model>                 time it at doubling context\n\
    \x20         [--deepest <n>]             depths, so the cost of a longer\n\
    \x20         [--engine <name>]           conversation is measured rather\n\
    \x20         [--on cpu|gpu]              than assumed — where MCF puts\n\
    \x20         [--draft-head]              it, unless --on says. A draft\n\
    \x20         [--rope-scaling <kind>]     head or a scaling is timed by\n\
    \x20         [--rope-scale <n>]          running it: each run says which\n\
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

/// What follows the table.
const NOTES: &str = "\
    Acquisition reaches an encrypted hub over MCF's own HTTP and a vendored\n\
    TLS stack, or a plain one where you name it — a mirror, or the\n\
    laboratory's own (B-322).\n\
    \n\
    `mcf run` answers with MCF's own stand-in and marks every answer as\n\
    one, because a timing taken from it would measure the stand-in rather\n\
    than the model (D31, B65). A model served where another program can\n\
    reach it is `mcf host`, which runs a provisioned engine and can be\n\
    timed; `mcf bench` and `mcf measure` are what time one.\n\
    \n\
    `mcf-helper` is beside this binary and does three things that need\n\
    rights this one does not have: the processor governor, a device's\n\
    exclusive mode, and the processor's energy counter (D35)";

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
                // `doctor` reports; it does not fail because the machine did.
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
        Request::Log { kind, last, full } => log::run(*kind, *last, *full),
        Request::Check {
            only,
            reach,
            from,
            offered,
        } => check::run(*only, *reach, *from, *offered),
        Request::Explain { model, json: false } => explain::run(model),
        Request::Explain { model, json: true } => explain::json(model),
        Request::Run {
            model,
            prompt,
            limit,
            seed,
            engine,
            turn,
            image,
            started,
        } => run::run(
            model,
            prompt,
            *limit,
            *seed,
            *engine,
            turn,
            image.map(std::path::Path::new),
            *started,
        ),
        Request::Bench {
            left,
            right,
            prompt,
            limit,
            seed,
            engine,
            resolving,
            cold,
            within,
            started,
        } => bench::bench(
            left,
            right,
            prompt,
            *limit,
            *seed,
            *engine,
            resolving.map(mcf_core::measurement::PartsPerMillion),
            *cold,
            within.map(|seconds| {
                mcf_core::time::Duration::from_nanos(seconds.saturating_mul(1_000_000_000))
            }),
            *started,
        ),
        Request::Verify { bundle } => verify::run(bundle),
        Request::Bundle { id, into } => bundle::run(id, *into),
        Request::Show { id } => show::run(id),
        Request::CrossCheck { model } => crosscheck::run(model),
        Request::Settings { model, at } => hosting::settings(model, *at),
        Request::Host { model, changes } => hosting::host(model, changes),
        Request::Hosted => hosting::held(),
        Request::Unhost => hosting::unhost(),
        Request::Offered { reference } => acquire::offered(reference, None, false),
        Request::Acquire { reference, file } => acquire::acquire(reference, file, None),
        Request::Measure {
            model,
            deepest,
            engine,
            on,
            started,
        } => measure::run(model, *deepest, *engine, *on, *started),
        Request::PromptReport {
            model,
            prompt,
            file,
            by,
            most,
            temperature,
            extras,
            turn,
            as_json,
        } => prompt::report(
            model,
            &prompt::Asked {
                prompt: *prompt,
                file: *file,
                by: *by,
                most: *most,
                temperature: *temperature,
                extras: *extras,
                turn: (**turn).clone(),
            },
            *as_json,
        ),
        Request::Eval { model, only } => eval::eval(model, *only),
        Request::Probe {
            model,
            engine,
            apply,
            up_to,
            only,
        } => probe::run(model, *engine, *apply, *up_to, *only),
        Request::Examine {
            model,
            engine,
            only,
        } => examine::run(model, *engine, *only),
        Request::Data {
            model,
            method,
            as_json,
        } => data::run(model, *method, *as_json),
        Request::Provision { name, into } => provision::run(*name, *into),
        Request::ProvisionList { into } => provision::list(*into),
        Request::ProvisionRemove {
            name,
            because,
            into,
        } => provision::remove(name, *because, *into),
        Request::Embed { model, text } => embed::run(model, text),
        Request::Share { into } => share::run(*into),
        Request::Status => serve::status(),
        Request::Stop { because } => serve::stop(because.unwrap_or_default()),
        Request::Support { into } => support::run(*into),
        Request::Segment { model, prompt } => segment::run(model, prompt),
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
    ///
    /// **The example has to be a command MCF does not have.** This was written
    /// with `measure`, which MCF did not answer at the time and does now — so
    /// the test went on passing while asserting that a command MCF has is
    /// unknown, which is the defect rather than the property (F139). The name
    /// below is not a word anybody would implement, and
    /// `every_command_the_help_lists_is_one_mcf_has` holds the general case.
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
        assert!(text.contains("mcf examine"), "{text}");
        assert!(text.contains("mcf data"), "{text}");
        assert!(text.contains("mcf provision"), "{text}");
        // And `mcf bench`, which compares two models on an engine that can be
        // timed and has no pass condition (B-080, A18).
        assert!(text.contains("mcf bench"), "{text}");
        // And `mcf segment`, which shows a prompt as the model's own
        // vocabulary produces it, with no generation and no judgement
        // (B-381, PR11, §3.15).
        assert!(text.contains("mcf segment"), "{text}");
        // And `mcf support`, the route by which hardware MCF cannot read
        // reaches somebody who can add it (F91).
        assert!(text.contains("mcf support"), "{text}");
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

    /// `explain` reads the pages here and the JSON through the daemon, and
    /// refuses anything else by name (A2).
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

    /// `--help` asked of a command is that command's usage, and runs nothing
    /// (B-426). `mcf measure --help` used to measure a model named `--help`,
    /// and started a daemon to do it.
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
                // That command's line, and not another's.
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

    /// A command that wants a name first, given a flag there, says so with
    /// the flag it saw and the name it wanted — it does not take the flag as
    /// the name (B-426), and it does not say the flag is one it never takes,
    /// which for `measure --deepest` would be false.
    #[test]
    fn a_flag_where_a_name_goes_is_refused_by_name() {
        let mut wanting = 0;
        for command in commands() {
            let Some(needs) = super::name_wanted_first(command) else {
                continue;
            };
            wanting += 1;
            for flag in ["--deepest", "--json", "--x", "-"] {
                let request = parse(&[command, flag]);
                assert_eq!(
                    request,
                    Request::NameExpected {
                        command,
                        argument: flag,
                        needs,
                    },
                    "mcf {command} {flag}"
                );
                let Response { text, served } = respond(&request, BuildIdentity::current());
                assert!(!served);
                assert!(
                    text.contains(command) && text.contains(flag) && text.contains(needs),
                    "mcf {command} {flag}: {text}"
                );
            }
        }
        assert!(
            wanting >= 10,
            "the table names {wanting} commands wanting a name first"
        );
        assert_eq!(super::name_wanted_first("measure"), Some("<model>"));
        assert_eq!(
            super::name_wanted_first("pull"),
            Some("<owner/name[:file]>")
        );
        assert_eq!(super::name_wanted_first("doctor"), None);
        assert_eq!(
            super::name_wanted_first("check"),
            None,
            "an optional name is not wanted"
        );
        assert_eq!(super::name_wanted_first("--version"), None);
    }

    /// A command the table has, given an argument no arm takes after its
    /// name, is refused with that argument — not *no such command*, which
    /// `mcf settings foo --bogus` and `mcf cross-check foo --json` used to
    /// answer (F139, B-426).
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

    /// `mcf measure` reads its flags in any order, and a flag without its
    /// value is a missing argument rather than a run (B-426).
    #[test]
    fn measure_reads_its_flags_in_any_order() {
        let both = Request::Measure {
            model: "m",
            deepest: 1024,
            engine: Some("e"),
            on: None,
            started: mcf_serve::declared::Started::default(),
        };
        assert_eq!(
            parse(&["measure", "m", "--engine", "e", "--deepest", "1024"]),
            both
        );
        assert!(
            matches!(
                parse(&["measure", "m", "--on", "gpu"]),
                Request::Measure {
                    on: Some(mcf_serve::control::On::Card),
                    ..
                }
            ),
            "--on gpu puts the whole model on the card"
        );
        assert!(
            matches!(
                parse(&["measure", "m", "--on", "elsewhere"]),
                Request::UnexpectedArgument { .. }
            ),
            "a place MCF does not put a model is not a run"
        );
        assert_eq!(
            parse(&["measure", "m", "--deepest", "1024", "--engine", "e"]),
            both
        );
        assert!(matches!(
            parse(&["measure", "m", "--deepest"]),
            Request::MissingArgument { .. }
        ));
        assert!(matches!(
            parse(&["measure", "m", "--engine"]),
            Request::MissingArgument { .. }
        ));
        assert!(matches!(
            parse(&["measure", "m", "--deepest", "100"]),
            Request::UnexpectedArgument {
                argument: "100",
                ..
            }
        ));
        assert!(matches!(
            parse(&["measure", "m", "--deepest", "1024", "--bogus"]),
            Request::UnexpectedArgument {
                command: "measure",
                argument: "--bogus"
            }
        ));
    }

    /// `mcf prompt` takes a document from the line or a file, a question after
    /// it, and the unit and the cap as choices — in any order, each said back
    /// when it is wrong (B-430).
    #[test]
    fn prompt_reads_its_flags_in_any_order() {
        let whole = Request::PromptReport {
            model: "m",
            prompt: None,
            file: Some("persona.md"),
            by: Some(mcf_serve::prompt::Unit::Paragraph),
            most: Some(40),
            temperature: Some(mcf_core::configuration::Thousandths(700)),
            extras: mcf_serve::prompt::Extras::NONE
                .with(mcf_serve::prompt::Extra::Floors, true)
                .with(mcf_serve::prompt::Extra::Alone, true)
                .with(mcf_serve::prompt::Extra::Prefixes, true)
                .with(mcf_serve::prompt::Extra::Swaps, true)
                .with(mcf_serve::prompt::Extra::Forms, true),
            turn: Box::new(mcf_serve::turn::Turn::default()),
            as_json: true,
        };
        assert_eq!(
            parse(&[
                "prompt",
                "m",
                "--file",
                "persona.md",
                "--by",
                "paragraph",
                "--most",
                "40",
                "--temperature",
                "0.7",
                "--floors",
                "--alone",
                "--prefixes",
                "--swaps",
                "--forms",
                "--json"
            ]),
            whole
        );
        assert_eq!(
            parse(&[
                "prompt",
                "m",
                "--json",
                "--forms",
                "--swaps",
                "--prefixes",
                "--alone",
                "--floors",
                "--temperature",
                "0.700",
                "--most",
                "40",
                "--by",
                "paragraphs",
                "--file",
                "persona.md",
            ]),
            whole
        );
        assert_eq!(
            parse(&["prompt", "m", "--prompt", "A. B."]),
            Request::PromptReport {
                model: "m",
                prompt: Some("A. B."),
                file: None,
                by: None,
                most: None,
                temperature: None,
                extras: mcf_serve::prompt::Extras::NONE,
                turn: Box::new(mcf_serve::turn::Turn::default()),
                as_json: false,
            }
        );
    }

    /// What `prompt` refuses: a missing model or file, a unit, a cap or a
    /// temperature that is not one, both sources, a flag it does not know.
    #[test]
    fn prompt_refuses_what_is_not_an_argument() {
        assert!(matches!(
            parse(&["prompt", "m"]),
            Request::MissingArgument { .. }
        ));
        assert!(matches!(
            parse(&["prompt", "m", "--file"]),
            Request::MissingArgument { .. }
        ));
        assert!(matches!(
            parse(&["prompt", "m", "--file", "a", "--by", "letter"]),
            Request::UnexpectedArgument {
                argument: "letter",
                ..
            }
        ));
        assert!(matches!(
            parse(&["prompt", "m", "--file", "a", "--most", "0"]),
            Request::UnexpectedArgument { argument: "0", .. }
        ));
        assert!(matches!(
            parse(&["prompt", "m", "--file", "a", "--temperature", "0"]),
            Request::UnexpectedArgument { argument: "0", .. }
        ));
        assert!(matches!(
            parse(&["prompt", "m", "--file", "a", "--temperature", "0.7001"]),
            Request::UnexpectedArgument {
                argument: "0.7001",
                ..
            }
        ));
        assert!(matches!(
            parse(&["prompt", "m", "--file", "a", "--prompt", "b"]),
            Request::UnexpectedArgument { .. }
        ));
        assert!(matches!(
            parse(&["prompt", "m", "--prompt", "a", "--bogus"]),
            Request::UnexpectedArgument {
                command: "prompt",
                argument: "--bogus"
            }
        ));
    }

    /// Every command the usage table introduces.
    fn commands() -> Vec<&'static str> {
        super::COMMANDS
            .lines()
            .filter_map(super::introduces)
            .collect()
    }
}
