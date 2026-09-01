//! Supervising a child process that is trying to fail badly.
//!
//! A3 is the rule under test: *no failure of a managed thing may take down MCF
//! itself*. §7.19 asks whether the substrate makes that cheap, and D4's answer
//! — Rust — is the one this prototype is here to confirm or amend.
//!
//! The four ways a supervised runtime dies badly, and the category each is
//! filed under:
//!
//! | What the child does | Category |
//! |---|---|
//! | Is not there to spawn | `engine.spawn.not_found` |
//! | Exits before any output | `engine.exit.immediate` |
//! | Emits some output, then is killed by a signal | `engine.exit.signal` |
//! | Stays alive and silent past its deadline | `engine.hang.no_output` |
//!
//! Two obligations shape the implementation. A4: whatever the child produced
//! before it died is kept and marked, so eleven tokens before a crash are
//! eleven tokens. And B37: the deadline is a monotonic interval, never a
//! difference of wall-clock readings.

use std::io::Read as _;
use std::os::unix::process::CommandExt as _;
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};

use mcf_core::failure::{Attribution, Category, Disposition, Failure, Subsystem};
use mcf_core::time::{Clock as _, Duration, Instant, Monotonic, SystemClock};

const WHERE: Subsystem = Subsystem::new("mcf-prototype::supervise");

/// What happened to a supervised child.
#[derive(Debug)]
pub(crate) struct Supervision {
    /// What the child managed to emit before it stopped, whether or not it
    /// stopped well. A4: a partial outcome is an outcome.
    pub(crate) output: String,
    /// How long the supervisor waited, from the monotonic clock.
    pub(crate) waited: Duration<Monotonic>,
    /// The classified failure, or `None` if the child did what it was asked.
    pub(crate) failure: Option<Failure>,
}

impl Supervision {
    /// Whether the supervisor kept what the child produced.
    #[must_use]
    pub(crate) fn preserved_partial_output(&self) -> bool {
        !self.output.is_empty()
    }
}

/// Runs a child under supervision, with a deadline.
///
/// Never returns an error to its caller and never propagates the child's
/// failure as its own: A3 says the manager survives the managed, and this
/// signature is that rule written down. Everything that goes wrong comes back
/// as a classified [`Failure`] inside a [`Supervision`].
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
        // **Its own process group, so that what it starts can be reached.**
        // A child that forks — a shell running one command is the ordinary
        // case — leaves descendants `Child` has no handle on at all. Killing
        // the child then kills the one process the supervisor can name while
        // its children carry on holding the pipe. The group is the only handle
        // there is on a process's descendants, and a supervisor that cannot
        // reach them cannot honour A27 (F137).
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

    // The child's output is drained on another thread. Reading it on this one
    // would make the deadline unenforceable: a child that writes nothing and
    // never exits would block the read for ever, which is precisely the
    // `engine.hang.no_output` case this is here to catch.
    let stdout = child.stdout.take();
    // **Shared, and appended to as it arrives.** Read to the end into a local
    // and handed back at the join, what the child produced is only reachable
    // once the pipe closes — so a supervisor that wanted the partial output
    // had to wait for the very thing it had given up on. A4's partial outcome
    // has to be readable *while* it is still partial.
    let collected = Arc::new(Mutex::new(String::new()));
    let into = Arc::clone(&collected);
    let reader = std::thread::spawn(move || {
        if let Some(mut stdout) = stdout {
            let mut buffer = [0_u8; 4096];
            // A read that fails ends the drain and leaves what was already
            // collected, which is A4's partial outcome rather than an error
            // that discards it.
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
    // **Taken before anything is joined.** `waited` says how long the
    // supervisor waited, and it was read after joining the drain — so a child
    // whose descendants held the pipe open made the supervisor report their
    // lifetime as its own. The deadline was enforced at 200 ms and the number
    // beside it said thirty seconds (F137).
    let waited = clock.now().saturating_duration_since(started);
    // **Past the deadline, the drain is not joined.** The child and its group
    // were killed, but nothing guarantees a descendant somewhere else has not
    // inherited the write end. Waiting on it would be the managed thing taking
    // down the manager, which is the one rule this prototype exists to test
    // (A3). What has arrived is taken; the drain ends when the pipe does.
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

/// What the drain has collected so far.
fn taken(collected: &Arc<Mutex<String>>) -> String {
    // A poisoned lock still holds what was written before the panic, and A4
    // says a partial outcome is an outcome.
    collected
        .lock()
        .map_or_else(|poisoned| poisoned.into_inner().clone(), |held| held.clone())
}

/// Takes what the drain collected and lets the thread go unjoined.
fn drop_and_take(reader: std::thread::JoinHandle<()>, collected: &Arc<Mutex<String>>) -> String {
    drop(reader);
    taken(collected)
}

/// Kills the process group this child leads.
///
/// A child spawned into its own group leads it, so the negative of its
/// identifier names every process it started and every one those started.
/// `Child` reaches only the one process it holds; there is no other handle on
/// the rest.
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
    // Negative: the group led by that identifier, rather than that process.
    // The result is not read — the group may already be gone, which is the
    // outcome being asked for.
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
        // A7: a process killed by a signal has no exit code, and `unknown` is
        // the honest rendering rather than a substituted number.
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

/// Waits for the child, or gives up at the deadline and kills it.
///
/// Polls rather than blocks, because the deadline has to be enforceable. The
/// interval is short enough that the measured wait is dominated by the child
/// and long enough that the supervisor is not itself a busy loop — a busy
/// supervisor would violate §3.13 while measuring §3.13.
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
            // The child cannot be waited for at all. Treating that as a
            // deadline is the conservative reading: it is alive as far as the
            // supervisor can tell, and it will be killed below.
            Err(_) => return Outcome::PastDeadline,
        }
        if clock.now().saturating_duration_since(started) >= deadline {
            // A27's habit at the smallest scale: MCF does not leave behind a
            // process it started — including the ones it started indirectly,
            // which is why this reaches the group and not just the child.
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

    /// Generous, because these scenarios are about *which* outcome is reached
    /// and not about how long it takes. A tight deadline made them flaky: on a
    /// loaded machine a child that exits promptly can still be scheduled late,
    /// and the supervisor then correctly reports a hang — the right answer to
    /// the wrong question. A18 requires a correctness test be deterministic,
    /// and a timing that decides the outcome makes it a benchmark instead.
    const DEADLINE: Duration<Monotonic> = Duration::from_nanos(5_000_000_000);

    /// Short, and used only where the child is *made* to outlast it. The gap
    /// between this and the child's thirty seconds is what makes the hang
    /// scenario deterministic under any load.
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

    /// A3: the manager survives the managed. Every scenario below returns to
    /// this thread, which is the claim — a supervisor that died would take the
    /// test with it.
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

    /// A4: eleven tokens before a runtime died are eleven tokens, marked
    /// truncated. The bytes are kept and the disposition says `partial`.
    #[test]
    fn output_before_a_signal_is_kept_and_marked_partial() {
        let outcome = shell("printf 'two of five chunks'; kill -9 $$");
        let failure = failure(&outcome);
        assert_eq!(failure.category(), Category::EngineExitSignal);
        assert_eq!(failure.disposition(), Disposition::Partial);
        assert_eq!(outcome.output, "two of five chunks");
        assert_eq!(failure.context_value("bytes_before_death"), Some("18"));
    }

    /// Output before a non-zero *exit* is the midstream case, which is a
    /// different category from the same bytes before a signal — because how a
    /// runtime died is what a reader needs (§6.17's shape, applied to MCF's own
    /// managed processes).
    #[test]
    fn output_before_a_nonzero_exit_is_the_midstream_case() {
        let outcome = shell("printf 'partial'; exit 1");
        let failure = failure(&outcome);
        assert_eq!(failure.category(), Category::EngineExitMidstream);
        assert_eq!(failure.disposition(), Disposition::Partial);
        assert_eq!(outcome.output, "partial");
    }

    /// Whether any process on this machine is running exactly this command.
    fn anything_running(marker: &str) -> bool {
        let Ok(entries) = std::fs::read_dir("/proc") else {
            return false;
        };
        for entry in entries.flatten() {
            let line = entry.path().join("cmdline");
            let Ok(raw) = std::fs::read(&line) else {
                continue;
            };
            // `/proc/<pid>/cmdline` separates arguments with NUL.
            let joined = String::from_utf8_lossy(&raw).replace('\0', " ");
            if joined.trim() == marker {
                return true;
            }
        }
        false
    }

    /// A27 at the smallest scale, and the half a `Child` cannot reach.
    ///
    /// A shell running one command forks it, so the supervisor's handle names
    /// the shell and not the thing actually sleeping. Killing what it can name
    /// leaves the rest running and holding the pipe — which is both a process
    /// left behind and, because the drain never ends, a manager waiting on the
    /// managed (F137).
    #[test]
    fn nothing_the_child_started_is_left_running() {
        // Distinctive enough that nothing else on the machine matches it.
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
        // The kill is delivered to the group; the descendants are reaped by
        // the kernel a moment later.
        std::thread::sleep(std::time::Duration::from_millis(250));
        assert!(
            !anything_running(marker),
            "the supervisor gave up on the child and left what the child had started still \
             running (A27, F137)"
        );
    }

    /// The one outcome B7 calls neither success nor diagnosis is a hang, so it
    /// is the one the deadline exists to convert into a diagnosis.
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
        // The upper bound is loose on purpose: it asserts that the supervisor
        // gives up rather than waiting for the child, which is the property,
        // and not how promptly a loaded machine schedules it.
        assert!(
            outcome.waited.as_nanos() < 10_000_000_000,
            "the supervisor waited for the child rather than its deadline: {}",
            outcome.waited
        );
    }

    /// A child that does what it was asked produces no failure at all, so the
    /// vocabulary above is not simply what this function always says.
    #[test]
    fn a_child_that_succeeds_produces_no_failure() {
        let outcome = shell("printf 'done'; exit 0");
        assert!(outcome.failure.is_none(), "{:?}", outcome.failure);
        assert_eq!(outcome.output, "done");
    }

    /// B37: the wait is a monotonic interval, and it is not simulated — a
    /// deadline enforced against a simulated clock would enforce nothing.
    #[test]
    fn the_wait_is_a_real_monotonic_interval() {
        let outcome = shell("exit 0");
        assert!(!outcome.waited.is_simulated());
    }
}
