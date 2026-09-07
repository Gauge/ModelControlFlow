//! The coding suite in a second and a third language: the same twenty
//! tasks asked for in JavaScript and in Rust, each answer run in its
//! language's own pinned image under the same discipline as the Python
//! one, and a row a language a task an attempt (B-523, D55, B-025).
//!
//! A task is the same problem in each language; its ask names the
//! language and, for Rust, the signature, and its cases are written in
//! the language's own literals and compared in the language's own
//! canonical text — `JSON.stringify` for JavaScript, `{:?}` for Rust —
//! so that what escapes the container is `ok` or `no` a case, as it is
//! for Python. A language whose image cannot be started says so once and
//! has no rows.

use std::path::Path;

use mcf_bench::eval::{Case, Ran, Task};
use mcf_record::json::Value;
use mcf_serve::examine::Reading;

/// One language the suite is asked in.
#[derive(Debug)]
pub(crate) struct Language {
    /// Its name as a dimension: `javascript`, `rust`.
    pub name: &'static str,
    /// The image the answers run in, for a reader.
    pub image: &'static str,
    /// The digest, which is what runs.
    pub digest: &'static str,
    /// The file the checker is written to inside the scratch directory.
    pub file: &'static str,
    /// The memory the container is allowed; a compiler needs more.
    pub memory: &'static str,
    /// The command run in the container, given the file at `/work`.
    pub command: &'static [&'static str],
    /// The command that shows the language can be run at all.
    pub present: &'static [&'static str],
    /// The tasks, in the Python suite's order.
    pub tasks: &'static [Task],
}

/// The languages beyond Python.
pub(crate) const LANGUAGES: &[Language] = &[
    Language {
        name: "javascript",
        image: "docker.io/library/node",
        // `node:22-slim`, read from the registry rather than written from memory.
        digest: "sha256:4d676821dff059fd00d277ee4261ef34ea712317fed0737c03941481b5760c96",
        file: "answer.js",
        memory: "512m",
        command: &["timeout", "20", "node", "/work/answer.js"],
        present: &["node", "--version"],
        tasks: JAVASCRIPT,
    },
    Language {
        name: "rust",
        image: "docker.io/library/rust",
        // `rust:1-slim`, read from the registry rather than written from memory.
        digest: "sha256:90fd7674d9f6c35662cbf59ec39c32175511a1b7f49e39adcbe91b7420e5e972",
        file: "answer.rs",
        memory: "1g",
        command: &[
            "timeout",
            "40",
            "sh",
            "-c",
            // The compiler's complaint and the program's own go to the error
            // stream, where the host reads their first lines back for the
            // model to correct; which of the two spoke is told by whether
            // `compiled` was printed (B-565, B-566).
            "if rustc --edition 2021 -A warnings -o /tmp/answer /work/answer.rs; then echo \
             compiled; /tmp/answer; else echo notcompiled; fi",
        ],
        present: &["rustc", "--version"],
        tasks: RUST,
    },
];

/// The model's answer as the checker can use it: a JavaScript answer with
/// its `export` words and `module.exports` line removed, since the
/// checker is the same file; a Rust answer with its own `fn main` block
/// removed, since the checker supplies one. Neither changes the function
/// asked for, the way stripping the fence around it does not.
#[must_use]
pub(crate) fn plain(language: &Language, written: &str) -> String {
    if language.name == "javascript" {
        return written
            .lines()
            .filter(|line| !line.trim_start().starts_with("module.exports"))
            .map(|line| {
                let mut line = line;
                line = line.strip_prefix("export default ").unwrap_or(line);
                line = line.strip_prefix("export ").unwrap_or(line);
                line
            })
            .collect::<Vec<&str>>()
            .join("\n");
    }
    without_main(written)
}

/// The Rust answer without a top-level `fn main` block, braces matched.
fn without_main(written: &str) -> String {
    without_main_named(written, "fn main(")
}

/// An answer without the block that begins with `head`, braces matched:
/// a Rust `fn main(` or a Go `func main(` the harness supplies itself.
pub(crate) fn without_main_named(written: &str, head: &str) -> String {
    let Some(at) = written.find(head) else {
        return written.to_owned();
    };
    let line_start = written
        .get(..at)
        .and_then(|before| before.rfind('\n'))
        .map_or(0, |nl| nl.wrapping_add(1));
    let Some(open) = written
        .get(at..)
        .and_then(|rest| rest.find('{'))
        .map(|off| at.wrapping_add(off))
    else {
        return written.to_owned();
    };
    let mut depth = 0_usize;
    let mut close = None;
    for (offset, character) in written.get(open..).unwrap_or("").char_indices() {
        match character {
            '{' => depth = depth.wrapping_add(1),
            '}' => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    close = Some(open.wrapping_add(offset).wrapping_add(1));
                    break;
                }
            }
            _ => {}
        }
    }
    let Some(close) = close else {
        return written.to_owned();
    };
    format!(
        "{}{}",
        written.get(..line_start).unwrap_or(""),
        written.get(close..).unwrap_or("").trim_start_matches('\n')
    )
}

/// The program the container runs for one language: the model's answer,
/// then the cases, each printing `ok` or `no` and nothing else.
#[must_use]
pub(crate) fn checker(language: &Language, task: &Task, written: &str) -> String {
    use std::fmt::Write as _;
    let mut out = plain(language, written);
    if language.name == "javascript" {
        out.push_str("\n\nconst __cases = [\n");
        for case in task.cases {
            let _wrote = writeln!(
                out,
                "  [() => {}, {}],",
                case.call,
                javascript_string(case.expects)
            );
        }
        out.push_str(
            "];\nfor (const [call, want] of __cases) {\n  let said = 'no';\n  try { said = \
                 String(JSON.stringify(call())) === want ? 'ok' : 'no'; } catch (e) {}\n  \
                 console.log(said);\n}\n",
        );
    } else {
        out.push_str("\n\nfn main() {\n    std::panic::set_hook(Box::new(|_| {}));\n");
        for case in task.cases {
            let _wrote = writeln!(
                out,
                "    {{\n        let said = std::panic::catch_unwind(|| format!(\"{{:?}}\", \
                     {}));\n        println!(\"{{}}\", if said.as_deref().ok() == Some({}) {{ \
                     \"ok\" }} else {{ \"no\" }});\n    }}",
                case.call,
                rust_string(case.expects)
            );
        }
        out.push_str("}\n");
    }
    out
}

/// A JavaScript string literal holding the text.
fn javascript_string(text: &str) -> String {
    format!("'{}'", text.replace('\\', "\\\\").replace('\'', "\\'"))
}

/// A Rust string literal holding the text.
fn rust_string(text: &str) -> String {
    format!("r#\"{text}\"#")
}

/// Why a compiled language's answer did not run: the compiler refused it.
pub(crate) const NOT_COMPILED: &str = "the answer did not compile";

/// Whether the language's image starts and its tool answers.
pub(crate) fn present(podman: &Path, language: &Language) -> Result<String, String> {
    let spoke = std::process::Command::new(podman)
        .env_remove("XDG_DATA_HOME")
        .arg("run")
        .arg("--rm")
        .arg("--network=none")
        .arg(format!("{}@{}", language.image, language.digest))
        .args(language.present)
        .output()
        .map_err(|error| format!("the container could not be started: {error}"))?;
    if spoke.status.success() {
        Ok(String::from_utf8_lossy(&spoke.stdout).trim().to_owned())
    } else {
        Err(format!(
            "the image {}@{} did not answer: {}",
            language.image,
            language.digest,
            String::from_utf8_lossy(&spoke.stderr)
                .lines()
                .last()
                .unwrap_or("")
                .trim()
        ))
    }
}

/// Runs one answer in the language's container, the same way the Python
/// answers run: no network, no capabilities, a read-only root, a scratch
/// of its own and a writable `/tmp` for a compiler's output (B-025).
pub(crate) fn run_in_container(
    podman: &Path,
    scratch: &Path,
    language: &Language,
    task: &Task,
    written: &str,
) -> Ran {
    if written.trim().is_empty() {
        return Ran::Refused {
            because: "the answer held no code".to_owned(),
            wrote_something: false,
        };
    }
    let said = match run_program(podman, scratch, language, &checker(language, task, written)) {
        Ok(said) => said,
        Err(because) => {
            return Ran::Refused {
                because,
                wrote_something: true,
            };
        }
    };
    if said.lines().any(|line| line == "notcompiled") {
        return Ran::Refused {
            because: NOT_COMPILED.to_owned(),
            wrote_something: true,
        };
    }
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

/// Runs one program in the language's container and returns what it
/// printed, under the same confinement as the Python runner: no network,
/// no capabilities, a read-only root with a private /tmp for a compiler's
/// output, a memory ceiling, a process limit and a deadline (B-025).
pub(crate) fn run_program(
    podman: &Path,
    scratch: &Path,
    language: &Language,
    program: &str,
) -> Result<String, String> {
    run_program_heard(podman, scratch, language, program).map(|(said, _)| said)
}

/// [`run_program`] with the container's error stream as well: what the
/// compiler said where the language compiles, since the harness sends
/// the program's own error stream nowhere (B-565).
pub(crate) fn run_program_heard(
    podman: &Path,
    scratch: &Path,
    language: &Language,
    program: &str,
) -> Result<(String, String), String> {
    if let Err(error) = std::fs::write(scratch.join(language.file), program) {
        return Err(format!("the answer could not be written down: {error}"));
    }
    let spoke = std::process::Command::new(podman)
        .env_remove("XDG_DATA_HOME")
        .arg("run")
        .arg("--rm")
        .arg("--network=none")
        .arg("--cap-drop=ALL")
        .arg("--security-opt=no-new-privileges")
        .arg("--read-only")
        .arg("--tmpfs")
        .arg("/tmp:rw,exec,size=256m")
        .arg(format!("--memory={}", language.memory))
        .arg("--pids-limit=64")
        .arg("-v")
        .arg(format!("{}:/work:ro,z", scratch.display()))
        .arg(format!("{}@{}", language.image, language.digest))
        .args(language.command)
        .output();
    let spoke = spoke.map_err(|error| format!("the checker could not be started: {error}"))?;
    Ok((
        String::from_utf8_lossy(&spoke.stdout).into_owned(),
        String::from_utf8_lossy(&spoke.stderr).into_owned(),
    ))
}

/// What a run of the languages came to: the lines said, the rows, a
/// condition a language saying whether it could be run, and the engine
/// the daemon ran the asks on where an account named one (B-523, B-542).
pub(crate) type Suite = (
    Vec<String>,
    Vec<Reading>,
    Vec<(&'static str, Value)>,
    Option<String>,
);

/// Runs every task in every language beyond Python, each answer in its
/// language's container.
pub(crate) fn run(
    socket: &Path,
    named: &str,
    podman: &Path,
    scratch: &Path,
    attempts: usize,
) -> Suite {
    let mut lines = Vec::new();
    let mut rows = Vec::new();
    let mut conditions = Vec::new();
    let mut engine_ran = None;
    for language in LANGUAGES {
        let version = match present(podman, language) {
            Ok(version) => version,
            Err(why) => {
                lines.push(format!("  {}: could not be run — {why}", language.name));
                lines.push(String::new());
                conditions.push((language.name, Value::Null));
                continue;
            }
        };
        conditions.push((language.name, Value::text(version.clone())));
        lines.push(format!(
            "  {}: {} task(s), {attempts} attempt(s) each, run in {}@{} ({version})",
            language.name,
            language.tasks.len(),
            language.image,
            language.digest.get(..19).unwrap_or(language.digest)
        ));
        for (at, task) in language.tasks.iter().enumerate() {
            crate::eval::progress(
                at,
                language.tasks.len(),
                &format!("coding · {} · {}", language.name, task.name),
            );
            let mut said = Vec::with_capacity(attempts);
            for attempt in 0..attempts {
                let began = std::time::Instant::now();
                let spoken =
                    mcf_serve::probes::spoken(socket, Path::new(named), task.asks, None, 400, None);
                let ask_ns = u64::try_from(began.elapsed().as_nanos()).unwrap_or(u64::MAX);
                if engine_ran.is_none() {
                    engine_ran.clone_from(&spoken.engine_ran);
                }
                let written = crate::eval::code_in(&spoken.text);
                let ran = run_in_container(podman, scratch, language, task, &written);
                said.push(match &ran {
                    Ran::Checked { passed, of } => format!("{passed}/{of}"),
                    Ran::Refused { because, .. } if because == NOT_COMPILED => {
                        "did not compile".to_owned()
                    }
                    Ran::Refused { .. } => "did not run".to_owned(),
                });
                rows.extend(crate::eval::attempt_rows(
                    language.name,
                    task.name,
                    attempt,
                    &ran,
                    ask_ns,
                    written.len(),
                ));
                if language.name == "rust" && !written.trim().is_empty() {
                    let compiled =
                        !matches!(&ran, Ran::Refused { because, .. } if because == NOT_COMPILED);
                    rows.push(Reading::new(
                        &[
                            ("language", Value::text(language.name.to_owned())),
                            ("task", Value::text(task.name.to_owned())),
                            (
                                "attempt",
                                Value::Integer(i64::try_from(attempt).unwrap_or(i64::MAX)),
                            ),
                        ],
                        "compiled",
                        i64::from(compiled),
                        "bool",
                    ));
                }
            }
            lines.push(format!("    {:<22} {}", task.name, said.join("   ")));
        }
        lines.push(String::new());
    }
    (lines, rows, conditions, engine_ran)
}

/// The tasks in JavaScript. The cases are compared as `JSON.stringify`
/// text.
const JAVASCRIPT: &[Task] = &[
    Task {
        name: "merge-sorted",
        function: "merge",
        asks: "Write a JavaScript function `merge(a, b)` that takes two arrays of integers, each \
               already sorted ascending, and returns one sorted array containing every element \
               of both. Do not call sort(). Reply with only the function.",
        cases: &[
            Case {
                call: "merge([1,3,5],[2,4,6])",
                expects: "[1,2,3,4,5,6]",
            },
            Case {
                call: "merge([],[1])",
                expects: "[1]",
            },
            Case {
                call: "merge([2,2],[2])",
                expects: "[2,2,2]",
            },
        ],
    },
    Task {
        name: "balanced-brackets",
        function: "balanced",
        asks: "Write a JavaScript function `balanced(s)` that returns true if every bracket in \
               the string s is closed in the right order, and false otherwise. The brackets are \
               (), [] and {}. Reply with only the function.",
        cases: &[
            Case {
                call: "balanced('([]{})')",
                expects: "true",
            },
            Case {
                call: "balanced('([)]')",
                expects: "false",
            },
            Case {
                call: "balanced('')",
                expects: "true",
            },
        ],
    },
    Task {
        name: "run-length",
        function: "encode",
        asks: "Write a JavaScript function `encode(s)` that returns the run-length encoding of \
               the string s as an array of [character, count] pairs, in order. Reply with only \
               the function.",
        cases: &[
            Case {
                call: "encode('aaabbc')",
                expects: "[[\"a\",3],[\"b\",2],[\"c\",1]]",
            },
            Case {
                call: "encode('')",
                expects: "[]",
            },
            Case {
                call: "encode('ab')",
                expects: "[[\"a\",1],[\"b\",1]]",
            },
        ],
    },
    Task {
        name: "edit-distance",
        function: "distance",
        asks: "Write a JavaScript function `distance(a, b)` returning the Levenshtein edit \
               distance between two strings — the fewest single-character insertions, deletions \
               or substitutions that turn a into b. Reply with only the function.",
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
        asks: "Write a JavaScript function `spiral(m)` that takes a rectangular array of arrays \
               of integers and returns a flat array of its elements in clockwise spiral order, \
               starting at the top-left and going right first. Reply with only the function.",
        cases: &[
            Case {
                call: "spiral([[1,2,3],[4,5,6],[7,8,9]])",
                expects: "[1,2,3,6,9,8,7,4,5]",
            },
            Case {
                call: "spiral([[1,2],[3,4],[5,6]])",
                expects: "[1,2,4,6,5,3]",
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
        asks: "Write a JavaScript function `value(s)` that evaluates an arithmetic expression \
               given as a string and returns an integer. The expression contains non-negative \
               integers, the operators + - * /, and parentheses. Multiplication and division \
               bind tighter than addition and subtraction; division truncates toward zero. Do \
               not use eval() or the Function constructor. Reply with only the function.",
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
        asks: "Write a JavaScript function `shortest(edges, start, goal)` where edges is an \
               array of [u, v, w] arrays describing a directed graph with non-negative integer \
               weights. Return the total weight of the cheapest path from start to goal, or -1 \
               if there is none. Reply with only the function.",
        cases: &[
            Case {
                call: "shortest([[1,2,4],[1,3,1],[3,2,1]],1,2)",
                expects: "2",
            },
            Case {
                call: "shortest([[1,2,1]],2,1)",
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
        asks: "Write a JavaScript function `matches(pattern, text)` returning true if the whole \
               of text matches the pattern. In the pattern, `?` matches exactly one character \
               and `*` matches any run of characters including none. Every other character \
               matches itself. Do not use regular expressions. Reply with only the function.",
        cases: &[
            Case {
                call: "matches('*a*b','xaxbx')",
                expects: "false",
            },
            Case {
                call: "matches('a?c','abc')",
                expects: "true",
            },
            Case {
                call: "matches('*','')",
                expects: "true",
            },
        ],
    },
    Task {
        name: "topological-order",
        function: "order",
        asks: "Write a JavaScript function `order(n, edges)` where the vertices are 1 to n and \
               edges is an array of [u, v] pairs meaning u must come before v. Return the \
               lexicographically smallest ordering that satisfies every edge as an array, or an \
               empty array if none exists. Reply with only the function.",
        cases: &[
            Case {
                call: "order(4,[[1,2],[1,3],[3,4]])",
                expects: "[1,2,3,4]",
            },
            Case {
                call: "order(2,[[1,2],[2,1]])",
                expects: "[]",
            },
            Case {
                call: "order(3,[])",
                expects: "[1,2,3]",
            },
        ],
    },
    Task {
        name: "days-between",
        function: "between",
        asks: "Write a JavaScript function `between(a, b)` taking two dates as 'YYYY-MM-DD' \
               strings and returning the number of whole days between them as a non-negative \
               integer. Do not use the Date object. Reply with only the function.",
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
        asks: "Write a JavaScript function `count(n)` returning how many ways n queens can be \
               placed on an n by n board so that no two attack each other. Reply with only the \
               function.",
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
        name: "intervals-touching",
        function: "merge",
        asks: "Write a JavaScript function `merge(xs)` taking an array of [start, end] intervals \
               and returning the array of merged intervals, sorted. Intervals are half-open: \
               [1,2] and [2,3] do not overlap and are not merged. Return an array of arrays. \
               Reply with only the function.",
        cases: &[
            Case {
                call: "merge([[1,2],[2,3]])",
                expects: "[[1,2],[2,3]]",
            },
            Case {
                call: "merge([[1,5],[2,3],[6,8]])",
                expects: "[[1,5],[6,8]]",
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
        asks: "Write a JavaScript function `balanced(text)` returning true if the brackets in \
               text are balanced. (), [] and {} are brackets. A single quote starts a quoted \
               stretch and the next single quote ends it; brackets inside a quoted stretch are \
               ordinary characters and do not count. Text ending inside a quoted stretch is not \
               balanced. Reply with only the function.",
        cases: &[
            Case {
                call: "balanced(\"(a['b'])\")",
                expects: "true",
            },
            Case {
                call: "balanced(\"('(')\")",
                expects: "true",
            },
            Case {
                call: "balanced(\"(']')\")",
                expects: "true",
            },
        ],
    },
    Task {
        name: "duration-seconds",
        function: "seconds",
        asks: "Write a JavaScript function `seconds(text)` turning a duration into a whole \
               number of seconds. The text is a run of number-unit pairs where the unit is h, m \
               or s — for example '1h30m'. The pairs may come in any order and a unit may appear \
               more than once, in which case they are all added up. Empty text is 0. Reply with \
               only the function.",
        cases: &[
            Case {
                call: "seconds('1h30m')",
                expects: "5400",
            },
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
        asks: "Write a JavaScript function `decode(text)` expanding a run-length encoding. The \
               text is a run of items; each item is one character followed by an optional \
               decimal count, which may have more than one digit. A missing count means one. A \
               count of zero means the character does not appear. Reply with only the function.",
        cases: &[
            Case {
                call: "decode('a12b')",
                expects: "\"aaaaaaaaaaaab\"",
            },
            Case {
                call: "decode('ab')",
                expects: "\"ab\"",
            },
            Case {
                call: "decode('a0b2')",
                expects: "\"bb\"",
            },
        ],
    },
    Task {
        name: "lru-cache",
        function: "run_cache",
        asks: "Write a JavaScript function `run_cache(capacity, ops)` simulating a \
               least-recently-used cache. ops is an array of ['put', key, value] or ['get', key] \
               arrays. Return the array of results of the get operations in order, using -1 when \
               a key is absent. Both put and get count as a use. Reply with only the function.",
        cases: &[
            Case {
                call: "run_cache(2,[['put',1,1],['put',2,2],['get',1],['put',3,3],['get',2]])",
                expects: "[1,-1]",
            },
            Case {
                call: "run_cache(1,[['put',1,1],['put',2,2],['get',1],['get',2]])",
                expects: "[-1,2]",
            },
            Case {
                call: "run_cache(2,[['get',9]])",
                expects: "[-1]",
            },
        ],
    },
    Task {
        name: "parse-duration",
        function: "seconds",
        asks: "Write a JavaScript function `seconds(text)` that parses a duration written as \
               hours and minutes, like \"1h30m\", \"2h\" or \"45m\", and returns the total \
               number of seconds as an integer. Return null for anything that is not in that \
               form, such as an empty string, \"90\" or \"1h30\". Reply with only the function.",
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
                expects: "null",
            },
            Case {
                call: "seconds('')",
                expects: "null",
            },
        ],
    },
    Task {
        name: "fix-the-median",
        function: "median",
        asks: "This JavaScript function is meant to return the median of a non-empty array of \
               integers: the middle value when the count is odd and the mean of the two middle \
               values when it is even; it is wrong. Fix it and reply with only the corrected \
               function.\n\nfunction median(xs) {\n  xs = [...xs].sort((a, b) => a - b);\n  \
               const n = xs.length;\n  return xs[Math.floor(n / 2)];\n}",
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
        asks: "Write a JavaScript function `top_words(text, k)` that splits the text on \
               whitespace, lowercases the words, strips the characters .,;:!? from each end of \
               each word, and returns an array of the k most frequent words as [word, count] \
               pairs, most frequent first; words with the same count are ordered alphabetically. \
               Reply with only the function.",
        cases: &[
            Case {
                call: "top_words('the cat and the hat. The end!', 2)",
                expects: "[[\"the\",3],[\"and\",1]]",
            },
            Case {
                call: "top_words('b a b a c', 3)",
                expects: "[[\"a\",2],[\"b\",2],[\"c\",1]]",
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
        asks: "Write a JavaScript function `roman(s)` that converts a Roman numeral written with \
               the letters I, V, X, L, C, D and M, including the subtractive forms IV, IX, XL, \
               XC, CD and CM, to an integer. Reply with only the function.",
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

/// The tasks in Rust. The ask states the signature, since the checker
/// calls it; the cases are compared as `{:?}` text.
const RUST: &[Task] = &[
    Task {
        name: "merge-sorted",
        function: "merge",
        asks: "Write a Rust function `fn merge(a: &[i64], b: &[i64]) -> Vec<i64>` that takes two \
               slices of integers, each already sorted ascending, and returns one sorted vector \
               containing every element of both. Do not call sort or sort_unstable. Reply with \
               only the function.",
        cases: &[
            Case {
                call: "merge(&[1,3,5],&[2,4,6])",
                expects: "[1, 2, 3, 4, 5, 6]",
            },
            Case {
                call: "merge(&[],&[1])",
                expects: "[1]",
            },
            Case {
                call: "merge(&[2,2],&[2])",
                expects: "[2, 2, 2]",
            },
        ],
    },
    Task {
        name: "balanced-brackets",
        function: "balanced",
        asks: "Write a Rust function `fn balanced(s: &str) -> bool` that returns true if every \
               bracket in the string s is closed in the right order, and false otherwise. The \
               brackets are (), [] and {}. Reply with only the function.",
        cases: &[
            Case {
                call: "balanced(\"([]{})\")",
                expects: "true",
            },
            Case {
                call: "balanced(\"([)]\")",
                expects: "false",
            },
            Case {
                call: "balanced(\"\")",
                expects: "true",
            },
        ],
    },
    Task {
        name: "run-length",
        function: "encode",
        asks: "Write a Rust function `fn encode(s: &str) -> Vec<(char, usize)>` that returns the \
               run-length encoding of the string s as a vector of (character, count) pairs, in \
               order. Reply with only the function.",
        cases: &[
            Case {
                call: "encode(\"aaabbc\")",
                expects: "[('a', 3), ('b', 2), ('c', 1)]",
            },
            Case {
                call: "encode(\"\")",
                expects: "[]",
            },
            Case {
                call: "encode(\"ab\")",
                expects: "[('a', 1), ('b', 1)]",
            },
        ],
    },
    Task {
        name: "edit-distance",
        function: "distance",
        asks: "Write a Rust function `fn distance(a: &str, b: &str) -> usize` returning the \
               Levenshtein edit distance between two strings — the fewest single-character \
               insertions, deletions or substitutions that turn a into b. Reply with only the \
               function.",
        cases: &[
            Case {
                call: "distance(\"kitten\",\"sitting\")",
                expects: "3",
            },
            Case {
                call: "distance(\"\",\"abc\")",
                expects: "3",
            },
            Case {
                call: "distance(\"same\",\"same\")",
                expects: "0",
            },
        ],
    },
    Task {
        name: "spiral-order",
        function: "spiral",
        asks: "Write a Rust function `fn spiral(m: &[Vec<i64>]) -> Vec<i64>` that takes a \
               rectangular matrix of integers and returns a flat vector of its elements in \
               clockwise spiral order, starting at the top-left and going right first. Reply \
               with only the function.",
        cases: &[
            Case {
                call: "spiral(&[vec![1,2,3],vec![4,5,6],vec![7,8,9]])",
                expects: "[1, 2, 3, 6, 9, 8, 7, 4, 5]",
            },
            Case {
                call: "spiral(&[vec![1,2],vec![3,4],vec![5,6]])",
                expects: "[1, 2, 4, 6, 5, 3]",
            },
            Case {
                call: "spiral(&[vec![1]])",
                expects: "[1]",
            },
        ],
    },
    Task {
        name: "expression-value",
        function: "value",
        asks: "Write a Rust function `fn value(s: &str) -> i64` that evaluates an arithmetic \
               expression given as a string and returns an integer. The expression contains \
               non-negative integers, the operators + - * /, and parentheses. Multiplication \
               and division bind tighter than addition and subtraction; division truncates \
               toward zero. Reply with only the function and any helper functions it needs.",
        cases: &[
            Case {
                call: "value(\"2+3*4\")",
                expects: "14",
            },
            Case {
                call: "value(\"(2+3)*4\")",
                expects: "20",
            },
            Case {
                call: "value(\"7/2\")",
                expects: "3",
            },
        ],
    },
    Task {
        name: "shortest-path",
        function: "shortest",
        asks: "Write a Rust function `fn shortest(edges: &[(usize, usize, u64)], start: usize, \
               goal: usize) -> i64` where edges are (u, v, w) triples describing a directed \
               graph with non-negative integer weights. Return the total weight of the cheapest \
               path from start to goal, or -1 if there is none. Reply with only the function.",
        cases: &[
            Case {
                call: "shortest(&[(1,2,4),(1,3,1),(3,2,1)],1,2)",
                expects: "2",
            },
            Case {
                call: "shortest(&[(1,2,1)],2,1)",
                expects: "-1",
            },
            Case {
                call: "shortest(&[],1,1)",
                expects: "0",
            },
        ],
    },
    Task {
        name: "glob-match",
        function: "matches",
        asks: "Write a Rust function `fn matches(pattern: &str, text: &str) -> bool` returning \
               true if the whole of text matches the pattern. In the pattern, `?` matches \
               exactly one character and `*` matches any run of characters including none. \
               Every other character matches itself. Use only the standard library. Reply with \
               only the function.",
        cases: &[
            Case {
                call: "matches(\"*a*b\",\"xaxbx\")",
                expects: "false",
            },
            Case {
                call: "matches(\"a?c\",\"abc\")",
                expects: "true",
            },
            Case {
                call: "matches(\"*\",\"\")",
                expects: "true",
            },
        ],
    },
    Task {
        name: "topological-order",
        function: "order",
        asks: "Write a Rust function `fn order(n: usize, edges: &[(usize, usize)]) -> Vec<usize>` \
               where the vertices are 1 to n and edges are (u, v) pairs meaning u must come \
               before v. Return the lexicographically smallest ordering that satisfies every \
               edge, or an empty vector if none exists. Reply with only the function.",
        cases: &[
            Case {
                call: "order(4,&[(1,2),(1,3),(3,4)])",
                expects: "[1, 2, 3, 4]",
            },
            Case {
                call: "order(2,&[(1,2),(2,1)])",
                expects: "[]",
            },
            Case {
                call: "order(3,&[])",
                expects: "[1, 2, 3]",
            },
        ],
    },
    Task {
        name: "days-between",
        function: "between",
        asks: "Write a Rust function `fn between(a: &str, b: &str) -> u64` taking two dates as \
               'YYYY-MM-DD' strings and returning the number of whole days between them. Use \
               only the standard library. Reply with only the function.",
        cases: &[
            Case {
                call: "between(\"2024-02-28\",\"2024-03-01\")",
                expects: "2",
            },
            Case {
                call: "between(\"1900-02-28\",\"1900-03-01\")",
                expects: "1",
            },
            Case {
                call: "between(\"2020-01-01\",\"2020-01-01\")",
                expects: "0",
            },
        ],
    },
    Task {
        name: "n-queens",
        function: "count",
        asks: "Write a Rust function `fn count(n: usize) -> u64` returning how many ways n \
               queens can be placed on an n by n board so that no two attack each other. Reply \
               with only the function and any helper functions it needs.",
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
        name: "intervals-touching",
        function: "merge",
        asks: "Write a Rust function `fn merge(xs: &[[i64; 2]]) -> Vec<[i64; 2]>` taking a slice \
               of [start, end] intervals and returning the merged intervals, sorted. Intervals \
               are half-open: [1,2] and [2,3] do not overlap and are not merged. Reply with only \
               the function.",
        cases: &[
            Case {
                call: "merge(&[[1,2],[2,3]])",
                expects: "[[1, 2], [2, 3]]",
            },
            Case {
                call: "merge(&[[1,5],[2,3],[6,8]])",
                expects: "[[1, 5], [6, 8]]",
            },
            Case {
                call: "merge(&[])",
                expects: "[]",
            },
        ],
    },
    Task {
        name: "brackets-in-quotes",
        function: "balanced",
        asks: "Write a Rust function `fn balanced(text: &str) -> bool` returning true if the \
               brackets in text are balanced. (), [] and {} are brackets. A single quote starts \
               a quoted stretch and the next single quote ends it; brackets inside a quoted \
               stretch are ordinary characters and do not count. Text ending inside a quoted \
               stretch is not balanced. Reply with only the function.",
        cases: &[
            Case {
                call: "balanced(\"(a['b'])\")",
                expects: "true",
            },
            Case {
                call: "balanced(\"('(')\")",
                expects: "true",
            },
            Case {
                call: "balanced(\"(']')\")",
                expects: "true",
            },
        ],
    },
    Task {
        name: "duration-seconds",
        function: "seconds",
        asks: "Write a Rust function `fn seconds(text: &str) -> u64` turning a duration into a \
               whole number of seconds. The text is a run of number-unit pairs where the unit is \
               h, m or s — for example '1h30m'. The pairs may come in any order and a unit may \
               appear more than once, in which case they are all added up. Empty text is 0. \
               Reply with only the function.",
        cases: &[
            Case {
                call: "seconds(\"1h30m\")",
                expects: "5400",
            },
            Case {
                call: "seconds(\"30m1h\")",
                expects: "5400",
            },
            Case {
                call: "seconds(\"1h1h\")",
                expects: "7200",
            },
        ],
    },
    Task {
        name: "rle-decode-counts",
        function: "decode",
        asks: "Write a Rust function `fn decode(text: &str) -> String` expanding a run-length \
               encoding. The text is a run of items; each item is one character followed by an \
               optional decimal count, which may have more than one digit. A missing count means \
               one. A count of zero means the character does not appear. Reply with only the \
               function.",
        cases: &[
            Case {
                call: "decode(\"a12b\")",
                expects: "\"aaaaaaaaaaaab\"",
            },
            Case {
                call: "decode(\"ab\")",
                expects: "\"ab\"",
            },
            Case {
                call: "decode(\"a0b2\")",
                expects: "\"bb\"",
            },
        ],
    },
    Task {
        name: "lru-cache",
        function: "run_cache",
        asks: "Write a Rust function `fn run_cache(capacity: usize, ops: &[(&str, i64, i64)]) -> \
               Vec<i64>` simulating a least-recently-used cache. Each op is (kind, key, value) \
               where kind is \"put\" or \"get\"; the value of a get is ignored. Return the \
               results of the get operations in order, using -1 when a key is absent. Both put \
               and get count as a use. Reply with only the function.",
        cases: &[
            Case {
                call: "run_cache(2,&[(\"put\",1,1),(\"put\",2,2),(\"get\",1,0),(\"put\",3,3),(\"get\",2,0)])",
                expects: "[1, -1]",
            },
            Case {
                call: "run_cache(1,&[(\"put\",1,1),(\"put\",2,2),(\"get\",1,0),(\"get\",2,0)])",
                expects: "[-1, 2]",
            },
            Case {
                call: "run_cache(2,&[(\"get\",9,0)])",
                expects: "[-1]",
            },
        ],
    },
    Task {
        name: "parse-duration",
        function: "seconds",
        asks: "Write a Rust function `fn seconds(text: &str) -> Option<u64>` that parses a \
               duration written as hours and minutes, like \"1h30m\", \"2h\" or \"45m\", and \
               returns the total number of seconds. Return None for anything that is not in that \
               form, such as an empty string, \"90\" or \"1h30\". Reply with only the function.",
        cases: &[
            Case {
                call: "seconds(\"1h30m\")",
                expects: "Some(5400)",
            },
            Case {
                call: "seconds(\"45m\")",
                expects: "Some(2700)",
            },
            Case {
                call: "seconds(\"2h\")",
                expects: "Some(7200)",
            },
            Case {
                call: "seconds(\"1h30\")",
                expects: "None",
            },
            Case {
                call: "seconds(\"\")",
                expects: "None",
            },
        ],
    },
    Task {
        name: "fix-the-median",
        function: "doubled_median",
        asks: "This Rust function is meant to return twice the median of a non-empty slice of \
               integers, so that the result is a whole number either way: twice the middle \
               value when the count is odd and the sum of the two middle values when it is \
               even; it is wrong. Fix it and reply with only the corrected function.\n\nfn \
               doubled_median(xs: &[i64]) -> i64 {\n    let mut xs = xs.to_vec();\n    \
               xs.sort();\n    let n = xs.len();\n    2 * xs[n / 2]\n}",
        cases: &[
            Case {
                call: "doubled_median(&[3,1,2])",
                expects: "4",
            },
            Case {
                call: "doubled_median(&[4,1,3,2])",
                expects: "5",
            },
            Case {
                call: "doubled_median(&[7])",
                expects: "14",
            },
            Case {
                call: "doubled_median(&[1,2,3,4,5,6])",
                expects: "7",
            },
        ],
    },
    Task {
        name: "word-frequency",
        function: "top_words",
        asks: "Write a Rust function `fn top_words(text: &str, k: usize) -> Vec<(String, usize)>` \
               that splits the text on whitespace, lowercases the words, strips the characters \
               .,;:!? from each end of each word, and returns the k most frequent words as \
               (word, count) pairs, most frequent first; words with the same count are ordered \
               alphabetically. Reply with only the function.",
        cases: &[
            Case {
                call: "top_words(\"the cat and the hat. The end!\", 2)",
                expects: "[(\"the\", 3), (\"and\", 1)]",
            },
            Case {
                call: "top_words(\"b a b a c\", 3)",
                expects: "[(\"a\", 2), (\"b\", 2), (\"c\", 1)]",
            },
            Case {
                call: "top_words(\"\", 2)",
                expects: "[]",
            },
        ],
    },
    Task {
        name: "roman-to-integer",
        function: "roman",
        asks: "Write a Rust function `fn roman(s: &str) -> u32` that converts a Roman numeral \
               written with the letters I, V, X, L, C, D and M, including the subtractive forms \
               IV, IX, XL, XC, CD and CM, to a number. Reply with only the function.",
        cases: &[
            Case {
                call: "roman(\"XIV\")",
                expects: "14",
            },
            Case {
                call: "roman(\"MCMXCIV\")",
                expects: "1994",
            },
            Case {
                call: "roman(\"III\")",
                expects: "3",
            },
            Case {
                call: "roman(\"XLII\")",
                expects: "42",
            },
        ],
    },
];

#[cfg(test)]
mod tests {
    use super::{JAVASCRIPT, LANGUAGES, RUST, checker};

    #[test]
    fn every_language_has_every_task_in_the_python_order() {
        let python: Vec<&str> = crate::eval::TASKS.iter().map(|task| task.name).collect();
        for language in LANGUAGES {
            let names: Vec<&str> = language.tasks.iter().map(|task| task.name).collect();
            assert_eq!(names, python, "{}", language.name);
            for (theirs, ours) in language.tasks.iter().zip(crate::eval::TASKS) {
                assert_eq!(
                    theirs.cases.len(),
                    ours.cases.len(),
                    "{} {}",
                    language.name,
                    theirs.name
                );
                assert!(
                    theirs.asks.contains(&format!("{}(", theirs.function)),
                    "{} {}",
                    language.name,
                    theirs.name
                );
            }
        }
    }

    #[test]
    fn the_javascript_checker_prints_ok_or_no_a_case() {
        let said = checker(
            &LANGUAGES[0],
            &JAVASCRIPT[0],
            "function merge(a, b) { return [] }",
        );
        assert!(
            said.contains("[() => merge([1,3,5],[2,4,6]), '[1,2,3,4,5,6]'],"),
            "{said}"
        );
        assert!(said.contains("console.log(said)"));
    }

    #[test]
    fn the_rust_checker_catches_a_panic_and_compares_debug_text() {
        let said = checker(
            &LANGUAGES[1],
            &RUST[0],
            "fn merge(a: &[i64], b: &[i64]) -> Vec<i64> { vec![] }",
        );
        assert!(
            said.contains("catch_unwind(|| format!(\"{:?}\", merge(&[1,3,5],&[2,4,6])))"),
            "{said}"
        );
        assert!(said.contains("Some(r#\"[1, 2, 3, 4, 5, 6]\"#)"), "{said}");
        assert!(said.contains("set_hook"));
    }

    #[test]
    fn a_javascript_answer_loses_its_exports_and_a_rust_answer_its_main() {
        use super::plain;
        assert_eq!(
            plain(
                &LANGUAGES[0],
                "export function f(a) {\n  return a;\n}\nmodule.exports = { f };\n"
            ),
            "function f(a) {\n  return a;\n}"
        );
        assert_eq!(
            plain(&LANGUAGES[0], "export default function f(a) { return a; }"),
            "function f(a) { return a; }"
        );
        assert_eq!(
            plain(
                &LANGUAGES[1],
                "fn f(a: i64) -> i64 {\n    a\n}\n\nfn main() {\n    let x = { f(1) };\n    println!(\"{}\", x);\n}\n"
            ),
            "fn f(a: i64) -> i64 {\n    a\n}\n\n"
        );
        assert_eq!(
            plain(
                &LANGUAGES[1],
                "fn main() {\n    println!(\"{}\", f(1));\n}\n\nfn f(a: i64) -> i64 { a }\n"
            ),
            "fn f(a: i64) -> i64 { a }\n"
        );
        assert_eq!(plain(&LANGUAGES[1], "fn f() {}"), "fn f() {}");
    }

    #[test]
    fn a_rust_string_with_quotes_is_a_raw_literal() {
        let said = checker(
            &LANGUAGES[1],
            &RUST[14],
            "fn decode(text: &str) -> String { String::new() }",
        );
        assert!(said.contains("Some(r#\"\"aaaaaaaaaaaab\"\"#)"), "{said}");
    }

    /// A correct answer to every task in every language, so the cases are
    /// known to be satisfiable and the checkers known to read them; run
    /// with `--ignored` where podman and the images are present.
    const REFERENCE: &[(&str, &str, &str)] = &[
        (
            "javascript",
            "merge-sorted",
            "function merge(a, b) { const out = []; let i = 0, j = 0; while (i < a.length && j < b.length) { if (a[i] <= b[j]) out.push(a[i++]); else out.push(b[j++]); } while (i < a.length) out.push(a[i++]); while (j < b.length) out.push(b[j++]); return out; }",
        ),
        (
            "javascript",
            "balanced-brackets",
            "function balanced(s) { const pairs = {')': '(', ']': '[', '}': '{'}; const st = []; for (const c of s) { if ('([{'.includes(c)) st.push(c); else if (c in pairs) { if (st.pop() !== pairs[c]) return false; } } return st.length === 0; }",
        ),
        (
            "javascript",
            "run-length",
            "function encode(s) { const out = []; for (const c of s) { if (out.length && out[out.length - 1][0] === c) out[out.length - 1][1]++; else out.push([c, 1]); } return out; }",
        ),
        (
            "javascript",
            "edit-distance",
            "function distance(a, b) { let prev = Array.from({length: b.length + 1}, (_, j) => j); for (let i = 1; i <= a.length; i++) { const cur = [i]; for (let j = 1; j <= b.length; j++) cur[j] = Math.min(prev[j] + 1, cur[j - 1] + 1, prev[j - 1] + (a[i - 1] === b[j - 1] ? 0 : 1)); prev = cur; } return prev[b.length]; }",
        ),
        (
            "javascript",
            "spiral-order",
            "function spiral(m) { const out = []; let top = 0, bottom = m.length - 1, left = 0, right = m[0].length - 1; while (top <= bottom && left <= right) { for (let j = left; j <= right; j++) out.push(m[top][j]); top++; for (let i = top; i <= bottom; i++) out.push(m[i][right]); right--; if (top <= bottom) { for (let j = right; j >= left; j--) out.push(m[bottom][j]); bottom--; } if (left <= right) { for (let i = bottom; i >= top; i--) out.push(m[i][left]); left++; } } return out; }",
        ),
        (
            "javascript",
            "expression-value",
            "function value(s) { let i = 0; function num() { if (s[i] === '(') { i++; const v = expr(); i++; return v; } let n = 0; while (i < s.length && s[i] >= '0' && s[i] <= '9') n = n * 10 + (s.charCodeAt(i++) - 48); return n; } function term() { let v = num(); while (s[i] === '*' || s[i] === '/') { const op = s[i++]; const r = num(); v = op === '*' ? v * r : Math.trunc(v / r); } return v; } function expr() { let v = term(); while (s[i] === '+' || s[i] === '-') { const op = s[i++]; const r = term(); v = op === '+' ? v + r : v - r; } return v; } return expr(); }",
        ),
        (
            "javascript",
            "shortest-path",
            "function shortest(edges, start, goal) { const dist = new Map([[start, 0]]); const done = new Set(); for (;;) { let u = null; for (const [k, d] of dist) if (!done.has(k) && (u === null || d < dist.get(u))) u = k; if (u === null) break; if (u === goal) return dist.get(u); done.add(u); for (const [a, b, w] of edges) if (a === u && (!dist.has(b) || dist.get(u) + w < dist.get(b))) dist.set(b, dist.get(u) + w); } return -1; }",
        ),
        (
            "javascript",
            "glob-match",
            "function matches(pattern, text) { const m = pattern.length, n = text.length; let dp = Array.from({length: m + 1}, () => Array(n + 1).fill(false)); dp[0][0] = true; for (let i = 1; i <= m; i++) if (pattern[i - 1] === '*') dp[i][0] = dp[i - 1][0]; for (let i = 1; i <= m; i++) for (let j = 1; j <= n; j++) { const p = pattern[i - 1]; if (p === '*') dp[i][j] = dp[i - 1][j] || dp[i][j - 1]; else dp[i][j] = dp[i - 1][j - 1] && (p === '?' || p === text[j - 1]); } return dp[m][n]; }",
        ),
        (
            "javascript",
            "topological-order",
            "function order(n, edges) { const indeg = Array(n + 1).fill(0); const adj = Array.from({length: n + 1}, () => []); for (const [u, v] of edges) { adj[u].push(v); indeg[v]++; } const out = []; const ready = []; for (let v = 1; v <= n; v++) if (indeg[v] === 0) ready.push(v); while (ready.length) { ready.sort((a, b) => a - b); const u = ready.shift(); out.push(u); for (const v of adj[u]) if (--indeg[v] === 0) ready.push(v); } return out.length === n ? out : []; }",
        ),
        (
            "javascript",
            "days-between",
            "function between(a, b) { function days(s) { const [y, m, d] = s.split('-').map(Number); const leap = yy => yy % 4 === 0 && (yy % 100 !== 0 || yy % 400 === 0); const ml = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]; let t = 0; for (let yy = 1; yy < y; yy++) t += leap(yy) ? 366 : 365; for (let mm = 1; mm < m; mm++) t += ml[mm - 1] + (mm === 2 && leap(y) ? 1 : 0); return t + d; } return Math.abs(days(a) - days(b)); }",
        ),
        (
            "javascript",
            "n-queens",
            "function count(n) { let total = 0; function place(row, cols, d1, d2) { if (row === n) { total++; return; } for (let c = 0; c < n; c++) { if (cols.has(c) || d1.has(row - c) || d2.has(row + c)) continue; cols.add(c); d1.add(row - c); d2.add(row + c); place(row + 1, cols, d1, d2); cols.delete(c); d1.delete(row - c); d2.delete(row + c); } } place(0, new Set(), new Set(), new Set()); return total; }",
        ),
        (
            "javascript",
            "intervals-touching",
            "function merge(xs) { const s = [...xs].sort((a, b) => a[0] - b[0]); const out = []; for (const [a, b] of s) { if (out.length && a < out[out.length - 1][1]) out[out.length - 1][1] = Math.max(out[out.length - 1][1], b); else out.push([a, b]); } return out; }",
        ),
        (
            "javascript",
            "brackets-in-quotes",
            "function balanced(text) { const pairs = {')': '(', ']': '[', '}': '{'}; const st = []; let quoted = false; for (const c of text) { if (c === \"'\") { quoted = !quoted; continue; } if (quoted) continue; if ('([{'.includes(c)) st.push(c); else if (c in pairs) { if (st.pop() !== pairs[c]) return false; } } return !quoted && st.length === 0; }",
        ),
        (
            "javascript",
            "duration-seconds",
            "function seconds(text) { let total = 0, n = 0; for (const c of text) { if (c >= '0' && c <= '9') n = n * 10 + (c.charCodeAt(0) - 48); else { total += n * (c === 'h' ? 3600 : c === 'm' ? 60 : 1); n = 0; } } return total; }",
        ),
        (
            "javascript",
            "rle-decode-counts",
            "function decode(text) { let out = ''; let i = 0; while (i < text.length) { const c = text[i++]; let n = ''; while (i < text.length && text[i] >= '0' && text[i] <= '9') n += text[i++]; out += c.repeat(n === '' ? 1 : Number(n)); } return out; }",
        ),
        (
            "javascript",
            "lru-cache",
            "function run_cache(capacity, ops) { const m = new Map(); const out = []; for (const op of ops) { if (op[0] === 'put') { m.delete(op[1]); m.set(op[1], op[2]); if (m.size > capacity) m.delete(m.keys().next().value); } else { if (m.has(op[1])) { const v = m.get(op[1]); m.delete(op[1]); m.set(op[1], v); out.push(v); } else out.push(-1); } } return out; }",
        ),
        (
            "javascript",
            "parse-duration",
            "function seconds(text) { const m = /^(?:(\\d+)h)?(?:(\\d+)m)?$/.exec(text); if (!m || text === '') return null; return (m[1] ? Number(m[1]) * 3600 : 0) + (m[2] ? Number(m[2]) * 60 : 0); }",
        ),
        (
            "javascript",
            "fix-the-median",
            "function median(xs) { xs = [...xs].sort((a, b) => a - b); const n = xs.length; return n % 2 ? xs[(n - 1) / 2] : (xs[n / 2 - 1] + xs[n / 2]) / 2; }",
        ),
        (
            "javascript",
            "word-frequency",
            "function top_words(text, k) { const counts = new Map(); for (let w of text.split(/\\s+/)) { w = w.toLowerCase().replace(/^[.,;:!?]+|[.,;:!?]+$/g, ''); if (!w) continue; counts.set(w, (counts.get(w) || 0) + 1); } return [...counts].sort((a, b) => b[1] - a[1] || (a[0] < b[0] ? -1 : 1)).slice(0, k); }",
        ),
        (
            "javascript",
            "roman-to-integer",
            "function roman(s) { const v = {I: 1, V: 5, X: 10, L: 50, C: 100, D: 500, M: 1000}; let t = 0; for (let i = 0; i < s.length; i++) { const a = v[s[i]], b = v[s[i + 1]] || 0; t += a < b ? -a : a; } return t; }",
        ),
        (
            "rust",
            "merge-sorted",
            "fn merge(a: &[i64], b: &[i64]) -> Vec<i64> { let (mut i, mut j) = (0, 0); let mut out = Vec::new(); while i < a.len() && j < b.len() { if a[i] <= b[j] { out.push(a[i]); i += 1; } else { out.push(b[j]); j += 1; } } out.extend_from_slice(&a[i..]); out.extend_from_slice(&b[j..]); out }",
        ),
        (
            "rust",
            "balanced-brackets",
            "fn balanced(s: &str) -> bool { let mut st = Vec::new(); for c in s.chars() { match c { '(' | '[' | '{' => st.push(c), ')' => if st.pop() != Some('(') { return false; }, ']' => if st.pop() != Some('[') { return false; }, '}' => if st.pop() != Some('{') { return false; }, _ => {} } } st.is_empty() }",
        ),
        (
            "rust",
            "run-length",
            "fn encode(s: &str) -> Vec<(char, usize)> { let mut out: Vec<(char, usize)> = Vec::new(); for c in s.chars() { match out.last_mut() { Some(last) if last.0 == c => last.1 += 1, _ => out.push((c, 1)), } } out }",
        ),
        (
            "rust",
            "edit-distance",
            "fn distance(a: &str, b: &str) -> usize { let a: Vec<char> = a.chars().collect(); let b: Vec<char> = b.chars().collect(); let mut prev: Vec<usize> = (0..=b.len()).collect(); for i in 1..=a.len() { let mut cur = vec![i]; for j in 1..=b.len() { let sub = prev[j - 1] + usize::from(a[i - 1] != b[j - 1]); cur.push((prev[j] + 1).min(cur[j - 1] + 1).min(sub)); } prev = cur; } prev[b.len()] }",
        ),
        (
            "rust",
            "spiral-order",
            "fn spiral(m: &[Vec<i64>]) -> Vec<i64> { let mut out = Vec::new(); if m.is_empty() { return out; } let (mut top, mut bottom, mut left, mut right) = (0i64, m.len() as i64 - 1, 0i64, m[0].len() as i64 - 1); while top <= bottom && left <= right { for j in left..=right { out.push(m[top as usize][j as usize]); } top += 1; for i in top..=bottom { out.push(m[i as usize][right as usize]); } right -= 1; if top <= bottom { for j in (left..=right).rev() { out.push(m[bottom as usize][j as usize]); } bottom -= 1; } if left <= right { for i in (top..=bottom).rev() { out.push(m[i as usize][left as usize]); } left += 1; } } out }",
        ),
        (
            "rust",
            "expression-value",
            "fn value(s: &str) -> i64 { let b = s.as_bytes(); let mut i = 0; fn num(b: &[u8], i: &mut usize) -> i64 { if b[*i] == b'(' { *i += 1; let v = expr(b, i); *i += 1; return v; } let mut n = 0; while *i < b.len() && b[*i].is_ascii_digit() { n = n * 10 + i64::from(b[*i] - b'0'); *i += 1; } n } fn term(b: &[u8], i: &mut usize) -> i64 { let mut v = num(b, i); while *i < b.len() && (b[*i] == b'*' || b[*i] == b'/') { let op = b[*i]; *i += 1; let r = num(b, i); v = if op == b'*' { v * r } else { v / r }; } v } fn expr(b: &[u8], i: &mut usize) -> i64 { let mut v = term(b, i); while *i < b.len() && (b[*i] == b'+' || b[*i] == b'-') { let op = b[*i]; *i += 1; let r = term(b, i); v = if op == b'+' { v + r } else { v - r }; } v } expr(b, &mut i) }",
        ),
        (
            "rust",
            "shortest-path",
            "fn shortest(edges: &[(usize, usize, u64)], start: usize, goal: usize) -> i64 { use std::collections::{BTreeMap, BTreeSet}; let mut dist: BTreeMap<usize, u64> = BTreeMap::new(); dist.insert(start, 0); let mut done = BTreeSet::new(); loop { let Some((&u, &d)) = dist.iter().filter(|(k, _)| !done.contains(*k)).min_by_key(|(_, d)| **d) else { return -1; }; if u == goal { return d as i64; } done.insert(u); for &(a, b, w) in edges { if a == u { let e = dist.entry(b).or_insert(u64::MAX); if d + w < *e { *e = d + w; } } } } }",
        ),
        (
            "rust",
            "glob-match",
            "fn matches(pattern: &str, text: &str) -> bool { let p: Vec<char> = pattern.chars().collect(); let t: Vec<char> = text.chars().collect(); let mut dp = vec![vec![false; t.len() + 1]; p.len() + 1]; dp[0][0] = true; for i in 1..=p.len() { if p[i - 1] == '*' { dp[i][0] = dp[i - 1][0]; } } for i in 1..=p.len() { for j in 1..=t.len() { dp[i][j] = if p[i - 1] == '*' { dp[i - 1][j] || dp[i][j - 1] } else { dp[i - 1][j - 1] && (p[i - 1] == '?' || p[i - 1] == t[j - 1]) }; } } dp[p.len()][t.len()] }",
        ),
        (
            "rust",
            "topological-order",
            "fn order(n: usize, edges: &[(usize, usize)]) -> Vec<usize> { let mut indeg = vec![0usize; n + 1]; let mut adj = vec![Vec::new(); n + 1]; for &(u, v) in edges { adj[u].push(v); indeg[v] += 1; } let mut ready: std::collections::BTreeSet<usize> = (1..=n).filter(|&v| indeg[v] == 0).collect(); let mut out = Vec::new(); while let Some(u) = ready.iter().next().copied() { ready.remove(&u); out.push(u); for &v in &adj[u] { indeg[v] -= 1; if indeg[v] == 0 { ready.insert(v); } } } if out.len() == n { out } else { Vec::new() } }",
        ),
        (
            "rust",
            "days-between",
            "fn between(a: &str, b: &str) -> u64 { fn days(s: &str) -> u64 { let p: Vec<u64> = s.split('-').map(|x| x.parse().unwrap()).collect(); let leap = |y: u64| y % 4 == 0 && (y % 100 != 0 || y % 400 == 0); let ml = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]; let mut t = 0; for y in 1..p[0] { t += if leap(y) { 366 } else { 365 }; } for m in 1..p[1] { t += ml[m as usize - 1] + u64::from(m == 2 && leap(p[0])); } t + p[2] } days(a).abs_diff(days(b)) }",
        ),
        (
            "rust",
            "n-queens",
            "fn count(n: usize) -> u64 { fn place(row: usize, n: usize, cols: &mut Vec<bool>, d1: &mut Vec<bool>, d2: &mut Vec<bool>) -> u64 { if row == n { return 1; } let mut t = 0; for c in 0..n { let (a, b) = (row + n - c, row + c); if cols[c] || d1[a] || d2[b] { continue; } cols[c] = true; d1[a] = true; d2[b] = true; t += place(row + 1, n, cols, d1, d2); cols[c] = false; d1[a] = false; d2[b] = false; } t } place(0, n, &mut vec![false; n], &mut vec![false; 2 * n + 1], &mut vec![false; 2 * n + 1]) }",
        ),
        (
            "rust",
            "intervals-touching",
            "fn merge(xs: &[[i64; 2]]) -> Vec<[i64; 2]> { let mut s = xs.to_vec(); s.sort(); let mut out: Vec<[i64; 2]> = Vec::new(); for [a, b] in s { match out.last_mut() { Some(last) if a < last[1] => last[1] = last[1].max(b), _ => out.push([a, b]), } } out }",
        ),
        (
            "rust",
            "brackets-in-quotes",
            "fn balanced(text: &str) -> bool { let mut st = Vec::new(); let mut quoted = false; for c in text.chars() { if c == '\\'' { quoted = !quoted; continue; } if quoted { continue; } match c { '(' | '[' | '{' => st.push(c), ')' => if st.pop() != Some('(') { return false; }, ']' => if st.pop() != Some('[') { return false; }, '}' => if st.pop() != Some('{') { return false; }, _ => {} } } !quoted && st.is_empty() }",
        ),
        (
            "rust",
            "duration-seconds",
            "fn seconds(text: &str) -> u64 { let mut total = 0; let mut n = 0; for c in text.chars() { if let Some(d) = c.to_digit(10) { n = n * 10 + u64::from(d); } else { total += n * match c { 'h' => 3600, 'm' => 60, _ => 1 }; n = 0; } } total }",
        ),
        (
            "rust",
            "rle-decode-counts",
            "fn decode(text: &str) -> String { let cs: Vec<char> = text.chars().collect(); let mut out = String::new(); let mut i = 0; while i < cs.len() { let c = cs[i]; i += 1; let mut n = None; while i < cs.len() && cs[i].is_ascii_digit() { n = Some(n.unwrap_or(0) * 10 + cs[i].to_digit(10).unwrap() as usize); i += 1; } for _ in 0..n.unwrap_or(1) { out.push(c); } } out }",
        ),
        (
            "rust",
            "lru-cache",
            "fn run_cache(capacity: usize, ops: &[(&str, i64, i64)]) -> Vec<i64> { let mut order: Vec<(i64, i64)> = Vec::new(); let mut out = Vec::new(); for &(kind, key, value) in ops { let at = order.iter().position(|(k, _)| *k == key); if kind == \"put\" { if let Some(at) = at { order.remove(at); } order.push((key, value)); if order.len() > capacity { order.remove(0); } } else if let Some(at) = at { let held = order.remove(at); order.push(held); out.push(held.1); } else { out.push(-1); } } out }",
        ),
        (
            "rust",
            "parse-duration",
            "fn seconds(text: &str) -> Option<u64> { if text.is_empty() { return None; } let mut rest = text; let mut total = 0; let mut seen_h = false; for (unit, mult) in [('h', 3600u64), ('m', 60)] { if let Some(at) = rest.find(unit) { let n: u64 = rest[..at].parse().ok()?; if unit == 'm' && seen_h && at == 0 { return None; } total += n * mult; rest = &rest[at + 1..]; if unit == 'h' { seen_h = true; } } } if rest.is_empty() { Some(total) } else { None } }",
        ),
        (
            "rust",
            "fix-the-median",
            "fn doubled_median(xs: &[i64]) -> i64 { let mut xs = xs.to_vec(); xs.sort(); let n = xs.len(); if n % 2 == 1 { 2 * xs[n / 2] } else { xs[n / 2 - 1] + xs[n / 2] } }",
        ),
        (
            "rust",
            "word-frequency",
            "fn top_words(text: &str, k: usize) -> Vec<(String, usize)> { let mut counts: std::collections::BTreeMap<String, usize> = std::collections::BTreeMap::new(); for w in text.split_whitespace() { let w = w.to_lowercase(); let w = w.trim_matches(|c| \".,;:!?\".contains(c)); if w.is_empty() { continue; } *counts.entry(w.to_owned()).or_insert(0) += 1; } let mut v: Vec<(String, usize)> = counts.into_iter().collect(); v.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0))); v.truncate(k); v }",
        ),
        (
            "rust",
            "roman-to-integer",
            "fn roman(s: &str) -> u32 { let v = |c: char| match c { 'I' => 1, 'V' => 5, 'X' => 10, 'L' => 50, 'C' => 100, 'D' => 500, 'M' => 1000, _ => 0 }; let cs: Vec<u32> = s.chars().map(v).collect(); let mut t: i64 = 0; for i in 0..cs.len() { let b = cs.get(i + 1).copied().unwrap_or(0); if cs[i] < b { t -= i64::from(cs[i]); } else { t += i64::from(cs[i]); } } t as u32 }",
        ),
    ];

    /// Runs the reference answers in the containers; needs podman and the
    /// pulled images, so it is ignored unless asked for.
    #[test]
    #[ignore = "needs podman and the pinned images; run with --ignored"]
    fn every_reference_answer_holds_every_case() {
        use mcf_bench::eval::Ran;
        let podman = std::path::Path::new("/usr/bin/podman");
        let scratch = std::env::temp_dir().join(format!("mcf-reference-{}", std::process::id()));
        std::fs::create_dir_all(&scratch).unwrap();
        let mut wrong = Vec::new();
        for language in LANGUAGES {
            for task in language.tasks {
                let Some((_, _, answer)) = REFERENCE
                    .iter()
                    .find(|(lang, name, _)| *lang == language.name && *name == task.name)
                else {
                    wrong.push(format!("{} {}: no reference", language.name, task.name));
                    continue;
                };
                match super::run_in_container(podman, &scratch, language, task, answer) {
                    Ran::Checked { passed, of } if passed == of => {}
                    other => wrong.push(format!("{} {}: {other:?}", language.name, task.name)),
                }
            }
        }
        let _gone = std::fs::remove_dir_all(&scratch);
        assert!(wrong.is_empty(), "{}", wrong.join("\n"));
    }
}
