use std::io::Read as _;
use std::os::unix::process::CommandExt as _;
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};

use mcf_core::failure::{Attribution, Category, Disposition, Failure, Subsystem};
use mcf_core::time::{Clock as _, Duration, Instant, Monotonic, SystemClock};

const WHERE: Subsystem = Subsystem::new("mcf-prototype::supervise");

#[derive(Debug)]
pub(crate) struct Supervision {
    pub(crate) output: String,
    pub(crate) waited: Duration<Monotonic>,
    pub(crate) failure: Option<Failure>,
}

impl Supervision {
    #[must_use]
    pub(crate) fn preserved_partial_output(&self) -> bool {
        !self.output.is_empty()
    }
}

#[must_use]
pub(crate) fn supervise(
    program: &str,
    arguments: &[&str],
    deadline: Duration<Monotonic>,
) -> Supervision {
    let clock = SystemClock;
    let started = clock.now();

    let spawned = Command::new(program)
        .args(arguments)
        .process_group(0)
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .stdin(Stdio::null())
        .spawn();

    let mut child = match spawned {
        Ok(child) => child,
        Err(error) => {
            return Supervision {
                output: String::new(),
                waited: clock.now().saturating_duration_since(started),
                failure: Some(
                    Failure::new(
                        Category::EngineSpawnNotFound,
                        Attribution::Managed,
                        Disposition::Refused,
                        WHERE,
                        "the runtime could not be started",
                    )
                    .with_context("program", program)
                    .with_context("os_error", error.to_string()),
                ),
            };
        }
    };

    let stdout = child.stdout.take();
    let collected = Arc::new(Mutex::new(String::new()));
    let into = Arc::clone(&collected);
    let reader = std::thread::spawn(move || {
        if let Some(mut stdout) = stdout {
            let mut buffer = [0_u8; 4096];
            while let Ok(read) = stdout.read(&mut buffer) {
                let Some(fresh) = buffer.get(..read) else {
                    break;
                };
                if fresh.is_empty() {
                    break;
                }
                if let Ok(mut held) = into.lock() {
                    held.push_str(&String::from_utf8_lossy(fresh));
                }
            }
        }
    });

    let outcome = wait_until(&mut child, clock, started, deadline);
    let waited = clock.now().saturating_duration_since(started);
    let output = if matches!(outcome, Outcome::PastDeadline) {
        drop_and_take(reader, &collected)
    } else {
        let _joined: std::thread::Result<()> = reader.join();
        taken(&collected)
    };

    let failure = match outcome {
        Outcome::Exited { code: Some(0) } => None,
        Outcome::Exited { code } => Some(exit_failure(code, &output)),
        Outcome::Signalled { signal } => Some(
            Failure::new(
                Category::EngineExitSignal,
                Attribution::Managed,
                if output.is_empty() {
                    Disposition::Aborted
                } else {
                    Disposition::Partial
                },
                WHERE,
                "the runtime was killed by a signal",
            )
            .with_context("signal", signal.to_string())
            .with_context("bytes_before_death", output.len().to_string()),
        ),
        Outcome::PastDeadline => Some(
            Failure::new(
                Category::EngineHangNoOutput,
                Attribution::Managed,
                Disposition::Aborted,
                WHERE,
                "the runtime was alive and silent past its deadline",
            )
            .with_context("deadline_ns", deadline.as_nanos().to_string())
            .with_context("bytes_before_deadline", output.len().to_string()),
        ),
    };

    Supervision {
        output,
        waited,
        failure,
    }
}

fn taken(collected: &Arc<Mutex<String>>) -> String {
    collected.lock().map_or_else(
        |poisoned| poisoned.into_inner().clone(),
        |held| held.clone(),
    )
}

fn drop_and_take(reader: std::thread::JoinHandle<()>, collected: &Arc<Mutex<String>>) -> String {
    drop(reader);
    taken(collected)
}

#[allow(
    unsafe_code,
    reason = "the workspace denies unsafe rather than forbidding it so that the C-ABI work D4 \
              anticipates can opt in per site with the reason written down, and `nvml.rs` in \
              this same prototype opts in the same way. `kill(2)` is the only way to signal a \
              process group; there is no safe wrapper for it in the standard library and this \
              workspace takes no dependencies to get one"
)]
fn kill_the_group(of: &Child) {
    unsafe extern "C" {
        fn kill(pid: i32, signal: i32) -> i32;
    }
    const SIGKILL: i32 = 9;
    let Ok(leader) = i32::try_from(of.id()) else {
        return;
    };
    if leader <= 0 {
        return;
    }
    let _sent: i32 = unsafe { kill(-leader, SIGKILL) };
}

fn exit_failure(code: Option<i32>, output: &str) -> Failure {
    let category = if output.is_empty() {
        Category::EngineExitImmediate
    } else {
        Category::EngineExitMidstream
    };
    Failure::new(
        category,
        Attribution::Managed,
        if output.is_empty() {
            Disposition::Aborted
        } else {
            Disposition::Partial
        },
        WHERE,
        "the runtime exited without completing",
    )
    .with_context(
        "exit_code",
        match code {
            Some(code) => code.to_string(),
            None => "unknown".to_owned(),
        },
    )
    .with_context("bytes_before_exit", output.len().to_string())
}

enum Outcome {
    Exited { code: Option<i32> },
    Signalled { signal: i32 },
    PastDeadline,
}

fn wait_until(
    child: &mut Child,
    clock: SystemClock,
    started: Instant<Monotonic>,
    deadline: Duration<Monotonic>,
) -> Outcome {
    const POLL: std::time::Duration = std::time::Duration::from_millis(2);
    loop {
        match child.try_wait() {
            Ok(Some(status)) => return finished(status),
            Ok(None) => {}
            Err(_) => return Outcome::PastDeadline,
        }
        if clock.now().saturating_duration_since(started) >= deadline {
            kill_the_group(child);
            let _killed: Result<(), std::io::Error> = child.kill();
            let _reaped: Result<std::process::ExitStatus, std::io::Error> = child.wait();
            return Outcome::PastDeadline;
        }
        std::thread::sleep(POLL);
    }
}

#[cfg(unix)]
fn finished(status: std::process::ExitStatus) -> Outcome {
    use std::os::unix::process::ExitStatusExt as _;
    match status.signal() {
        Some(signal) => Outcome::Signalled { signal },
        None => Outcome::Exited {
            code: status.code(),
        },
    }
}

#[cfg(not(unix))]
fn finished(status: std::process::ExitStatus) -> Outcome {
    Outcome::Exited {
        code: status.code(),
    }
}

#[cfg(test)]
mod tests {
    use super::{Supervision, supervise};
    use mcf_core::failure::{Attribution, Category, Disposition, Failure};
    use mcf_core::time::{Duration, Monotonic};

    const DEADLINE: Duration<Monotonic> = Duration::from_nanos(5_000_000_000);

    const SHORT: Duration<Monotonic> = Duration::from_nanos(200_000_000);

    fn shell(script: &str) -> Supervision {
        supervise("/bin/sh", &["-c", script], DEADLINE)
    }

    fn failure(outcome: &Supervision) -> &Failure {
        outcome
            .failure
            .as_ref()
            .expect("the scenario is one where the child fails")
    }

    #[test]
    fn a_binary_that_is_not_there_is_a_classified_refusal() {
        let outcome = supervise("/nonexistent/mcf-no-such-runtime", &[], DEADLINE);
        let failure = failure(&outcome);
        assert_eq!(failure.category(), Category::EngineSpawnNotFound);
        assert_eq!(failure.attribution(), Attribution::Managed);
        assert_eq!(failure.disposition(), Disposition::Refused);
        assert!(failure.context_value("os_error").is_some());
    }

    #[test]
    fn a_child_that_exits_before_output_is_an_immediate_exit() {
        let outcome = shell("exit 3");
        let failure = failure(&outcome);
        assert_eq!(failure.category(), Category::EngineExitImmediate);
        assert_eq!(failure.disposition(), Disposition::Aborted);
        assert_eq!(failure.context_value("exit_code"), Some("3"));
        assert!(!outcome.preserved_partial_output());
    }

    #[test]
    fn output_before_a_signal_is_kept_and_marked_partial() {
        let outcome = shell("printf 'two of five chunks'; kill -9 $$");
        let failure = failure(&outcome);
        assert_eq!(failure.category(), Category::EngineExitSignal);
        assert_eq!(failure.disposition(), Disposition::Partial);
        assert_eq!(outcome.output, "two of five chunks");
        assert_eq!(failure.context_value("bytes_before_death"), Some("18"));
    }

    #[test]
    fn output_before_a_nonzero_exit_is_the_midstream_case() {
        let outcome = shell("printf 'partial'; exit 1");
        let failure = failure(&outcome);
        assert_eq!(failure.category(), Category::EngineExitMidstream);
        assert_eq!(failure.disposition(), Disposition::Partial);
        assert_eq!(outcome.output, "partial");
    }

    fn anything_running(marker: &str) -> bool {
        let Ok(entries) = std::fs::read_dir("/proc") else {
            return false;
        };
        for entry in entries.flatten() {
            let line = entry.path().join("cmdline");
            let Ok(raw) = std::fs::read(&line) else {
                continue;
            };
            let joined = String::from_utf8_lossy(&raw).replace('\0', " ");
            if joined.trim() == marker {
                return true;
            }
        }
        false
    }

    #[test]
    fn nothing_the_child_started_is_left_running() {
        let marker = "sleep 2147";
        assert!(
            !anything_running(marker),
            "something was already running `{marker}`, so this proves nothing"
        );
        let outcome = supervise("/bin/sh", &["-c", marker], SHORT);
        assert_eq!(
            failure(&outcome).category(),
            Category::EngineHangNoOutput,
            "the deadline did not fire, so nothing was killed and this proves nothing"
        );
        std::thread::sleep(std::time::Duration::from_millis(250));
        assert!(
            !anything_running(marker),
            "the supervisor gave up on the child and left what the child had started still \
             running (A27, F137)"
        );
    }

    #[test]
    fn a_silent_child_past_its_deadline_is_a_hang() {
        let outcome = supervise("/bin/sh", &["-c", "sleep 30"], SHORT);
        let failure = failure(&outcome);
        assert_eq!(failure.category(), Category::EngineHangNoOutput);
        assert_eq!(failure.disposition(), Disposition::Aborted);
        assert!(
            outcome.waited >= SHORT,
            "the supervisor gave up before its deadline: {}",
            outcome.waited
        );
        assert!(
            outcome.waited.as_nanos() < 10_000_000_000,
            "the supervisor waited for the child rather than its deadline: {}",
            outcome.waited
        );
    }

    #[test]
    fn a_child_that_succeeds_produces_no_failure() {
        let outcome = shell("printf 'done'; exit 0");
        assert!(outcome.failure.is_none(), "{:?}", outcome.failure);
        assert_eq!(outcome.output, "done");
    }

    #[test]
    fn the_wait_is_a_real_monotonic_interval() {
        let outcome = shell("exit 0");
        assert!(!outcome.waited.is_simulated());
    }
}
