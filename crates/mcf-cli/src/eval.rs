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
use mcf_record::json::Value;

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
pub(crate) const IMAGE: &str = "docker.io/library/python";
/// `python:3.12-slim`, read from the registry rather than written from memory.
pub(crate) const IMAGE_DIGEST: &str =
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
    // **Four tasks a remembered answer gets wrong.** Measured across three
    // models, six of the twelve tasks before these were passed by all of them
    // — including a 2B general model — and exactly one separated an 8B coding
    // model from a 30B one. A corpus of canonical exercises cannot order
    // strong coding models, because the answers are recalled rather than
    // worked out; what separates them has to be a sentence that has to be
    // read. Each of these is a familiar problem with one rule changed, so the
    // remembered solution is available and wrong (B-110, §XIII).
    Task {
        name: "intervals-touching",
        function: "merge",
        asks: "Write a Python function `merge(xs)` taking a list of [start, end] intervals and \
               returning the list of merged intervals, sorted. Intervals are half-open: [1,2] \
               and [2,3] do not overlap and are not merged. Return a list of lists. Reply with \
               only the function.",
        cases: &[
            // The whole task: the remembered merge uses `start <= last_end`
            // and answers [[1, 3]].
            Case {
                call: "merge([[1,2],[2,3]])",
                expects: "[[1, 2], [2, 3]]",
            },
            Case {
                call: "merge([[1,5],[2,3],[6,8]])",
                expects: "[[1, 5], [6, 8]]",
            },
            Case {
                call: "merge([])",
                expects: "[]",
            },
        ],
    },
    Task {
        name: "brackets-in-quotes",
        function: "balanced",
        asks: "Write a Python function `balanced(text)` returning True if the brackets in text \
               are balanced. (), [] and {} are brackets. A single quote starts a quoted stretch \
               and the next single quote ends it; brackets inside a quoted stretch are ordinary \
               characters and do not count. Text ending inside a quoted stretch is not balanced. \
               Reply with only the function.",
        cases: &[
            Case {
                call: "balanced(\"(a['b'])\")",
                expects: "True",
            },
            // A stack that does not know about quotes sees ( ( ) and says
            // False.
            Case {
                call: "balanced(\"('(')\")",
                expects: "True",
            },
            Case {
                call: "balanced(\"(']')\")",
                expects: "True",
            },
        ],
    },
    Task {
        name: "duration-seconds",
        function: "seconds",
        asks: "Write a Python function `seconds(text)` turning a duration into a whole number of \
               seconds. The text is a run of number-unit pairs where the unit is h, m or s — for \
               example '1h30m'. The pairs may come in any order and a unit may appear more than \
               once, in which case they are all added up. Empty text is 0. Reply with only the \
               function.",
        cases: &[
            Case {
                call: "seconds('1h30m')",
                expects: "5400",
            },
            // Order and repetition, which are the two rules the sentence adds.
            Case {
                call: "seconds('30m1h')",
                expects: "5400",
            },
            Case {
                call: "seconds('1h1h')",
                expects: "7200",
            },
        ],
    },
    Task {
        name: "rle-decode-counts",
        function: "decode",
        asks: "Write a Python function `decode(text)` expanding a run-length encoding. The text \
               is a run of items; each item is one character followed by an optional decimal \
               count, which may have more than one digit. A missing count means one. A count of \
               zero means the character does not appear. Reply with only the function.",
        cases: &[
            // Twelve, not one then a literal 2.
            Case {
                call: "decode('a12b')",
                expects: "'aaaaaaaaaaaab'",
            },
            Case {
                call: "decode('ab')",
                expects: "'ab'",
            },
            Case {
                call: "decode('a0b2')",
                expects: "'bb'",
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
    // **Four tasks that read input, handle errors and edit code** (B-518):
    // parse text into a number and refuse what does not parse, fix a
    // function that is handed over broken, count words under a stated tie
    // order, and read a numeral system with a subtraction rule.
    Task {
        name: "parse-duration",
        function: "seconds",
        asks: "Write a Python function `seconds(text)` that parses a duration written as hours \
               and minutes, like \"1h30m\", \"2h\" or \"45m\", and returns the total number of \
               seconds as an int. Return None for anything that is not in that form, such as \
               an empty string, \"90\" or \"1h30\". Reply with only the function.",
        cases: &[
            Case {
                call: "seconds('1h30m')",
                expects: "5400",
            },
            Case {
                call: "seconds('45m')",
                expects: "2700",
            },
            Case {
                call: "seconds('2h')",
                expects: "7200",
            },
            Case {
                call: "seconds('1h30')",
                expects: "None",
            },
            Case {
                call: "seconds('')",
                expects: "None",
            },
        ],
    },
    Task {
        name: "fix-the-median",
        function: "median",
        asks: "This Python function is meant to return the median of a non-empty list of \
               integers as an int when the count is odd and as the mean of the two middle \
               values, as a float, when it is even; it is wrong. Fix it and reply with only the \
               corrected function.\n\ndef median(xs):\n    xs = sorted(xs)\n    n = len(xs)\n    \
               return xs[n // 2]",
        cases: &[
            Case {
                call: "median([3,1,2])",
                expects: "2",
            },
            Case {
                call: "median([4,1,3,2])",
                expects: "2.5",
            },
            Case {
                call: "median([7])",
                expects: "7",
            },
            Case {
                call: "median([1,2,3,4,5,6])",
                expects: "3.5",
            },
        ],
    },
    Task {
        name: "word-frequency",
        function: "top_words",
        asks: "Write a Python function `top_words(text, k)` that splits the text on whitespace, \
               lowercases the words, strips the characters .,;:!? from each end of each word, \
               and returns a list of the k most frequent words as [word, count] pairs, most \
               frequent first; words with the same count are ordered alphabetically. Reply \
               with only the function.",
        cases: &[
            Case {
                call: "top_words('the cat and the hat. The end!', 2)",
                expects: "[['the', 3], ['and', 1]]",
            },
            Case {
                call: "top_words('b a b a c', 3)",
                expects: "[['a', 2], ['b', 2], ['c', 1]]",
            },
            Case {
                call: "top_words('', 2)",
                expects: "[]",
            },
        ],
    },
    Task {
        name: "roman-to-integer",
        function: "roman",
        asks: "Write a Python function `roman(s)` that converts a Roman numeral written with \
               the letters I, V, X, L, C, D and M, including the subtractive forms IV, IX, \
               XL, XC, CD and CM, to an int. Reply with only the function.",
        cases: &[
            Case {
                call: "roman('XIV')",
                expects: "14",
            },
            Case {
                call: "roman('MCMXCIV')",
                expects: "1994",
            },
            Case {
                call: "roman('III')",
                expects: "3",
            },
            Case {
                call: "roman('XLII')",
                expects: "42",
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

/// Which cases held, in the task's order, from the checker's lines; empty
/// where the program did not reach every case.
fn cases_held(podman: &Path, scratch: &Path, task: &Task, written: &str) -> (Ran, Vec<bool>) {
    let ran = run_in_container(podman, scratch, task, written);
    let held = match &ran {
        Ran::Checked { .. } => {
            // Read again from the program's own output kept beside the
            // answer: the runner reports the count, and the repair needs
            // to know which.
            let said = std::fs::read_to_string(scratch.join("answer.out")).unwrap_or_default();
            said.lines()
                .filter(|line| matches!(*line, "ok" | "no"))
                .map(|line| line == "ok")
                .collect()
        }
        Ran::Refused { .. } => Vec::new(),
    };
    (ran, held)
}

/// The feedback a failed attempt is handed back with: the cases that did
/// not hold, each with what the call should return, or that the answer
/// did not run to the end of the checks (B-521).
fn feedback(task: &Task, written: &str, ran: &Ran, held: &[bool]) -> String {
    let mut out = format!(
        "{}\n\nYou answered:\n```python\n{written}\n```\n",
        task.asks
    );
    match ran {
        Ran::Checked { .. } => {
            out.push_str("That answer is wrong:\n");
            for (case, ok) in task.cases.iter().zip(held) {
                if !ok {
                    use std::fmt::Write as _;
                    let _wrote = writeln!(out, "- {} should return {}", case.call, case.expects);
                }
            }
        }
        Ran::Refused { .. } => {
            out.push_str(
                "That answer did not run to the end of the checks: it raised, did not parse, or \
                 did not define the function.\n",
            );
        }
    }
    out.push_str("Reply with only the corrected function.");
    out
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
pub(crate) const SUITES: [&str; 6] = [
    "coding",
    "languages",
    "editing",
    "tests",
    "queries",
    "challenges",
];

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
}

/// Evaluates one model against every task, or one suite's.
#[allow(
    clippy::too_many_lines,
    reason = "the laboratory's one drive: the container found, each task asked and run, every attempt a row, the report said"
)]
pub(crate) fn eval(named: &str, asked: &Asked<'_>) -> Response {
    let only = asked.only;
    let plan = match crate::challenges::Plan::asked(asked.languages, asked.retries, asked.tier) {
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
        if wants("coding") {
            format!(
                "  {} task(s), {ATTEMPTS} attempt(s) each, every answer run in a container with \
                 no network",
                TASKS.len()
            )
        } else {
            format!("  only the {} suite", only.unwrap_or(""))
        },
        match &addressed {
            Some(how) => format!("  addressed as {how}"),
            None => "  addressed as: nothing was applied, so each task is sent as raw text — a                      model trained to be addressed will complete it rather than answer it, and                      that is a condition of everything below. `mcf probe <model> --apply`                      settles it"
                .to_owned(),
        },
        String::new(),
    ];
    let mut held: Vec<Trials> = Vec::new();
    // Every attempt a row: what it wrote, whether it ran, the cases held,
    // and what the asking and the running took (D54, B-518).
    let mut rows: Vec<mcf_serve::examine::Reading> = Vec::new();
    // And every failed attempt handed back with what did not hold, for a
    // second try: whether it was repaired is its own row (B-521).
    let mut repairs: Vec<mcf_serve::examine::Reading> = Vec::new();
    let mut second_tries: std::collections::BTreeMap<(String, usize), String> =
        std::collections::BTreeMap::new();
    let (mut failed, mut repaired) = (0_usize, 0_usize);
    // The engine the daemon ran the asks on, as the first account named
    // it: the rows' conditions say what answered, not only that the
    // daemon did (B-542).
    let mut engine_ran: Option<String> = None;
    for (at, task) in if wants("coding") { TASKS } else { &[] }.iter().enumerate() {
        progress(at, TASKS.len(), &format!("coding · python · {}", task.name));
        let mut timings: Vec<(u64, usize, String)> = Vec::new();
        let mut cases: Vec<Vec<bool>> = Vec::new();
        let mut ask = |task: &Task| {
            let began = std::time::Instant::now();
            let spoken =
                mcf_serve::probes::spoken(&socket, Path::new(named), task.asks, None, 400, None);
            let took = u64::try_from(began.elapsed().as_nanos()).unwrap_or(u64::MAX);
            if engine_ran.is_none() {
                engine_ran.clone_from(&spoken.engine_ran);
            }
            let written = (!spoken.text.trim().is_empty()).then(|| code_in(&spoken.text));
            timings.push((
                took,
                written.as_ref().map_or(0, String::len),
                written.clone().unwrap_or_default(),
            ));
            written
        };
        let mut run = |task: &Task, written: &str| {
            let (ran, which) = cases_held(&podman, &scratch, task, written);
            cases.push(which);
            ran
        };
        let trials = measure(task, ATTEMPTS, &mut ask, &mut run);
        for (attempt, ran) in trials.attempts.iter().enumerate() {
            let (ask_ns, code_bytes, written) =
                timings
                    .get(attempt)
                    .cloned()
                    .unwrap_or((0, 0, String::new()));
            rows.extend(attempt_rows(
                "python", task.name, attempt, ran, ask_ns, code_bytes,
            ));
            let whole = matches!(ran, Ran::Checked { passed, of } if passed == of);
            if whole || written.is_empty() {
                continue;
            }
            failed = failed.saturating_add(1);
            let which = cases.get(attempt).cloned().unwrap_or_default();
            let again = feedback(task, &written, ran, &which);
            let began = std::time::Instant::now();
            let spoken =
                mcf_serve::probes::spoken(&socket, Path::new(named), &again, None, 400, None);
            let took = u64::try_from(began.elapsed().as_nanos()).unwrap_or(u64::MAX);
            let fixed = code_in(&spoken.text);
            let second = run_in_container(&podman, &scratch, task, &fixed);
            let fixed_whole = matches!(second, Ran::Checked { passed, of } if passed == of);
            if fixed_whole {
                repaired = repaired.saturating_add(1);
            }
            second_tries.insert(
                (task.name.to_owned(), attempt),
                match &second {
                    Ran::Checked { passed, of } => {
                        format!("handed back what did not hold; second try: {passed} of {of} held")
                    }
                    Ran::Refused { because, .. } => {
                        format!("handed back what did not hold; second try did not run — {because}")
                    }
                },
            );
            let dims = [
                ("task", Value::text(task.name)),
                (
                    "attempt",
                    Value::Integer(i64::try_from(attempt).unwrap_or(i64::MAX)),
                ),
            ];
            repairs.push(mcf_serve::examine::Reading::new(
                &dims,
                "repaired",
                i64::from(fixed_whole),
                "bool",
            ));
            if let Ran::Checked { passed, of } = second {
                repairs.push(mcf_serve::examine::Reading::new(
                    &dims,
                    "cases_held",
                    i64::try_from(passed).unwrap_or(i64::MAX),
                    "count",
                ));
                repairs.push(mcf_serve::examine::Reading::new(
                    &dims,
                    "cases",
                    i64::try_from(of).unwrap_or(i64::MAX),
                    "count",
                ));
            }
            repairs.push(mcf_serve::examine::Reading::new(
                &dims,
                "ran",
                i64::from(matches!(second, Ran::Checked { .. })),
                "bool",
            ));
            repairs.push(mcf_serve::examine::Reading::new(
                &dims,
                "ask_ns",
                i64::try_from(took).unwrap_or(i64::MAX),
                "ns",
            ));
        }
        held.push(trials);
    }
    // The same tasks in the other languages, each in its own image, their
    // rows beside the Python ones under the same method (B-523).
    let (language_lines, language_rows, language_conditions, language_engine) =
        if wants("languages") {
            crate::languages::run(&socket, named, &podman, &scratch, ATTEMPTS)
        } else {
            (Vec::new(), Vec::new(), Vec::new(), None)
        };
    rows.extend(language_rows);
    if engine_ran.is_none() {
        engine_ran = language_engine;
    }
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
        crate::challenges::run(
            &socket,
            named,
            &podman,
            &scratch,
            &plan.languages,
            plan.retries,
            plan.tier,
        )
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
    let repairs_recorded = if repairs.is_empty() {
        None
    } else {
        Some(mcf_serve::examine::record_rows(
            Path::new(named),
            "coding-repair",
            &engine_said,
            vec![
                (
                    "failed",
                    Value::Integer(i64::try_from(failed).unwrap_or(i64::MAX)),
                ),
                (
                    "repaired",
                    Value::Integer(i64::try_from(repaired).unwrap_or(i64::MAX)),
                ),
            ],
            &repairs,
        ))
    };
    let recorded = (!rows.is_empty()).then(|| {
        mcf_serve::examine::record_rows(
            Path::new(named),
            "coding",
            &engine_said,
            vec![
                (
                    "tasks",
                    Value::Integer(i64::try_from(TASKS.len()).unwrap_or(i64::MAX)),
                ),
                (
                    "attempts",
                    Value::Integer(i64::try_from(ATTEMPTS).unwrap_or(i64::MAX)),
                ),
                (
                    "addressed",
                    addressed.clone().map_or(Value::Null, Value::text),
                ),
            ]
            .into_iter()
            .chain(language_conditions)
            .collect(),
            &rows,
        )
    });

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
            if let Some(said) = second_tries.get(&(trials.task.to_owned(), at)) {
                lines.push(format!("        {said}"));
            }
        }
        lines.push(String::new());
    }
    if wants("coding") {
        lines.push(
            "  There is no total. Four outcomes and no overall rating: a task that could not be \
             run is not a low score, and *which model is better* has no referent once quality is \
             plural."
                .to_owned(),
        );
    }
    if addressed.is_none() {
        lines.push(String::new());
        lines.push(
            "  Nothing above was addressed as this model was trained to be. Where a task reads \
             *unknown*, that may be the addressing rather than the model."
                .to_owned(),
        );
    }
    lines.push(String::new());
    if let Some(recorded) = recorded {
        lines.push(match recorded {
            Ok(_) => format!(
                "  {} reading(s) recorded; `mcf data {named} --method coding` writes them",
                rows.len()
            ),
            Err(why) => format!("  READINGS NOT RECORDED: {why}"),
        });
    }
    if wants("coding") {
        lines.push(format!(
            "  repaired on a second try, handed back what did not hold: {repaired} of {failed} \
             failed attempt(s){}",
            match repairs_recorded {
                Some(Ok(_)) => format!(" — {} reading(s) under coding-repair", repairs.len()),
                Some(Err(why)) => format!(" — READINGS NOT RECORDED: {why}"),
                None => String::new(),
            }
        ));
    }
    if wants("coding") {
        lines.push(String::new());
    }
    lines.extend(language_lines);
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

/// One attempt at one task as rows: whether anything was written, whether
/// it ran, the cases held of the cases, the code's size, and what the
/// asking took (D54, B-518).
pub(crate) fn attempt_rows(
    language: &str,
    task: &str,
    attempt: usize,
    ran: &Ran,
    ask_ns: u64,
    code_bytes: usize,
) -> Vec<mcf_serve::examine::Reading> {
    use mcf_serve::examine::Reading;
    let whole = |held: usize| i64::try_from(held).unwrap_or(i64::MAX);
    let dims = [
        ("language", Value::text(language.to_owned())),
        ("task", Value::text(task.to_owned())),
        ("attempt", Value::Integer(whole(attempt))),
    ];
    let mut rows = vec![
        Reading::new(
            &dims,
            "ask_ns",
            i64::try_from(ask_ns).unwrap_or(i64::MAX),
            "ns",
        ),
        Reading::new(&dims, "code_bytes", whole(code_bytes), "bytes"),
    ];
    match ran {
        Ran::Checked { passed, of } => {
            rows.push(Reading::new(&dims, "wrote", 1, "bool"));
            rows.push(Reading::new(&dims, "ran", 1, "bool"));
            rows.push(Reading::new(&dims, "cases_held", whole(*passed), "count"));
            rows.push(Reading::new(&dims, "cases", whole(*of), "count"));
            rows.push(Reading::new(
                &dims,
                "whole",
                i64::from(passed == of),
                "bool",
            ));
        }
        Ran::Refused {
            wrote_something, ..
        } => {
            rows.push(Reading::new(
                &dims,
                "wrote",
                i64::from(*wrote_something),
                "bool",
            ));
            rows.push(Reading::new(&dims, "ran", 0, "bool"));
        }
    }
    rows
}

#[cfg(test)]
mod tests {
    use super::{Case, Ran, Task, feedback};

    const TASK: Task = Task {
        name: "double",
        function: "double",
        asks: "Write double(n).",
        cases: &[
            Case {
                call: "double(2)",
                expects: "4",
            },
            Case {
                call: "double(3)",
                expects: "6",
            },
        ],
    };

    #[test]
    fn a_failed_case_is_handed_back_with_what_it_should_return() {
        let said = feedback(
            &TASK,
            "def double(n):\n    return n + n + 1",
            &Ran::Checked { passed: 1, of: 2 },
            &[true, false],
        );
        assert!(said.contains("Write double(n)."), "{said}");
        assert!(said.contains("return n + n + 1"), "{said}");
        assert!(said.contains("- double(3) should return 6"), "{said}");
        assert!(!said.contains("double(2) should"), "{said}");
        assert!(
            said.ends_with("Reply with only the corrected function."),
            "{said}"
        );
    }

    #[test]
    fn an_answer_that_did_not_run_is_told_so() {
        let said = feedback(
            &TASK,
            "def double(n)\n    return",
            &Ran::Refused {
                because: "syntax".to_owned(),
                wrote_something: true,
            },
            &[],
        );
        assert!(
            said.contains("did not run to the end of the checks"),
            "{said}"
        );
        assert!(!said.contains("should return"), "{said}");
    }
}
