//! `mcf eval`: asking a model to do the work, and checking what it did
//! (B-110, §IX, B40, B-025, §6.4).
//!
//! **This is the command that runs code a model wrote.** Everywhere else MCF
//! runs only what it built itself from a pinned commit; here it runs something
//! a model produced seconds ago, which is a different act and gets a different
//! boundary. Every answer is executed inside a container with no network, no
//! mount of anything but its own scratch directory, a memory ceiling and a
//! deadline — and the container is thrown away afterwards. What escapes it is
//! one line of output, read as text.
//!
//! **The laboratory does not run anything.** [`mcf_bench::eval`] produces the
//! source and the cases and grades what came back; this module is the only
//! place that starts a process, which is what keeps B-025's declaration
//! honest.
//!
//! **What a result may claim.** That this function, against these cases, on
//! this machine, held or did not. Not that a model is good at programming —
//! that is a judgement about a population of tasks nobody here has sampled,
//! and the four outcomes of [`mcf_core::graded::Graded`] exist so that *not
//! measured* can never become a low mark (B40, B41).

use std::path::{Path, PathBuf};

use mcf_bench::eval::{Ran, Task};
use mcf_core::failure::{Attribution, Category, Disposition, Failure, Subsystem};
use mcf_record::json::Value;

use crate::Response;

const WHERE: Subsystem = Subsystem::new("mcf-cli::eval");

/// How long one answer may run before it is stopped.
///
/// Generous for a function of a few lines, and finite: a model can write a
/// loop that does not end, and a laboratory that waited for one would hang
/// rather than record it.
const SECONDS: &str = "20";

/// How much memory one answer may have.
const MEMORY: &str = "512m";

/// The image the checker runs in, pinned by digest.
///
/// The tag is for a reader; the digest is what runs. The same discipline the
/// provisioned components hold to, for the same reason: an image that drifted
/// would make two runs of one task two different conditions (§3.4).
pub(crate) const IMAGE: &str = "docker.io/library/python";
/// `python:3.12-slim`, read from the registry rather than written from memory.
pub(crate) const IMAGE_DIGEST: &str =
    "sha256:09f7da3bc104798d0afb40bc08d23ab2da20a76130cec1f2ef170848f5d85217";

/// The code a model wrote, taken out of what it said.
///
/// **A model answers in prose with code in it.** A fenced block is the usual
/// shape and is preferred where there is one; otherwise what is returned is
/// the whole answer, and the checker decides whether it is a program. Nothing
/// here tries to repair the code: a laboratory that fixed an answer would be
/// measuring the repair (A19).
#[must_use]
pub(crate) fn code_in(said: &str) -> String {
    let Some((_, after)) = said.split_once("```") else {
        return said.trim().to_owned();
    };
    // The word after the fence is the language, where the model wrote one.
    let body = after.split_once('\n').map_or(after, |(_, rest)| rest);
    body.split_once("```")
        .map_or(body, |(inside, _)| inside)
        .trim()
        .to_owned()
}

/// The program the container runs: the model's function, then the cases.
///
/// Each case is printed as `ok` or `no` on its own line, and nothing else
/// reaches the reader — so what escapes the container is a fixed alphabet
/// however the model's code behaves.
fn checker(task: &Task, written: &str) -> String {
    let mut out = String::from(written);
    out.push_str("\n\nimport sys\n");
    for case in task.cases {
        use std::fmt::Write as _;
        // The expected text as a Python literal, quoted by repr of a str.
        let _wrote = writeln!(
            out,
            "try:\n    print('ok' if repr({}) == {} else 'no')\nexcept Exception:\n    \
             print('no')",
            case.call,
            python_string(case.expects)
        );
    }
    out
}

/// A Rust string as a Python string literal.
fn python_string(held: &str) -> String {
    let escaped = held.replace('\\', "\\\\").replace('\'', "\\'");
    format!("'{escaped}'")
}

/// Runs one written answer in a container and counts what held.
pub(crate) fn run_in_container(podman: &Path, scratch: &Path, task: &Task, written: &str) -> Ran {
    if written.trim().is_empty() {
        return Ran::Refused {
            because: "the answer held no code".to_owned(),
            wrote_something: false,
        };
    }
    let said = match run_python(podman, scratch, &checker(task, written)) {
        Ok(said) => said,
        Err(because) => {
            return Ran::Refused {
                because,
                wrote_something: true,
            };
        }
    };
    let _kept = std::fs::write(scratch.join("answer.out"), said.as_bytes());
    let held: Vec<&str> = said
        .lines()
        .filter(|line| matches!(*line, "ok" | "no"))
        .collect();
    if held.len() != task.cases.len() {
        // The program did not reach every case: it failed to parse, raised
        // before the checks, or was stopped. That is not a wrong answer.
        return Ran::Refused {
            because: format!(
                "the answer did not run to the end of the cases ({} of {} reported)",
                held.len(),
                task.cases.len()
            ),
            wrote_something: true,
        };
    }
    Ran::Checked {
        passed: held.iter().filter(|line| **line == "ok").count(),
        of: task.cases.len(),
    }
}

/// Runs one Python program in the container and returns what it printed:
/// no network, no capabilities, a read-only root, a memory ceiling, a
/// process limit and a deadline, over the scratch directory mounted
/// read-only. What a model wrote is not what MCF built (B-025).
pub(crate) fn run_python(podman: &Path, scratch: &Path, program: &str) -> Result<String, String> {
    if let Err(error) = std::fs::write(scratch.join("answer.py"), program) {
        return Err(format!("the answer could not be written down: {error}"));
    }
    let pinned = format!("{IMAGE}@{IMAGE_DIGEST}");
    let spoke = std::process::Command::new(podman)
        .env_remove("XDG_DATA_HOME")
        .arg("run")
        .arg("--rm")
        // Nothing this program does may reach anything: no network, no
        // capabilities, a read-only root, its own scratch and nothing else.
        // What a model wrote is not what MCF built (B-025).
        .arg("--network=none")
        .arg("--cap-drop=ALL")
        .arg("--security-opt=no-new-privileges")
        .arg("--read-only")
        .arg(format!("--memory={MEMORY}"))
        .arg("--pids-limit=64")
        .arg("-v")
        .arg(format!("{}:/work:ro,z", scratch.display()))
        .arg(&pinned)
        .arg("timeout")
        .arg(SECONDS)
        .arg("python3")
        .arg("/work/answer.py")
        .output()
        .map_err(|error| format!("the checker could not be started: {error}"))?;
    Ok(String::from_utf8_lossy(&spoke.stdout).into_owned())
}

/// Where podman is, or the refusal that says why there is none.
fn which_podman() -> Result<PathBuf, Failure> {
    for candidate in ["/usr/bin/podman", "/usr/local/bin/podman"] {
        let path = PathBuf::from(candidate);
        if path.is_file() {
            return Ok(path);
        }
    }
    Err(Failure::new(
        Category::PlatformMechanismUnavailable,
        Attribution::Machine,
        Disposition::Refused,
        WHERE,
        "this machine has no podman, and code a model wrote is not run outside a container",
    )
    .with_context(
        "what_to_do",
        "install podman from the platform's own repository",
    ))
}

/// Says how far a suite is, on the output stream at once, in the one
/// shape every suite uses — `progress: done/of what` — so that the
/// window's bar and a person at a terminal see the work as it goes and
/// not only the report at the end (D56).
pub(crate) fn progress(done: usize, of: usize, what: &str) {
    use std::io::Write as _;
    println!("progress: {done}/{of} {what}");
    let _flushed = std::io::stdout().flush();
}

/// The suites `mcf eval` runs, by the name `--only` takes.
pub(crate) const SUITES: [&str; 4] = ["challenges", "editing", "tests", "queries"];

/// What `mcf eval` was asked: one suite or all, and the catalogue's
/// retries, languages and tier where the person set them (D56).
#[derive(Debug, Default)]
pub(crate) struct Asked<'a> {
    /// One suite to run rather than all of them.
    pub only: Option<&'a str>,
    /// How many attempts a challenge gets in a language.
    pub retries: Option<usize>,
    /// The languages the catalogue runs in, separated by commas.
    pub languages: Option<&'a str>,
    /// One tier of the catalogue.
    pub tier: Option<&'a str>,
    /// The window every challenge ask is made in, where the person set it.
    pub window: Option<u64>,
}

/// Evaluates one model against every task, or one suite's.
#[allow(
    clippy::too_many_lines,
    reason = "the laboratory's one drive: the container found, each task asked and run, every attempt a row, the report said"
)]
pub(crate) fn eval(named: &str, asked: &Asked<'_>) -> Response {
    let only = asked.only;
    let plan = match crate::challenges::Plan::asked(
        asked.languages,
        asked.retries,
        asked.tier,
        asked.window,
    ) {
        Ok(plan) => plan,
        Err(why) => {
            return Response {
                text: format!("mcf: {why}"),
                served: false,
            };
        }
    };
    if let Some(suite) = only
        && !SUITES.contains(&suite)
    {
        return Response {
            text: format!(
                "mcf: no suite is called {suite}; --only takes one of {}",
                SUITES.join(", ")
            ),
            served: false,
        };
    }
    let wants = |suite: &str| only.is_none_or(|named| named == suite);
    let podman = match which_podman() {
        Ok(podman) => podman,
        Err(failure) => {
            return Response {
                text: format!("mcf: {failure}"),
                served: false,
            };
        }
    };
    let Some(socket) = crate::serve::socket_path() else {
        return Response {
            text: "mcf: MCF has nowhere to put a control socket on this machine".to_owned(),
            served: false,
        };
    };
    if let Some(why) = crate::serve::ensure_running(&socket) {
        return Response {
            text: format!("mcf: MCF could not start\n  {why}"),
            served: false,
        };
    }
    let scratch = std::env::temp_dir().join(format!("mcf-eval-{}", std::process::id()));
    if let Err(error) = std::fs::create_dir_all(&scratch) {
        return Response {
            text: format!("mcf: the laboratory has nowhere to work: {error}"),
            served: false,
        };
    }

    // **How the model is being addressed is a condition of every reading
    // below.** A model sent raw text completes the prompt instead of answering
    // it — which produces prose where a task asked for a function, and a
    // laboratory that did not say so would report *this model cannot write
    // code* about a model nobody addressed properly (§3.8, §3.4, B40).
    let addressed = crate::models::default_root()
        .and_then(|models| models.parent().map(std::path::Path::to_path_buf))
        .map(|home| mcf_serve::configured::read_derived(&home, Path::new(named)))
        .and_then(|derived| derived.addressing.map(|held| held.provenance()));

    let mut lines = vec![
        format!("evaluated {named}"),
        String::new(),
        match only {
            Some(suite) => format!("  only the {suite} suite"),
            None => "  every suite, every answer run in a container with no network".to_owned(),
        },
        match &addressed {
            Some(how) => format!("  addressed as {how}"),
            None => "  addressed as: nothing was applied, so each task is sent as raw text — a                      model trained to be addressed will complete it rather than answer it, and                      that is a condition of everything below. `mcf probe <model> --apply`                      settles it"
                .to_owned(),
        },
        String::new(),
    ];
    // The engine the daemon ran the asks on, as the first account named
    // it: the rows' conditions say what answered, not only that the
    // daemon did (B-542).
    let mut engine_ran: Option<String> = None;
    // The edit tasks after the writing ones, in the same container (B-522).
    let (edit_lines, edit_rows, edit_engine) = if wants("editing") {
        crate::edits::run(&socket, named, &podman, &scratch)
    } else {
        (Vec::new(), Vec::new(), None)
    };
    if engine_ran.is_none() {
        engine_ran = edit_engine;
    }
    let said_of = |engine: &Option<String>| {
        engine.as_ref().map_or_else(
            || "through the daemon, run in a container".to_owned(),
            |engine| format!("through {engine}, run in a container"),
        )
    };
    let engine_said = said_of(&engine_ran);
    let edits_recorded = wants("editing").then(|| {
        mcf_serve::examine::record_rows(
            Path::new(named),
            "editing",
            &engine_said,
            vec![
                (
                    "tasks",
                    Value::Integer(i64::try_from(crate::edits::EDITS.len()).unwrap_or(i64::MAX)),
                ),
                (
                    "attempts",
                    Value::Integer(i64::try_from(crate::edits::ATTEMPTS).unwrap_or(i64::MAX)),
                ),
            ],
            &edit_rows,
        )
    });
    // Tests written for a stated function, run against a correct and
    // broken implementations (B-524).
    let (test_lines, test_rows, test_engine) = if wants("tests") {
        crate::testing::run(&socket, named, &podman, &scratch)
    } else {
        (Vec::new(), Vec::new(), None)
    };
    if engine_ran.is_none() {
        engine_ran = test_engine;
    }
    // SQL and patterns, run in the same container (B-551).
    let (query_lines, query_rows, query_engine) = if wants("queries") {
        crate::queries::run(&socket, named, &podman, &scratch)
    } else {
        (Vec::new(), Vec::new(), None)
    };
    if engine_ran.is_none() {
        engine_ran = query_engine;
    }
    // The catalogue: every challenge in every language named, with its
    // retries, the rows under their own method (B-563, D56).
    let (challenge_lines, challenge_rows, challenge_engine) = if wants("challenges") {
        // Said before the run, on the output stream where the progress
        // goes, so that a person watching knows what it runs under (B-564).
        let under = plan.said(Path::new(named));
        for line in &under {
            println!("{line}");
        }
        let (mut said, rows, engine) = crate::challenges::run(
            &socket,
            named,
            &podman,
            &scratch,
            &plan.languages,
            plan.retries,
            plan.tier,
            plan.window,
        );
        said.splice(
            0..0,
            under.into_iter().chain(std::iter::once(String::new())),
        );
        (said, rows, engine)
    } else {
        (Vec::new(), Vec::new(), None)
    };
    if engine_ran.is_none() {
        engine_ran = challenge_engine;
    }
    let engine_said = said_of(&engine_ran);
    let challenges_recorded = wants("challenges").then(|| {
        mcf_serve::examine::record_rows(
            Path::new(named),
            "challenges",
            &engine_said,
            plan.conditions(),
            &challenge_rows,
        )
    });
    let queries_recorded = wants("queries").then(|| {
        mcf_serve::examine::record_rows(
            Path::new(named),
            "queries",
            &engine_said,
            vec![
                (
                    "sql_tasks",
                    Value::Integer(
                        i64::try_from(crate::queries::QUERIES.len()).unwrap_or(i64::MAX),
                    ),
                ),
                (
                    "pattern_tasks",
                    Value::Integer(
                        i64::try_from(crate::queries::PATTERNS.len()).unwrap_or(i64::MAX),
                    ),
                ),
            ],
            &query_rows,
        )
    });
    let tests_recorded = wants("tests").then(|| {
        mcf_serve::examine::record_rows(
            Path::new(named),
            "test-writing",
            &engine_said,
            vec![
                (
                    "tasks",
                    Value::Integer(i64::try_from(crate::testing::TASKS.len()).unwrap_or(i64::MAX)),
                ),
                (
                    "attempts",
                    Value::Integer(i64::try_from(crate::testing::ATTEMPTS).unwrap_or(i64::MAX)),
                ),
            ],
            &test_rows,
        )
    });
    let _gone = std::fs::remove_dir_all(&scratch);
    if addressed.is_none() {
        lines.push(String::new());
        lines.push(
            "  Nothing above was addressed as this model was trained to be. Where a task reads \
             *unknown*, that may be the addressing rather than the model."
                .to_owned(),
        );
    }
    lines.push(String::new());
    lines.extend(edit_lines);
    if let Some(recorded) = edits_recorded {
        lines.push(match recorded {
            Ok(_) => format!(
                "  {} reading(s) recorded under editing; `mcf data {named} --method editing` writes them",
                edit_rows.len()
            ),
            Err(why) => format!("  EDIT READINGS NOT RECORDED: {why}"),
        });
    }
    if wants("editing") {
        lines.push(String::new());
    }
    lines.extend(test_lines);
    if let Some(recorded) = tests_recorded {
        lines.push(match recorded {
            Ok(_) => format!(
                "  {} reading(s) recorded under test-writing; `mcf data {named} --method test-writing` writes them",
                test_rows.len()
            ),
            Err(why) => format!("  TEST-WRITING READINGS NOT RECORDED: {why}"),
        });
    }
    if wants("tests") {
        lines.push(String::new());
    }
    lines.extend(query_lines);
    if let Some(recorded) = queries_recorded {
        lines.push(match recorded {
            Ok(_) => format!(
                "  {} reading(s) recorded under queries; `mcf data {named} --method queries` writes them",
                query_rows.len()
            ),
            Err(why) => format!("  QUERIES READINGS NOT RECORDED: {why}"),
        });
    }
    if wants("queries") {
        lines.push(String::new());
    }
    lines.extend(challenge_lines);
    if let Some(recorded) = challenges_recorded {
        lines.push(match recorded {
            Ok(_) => format!(
                "  {} reading(s) recorded under challenges; `mcf data {named} --method challenges` \
                 writes them",
                challenge_rows.len()
            ),
            Err(why) => format!("  CHALLENGE READINGS NOT RECORDED: {why}"),
        });
    }
    Response {
        text: lines.join("\n"),
        served: true,
    }
}
