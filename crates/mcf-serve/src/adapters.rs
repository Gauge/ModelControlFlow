//! Engines that are processes: started, read, and classified when they die
//! (B-032, B-033, D39, §3.1, §7.1).
//!
//! **The supervision contract, in one function.** [`supervise`] starts a
//! command, hands every chunk of its output to the caller as it arrives, and
//! turns how the process ended into a classified failure or a clean exit. The
//! stages §3.1 names are the stages a process can die *in*, and each has its
//! own category so that the record can tell them apart: it could not be
//! started (`engine.spawn.not_found`, `engine.spawn.refused`), it died before
//! saying anything (`engine.exit.immediate`), it died after some output
//! (`engine.exit.midstream`), or the platform killed it
//! (`engine.exit.signal`). None of them takes the daemon down: this function
//! returns, and the account of what happened is what the caller writes down.
//!
//! **Nothing is retried** (PR9, B2). A retried generation is a different
//! generation, and a record that showed one where two happened would be lying
//! about time.
//!
//! **Which engine is the provisioned one is read from the disk, not chosen.**
//! [`provisioned_llama`] looks for exactly one provisioned `llama.cpp` prefix
//! under MCF's data home. Two is a question for the operator, not a preference
//! MCF invents (§3.15); none means MCF's own engine is the engine, which is
//! D39's fourth condition.

use std::io::Read as _;
use std::os::unix::process::ExitStatusExt as _;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use mcf_core::failure::{Attribution, Category, Disposition, Failure, Subsystem};

const WHERE: Subsystem = Subsystem::new("mcf-serve::adapters");

/// How a supervised process ended.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ended {
    /// Every byte the process wrote to its standard output.
    pub produced: usize,
    /// Its standard error, bounded — the engine's own words about why.
    pub said: String,
    /// The most memory it held resident, in bytes, where the kernel said —
    /// sampled as its output arrived, so the mark is at least what it had
    /// reached by its last word.
    pub peak_resident: Option<u64>,
}

/// The most memory a process has held resident, in bytes, from the kernel's
/// own high-water mark (`VmHWM` in `/proc/<pid>/status`).
///
/// `None` where there is no such file — another kernel, or a process already
/// gone — which is *not observed*, never zero (A7). Reading a process's own
/// accounting is not sampling the hardware (B4): nothing here touches a
/// counter that costs anything, and it is read only of a child MCF started.
#[must_use]
pub fn peak_resident_of(pid: u32) -> Option<u64> {
    let status = std::fs::read_to_string(format!("/proc/{pid}/status")).ok()?;
    let line = status.lines().find(|line| line.starts_with("VmHWM:"))?;
    let kibibytes: u64 = line
        .trim_start_matches("VmHWM:")
        .trim()
        .trim_end_matches("kB")
        .trim()
        .parse()
        .ok()?;
    kibibytes.checked_mul(1024)
}

/// Starts `command`, streams its standard output to `on_chunk` as it arrives,
/// and classifies the exit.
///
/// # Errors
///
/// One of the `engine.*` categories above, each carrying the exit status or
/// signal, how many bytes had been produced, and a bounded tail of standard
/// error — because an engine that died said why on that stream, and A2 wants
/// it written down rather than lost with the process.
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

    // Standard error is drained on its own thread: an engine that fills its
    // stderr pipe while this thread reads stdout would block, and a blocked
    // engine looks like a hung one (B7).
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
    // The mark is read while the process is still there to be read: after
    // `wait` its accounting is gone with it. Each chunk is a moment it is
    // known to be alive, and the last chunk comes after the work.
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

/// The one provisioned `llama.cpp`, if there is exactly one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProvisionedLlama {
    /// The prefix it was built into.
    pub prefix: PathBuf,
    /// The pinned commit, from its provenance.
    pub commit: String,
    /// What the component is called, from its provenance.
    ///
    /// **Because the account used to spell it from a literal.** Every
    /// generation recorded its engine as `provisioned llama.cpp @<commit>`
    /// however it had been built, so a run on `llama.cpp-cuda` and a run on
    /// `llama.cpp` wrote the same name — and the path beside it said
    /// otherwise. Two engines with one recorded identity is a comparison that
    /// reports no moved condition when one moved (A6, F45).
    pub component: String,
}

/// How long a window one request needs.
///
/// **Counted in bytes rather than tokens, deliberately.** Sizing this properly
/// would mean tokenizing the prompt, which needs the vocabulary, which needs
/// the model this command exists to hand to a subprocess. A token is several
/// bytes, so counting bytes over-counts — the safe direction for a window, and
/// still four orders of magnitude below the trained context that was being
/// opened instead.
///
/// Doubled, because a prompt and its answer both sit in the window and neither
/// is known exactly here; floored, because a window smaller than the smallest
/// useful one buys nothing.
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
    /// The completion tool inside the prefix.
    #[must_use]
    pub fn completion(&self) -> PathBuf {
        self.prefix
            .join("build")
            .join("bin")
            .join("llama-completion")
    }

    /// The command for one greedy generation, the same shape the oracle tier
    /// uses — so what the daemon serves through this engine is what the
    /// oracle compared.
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
            // The length is the limit, not a ceiling on it (B-396). The
            // tool honours this; what it does not do is count, so a caller
            // that needs the count proven cannot have it from this path.
            command.arg("--ignore-eos");
        }
        command
            .arg("-m")
            .arg(model)
            .arg("-p")
            .arg(prompt)
            .arg("-n")
            .arg(limit.to_string())
            // **A window sized to this request, not to the model.** Nothing
            // was passed here, and llama.cpp reads that as *the model's whole
            // trained context*: 131,072 tokens on a 14.5 GB model held
            // **81.3 GB resident** to generate a few hundred tokens, on every
            // request, because the tool is started once per generation. It is
            // F133's defect exactly — a hidden value that was not the stated
            // condition — in the tool beside the server where it was fixed,
            // and it went unseen because nothing measured what a request
            // costs (F146).
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
            .arg("--no-warmup")
            .arg("-ngl")
            .arg("0")
            .arg("-no-cnv")
            .arg("--no-display-prompt");
        // Not `--log-disable`: in this build the completion itself travels
        // through the same output the flag silences, and the first provisioned
        // generation produced an empty answer with a clean exit (F36). What the
        // tool logs goes to standard error, which is read separately.
        command
    }
}

/// What the data home holds by way of a provisioned `llama.cpp`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Found {
    /// Exactly one, usable.
    One(ProvisionedLlama),
    /// None: MCF's own engine is the engine.
    None,
    /// More than one pin, which is the operator's choice to make.
    Several(Vec<PathBuf>),
}

/// Looks for provisioned `llama.cpp` prefixes under `<data>/mcf/provisioned`.
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
        // **By shape, not by name.** This matched `component == "llama.cpp"`
        // exactly, so `llama.cpp-cuda` was invisible to every generation and
        // every measurement MCF took — the same defect as F129 and F130, in a
        // copy that was fixed in `engines.rs` and not here. What makes a
        // prefix an engine is that it holds one, which is checked below.
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

/// Exactly one provisioned engine, or the refusal that says why not.
///
/// Two pins is the operator's choice to make, and MCF says so rather than
/// picking: which of two builds served an answer is a condition (§3.15).
///
/// # Errors
///
/// `config.conflict` naming every prefix found, when there is more than one.
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
