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

use mcf_bench::eval::{Case, Ran, Task, Trials, measure};
use mcf_core::failure::{Attribution, Category, Disposition, Failure, Subsystem};

use crate::Response;

const WHERE: Subsystem = Subsystem::new("mcf-cli::eval");

/// How many times each task is put to the model.
///
/// A model is not deterministic and one attempt is one draw, so a laboratory
/// that asked once would report a sample of one as a finding (A4).
const ATTEMPTS: usize = 3;

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
const IMAGE: &str = "docker.io/library/python";
/// `python:3.12-slim`, read from the registry rather than written from memory.
const IMAGE_DIGEST: &str =
    "sha256:09f7da3bc104798d0afb40bc08d23ab2da20a76130cec1f2ef170848f5d85217";

/// The tasks this laboratory holds.
///
/// **Stated rather than sampled, and graded in difficulty on purpose.** The
/// first three of these are ones any working coding model passes, and a set
/// made only of those measures nothing: every model scores full marks and the
/// laboratory cannot tell them apart. The rest are chosen to be failable —
/// precedence in an expression, a shortest path, a cache with an eviction
/// order — so that a reading has somewhere to fall.
///
/// **Every expected answer is unambiguous.** The checker compares `repr()` as
/// text, so a task whose answer has several valid spellings would fail a
/// correct program: no dictionaries whose order is incidental, no floats, no
/// sets. Where an order could be argued, the task states it.
///
/// This is still not a measurement of programming ability and is not offered
/// as one. A corpus that could support a claim about *coding* is B-110's
/// remaining work; what this supports is *these tasks, on this machine*.
pub(crate) const TASKS: &[Task] = &[
    Task {
        name: "merge-sorted",
        function: "merge",
        asks: "Write a Python function `merge(a, b)` that takes two lists of integers, each \
               already sorted ascending, and returns one sorted list containing every element \
               of both. Do not call sort() or sorted(). Reply with only the function.",
        cases: &[
            Case {
                call: "merge([1,3,5],[2,4,6])",
                expects: "[1, 2, 3, 4, 5, 6]",
            },
            Case {
                call: "merge([],[1])",
                expects: "[1]",
            },
            Case {
                call: "merge([2,2],[2])",
                expects: "[2, 2, 2]",
            },
        ],
    },
    Task {
        name: "balanced-brackets",
        function: "balanced",
        asks: "Write a Python function `balanced(s)` that returns True if every bracket in the \
               string s is closed in the right order, and False otherwise. The brackets are (), \
               [] and {}. Reply with only the function.",
        cases: &[
            Case {
                call: "balanced('([]{})')",
                expects: "True",
            },
            Case {
                call: "balanced('([)]')",
                expects: "False",
            },
            Case {
                call: "balanced('')",
                expects: "True",
            },
        ],
    },
    Task {
        name: "run-length",
        function: "encode",
        asks: "Write a Python function `encode(s)` that returns the run-length encoding of the \
               string s as a list of (character, count) tuples, in order. Reply with only the \
               function.",
        cases: &[
            Case {
                call: "encode('aaabbc')",
                expects: "[('a', 3), ('b', 2), ('c', 1)]",
            },
            Case {
                call: "encode('')",
                expects: "[]",
            },
            Case {
                call: "encode('ab')",
                expects: "[('a', 1), ('b', 1)]",
            },
        ],
    },
    Task {
        name: "edit-distance",
        function: "distance",
        asks: "Write a Python function `distance(a, b)` returning the Levenshtein edit distance \
               between two strings — the fewest single-character insertions, deletions or \
               substitutions that turn a into b. Reply with only the function.",
        cases: &[
            Case {
                call: "distance('kitten','sitting')",
                expects: "3",
            },
            Case {
                call: "distance('','abc')",
                expects: "3",
            },
            Case {
                call: "distance('same','same')",
                expects: "0",
            },
        ],
    },
    Task {
        name: "spiral-order",
        function: "spiral",
        asks: "Write a Python function `spiral(m)` that takes a rectangular list of lists of \
               integers and returns a flat list of its elements in clockwise spiral order, \
               starting at the top-left and going right first. Reply with only the function.",
        cases: &[
            Case {
                call: "spiral([[1,2,3],[4,5,6],[7,8,9]])",
                expects: "[1, 2, 3, 6, 9, 8, 7, 4, 5]",
            },
            Case {
                call: "spiral([[1,2],[3,4],[5,6]])",
                expects: "[1, 2, 4, 6, 5, 3]",
            },
            Case {
                call: "spiral([[1]])",
                expects: "[1]",
            },
        ],
    },
    Task {
        name: "expression-value",
        function: "value",
        asks: "Write a Python function `value(s)` that evaluates an arithmetic expression given \
               as a string and returns an integer. The expression contains non-negative \
               integers, the operators + - * /, and parentheses. Multiplication and division \
               bind tighter than addition and subtraction; division truncates toward zero. Do \
               not use eval(). Reply with only the function.",
        cases: &[
            Case {
                call: "value('2+3*4')",
                expects: "14",
            },
            Case {
                call: "value('(2+3)*4')",
                expects: "20",
            },
            Case {
                call: "value('7/2')",
                expects: "3",
            },
        ],
    },
    Task {
        name: "shortest-path",
        function: "shortest",
        asks: "Write a Python function `shortest(edges, start, goal)` where edges is a list of \
               (u, v, w) tuples describing a directed graph with non-negative integer weights. \
               Return the total weight of the cheapest path from start to goal, or -1 if there \
               is none. Reply with only the function.",
        cases: &[
            Case {
                call: "shortest([(1,2,4),(1,3,1),(3,2,1)],1,2)",
                expects: "2",
            },
            Case {
                call: "shortest([(1,2,1)],2,1)",
                expects: "-1",
            },
            Case {
                call: "shortest([],1,1)",
                expects: "0",
            },
        ],
    },
    Task {
        name: "glob-match",
        function: "matches",
        asks: "Write a Python function `matches(pattern, text)` returning True if the whole of \
               text matches the pattern. In the pattern, `?` matches exactly one character and \
               `*` matches any run of characters including none. Every other character matches \
               itself. Do not use the re or fnmatch modules. Reply with only the function.",
        cases: &[
            Case {
                call: "matches('*a*b','xaxbx')",
                expects: "False",
            },
            Case {
                call: "matches('a?c','abc')",
                expects: "True",
            },
            Case {
                call: "matches('*','')",
                expects: "True",
            },
        ],
    },
    Task {
        name: "topological-order",
        function: "order",
        asks: "Write a Python function `order(n, edges)` where the vertices are 1 to n and edges \
               is a list of (u, v) tuples meaning u must come before v. Return the \
               lexicographically smallest ordering that satisfies every edge, or an empty list \
               if none exists. Reply with only the function.",
        cases: &[
            Case {
                call: "order(4,[(1,2),(1,3),(3,4)])",
                expects: "[1, 2, 3, 4]",
            },
            Case {
                call: "order(2,[(1,2),(2,1)])",
                expects: "[]",
            },
            Case {
                call: "order(3,[])",
                expects: "[1, 2, 3]",
            },
        ],
    },
    Task {
        name: "days-between",
        function: "between",
        asks: "Write a Python function `between(a, b)` taking two dates as 'YYYY-MM-DD' strings \
               and returning the number of whole days between them as a non-negative integer. \
               Do not use the datetime or calendar modules. Reply with only the function.",
        cases: &[
            Case {
                call: "between('2024-02-28','2024-03-01')",
                expects: "2",
            },
            Case {
                call: "between('1900-02-28','1900-03-01')",
                expects: "1",
            },
            Case {
                call: "between('2020-01-01','2020-01-01')",
                expects: "0",
            },
        ],
    },
    Task {
        name: "n-queens",
        function: "count",
        asks: "Write a Python function `count(n)` returning how many ways n queens can be placed \
               on an n by n board so that no two attack each other. Reply with only the function.",
        cases: &[
            Case {
                call: "count(6)",
                expects: "4",
            },
            Case {
                call: "count(8)",
                expects: "92",
            },
            Case {
                call: "count(1)",
                expects: "1",
            },
        ],
    },
    Task {
        name: "lru-cache",
        function: "run_cache",
        asks: "Write a Python function `run_cache(capacity, ops)` simulating a least-recently-used \
               cache. ops is a list of ('put', key, value) or ('get', key) tuples. Return the \
               list of results of the get operations in order, using -1 when a key is absent. \
               Both put and get count as a use. Reply with only the function.",
        cases: &[
            Case {
                call: "run_cache(2,[('put',1,1),('put',2,2),('get',1),('put',3,3),('get',2)])",
                expects: "[1, -1]",
            },
            Case {
                call: "run_cache(1,[('put',1,1),('put',2,2),('get',1),('get',2)])",
                expects: "[-1, 2]",
            },
            Case {
                call: "run_cache(2,[('get',9)])",
                expects: "[-1]",
            },
        ],
    },
];

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
fn run_in_container(podman: &Path, scratch: &Path, task: &Task, written: &str) -> Ran {
    if written.trim().is_empty() {
        return Ran::Refused {
            because: "the answer held no code".to_owned(),
            wrote_something: false,
        };
    }
    let program = scratch.join("answer.py");
    if let Err(error) = std::fs::write(&program, checker(task, written)) {
        return Ran::Refused {
            because: format!("the answer could not be written down: {error}"),
            wrote_something: true,
        };
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
        .output();
    let spoke = match spoke {
        Ok(spoke) => spoke,
        Err(error) => {
            return Ran::Refused {
                because: format!("the checker could not be started: {error}"),
                wrote_something: true,
            };
        }
    };
    let said = String::from_utf8_lossy(&spoke.stdout);
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

/// Evaluates one model against every task.
pub(crate) fn eval(named: &str) -> Response {
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
        format!(
            "  {} task(s), {ATTEMPTS} attempt(s) each, every answer run in a container with no \
             network",
            TASKS.len()
        ),
        match &addressed {
            Some(how) => format!("  addressed as {how}"),
            None => "  addressed as: nothing was applied, so each task is sent as raw text — a                      model trained to be addressed will complete it rather than answer it, and                      that is a condition of everything below. `mcf probe <model> --apply`                      settles it"
                .to_owned(),
        },
        String::new(),
    ];
    let mut held: Vec<Trials> = Vec::new();
    for task in TASKS {
        let mut ask = |task: &Task| {
            let spoken = mcf_serve::probes::spoken(&socket, Path::new(named), task.asks, None, 400, None);
            (!spoken.text.trim().is_empty()).then(|| code_in(&spoken.text))
        };
        let mut run = |task: &Task, written: &str| {
            run_in_container(&podman, &scratch, task, written)
        };
        held.push(measure(task, ATTEMPTS, &mut ask, &mut run));
    }
    let _gone = std::fs::remove_dir_all(&scratch);

    for trials in &held {
        lines.push(format!("  {}", trials.task));
        lines.push(format!(
            "    {} of {} attempt(s) ran; {} satisfied every case",
            trials.ran(),
            trials.attempts.len(),
            trials.whole()
        ));
        lines.push(format!("    {}", trials.graded()));
        for (at, attempt) in trials.attempts.iter().enumerate() {
            lines.push(match attempt {
                Ran::Checked { passed, of } => {
                    format!("      attempt {}: {passed} of {of} case(s) held", at + 1)
                }
                Ran::Refused { because, .. } => {
                    format!("      attempt {}: did not run — {because}", at + 1)
                }
            });
        }
        lines.push(String::new());
    }
    lines.push(
        "  There is no total. Four outcomes and no overall rating: a task that could not be run \
         is not a low score, and *which model is better* has no referent once quality is plural."
            .to_owned(),
    );
    if addressed.is_none() {
        lines.push(String::new());
        lines.push(
            "  Nothing above was addressed as this model was trained to be. Where a task reads \
             *unknown*, that may be the addressing rather than the model."
                .to_owned(),
        );
    }
    Response {
        text: lines.join("\n"),
        served: true,
    }
}
