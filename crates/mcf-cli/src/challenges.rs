use std::path::Path;

use mcf_record::json::Value;
use mcf_serve::examine::Reading;

use crate::catalogue::{Challenge, Kind, Lit, Tier};
use crate::languages::Language;

pub(crate) const RETRIES_DEFAULT: usize = 10;

const BUDGET: usize = 1400;

const SMALLEST_WINDOW: u64 = 4096;

const COMPILER_LINES: usize = 12;
const COMPILER_WIDTH: usize = 200;

#[must_use]
pub(crate) fn compiler_said(heard: &str) -> String {
    heard
        .lines()
        .filter(|line| !line.trim().is_empty())
        .take(COMPILER_LINES)
        .map(|line| line.chars().take(COMPILER_WIDTH).collect::<String>())
        .collect::<Vec<_>>()
        .join("\n")
}

pub(crate) const LANGUAGE_NAMES: [&str; 4] = ["python", "javascript", "rust", "go"];

pub(crate) const GO: Language = Language {
    name: "go",
    image: "docker.io/library/golang",
    digest: "sha256:f86f1a6701e3dcc445fec097a42f78b758f15950ccf032c2d3e54e2754d32fdb",
    file: "answer.go",
    memory: "1g",
    command: &[
        "timeout",
        "120",
        "sh",
        "-c",
        "export GOCACHE=/tmp/gocache GOPATH=/tmp/gopath HOME=/tmp GOMAXPROCS=2 GOFLAGS=-p=1; cd \
         /tmp && if go build -o /tmp/answer /work/answer.go; then echo compiled; /tmp/answer; \
         else echo notcompiled; fi",
    ],
    present: &["go", "version"],
};

pub(crate) const PYTHON: Language = Language {
    name: "python",
    image: crate::eval::IMAGE,
    digest: crate::eval::IMAGE_DIGEST,
    file: "answer.py",
    memory: "512m",
    command: &["timeout", "20", "python3", "/work/answer.py"],
    present: &["python3", "--version"],
};

#[derive(Debug)]
pub(crate) struct Plan {
    pub languages: Vec<&'static Language>,
    pub retries: usize,
    pub tier: Option<Tier>,
    pub window: Option<u64>,
}

impl Plan {
    pub(crate) fn asked(
        languages: Option<&str>,
        retries: Option<usize>,
        tier: Option<&str>,
        window: Option<u64>,
    ) -> Result<Self, String> {
        let mut held = Vec::new();
        for name in languages.map_or_else(
            || LANGUAGE_NAMES.to_vec(),
            |named| named.split(',').map(str::trim).collect(),
        ) {
            match language_named(name) {
                Some(language) if !held.iter().any(|kept: &&Language| kept.name == name) => {
                    held.push(language);
                }
                Some(_) => {}
                None => {
                    return Err(format!(
                        "no language is called {name}; --languages takes any of {}",
                        LANGUAGE_NAMES.join(", ")
                    ));
                }
            }
        }
        let tier = match tier {
            None => None,
            Some(name) => Some(Tier::named(name).ok_or_else(|| {
                format!("no tier is called {name}; --tier takes easy, medium, hard or expert")
            })?),
        };
        let retries = retries.unwrap_or(RETRIES_DEFAULT);
        if retries == 0 {
            return Err("--retries takes at least one attempt".to_owned());
        }
        if window.is_some_and(|window| window < SMALLEST_WINDOW) {
            return Err(format!(
                "--window takes at least {SMALLEST_WINDOW} tokens: an answer may take {BUDGET} and \
                 the correction carries the answer back"
            ));
        }
        Ok(Self {
            languages: held,
            retries,
            tier,
            window,
        })
    }

    #[must_use]
    pub(crate) fn said(&self, model: &Path) -> Vec<String> {
        let resolved = crate::explain::choice_for(model);
        let mut lines = vec!["  the challenges run under".to_owned()];
        lines.push(match &resolved {
            Ok(choice) => format!(
                "    engine     {} on {}, the daemon's choice unless it holds another; each \
                 row's record names what answered",
                choice.engine, choice.device.name
            ),
            Err(why) => format!("    engine     not resolved here — {why}; the daemon's choice"),
        });
        lines.push(match self.window {
            Some(window) => format!("    window     {window} tokens, every attempt"),
            None => format!(
                "    window     sized to each turn: twice the prompt and the budget, at least \
                 {SMALLEST_WINDOW}, at most what the model and the machine hold{}",
                resolved
                    .as_ref()
                    .map_or_else(|_| String::new(), |choice| format!(" ({})", choice.context))
            ),
        });
        lines.push(format!("    budget     {BUDGET} tokens an answer"));
        lines.push(format!(
            "    retries    {} attempt(s) a challenge in a language",
            self.retries
        ));
        lines.push(format!(
            "    languages  {}",
            self.languages
                .iter()
                .map(|language| language.name)
                .collect::<Vec<_>>()
                .join(", ")
        ));
        lines.push(format!(
            "    tier       {}",
            self.tier.map_or("every tier", Tier::name)
        ));
        lines.push("    seed       0, temperature as the daemon has it".to_owned());
        lines
    }

    pub(crate) fn conditions(&self) -> Vec<(&'static str, Value)> {
        vec![
            (
                "languages",
                Value::text(
                    self.languages
                        .iter()
                        .map(|language| language.name)
                        .collect::<Vec<_>>()
                        .join(","),
                ),
            ),
            (
                "retries",
                Value::Integer(i64::try_from(self.retries).unwrap_or(i64::MAX)),
            ),
            (
                "tier",
                self.tier
                    .map_or(Value::Null, |tier| Value::text(tier.name())),
            ),
            (
                "budget",
                Value::Integer(i64::try_from(BUDGET).unwrap_or(i64::MAX)),
            ),
            (
                "window",
                self.window.map_or(Value::Null, |window| {
                    Value::Integer(i64::try_from(window).unwrap_or(i64::MAX))
                }),
            ),
            (
                "challenges",
                Value::Integer(
                    i64::try_from(
                        crate::catalogue::CHALLENGES
                            .iter()
                            .filter(|challenge| self.tier.is_none_or(|tier| challenge.tier == tier))
                            .count(),
                    )
                    .unwrap_or(i64::MAX),
                ),
            ),
        ]
    }
}

pub(crate) fn language_named(name: &str) -> Option<&'static Language> {
    match name {
        "python" => Some(&PYTHON),
        "go" => Some(&GO),
        other => crate::languages::LANGUAGES
            .iter()
            .find(|language| language.name == other),
    }
}

fn kind_said(language: &str, kind: Kind, returned: bool) -> &'static str {
    match (language, kind, returned) {
        ("rust", Kind::Int, _) => "i64",
        ("rust" | "go", Kind::Bool, _) => "bool",
        ("rust", Kind::Text, false) => "&str",
        ("rust", Kind::Text, true) => "String",
        ("rust", Kind::Ints, false) => "&[i64]",
        ("rust", Kind::Ints, true) => "Vec<i64>",
        ("rust", Kind::Texts, false) => "&[&str]",
        ("rust", Kind::Texts, true) => "Vec<String>",
        ("rust", Kind::OptInt, _) => "Option<i64>",
        ("go", Kind::Int, _) => "int",
        ("go", Kind::Ints, _) => "[]int",
        ("go", Kind::Texts, _) => "[]string",
        ("go", Kind::OptInt, _) => "(int, bool)",
        (_, Kind::Int, _) => "integer",
        (_, Kind::Bool, _) => "boolean",
        (_, Kind::Text, _) => "string",
        (_, Kind::Ints, _) => "list of integers",
        (_, Kind::Texts, _) => "list of strings",
        (_, Kind::OptInt, _) => "integer or null",
    }
}

#[must_use]
pub(crate) fn signature(language: &str, challenge: &Challenge) -> String {
    let params: Vec<String> = challenge
        .params
        .iter()
        .map(|(name, kind)| match language {
            "rust" => format!("{name}: {}", kind_said("rust", *kind, false)),
            "go" => format!("{name} {}", kind_said("go", *kind, false)),
            _ => (*name).to_owned(),
        })
        .collect();
    match language {
        "rust" => format!(
            "fn {}({}) -> {}",
            challenge.function,
            params.join(", "),
            kind_said("rust", challenge.returns, true)
        ),
        "go" => format!(
            "func {}({}) {}",
            challenge.function,
            params.join(", "),
            kind_said("go", challenge.returns, true)
        ),
        "javascript" => format!("function {}({})", challenge.function, params.join(", ")),
        _ => format!("def {}({})", challenge.function, params.join(", ")),
    }
}

#[must_use]
pub(crate) fn ask_for(language: &Language, challenge: &Challenge) -> String {
    let typed: Vec<String> = challenge
        .params
        .iter()
        .map(|(name, kind)| format!("{name} is a {}", kind_said("words", *kind, false)))
        .collect();
    let word = match language.name {
        "javascript" => "JavaScript",
        "rust" => "Rust",
        "go" => "Go",
        _ => "Python",
    };
    let extra = match language.name {
        "go" => {
            " Do not write a package line or any import; the standard packages fmt, strings, \
                 sort, strconv, math and unicode are already imported."
        }
        "rust" => " Use only the standard library.",
        _ => "",
    };
    format!(
        "{} Write it in {word} as `{}`, where {} and it returns {}.{extra} Reply with only the \
         code.",
        challenge.statement,
        signature(language.name, challenge),
        typed.join(", "),
        kind_said("words", challenge.returns, true)
    )
}

fn literal(language: &str, lit: &Lit) -> String {
    let text = |t: &str| {
        let escaped = t.replace('\\', "\\\\").replace('"', "\\\"");
        format!("\"{escaped}\"")
    };
    match (language, lit) {
        (_, Lit::Int(n)) => n.to_string(),
        ("python", Lit::Bool(b)) => if *b { "True" } else { "False" }.to_owned(),
        (_, Lit::Bool(b)) => b.to_string(),
        (_, Lit::Text(t)) => text(t),
        ("rust", Lit::Ints(items)) => format!(
            "&[{}]",
            items
                .iter()
                .map(i64::to_string)
                .collect::<Vec<_>>()
                .join(", ")
        ),
        ("go", Lit::Ints(items)) => format!(
            "[]int{{{}}}",
            items
                .iter()
                .map(i64::to_string)
                .collect::<Vec<_>>()
                .join(", ")
        ),
        (_, Lit::Ints(items)) => format!(
            "[{}]",
            items
                .iter()
                .map(i64::to_string)
                .collect::<Vec<_>>()
                .join(", ")
        ),
        ("rust", Lit::Texts(items)) => format!(
            "&[{}]",
            items.iter().map(|t| text(t)).collect::<Vec<_>>().join(", ")
        ),
        ("go", Lit::Texts(items)) => format!(
            "[]string{{{}}}",
            items.iter().map(|t| text(t)).collect::<Vec<_>>().join(", ")
        ),
        (_, Lit::Texts(items)) => format!(
            "[{}]",
            items.iter().map(|t| text(t)).collect::<Vec<_>>().join(", ")
        ),
        ("python" | "rust", Lit::None) => "None".to_owned(),
        (_, Lit::None) => "null".to_owned(),
    }
}

fn expected_said(language: &str, kind: Kind, lit: &Lit) -> String {
    match (language, kind, lit) {
        ("rust", Kind::OptInt, Lit::Int(n)) => format!("Some({n})"),
        ("rust", Kind::OptInt, Lit::None) => "None".to_owned(),
        ("rust", _, Lit::Text(t)) => format!("{t:?}"),
        ("rust", _, Lit::Texts(items)) => format!("{items:?}"),
        ("rust", _, Lit::Ints(items)) => format!("{items:?}"),
        ("rust", _, other) => literal("rust", other),
        ("go", Kind::OptInt, Lit::Int(n)) => format!("{n} true"),
        ("go", Kind::OptInt, Lit::None) => "0 false".to_owned(),
        (_, _, other) => json_of(other),
    }
}

fn json_of(lit: &Lit) -> String {
    match lit {
        Lit::Int(n) => n.to_string(),
        Lit::Bool(b) => b.to_string(),
        Lit::Text(t) => Value::text((*t).to_owned()).to_line(),
        Lit::Ints(items) => format!(
            "[{}]",
            items
                .iter()
                .map(i64::to_string)
                .collect::<Vec<_>>()
                .join(",")
        ),
        Lit::Texts(items) => format!(
            "[{}]",
            items
                .iter()
                .map(|t| Value::text((*t).to_owned()).to_line())
                .collect::<Vec<_>>()
                .join(",")
        ),
        Lit::None => "null".to_owned(),
    }
}

#[must_use]
pub(crate) fn checker(language: &Language, challenge: &Challenge, written: &str) -> String {
    use std::fmt::Write as _;
    let call = |case: &crate::catalogue::Case| {
        format!(
            "{}({})",
            challenge.function,
            case.args
                .iter()
                .zip(challenge.params)
                .map(|(arg, _)| literal(language.name, arg))
                .collect::<Vec<_>>()
                .join(", ")
        )
    };
    let mut out = String::new();
    match language.name {
        "python" => {
            out.push_str(written);
            out.push_str("\n\nimport json, sys\n");
            for case in challenge.cases {
                let _wrote = writeln!(
                    out,
                    "try:\n    got = {}\n    print('ok' if json.dumps(got, separators=(',', \
                     ':')) == {} else 'no ' + json.dumps(got, separators=(',', ':'), \
                     default=str)[:200])\nexcept Exception as failed:\n    print('no ' + \
                     type(failed).__name__)",
                    call(case),
                    python_string(&expected_said("python", challenge.returns, &case.expects))
                );
            }
        }
        "javascript" => {
            out.push_str(&crate::languages::plain(language, written));
            out.push_str("\n\nconst __cases = [\n");
            for case in challenge.cases {
                let _wrote = writeln!(
                    out,
                    "  [() => {}, {}],",
                    call(case),
                    Value::text(expected_said(
                        "javascript",
                        challenge.returns,
                        &case.expects
                    ))
                    .to_line()
                );
            }
            out.push_str(
                "];\nfor (const [call, want] of __cases) {\n  try { const got = \
                 JSON.stringify(call()); console.log(got === want ? 'ok' : 'no ' + \
                 String(got).slice(0, 200)); } catch (e) { console.log('no ' + e.name); }\n}\n",
            );
        }
        "rust" => {
            out.push_str(&crate::languages::plain(language, written));
            out.push_str("\n\nfn main() {\n    std::panic::set_hook(Box::new(|_| {}));\n");
            for case in challenge.cases {
                let _wrote = writeln!(
                    out,
                    "    {{\n        let said = std::panic::catch_unwind(|| format!(\"{{:?}}\", \
                     {}));\n        match said {{\n            Ok(got) if got == r#\"{}\"# => \
                     println!(\"ok\"),\n            Ok(got) => println!(\"no {{}}\", \
                     got.chars().take(200).collect::<String>()),\n            Err(_) => \
                     println!(\"no panic\"),\n        }}\n    }}",
                    call(case),
                    expected_said("rust", challenge.returns, &case.expects)
                );
            }
            out.push_str("}\n");
        }
        _ => {
            out.push_str(
                "package main\n\nimport (\n\t\"encoding/json\"\n\t\"fmt\"\n\t\"math\"\n\t\"sort\"\n\t\"strconv\"\n\t\"strings\"\n\t\"unicode\"\n)\n\nvar _ = math.Abs\nvar _ = sort.Ints\nvar _ = strconv.Itoa\nvar _ = strings.ToLower\nvar _ = unicode.IsLetter\nvar _ = json.Marshal\n\n",
            );
            out.push_str(&go_plain(written));
            out.push_str("\n\nfunc __say(got string, want string) {\n\tif got == want {\n\t\tfmt.Println(\"ok\")\n\t} else {\n\t\tif len(got) > 200 {\n\t\t\tgot = got[:200]\n\t\t}\n\t\tfmt.Println(\"no \" + got)\n\t}\n}\n\nfunc main() {\n");
            for case in challenge.cases {
                let want = expected_said("go", challenge.returns, &case.expects);
                if challenge.returns == Kind::OptInt {
                    let _wrote = writeln!(
                        out,
                        "\t{{\n\t\tv, ok := {}\n\t\t__say(fmt.Sprintf(\"%d %t\", v, ok), {})\n\t}}",
                        call(case),
                        go_string(&want)
                    );
                } else {
                    let _wrote = writeln!(
                        out,
                        "\t{{\n\t\tb, _ := json.Marshal({})\n\t\t__say(string(b), {})\n\t}}",
                        call(case),
                        go_string(&want)
                    );
                }
            }
            out.push_str("}\n");
        }
    }
    out
}

#[must_use]
pub(crate) fn go_plain(written: &str) -> String {
    let mut out = Vec::new();
    let mut in_import = false;
    for line in written.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("package ") {
            continue;
        }
        if trimmed.starts_with("import (") {
            in_import = true;
            continue;
        }
        if in_import {
            if trimmed.starts_with(')') {
                in_import = false;
            }
            continue;
        }
        if trimmed.starts_with("import ") {
            continue;
        }
        out.push(line);
    }
    let joined = out.join("\n");
    crate::languages::without_main_named(&joined, "func main(")
}

fn python_string(text: &str) -> String {
    format!(
        "'''{}'''",
        text.replace('\\', "\\\\").replace("'''", "\\'\\'\\'")
    )
}

/// A Go raw string literal holding the text.
fn go_string(text: &str) -> String {
    format!("`{}`", text.replace('`', "` + \"`\" + `"))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Attempt {
    pub wrote: bool,
    pub compiled: Option<bool>,
    pub ran: bool,
    pub cases: Vec<(bool, String)>,
    pub tokens: usize,
    pub ask_ns: u64,
    pub window: Option<u64>,
    pub compiler: String,
    pub runtime: String,
    pub code: String,
}

impl Attempt {
    #[must_use]
    pub(crate) fn whole(&self) -> bool {
        self.ran && self.cases.iter().all(|(held, _)| *held)
    }

    #[must_use]
    pub(crate) fn held(&self) -> usize {
        self.cases.iter().filter(|(held, _)| *held).count()
    }
}

#[must_use]
pub(crate) fn read_harness(
    said: &str,
    cases: usize,
    compiles: bool,
) -> (Option<bool>, bool, Vec<(bool, String)>) {
    let built = compiles.then(|| !said.lines().any(|line| line == "notcompiled"));
    let mut read = Vec::new();
    for line in said.lines() {
        if line == "ok" {
            read.push((true, String::new()));
        } else if let Some(got) = line.strip_prefix("no ") {
            read.push((false, got.to_owned()));
        } else if line == "no" {
            read.push((false, String::new()));
        }
    }
    let ran = read.len() == cases && built.is_none_or(|held| held);
    (built, ran, read)
}

#[must_use]
pub(crate) fn feedback(language: &Language, challenge: &Challenge, last: &Attempt) -> String {
    use std::fmt::Write as _;
    let mut out = format!(
        "{}\n\nYou answered:\n```\n{}\n```\n",
        ask_for(language, challenge),
        last.code
    );
    if last.compiled == Some(false) {
        if last.compiler.is_empty() {
            out.push_str("That answer did not compile.\n");
        } else {
            let _wrote = writeln!(
                out,
                "That answer did not compile. The compiler said:\n```\n{}\n```",
                last.compiler
            );
        }
    } else if !last.ran {
        if last.runtime.is_empty() {
            out.push_str(
                "That answer did not run to the end of the checks, and said nothing on the way \
                 out: it may have run past the deadline.\n",
            );
        } else {
            let _wrote = writeln!(
                out,
                "That answer did not run to the end of the checks. It said:\n```\n{}\n```",
                last.runtime
            );
        }
    } else {
        out.push_str("That answer is wrong:\n");
        for ((held, got), case) in last.cases.iter().zip(challenge.cases) {
            if *held {
                continue;
            }
            let call = format!(
                "{}({})",
                challenge.function,
                case.args
                    .iter()
                    .zip(challenge.params)
                    .map(|(arg, _)| literal(language.name, arg))
                    .collect::<Vec<_>>()
                    .join(", ")
            );
            let _wrote = writeln!(
                out,
                "- {call} should return {}{}",
                expected_said(language.name, challenge.returns, &case.expects),
                if got.is_empty() {
                    String::new()
                } else {
                    format!(" but returned {got}")
                }
            );
        }
    }
    out.push_str("Reply with only the corrected code.");
    out
}

#[must_use]
#[allow(
    clippy::too_many_arguments,
    reason = "one run's conditions, each named in the rows"
)]
pub(crate) fn attempt_all(
    socket: &Path,
    named: &str,
    podman: &Path,
    scratch: &Path,
    language: &Language,
    challenge: &Challenge,
    retries: usize,
    window: Option<u64>,
    engine_ran: &mut Option<String>,
) -> Vec<Attempt> {
    let mut attempts: Vec<Attempt> = Vec::new();
    for _ in 0..retries.max(1) {
        let asked = match attempts.last() {
            Some(last) => feedback(language, challenge, last),
            None => ask_for(language, challenge),
        };
        let began = std::time::Instant::now();
        let spoken = mcf_serve::probes::spoken_as(
            socket,
            Path::new(named),
            &asked,
            None,
            BUDGET,
            None,
            mcf_serve::declared::Started {
                window,
                ..mcf_serve::declared::Started::default()
            },
        );
        let ask_ns = u64::try_from(began.elapsed().as_nanos()).unwrap_or(u64::MAX);
        if engine_ran.is_none() {
            engine_ran.clone_from(&spoken.engine_ran);
        }
        let tokens = match spoken.trial {
            mcf_serve::probes::Trial::Stopped { after, .. } => after,
            mcf_serve::probes::Trial::RanOut => BUDGET,
            mcf_serve::probes::Trial::CouldNotTell(_) => 0,
        };
        let code = crate::eval::code_in(&spoken.text);
        let builds = matches!(language.name, "rust" | "go");
        let mut complaint = String::new();
        let mut on_the_way_out = String::new();
        let (compiled, ran, cases) = if code.trim().is_empty() {
            (None, false, Vec::new())
        } else {
            let program = checker(language, challenge, &code);
            let (said, heard) =
                crate::languages::run_program_heard(podman, scratch, language, &program)
                    .unwrap_or_default();
            let read = read_harness(&said, challenge.cases.len(), builds);
            if read.0 == Some(false) {
                complaint = compiler_said(&heard);
            } else if !read.1 {
                on_the_way_out = compiler_said(&heard);
            }
            read
        };
        let attempt = Attempt {
            wrote: !code.trim().is_empty(),
            compiled,
            ran,
            cases,
            tokens,
            ask_ns,
            window: spoken.window_ran,
            compiler: complaint,
            runtime: on_the_way_out,
            code,
        };
        let solved = attempt.whole();
        crate::eval::result(&format!(
            "        attempt {} of {retries}: {} · {} of {} held · {} token(s) · {}",
            attempts.len().saturating_add(1),
            if !attempt.wrote {
                "wrote nothing"
            } else if attempt.compiled == Some(false) {
                "did not compile"
            } else if !attempt.ran {
                "did not run to the end"
            } else {
                "ran"
            },
            attempt.held(),
            challenge.cases.len(),
            attempt.tokens,
            took_said(attempt.ask_ns)
        ));
        attempts.push(attempt);
        if solved || crate::eval::stop_asked() {
            break;
        }
    }
    attempts
}

#[derive(Debug)]
pub(crate) struct Resumable {
    pub run: String,
    pub at: String,
    pub pairs: Vec<(String, String)>,
    pub rows: usize,
}

pub(crate) fn resumable(
    named: &str,
    method: &str,
    plan: &Plan,
) -> Result<Option<Resumable>, String> {
    let answered = crate::hosting::ask(&mcf_serve::control::Request::Readings {
        model: named.to_owned(),
        method: Some(method.to_owned()),
    })?;
    let runs = answered.get("runs").and_then(Value::as_list).unwrap_or(&[]);
    let wanted: std::collections::BTreeMap<&str, Value> = plan
        .conditions()
        .into_iter()
        .filter(|(key, _)| matches!(*key, "languages" | "retries" | "tier" | "window"))
        .collect();
    for run in runs {
        if !mcf_record::readings::in_parts(run)
            || mcf_record::readings::ended_of(run).as_deref() == Some("finished")
        {
            continue;
        }
        let same = wanted.iter().all(|(key, value)| {
            run.get("conditions")
                .and_then(|conditions| conditions.get(key))
                .is_some_and(|held| held == value)
        });
        if !same {
            continue;
        }
        let rows = mcf_record::readings::rows_of(run);
        let mut pairs: Vec<(String, String)> = rows
            .iter()
            .filter(|row| row.metric == "solved")
            .map(|row| (row.dim("challenge"), row.dim("language")))
            .collect();
        pairs.sort();
        pairs.dedup();
        return Ok(Some(Resumable {
            run: run
                .get("run")
                .and_then(Value::as_text)
                .unwrap_or_default()
                .to_owned(),
            at: run
                .get("at")
                .and_then(Value::as_text)
                .unwrap_or("an earlier time")
                .to_owned(),
            pairs,
            rows: rows.len(),
        }));
    }
    Ok(None)
}

#[must_use]
#[allow(clippy::too_many_lines, reason = "one row a line, each named")]
pub(crate) fn rows_of(challenge: &Challenge, language: &str, attempts: &[Attempt]) -> Vec<Reading> {
    let whole = |count: usize| i64::try_from(count).unwrap_or(i64::MAX);
    let base = [
        ("challenge", Value::text(challenge.name)),
        ("tier", Value::text(challenge.tier.name())),
        ("category", Value::text(challenge.category)),
        ("language", Value::text(language)),
    ];
    let mut rows = Vec::new();
    for (at, attempt) in attempts.iter().enumerate() {
        let mut dims = base.to_vec();
        dims.push(("attempt", Value::Integer(whole(at.saturating_add(1)))));
        rows.push(Reading::new(
            &dims,
            "wrote",
            i64::from(attempt.wrote),
            "bool",
        ));
        if let Some(compiled) = attempt.compiled {
            rows.push(Reading::new(&dims, "compiled", i64::from(compiled), "bool"));
        }
        rows.push(Reading::new(&dims, "ran", i64::from(attempt.ran), "bool"));
        rows.push(Reading::new(
            &dims,
            "cases_held",
            whole(attempt.held()),
            "count",
        ));
        rows.push(Reading::new(
            &dims,
            "cases",
            whole(challenge.cases.len()),
            "count",
        ));
        rows.push(Reading::new(
            &dims,
            "whole",
            i64::from(attempt.whole()),
            "bool",
        ));
        rows.push(Reading::new(
            &dims,
            "tokens",
            whole(attempt.tokens),
            "tokens",
        ));
        rows.push(Reading::new(
            &dims,
            "code_bytes",
            whole(attempt.code.len()),
            "bytes",
        ));
        rows.push(Reading::new(
            &dims,
            "ask_ns",
            i64::try_from(attempt.ask_ns).unwrap_or(i64::MAX),
            "ns",
        ));
        if let Some(window) = attempt.window {
            rows.push(Reading::new(
                &dims,
                "window",
                i64::try_from(window).unwrap_or(i64::MAX),
                "tokens",
            ));
        }
        if attempt.compiled == Some(false) {
            rows.push(Reading::new(
                &dims,
                "compiler_lines",
                whole(attempt.compiler.lines().count()),
                "count",
            ));
        } else if attempt.wrote && !attempt.ran {
            rows.push(Reading::new(
                &dims,
                "runtime_lines",
                whole(attempt.runtime.lines().count()),
                "count",
            ));
        }
    }
    let solved_at = attempts
        .iter()
        .position(Attempt::whole)
        .map(|at| at.saturating_add(1));
    rows.push(Reading::new(
        &base,
        "solved",
        i64::from(solved_at.is_some()),
        "bool",
    ));
    rows.push(Reading::new(
        &base,
        "solved_at_attempt",
        whole(solved_at.unwrap_or(0)),
        "count",
    ));
    rows.push(Reading::new(
        &base,
        "attempts",
        whole(attempts.len()),
        "count",
    ));
    rows.push(Reading::new(
        &base,
        "corrections",
        whole(solved_at.map_or(attempts.len().saturating_sub(1), |at| at.saturating_sub(1))),
        "count",
    ));
    rows.push(Reading::new(
        &base,
        "tokens_total",
        whole(
            attempts
                .iter()
                .fold(0_usize, |so_far, a| so_far.saturating_add(a.tokens)),
        ),
        "tokens",
    ));
    rows.push(Reading::new(
        &base,
        "ask_ns_total",
        i64::try_from(
            attempts
                .iter()
                .fold(0_u64, |so_far, a| so_far.saturating_add(a.ask_ns)),
        )
        .unwrap_or(i64::MAX),
        "ns",
    ));
    rows.push(Reading::new(
        &base,
        "first_attempt_cases_held",
        whole(attempts.first().map_or(0, Attempt::held)),
        "count",
    ));
    rows
}

#[expect(
    clippy::integer_division,
    reason = "a tenth of a second is the unit said"
)]
fn took_said(ns: u64) -> String {
    let tenths = ns / 100_000_000;
    format!("{}.{}s", tenths / 10, tenths % 10)
}

#[must_use]
pub(crate) fn said_of(language: &str, attempts: &[Attempt]) -> String {
    let tokens = attempts
        .iter()
        .fold(0_usize, |so_far, a| so_far.saturating_add(a.tokens));
    let ns = attempts
        .iter()
        .fold(0_u64, |so_far, a| so_far.saturating_add(a.ask_ns));
    let seconds = took_said(ns);
    if let Some(at) = attempts.iter().position(Attempt::whole) {
        format!(
            "{language:<10} solved at attempt {} · {} correction(s) · {tokens} token(s) · {seconds}",
            at.saturating_add(1),
            at
        )
    } else {
        {
            let best = attempts.iter().map(Attempt::held).max().unwrap_or(0);
            let cases = attempts.first().map_or(0, |a| a.cases.len());
            let why = match attempts.last() {
                Some(last) if last.compiled == Some(false) => "the last did not compile",
                Some(last) if !last.wrote => "the last wrote nothing",
                Some(last) if !last.ran => "the last did not run",
                _ => "the best held",
            };
            format!(
                "{language:<10} never in {} attempt(s) · {why}{} · {tokens} token(s) · {seconds}",
                attempts.len(),
                if why == "the best held" {
                    format!(" {best} of {cases}")
                } else {
                    String::new()
                }
            )
        }
    }
}

#[allow(
    clippy::too_many_arguments,
    reason = "one run's conditions, each named in the rows"
)]
pub(crate) fn run(
    socket: &Path,
    named: &str,
    podman: &Path,
    scratch: &Path,
    languages: &[&Language],
    retries: usize,
    only_tier: Option<Tier>,
    window: Option<u64>,
    landing: Option<&mut mcf_serve::examine::Landing>,
    done_already: &[(String, String)],
) -> (Vec<String>, Vec<Reading>, Option<String>, Option<String>) {
    let mut landing = landing;
    let mut stopped: Option<String> = None;
    let challenges: Vec<&Challenge> = crate::catalogue::CHALLENGES
        .iter()
        .filter(|challenge| only_tier.is_none_or(|tier| challenge.tier == tier))
        .collect();
    let mut lines = vec![
        format!(
            "  {} challenge(s) in {} language(s), up to {retries} attempt(s) each; a failed \
             attempt is handed back with the cases that did not hold and what came back",
            challenges.len(),
            languages.len()
        ),
        String::new(),
    ];
    let mut rows = Vec::new();
    let mut engine_ran = None;
    let mut present: Vec<&Language> = Vec::new();
    for language in languages {
        match crate::languages::present(podman, language) {
            Ok(_) => present.push(language),
            Err(why) => lines.push(format!("  {}: could not be run — {why}", language.name)),
        }
    }
    let of = challenges.len().saturating_mul(present.len().max(1));
    let mut done = 0_usize;
    for challenge in &challenges {
        crate::eval::result(&format!(
            "{} · {} · {}",
            challenge.tier.name(),
            challenge.category,
            challenge.name
        ));
        crate::eval::result(&format!(
            "    {}",
            challenge.statement.chars().take(160).collect::<String>()
        ));
        for language in &present {
            if done_already
                .iter()
                .any(|(held, in_language)| held == challenge.name && in_language == language.name)
            {
                crate::eval::result(&format!(
                    "    {:<10} already recorded by the run being resumed",
                    language.name
                ));
                done = done.saturating_add(1);
                continue;
            }
            crate::eval::progress(
                done,
                of,
                &format!(
                    "challenges · {} · {} · {}",
                    challenge.tier.name(),
                    language.name,
                    challenge.name
                ),
            );
            let attempts = attempt_all(
                socket,
                named,
                podman,
                scratch,
                language,
                challenge,
                retries,
                window,
                &mut engine_ran,
            );
            let pair = rows_of(challenge, language.name, &attempts);
            if let Some(landing) = landing.as_deref_mut()
                && let Err(why) = landing.land(engine_ran.as_deref(), &pair)
            {
                crate::eval::result(&format!("    ROWS NOT RECORDED: {why}"));
            }
            rows.extend(pair);
            crate::eval::result(&format!("    {}", said_of(language.name, &attempts)));
            done = done.saturating_add(1);
            if crate::eval::stop_asked() {
                stopped = Some(format!("stopped after {done} of {of}"));
                crate::eval::result(&format!(
                    "stopped at your asking after {done} of {of} pair(s); every row of them is \
                     recorded, and `--resume` goes on from here"
                ));
                break;
            }
        }
        if stopped.is_some() {
            break;
        }
    }
    lines.push(format!(
        "  {} result line(s) were written above as they came in",
        challenges
            .len()
            .saturating_mul(present.len().saturating_add(2))
    ));
    (lines, rows, engine_ran, stopped)
}

#[cfg(test)]
mod tests {
    use super::{Attempt, expected_said, feedback, read_harness, signature};
    use crate::catalogue::{CHALLENGES, Kind, Lit};

    #[test]
    #[ignore = "needs podman and the pinned images; run with --ignored"]
    fn every_harness_holds_a_reference_answer() {
        let merge = CHALLENGES
            .iter()
            .find(|c| c.name == "merge-sorted")
            .unwrap();
        let answers = [
            (
                "python",
                "def merge(a, b):\n    out = []\n    i = j = 0\n    while i < len(a) and j < len(b):\n        if a[i] <= b[j]:\n            out.append(a[i]); i += 1\n        else:\n            out.append(b[j]); j += 1\n    return out + a[i:] + b[j:]\n",
            ),
            (
                "javascript",
                "function merge(a, b) {\n  const out = [];\n  let i = 0, j = 0;\n  while (i < a.length && j < b.length) {\n    if (a[i] <= b[j]) out.push(a[i++]); else out.push(b[j++]);\n  }\n  return out.concat(a.slice(i), b.slice(j));\n}\n",
            ),
            (
                "rust",
                "fn merge(a: &[i64], b: &[i64]) -> Vec<i64> {\n    let mut out = Vec::new();\n    let (mut i, mut j) = (0, 0);\n    while i < a.len() && j < b.len() {\n        if a[i] <= b[j] { out.push(a[i]); i += 1; } else { out.push(b[j]); j += 1; }\n    }\n    out.extend_from_slice(&a[i..]);\n    out.extend_from_slice(&b[j..]);\n    out\n}\n\nfn main() {}\n",
            ),
            (
                "go",
                "package main\n\nimport \"fmt\"\n\nfunc merge(a []int, b []int) []int {\n\tout := []int{}\n\ti, j := 0, 0\n\tfor i < len(a) && j < len(b) {\n\t\tif a[i] <= b[j] {\n\t\t\tout = append(out, a[i])\n\t\t\ti++\n\t\t} else {\n\t\t\tout = append(out, b[j])\n\t\t\tj++\n\t\t}\n\t}\n\tout = append(out, a[i:]...)\n\treturn append(out, b[j:]...)\n}\n\nfunc main() {\n\tfmt.Println(merge([]int{1}, []int{2}))\n}\n",
            ),
        ];
        let podman = std::path::Path::new("/usr/bin/podman");
        let scratch = std::env::temp_dir().join(format!("mcf-harness-{}", std::process::id()));
        std::fs::create_dir_all(&scratch).unwrap();
        let mut wrong = Vec::new();
        for (name, answer) in answers {
            let language = super::language_named(name).unwrap();
            let program = super::checker(language, merge, answer);
            let (said, _) =
                crate::languages::run_program_heard(podman, &scratch, language, &program).unwrap();
            let (_, ran, cases) =
                read_harness(&said, merge.cases.len(), matches!(name, "rust" | "go"));
            if !ran || cases.iter().any(|(held, _)| !held) {
                wrong.push(format!("{name}: {said}\n{program}"));
            }
        }
        let _gone = std::fs::remove_dir_all(&scratch);
        assert!(wrong.is_empty(), "{}", wrong.join("\n"));
    }

    #[test]
    fn the_plan_says_what_it_runs_under_and_refuses_a_window_too_small() {
        let plan =
            super::Plan::asked(Some("go, python"), Some(3), Some("hard"), Some(8192)).unwrap();
        assert_eq!(
            plan.languages.iter().map(|l| l.name).collect::<Vec<_>>(),
            vec!["go", "python"]
        );
        let conditions = plan.conditions();
        let of = |key: &str| {
            conditions
                .iter()
                .find(|(k, _)| *k == key)
                .map(|(_, v)| v.clone())
        };
        assert_eq!(of("retries"), Some(mcf_record::json::Value::Integer(3)));
        assert_eq!(of("window"), Some(mcf_record::json::Value::Integer(8192)));
        assert_eq!(of("tier"), Some(mcf_record::json::Value::text("hard")));
        assert_eq!(
            of("languages"),
            Some(mcf_record::json::Value::text("go,python"))
        );
        let said = plan
            .said(std::path::Path::new("/nowhere/a.gguf"))
            .join("\n");
        assert!(said.contains("retries    3 attempt(s)"), "{said}");
        assert!(
            said.contains("window     8192 tokens, every attempt"),
            "{said}"
        );
        assert!(said.contains("languages  go, python"), "{said}");
        assert!(said.contains("tier       hard"), "{said}");
        assert!(
            said.contains("engine     not resolved here"),
            "no file at that path: {said}"
        );

        let bare = super::Plan::asked(None, None, None, None).unwrap();
        assert_eq!(bare.retries, super::RETRIES_DEFAULT);
        assert_eq!(bare.languages.len(), super::LANGUAGE_NAMES.len());
        assert_eq!(
            bare.conditions()
                .iter()
                .find(|(k, _)| *k == "window")
                .map(|(_, v)| v.clone()),
            Some(mcf_record::json::Value::Null),
            "no window asked is sized to the turn, said as null and not as nought"
        );
        assert!(
            bare.said(std::path::Path::new("/nowhere/a.gguf"))
                .join("\n")
                .contains("sized to each turn")
        );

        let why = super::Plan::asked(None, None, None, Some(1024)).unwrap_err();
        assert!(why.contains("at least 4096"), "{why}");
        let why = super::Plan::asked(Some("cobol"), None, None, None).unwrap_err();
        assert!(why.contains("cobol"), "{why}");
        let why = super::Plan::asked(None, Some(0), None, None).unwrap_err();
        assert!(why.contains("at least one"), "{why}");
    }

    #[test]
    fn a_compile_failure_is_handed_back_with_what_the_compiler_said() {
        let merge = CHALLENGES
            .iter()
            .find(|c| c.name == "merge-sorted")
            .unwrap();
        let heard = (0..30)
            .map(|at| {
                format!(
                    "error[E0308]: mismatched types on line {at} {}",
                    "x".repeat(400)
                )
            })
            .collect::<Vec<_>>()
            .join("\n\n");
        let said = super::compiler_said(&heard);
        assert_eq!(said.lines().count(), 12, "twelve lines at most");
        assert!(
            said.lines().all(|line| line.chars().count() <= 200),
            "each bounded"
        );
        let last = Attempt {
            wrote: true,
            compiled: Some(false),
            ran: false,
            cases: Vec::new(),
            tokens: 40,
            ask_ns: 1,
            window: None,
            compiler: said.clone(),
            runtime: String::new(),
            code: "fn merge() {}".to_owned(),
        };
        let rust = super::language_named("rust").unwrap();
        let fed = feedback(rust, merge, &last);
        assert!(fed.contains("The compiler said:"), "{fed}");
        assert!(
            fed.contains("error[E0308]: mismatched types on line 0"),
            "{fed}"
        );
        assert!(
            !fed.contains("on line 12"),
            "the thirteenth line is not handed back"
        );
        let rows = super::rows_of(merge, "rust", &[last]);
        let count = rows
            .iter()
            .find(|row| row.metric == "compiler_lines")
            .map(|row| row.value);
        assert_eq!(count, Some(12));
    }

    #[test]
    fn a_run_that_did_not_reach_the_end_is_handed_back_with_what_it_said() {
        let merge = CHALLENGES
            .iter()
            .find(|c| c.name == "merge-sorted")
            .unwrap();
        let python = super::language_named("python").unwrap();
        let traceback = "Traceback (most recent call last):\n  File \"/work/answer.py\", line 9, in \
                         <module>\n    __say(json.dumps(merge([1, 3], [2])))\nTypeError: 'int' object is not \
                         iterable";
        let last = Attempt {
            wrote: true,
            compiled: None,
            ran: false,
            cases: vec![(true, String::new())],
            tokens: 40,
            ask_ns: 1,
            window: None,
            compiler: String::new(),
            runtime: super::compiler_said(traceback),
            code: "def merge(a, b): return 1".to_owned(),
        };
        let fed = feedback(python, merge, &last);
        assert!(
            fed.contains("did not run to the end of the checks. It said:"),
            "{fed}"
        );
        assert!(
            fed.contains("TypeError: 'int' object is not iterable"),
            "{fed}"
        );
        let rows = super::rows_of(merge, "python", std::slice::from_ref(&last));
        let count = rows
            .iter()
            .find(|row| row.metric == "runtime_lines")
            .map(|row| row.value);
        assert_eq!(count, Some(4));
        let quiet = Attempt {
            runtime: String::new(),
            ..last
        };
        let fed = feedback(python, merge, &quiet);
        assert!(fed.contains("run past the deadline"), "{fed}");
    }

    #[test]
    #[ignore = "needs podman and the pinned image; run with --ignored"]
    fn a_traceback_reaches_the_correction() {
        let merge = CHALLENGES
            .iter()
            .find(|c| c.name == "merge-sorted")
            .unwrap();
        let python = super::language_named("python").unwrap();
        let program = super::checker(
            python,
            merge,
            "def merge(a, b):\n    return sorted(a + b)\n\nraise ValueError('boom on the way in')\n",
        );
        let podman = std::path::Path::new("/usr/bin/podman");
        let scratch = std::env::temp_dir().join(format!("mcf-traceback-{}", std::process::id()));
        std::fs::create_dir_all(&scratch).unwrap();
        let (said, heard) =
            crate::languages::run_program_heard(podman, &scratch, python, &program).unwrap();
        let _gone = std::fs::remove_dir_all(&scratch);
        let (_, ran, cases) = read_harness(&said, merge.cases.len(), false);
        assert!(!ran, "{said}");
        assert!(cases.is_empty(), "{said}");
        let on_the_way_out = super::compiler_said(&heard);
        assert!(
            on_the_way_out.contains("ValueError: boom on the way in"),
            "{heard}"
        );
        assert!(on_the_way_out.lines().count() <= 12);
    }

    #[test]
    fn signatures_and_expectations_are_rendered_in_each_language() {
        let merge = CHALLENGES
            .iter()
            .find(|c| c.name == "merge-sorted")
            .unwrap();
        assert_eq!(
            signature("rust", merge),
            "fn merge(a: &[i64], b: &[i64]) -> Vec<i64>"
        );
        assert_eq!(signature("go", merge), "func merge(a []int, b []int) []int");
        assert_eq!(signature("python", merge), "def merge(a, b)");
        assert_eq!(
            expected_said("rust", Kind::Ints, &Lit::Ints(&[1, 2])),
            "[1, 2]"
        );
        assert_eq!(
            expected_said("go", Kind::Ints, &Lit::Ints(&[1, 2])),
            "[1,2]"
        );
        assert_eq!(
            expected_said("javascript", Kind::OptInt, &Lit::None),
            "null"
        );
        assert_eq!(expected_said("rust", Kind::OptInt, &Lit::Int(5)), "Some(5)");
        assert_eq!(expected_said("go", Kind::OptInt, &Lit::None), "0 false");
        assert_eq!(
            expected_said("python", Kind::Text, &Lit::Text("ab")),
            "\"ab\""
        );
    }

    #[test]
    fn the_harness_lines_are_read_and_a_failure_is_handed_back() {
        let (compiled, ran, cases) = read_harness("compiled\nok\nno [1,3]\n", 2, true);
        assert_eq!(compiled, Some(true));
        assert!(ran);
        assert_eq!(
            cases,
            vec![(true, String::new()), (false, "[1,3]".to_owned())]
        );
        let (compiled, ran, _) = read_harness("notcompiled\n", 2, true);
        assert_eq!((compiled, ran), (Some(false), false));
        let merge = CHALLENGES
            .iter()
            .find(|c| c.name == "merge-sorted")
            .unwrap();
        let last = Attempt {
            wrote: true,
            compiled: None,
            ran: true,
            cases,
            tokens: 10,
            ask_ns: 1,
            window: None,
            compiler: String::new(),
            runtime: String::new(),
            code: "def merge(a, b): return a".to_owned(),
        };
        let said = feedback(&crate::languages::LANGUAGES[0], merge, &last);
        assert!(
            said.contains("should return [1,2,3,4,5,6] but returned [1,3]")
                || said.contains("but returned [1,3]"),
            "{said}"
        );
    }
}
