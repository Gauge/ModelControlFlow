//! What MCF costs, measured rather than asserted.
//!
//! D24 gives the budgets and calls them ceilings rather than targets. §7.19
//! asks whether the substrate can meet them; B-011 is the suite that asserts
//! them in CI. This module measures the three that are measurable before a
//! daemon exists, and says plainly which ones are not.
//!
//! Every figure is a [`Measurement`] with its conditions and its spread — A6,
//! applied to MCF's own cost, because §3.8 holds that an uncharacterized
//! instrument is not a scientific one.

use mcf_core::attested::Attested;
use mcf_core::build_identity::BuildIdentity;
use mcf_core::measurement::{Bytes, ConditionValue, Conditions, Floor, Measurement, Quantity};
use mcf_core::time::{Clock as _, Duration, Monotonic, SystemClock};

/// The conditions these figures were taken under.
///
/// Sparse and honest: MCF cannot yet read its hardware (B-013) or its thermal
/// state, and A7 forbids filling either in. What it does know is what compiled
/// it and what it was told to run as.
#[must_use]
pub(crate) fn conditions(note: &str) -> Conditions {
    Conditions::new(
        BuildIdentity::current(),
        Floor {
            mcf_configuration: Attested::Known(ConditionValue::text(note)),
            ..Floor::nothing_known()
        },
    )
}

/// This process's resident set, in bytes.
///
/// Read from the kernel's own accounting rather than computed, because a
/// number MCF derived about its own memory is a number MCF would have to
/// defend. Returns [`Attested::Unknown`] where the platform does not publish
/// it — this route is Linux's, and saying so is cheaper than pretending
/// otherwise (A7).
#[must_use]
pub(crate) fn resident_bytes() -> Attested<Bytes> {
    let Ok(status) = std::fs::read_to_string("/proc/self/status") else {
        return Attested::Unknown;
    };
    let Some(line) = status.lines().find(|line| line.starts_with("VmRSS:")) else {
        return Attested::Unknown;
    };
    let Some(kibibytes) = line
        .split_whitespace()
        .nth(1)
        .and_then(|value| value.parse::<u64>().ok())
    else {
        return Attested::Unknown;
    };
    Attested::Known(Bytes(kibibytes.saturating_mul(1024)))
}

/// The size of a binary on disk, in bytes.
#[must_use]
pub(crate) fn artifact_bytes(path: &std::path::Path) -> Attested<Bytes> {
    match std::fs::metadata(path) {
        Ok(metadata) => Attested::Known(Bytes(metadata.len())),
        Err(_) => Attested::Unknown,
    }
}

/// How long a binary takes to answer, over `trials` runs.
///
/// D24 budgets *cold start to first command response* at 100 ms. This measures
/// the whole round trip a shell sees: spawn, load, answer, exit. It is
/// deliberately the pessimistic reading — a warm page cache makes it optimistic
/// in the other direction, and which of those a user experiences is a condition
/// nobody has decided how to state yet.
///
/// Returns `None` if fewer than two trials were asked for, because §3.4 makes a
/// single-shot timing an anecdote and [`Measurement`] has no way to hold one.
#[must_use]
pub(crate) fn cold_start(
    binary: &std::path::Path,
    arguments: &[&str],
    trials: usize,
    conditions: Conditions,
) -> Option<Measurement<Duration<Monotonic>>> {
    let clock = SystemClock;
    let mut samples = Vec::with_capacity(trials);
    for _ in 0..trials {
        let started = clock.now();
        let outcome = std::process::Command::new(binary)
            .args(arguments)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status();
        let elapsed = clock.now().saturating_duration_since(started);
        // A trial that could not run is not a slow trial. It is excluded and
        // the sample count says so, rather than being averaged in as a large
        // number (B24's shape: an unattributable trial is not a data point).
        if outcome.is_ok() {
            samples.push(elapsed);
        }
    }
    Measurement::from_samples(samples, conditions)
}

/// Renders a measurement against a budget, saying which side of it the reading
/// falls on.
///
/// The verdict is `pass` or `over`, never a percentage of the budget: D24 calls
/// every figure a ceiling rather than a target, and a percentage invites
/// optimizing toward the ceiling.
#[must_use]
pub(crate) fn against_budget<Q: Quantity>(measured: Q, budget: Q) -> &'static str {
    if measured <= budget { "pass" } else { "OVER" }
}
