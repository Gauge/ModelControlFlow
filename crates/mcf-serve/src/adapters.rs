use std::io::Read as _;
use std::os::unix::process::ExitStatusExt as _;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use mcf_core::failure::{Attribution, Category, Disposition, Failure, Subsystem};

const WHERE: Subsystem = Subsystem::new("mcf-serve::adapters");

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ended {
    pub produced: usize,
    pub said: String,
    pub peak_resident: Option<u64>,
}

#[must_use]
pub fn peak_resident_of(pid: u32) -> Option<u64> {
    status_bytes_of(pid, "VmHWM:")
}

#[must_use]
pub fn resident_of(pid: u32) -> Option<u64> {
    status_bytes_of(pid, "VmRSS:")
}

fn status_bytes_of(pid: u32, key: &str) -> Option<u64> {
    let status = std::fs::read_to_string(format!("/proc/{pid}/status")).ok()?;
    let line = status.lines().find(|line| line.starts_with(key))?;
    let kibibytes: u64 = line
        .trim_start_matches(key)
        .trim()
        .trim_end_matches("kB")
        .trim()
        .parse()
        .ok()?;
    kibibytes.checked_mul(1024)
}

#[allow(
    clippy::too_many_lines,
    reason = "the supervision contract is one sequence — start, drain, wait, classify — and \
              the stages a process can die in are told apart at the end of it; splitting it \
              would put the reading of the exit away from the reading of the output it \
              qualifies"
)]
pub fn supervise(command: &mut Command, on_chunk: &mut dyn FnMut(&[u8])) -> Result<Ended, Failure> {
    let mut child = command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| {
            let category = if error.kind() == std::io::ErrorKind::NotFound {
                Category::EngineSpawnNotFound
            } else {
                Category::EngineSpawnRefused
            };
            Failure::new(
                category,
                Attribution::Machine,
                Disposition::Refused,
                WHERE,
                "the engine could not be started",
            )
            .with_context(
                "program",
                command.get_program().to_string_lossy().into_owned(),
            )
            .with_context("os_error", error.to_string())
        })?;

    let stderr = child.stderr.take();
    let complaints = std::thread::spawn(move || {
        let mut said = String::new();
        if let Some(mut stderr) = stderr {
            let mut raw = Vec::new();
            let _read = stderr.read_to_end(&mut raw);
            said = String::from_utf8_lossy(&raw).into_owned();
        }
        said
    });

    let mut produced = 0_usize;
    let mut peak_resident: Option<u64> = None;
    if let Some(mut stdout) = child.stdout.take() {
        let mut buffer = [0_u8; 4096];
        loop {
            match stdout.read(&mut buffer) {
                Ok(0) | Err(_) => break,
                Ok(count) => {
                    produced = produced.saturating_add(count);
                    if let Some(chunk) = buffer.get(..count) {
                        on_chunk(chunk);
                    }
                    peak_resident = peak_resident_of(child.id()).max(peak_resident);
                }
            }
        }
    }
    peak_resident = peak_resident_of(child.id()).max(peak_resident);
    let status = child.wait().map_err(|error| {
        Failure::new(
            Category::EngineExitSignal,
            Attribution::Machine,
            Disposition::Aborted,
            WHERE,
            "the engine's exit could not be read",
        )
        .with_context("os_error", error.to_string())
    })?;
    let said = complaints.join().unwrap_or_default();
    let tail: String = said
        .lines()
        .rev()
        .take(12)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect::<Vec<_>>()
        .join("\n");

    if status.success() {
        return Ok(Ended {
            produced,
            said: tail,
            peak_resident,
        });
    }

    let (category, what) = match (status.signal(), produced) {
        (Some(_), _) => (
            Category::EngineExitSignal,
            "the engine was killed by a signal",
        ),
        (None, 0) => (
            Category::EngineExitImmediate,
            "the engine exited before producing anything",
        ),
        (None, _) => (
            Category::EngineExitMidstream,
            "the engine exited after producing part of an answer",
        ),
    };
    let disposition = if produced == 0 {
        Disposition::Aborted
    } else {
        Disposition::Partial
    };
    Err(
        Failure::new(category, Attribution::Managed, disposition, WHERE, what)
            .with_context("exit", status.to_string())
            .with_context("produced_bytes", produced.to_string())
            .with_context("engine_said", tail),
    )
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProvisionedLlama {
    pub prefix: PathBuf,
    pub commit: String,
    pub component: String,
}

fn window_for(prompt: &str, limit: usize) -> u64 {
    const SMALLEST: u64 = 4096;
    let prompt_bytes = u64::try_from(prompt.len()).unwrap_or(SMALLEST);
    let asked = u64::try_from(limit).unwrap_or(SMALLEST);
    prompt_bytes
        .saturating_add(asked)
        .saturating_mul(2)
        .max(SMALLEST)
}

impl ProvisionedLlama {
    #[must_use]
    pub fn completion(&self) -> PathBuf {
        self.prefix
            .join("build")
            .join("bin")
            .join("llama-completion")
    }

    #[must_use]
    pub fn generate(
        &self,
        model: &Path,
        prompt: &str,
        limit: usize,
        draw: crate::generation::Draw,
        pinned: bool,
    ) -> Command {
        let mut command = Command::new(self.completion());
        if pinned {
            command.arg("--ignore-eos");
        }
        command
            .arg("-m")
            .arg(model)
            .arg("-p")
            .arg(prompt)
            .arg("-n")
            .arg(limit.to_string())
            .arg("--ctx-size")
            .arg(window_for(prompt, limit).to_string())
            .arg("--temp")
            .arg(if draw.is_greedy() {
                "0".to_owned()
            } else {
                draw.temperature.to_string()
            })
            .arg("--seed")
            .arg(draw.seed.to_string())
            .arg("--top-k")
            .arg(draw.truncation.top_k_sent().to_string())
            .arg("--top-p")
            .arg(draw.truncation.top_p_sent().to_string())
            .arg("--min-p")
            .arg(draw.truncation.min_p_sent().to_string())
            .arg("--no-warmup")
            .arg("-ngl")
            .arg("0")
            .arg("-no-cnv")
            .arg("--no-display-prompt");
        command
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Found {
    One(ProvisionedLlama),
    None,
    Several(Vec<PathBuf>),
}

#[must_use]
pub fn provisioned_llama(mcf_home: &Path) -> Found {
    let root = mcf_home.join("provisioned");
    let Ok(entries) = std::fs::read_dir(&root) else {
        return Found::None;
    };
    let mut prefixes: Vec<(PathBuf, String, String)> = Vec::new();
    for entry in entries.flatten() {
        let prefix = entry.path();
        let provenance = prefix.join("mcf-provenance.json");
        let Ok(text) = std::fs::read_to_string(&provenance) else {
            continue;
        };
        let Ok(value) = mcf_record::json::parse(text.trim()) else {
            continue;
        };
        let component = value
            .get("component")
            .and_then(mcf_record::json::Value::as_text)
            .unwrap_or_default();
        if !component.starts_with("llama.cpp") {
            continue;
        }
        let commit = value
            .get("commit")
            .and_then(mcf_record::json::Value::as_text)
            .unwrap_or("unstated")
            .to_owned();
        if prefix
            .join("build")
            .join("bin")
            .join("llama-completion")
            .is_file()
        {
            prefixes.push((prefix, commit, component.to_owned()));
        }
    }
    match prefixes.len() {
        0 => Found::None,
        1 => match prefixes.pop() {
            Some((prefix, commit, component)) => Found::One(ProvisionedLlama {
                prefix,
                commit,
                component,
            }),
            None => Found::None,
        },
        _ => Found::Several(prefixes.into_iter().map(|(prefix, _, _)| prefix).collect()),
    }
}

pub fn only_one(found: Found) -> Result<Option<ProvisionedLlama>, Failure> {
    match found {
        Found::One(llama) => Ok(Some(llama)),
        Found::None => Ok(None),
        Found::Several(prefixes) => Err(Failure::new(
            Category::ConfigConflict,
            Attribution::User,
            Disposition::Refused,
            WHERE,
            "more than one llama.cpp is provisioned, and MCF will not choose between builds",
        )
        .with_context(
            "prefixes",
            prefixes
                .iter()
                .map(|prefix| prefix.display().to_string())
                .collect::<Vec<_>>()
                .join(", "),
        )
        .with_context(
            "what_to_do",
            "remove all but one (`mcf provision --remove`)",
        )),
    }
}

#[cfg(test)]
mod command_tests {
    #![allow(clippy::panic, clippy::expect_used)]

    use std::path::{Path, PathBuf};

    use mcf_core::configuration::Thousandths;

    use super::ProvisionedLlama;
    use crate::generation::{Draw, Stated, Truncation, Whose};

    fn tool() -> ProvisionedLlama {
        ProvisionedLlama {
            prefix: PathBuf::from("/nowhere/llama.cpp@abc"),
            commit: "abc".to_owned(),
            component: "llama.cpp".to_owned(),
        }
    }

    fn arguments(draw: Draw) -> Vec<String> {
        tool()
            .generate(Path::new("/nowhere/model.gguf"), "hello", 8, draw, false)
            .get_args()
            .map(|held| held.to_string_lossy().into_owned())
            .collect()
    }

    fn after<'a>(arguments: &'a [String], flag: &str) -> Option<&'a str> {
        arguments
            .iter()
            .position(|held| held == flag)
            .and_then(|at| arguments.get(at.checked_add(1)?))
            .map(String::as_str)
    }

    #[test]
    fn the_cut_is_on_the_command_line() {
        let off = arguments(Draw {
            seed: 1,
            temperature: Thousandths(700),
            truncation: Truncation::OFF,
        });
        assert_eq!(after(&off, "--top-k"), Some("0"));
        assert_eq!(after(&off, "--top-p"), Some("1.000"));
        assert_eq!(after(&off, "--min-p"), Some("0.000"));
        assert_eq!(after(&off, "--temp"), Some("0.700"));

        let declared = arguments(Draw {
            seed: 1,
            temperature: Thousandths(700),
            truncation: Truncation {
                top_k: Stated::Declared(20),
                top_p: Stated::Declared(Thousandths(950)),
                min_p: Stated::Declared(Thousandths(50)),
                whose: Whose::File,
            },
        });
        assert_eq!(after(&declared, "--top-k"), Some("20"));
        assert_eq!(after(&declared, "--top-p"), Some("0.950"));
        assert_eq!(after(&declared, "--min-p"), Some("0.050"));
    }
}
