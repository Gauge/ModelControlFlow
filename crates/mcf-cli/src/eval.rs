use std::path::{Path, PathBuf};

use mcf_bench::eval::{Ran, Task};
use mcf_core::failure::{Attribution, Category, Disposition, Failure, Subsystem};
use mcf_record::json::Value;

use crate::Response;

const WHERE: Subsystem = Subsystem::new("mcf-cli::eval");

const SECONDS: &str = "20";

const MEMORY: &str = "512m";

pub(crate) const IMAGE: &str = "docker.io/library/python";
pub(crate) const IMAGE_DIGEST: &str =
    "sha256:09f7da3bc104798d0afb40bc08d23ab2da20a76130cec1f2ef170848f5d85217";

#[must_use]
pub(crate) fn code_in(said: &str) -> String {
    let Some((_, after)) = said.split_once("```") else {
        return said.trim().to_owned();
    };
    let body = after.split_once('\n').map_or(after, |(_, rest)| rest);
    body.split_once("```")
        .map_or(body, |(inside, _)| inside)
        .trim()
        .to_owned()
}

fn checker(task: &Task, written: &str) -> String {
    let mut out = String::from(written);
    out.push_str("\n\nimport sys\n");
    for case in task.cases {
        use std::fmt::Write as _;
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

fn python_string(held: &str) -> String {
    let escaped = held.replace('\\', "\\\\").replace('\'', "\\'");
    format!("'{escaped}'")
}

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

pub(crate) fn run_python(podman: &Path, scratch: &Path, program: &str) -> Result<String, String> {
    if let Err(error) = std::fs::write(scratch.join("answer.py"), program) {
        return Err(format!("the answer could not be written down: {error}"));
    }
    let pinned = format!("{IMAGE}@{IMAGE_DIGEST}");
    let spoke = std::process::Command::new(podman)
        .env_remove("XDG_DATA_HOME")
        .arg("run")
        .arg("--rm")
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

static STOP_ASKED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

pub(crate) fn listen_for_stop() {
    let _listener = std::thread::spawn(|| {
        use std::io::BufRead as _;
        let stdin = std::io::stdin();
        for line in stdin.lock().lines() {
            let Ok(line) = line else { break };
            if line.trim() == "stop" {
                STOP_ASKED.store(true, std::sync::atomic::Ordering::SeqCst);
                break;
            }
        }
    });
}

#[must_use]
pub(crate) fn stop_asked() -> bool {
    STOP_ASKED.load(std::sync::atomic::Ordering::SeqCst)
}

pub(crate) fn result(line: &str) {
    use std::io::Write as _;
    println!("result: {line}");
    let _flushed = std::io::stdout().flush();
}

pub(crate) fn progress(done: usize, of: usize, what: &str) {
    use std::io::Write as _;
    println!("progress: {done}/{of} {what}");
    let _flushed = std::io::stdout().flush();
}

pub(crate) const SUITES: [&str; 4] = ["challenges", "editing", "tests", "queries"];

#[derive(Debug, Default)]
pub(crate) struct Asked<'a> {
    pub only: Option<&'a str>,
    pub retries: Option<usize>,
    pub languages: Option<&'a str>,
    pub tier: Option<&'a str>,
    pub window: Option<u64>,
    pub resume: bool,
}

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
    let mut engine_ran: Option<String> = None;
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
    let (test_lines, test_rows, test_engine) = if wants("tests") {
        crate::testing::run(&socket, named, &podman, &scratch)
    } else {
        (Vec::new(), Vec::new(), None)
    };
    if engine_ran.is_none() {
        engine_ran = test_engine;
    }
    let (query_lines, query_rows, query_engine) = if wants("queries") {
        crate::queries::run(&socket, named, &podman, &scratch)
    } else {
        (Vec::new(), Vec::new(), None)
    };
    if engine_ran.is_none() {
        engine_ran = query_engine;
    }
    let challenges_method = plan.tier.map_or_else(
        || "challenges".to_owned(),
        |tier| format!("challenges-{}", tier.name()),
    );
    listen_for_stop();
    let resumed = if asked.resume && wants("challenges") {
        match crate::challenges::resumable(named, &challenges_method, &plan) {
            Ok(Some(found)) => Some(found),
            Ok(None) => {
                println!(
                    "  nothing to resume: no unfinished run of these conditions; starting afresh"
                );
                None
            }
            Err(why) => {
                return Response {
                    text: format!("mcf: {why}"),
                    served: false,
                };
            }
        }
    } else {
        None
    };
    let done_pairs: Vec<(String, String)> = resumed
        .as_ref()
        .map(|found| found.pairs.clone())
        .unwrap_or_default();
    let mut landing = wants("challenges")
        .then(|| match &resumed {
            Some(found) => Ok(mcf_serve::examine::Landing::resume(
                Path::new(named),
                &challenges_method,
                &found.run,
                found.rows,
            )),
            None => mcf_serve::examine::Landing::open(
                Path::new(named),
                &challenges_method,
                plan.conditions(),
            ),
        })
        .transpose();
    let mut challenges_stopped: Option<String> = None;
    let (challenge_lines, _challenge_rows, challenge_engine) = if wants("challenges") {
        let under = plan.said(Path::new(named));
        for line in &under {
            println!("{line}");
        }
        if let Some(found) = &resumed {
            println!(
                "  resuming the run of {} with {} pair(s) already recorded",
                found.at,
                found.pairs.len()
            );
        }
        let (mut said, rows, engine, stopped) = crate::challenges::run(
            &socket,
            named,
            &podman,
            &scratch,
            &plan.languages,
            plan.retries,
            plan.tier,
            plan.window,
            landing.as_mut().ok().and_then(Option::as_mut),
            &done_pairs,
        );
        said.splice(
            0..0,
            under.into_iter().chain(std::iter::once(String::new())),
        );
        challenges_stopped = stopped;
        (said, rows, engine)
    } else {
        (Vec::new(), Vec::new(), None)
    };
    if engine_ran.is_none() {
        engine_ran = challenge_engine;
    }
    let engine_said = said_of(&engine_ran);
    let challenges_recorded: Option<Result<usize, String>> = match landing {
        Ok(Some(mut landing)) => {
            let _named = landing.land(Some(&engine_said), &[]);
            let ended = challenges_stopped
                .clone()
                .unwrap_or_else(|| "finished".to_owned());
            Some(landing.close(&ended).map_err(|why| why.to_string()))
        }
        Ok(None) => None,
        Err(why) => Some(Err(why.to_string())),
    };
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
            Ok(landed) => format!(
                "  {landed} reading(s) recorded under {challenges_method} as they were taken, and \
                 the run closed as {}; `mcf data {named} --method {challenges_method}` writes \
                 them{}",
                challenges_stopped.as_deref().unwrap_or("finished"),
                if challenges_stopped.is_some() {
                    "; `--resume` goes on from here"
                } else {
                    ""
                }
            ),
            Err(why) => format!("  CHALLENGE READINGS NOT CLOSED: {why}"),
        });
    }
    Response {
        text: lines.join("\n"),
        served: true,
    }
}
