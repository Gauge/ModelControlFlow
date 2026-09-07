//! The languages beyond Python the catalogue asks in, each with its pinned
//! image: what an answer is stripped of before the harness wraps it, whether
//! the image can be run at all, and one runner for a program under the same
//! confinement as the Python one (B-523, B-563, B-025).

use std::path::Path;

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
}

/// The languages beyond Python, Go and the catalogue's own Python entry:
/// the two the catalogue reaches through this file.
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

/// Runs one program in the language's container and returns what it
/// printed and what reached its error stream — the compiler's complaint or
/// the program's own — under the same confinement as the Python runner: no
/// network, no capabilities, a read-only root with a private /tmp for a
/// compiler's output, a memory ceiling, a process limit and a deadline
/// (B-025, B-565, B-566).
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

#[cfg(test)]
mod tests {
    use super::{LANGUAGES, plain};

    #[test]
    fn a_javascript_answer_loses_its_exports_and_a_rust_answer_its_main() {
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
}
