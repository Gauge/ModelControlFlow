use mcf_core::attested::Attested;
use mcf_core::build_identity::BuildIdentity;
use mcf_core::measurement::{Bytes, ConditionValue, Conditions, Floor, Measurement, Quantity};
use mcf_core::time::{Clock as _, Duration, Monotonic, SystemClock};

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

#[must_use]
pub(crate) fn artifact_bytes(path: &std::path::Path) -> Attested<Bytes> {
    match std::fs::metadata(path) {
        Ok(metadata) => Attested::Known(Bytes(metadata.len())),
        Err(_) => Attested::Unknown,
    }
}

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
        if outcome.is_ok() {
            samples.push(elapsed);
        }
    }
    Measurement::from_samples(samples, conditions)
}

#[must_use]
pub(crate) fn against_budget<Q: Quantity>(measured: Q, budget: Q) -> &'static str {
    if measured <= budget { "pass" } else { "OVER" }
}
