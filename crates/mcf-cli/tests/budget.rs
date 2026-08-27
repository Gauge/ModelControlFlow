//! The performance budget tier (B-011).
//!
//! B20: *idle CPU, resident memory, disk footprint, cold start and the latency
//! MCF interposes are budgeted, measured and regression-tested.* D24 gives the
//! numbers and D27 says which reading each one is about.
//!
//! **Why this tier is `#[ignore]`d by default.** An event-class figure needs a
//! hundred trials (D27), each of which is a process spawn, and B38 requires the
//! gating tier stay fast because a gate people skip does not gate. This is one
//! of B38's scheduled tiers: `scripts/ci.sh --with-budget` runs it, and it runs
//! before a release.
//!
//! **Why it asserts only on a release build.** D24's ceilings are for the
//! artifact MCF ships, and a debug binary is a different artifact — larger,
//! slower, and with different code. The profile is a condition (§3.4), and a
//! figure measured under the wrong one is reported rather than asserted. A
//! debug run of this tier is therefore informative and never green-by-luck.
//!
//! **Why a busy machine does not fail it.** B35: a timing taken under
//! contention measures the contention. D27 makes such a run *unattributable* —
//! neither a pass nor a failure — and B38's staleness discipline is what stops
//! that becoming a hiding place: an unattributable run does not refresh the
//! tier's age.
//!
//! **The verdict brackets each measurement rather than the run** (D30). It is a
//! question about a reading, not about the machine, so two figures taken
//! seconds apart get two answers.
//!
//! **Every figure is compared with the last one recorded** (B20, B-011). A
//! ceiling catches a figure that became bad; a baseline catches one that became
//! worse, which is the earlier and more useful signal — and B20 is explicit
//! that a performance change without a before-and-after under stated conditions
//! is not a performance change but a guess. The previous readings live beside
//! the tier ages in `.mcf-tiers/performance/`, machine-local, because a
//! baseline from somebody else's machine is not a baseline (B-166's habit).
//!
//! **Not every figure is judged against its baseline, and the ones that are not
//! say so.** A18 makes a regression detector a third thing, whose thresholds
//! are statistical judgments rather than assertions. A file's size is
//! deterministic and a fresh process's resident set is nearly so, so a
//! tolerance on those means something. The event-class figures are dominated on
//! some storage by conditions MCF does not yet record — [findings.md] F5, and
//! B-193 is the fix — so their change is *reported* and not asserted, because a
//! detector that cries wolf is one people switch off.
//!
//! [findings.md]: ../../../doc/findings.md

// Every item in this file is test code; see the note in checks/tests/taxonomy_agreement.rs.
#![allow(clippy::panic, clippy::expect_used)]

use std::path::{Path, PathBuf};

use mcf_core::attested::Attested;
use mcf_core::build_identity::BuildIdentity;
use mcf_core::hardware::{Attributability, Machine, Watch};
use mcf_core::measurement::{Bytes, ConditionValue, Conditions, Floor, Measurement, Quantity};
use mcf_core::self_cost::{
    self, ADDED_LATENCY, Budget, COLD_START, CORE_BINARY, EVENT_TRIALS, RESIDENT_IDLE, Verdict,
};
use mcf_core::time::{Duration, Monotonic};

/// The binary under test: the one cargo built for this test run.
fn binary() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_mcf"))
}

/// The conditions every figure below is taken under.
///
/// A6: no number without them. The load average is in here because D27 makes
/// attributability part of what a budget reading means, and a reader who
/// disagrees with the threshold needs the reading it was applied to.
fn conditions(machine: &Machine, attributability: &Attributability) -> Conditions {
    Conditions::new(
        BuildIdentity::current(),
        Floor {
            hardware_state: Attested::Known(ConditionValue::text(format!(
                "{} · {}",
                machine.processor.model, attributability
            ))),
            mcf_configuration: Attested::Known(ConditionValue::text(
                "the budget tier, release profile",
            )),
            // The storage the artifact was read from (B-193). F5 measured it
            // changing a cold start by three orders of magnitude, and a
            // baseline comparison below refuses two readings that do not share
            // it (A8).
            artifact_storage: match mcf_core::hardware::storage_of(&binary()) {
                Attested::Known(storage) => {
                    Attested::Known(ConditionValue::text(storage.to_string()))
                }
                Attested::Unknown => Attested::Unknown,
            },
            ..Floor::nothing_known()
        },
    )
}

/// Whether this run is against the artifact D24's ceilings are about.
fn is_release() -> bool {
    BuildIdentity::current().profile == "release"
}

/// Reports a figure, and asserts it only where D27 permits.
fn judge<Q: Quantity>(budget: &Budget<Q>, verdict: Verdict, reading: &str) {
    println!("  {:<40} {reading} — {verdict}", budget.name);
    match verdict {
        Verdict::Over => {
            assert!(
                !is_release(),
                "{} is over its ceiling of {} (D24, B20): {reading}",
                budget.name,
                budget.ceiling
            );
            println!(
                "    not a failure: this is a {} build, and D24's ceilings are for the \
                 release artifact",
                BuildIdentity::current().profile
            );
        }
        // Each of these is its own outcome and none of them is a pass. Printing
        // them is the point: a tier that reported only failures would let a
        // figure quietly stop being measured (A2, applied to the suite).
        Verdict::Within
        | Verdict::NotMeasured
        | Verdict::Unattributable
        | Verdict::TooFewTrials { .. } => {}
    }
}

/// D24's installed-footprint figure for the core binary. State-class: read at
/// the maximum, which for one file is its size.
#[test]
#[ignore = "the budget tier is scheduled, not gating (B38): scripts/ci.sh --with-budget"]
fn the_core_binary_is_within_its_footprint() {
    let measured = self_cost::artifact_bytes(&binary());
    judge(
        &CORE_BINARY,
        CORE_BINARY.read(measured),
        &match measured {
            Attested::Known(size) => size.to_string(),
            Attested::Unknown => "unknown".to_owned(),
        },
    );
    if let Attested::Known(Bytes(size)) = measured {
        // Two per cent. The build is reproducible byte for byte (B-001), so a
        // file's size does not move on its own: what this tolerates is the
        // compiler making a different inlining decision about the same code,
        // and what it catches is a dependency or a feature arriving unnoticed.
        // Growing it deliberately means recording a new baseline deliberately.
        against_baseline(
            CORE_BINARY.name,
            i64::try_from(size).unwrap_or(i64::MAX),
            "B",
            &conditions(&Machine::read_through(&[]), &Attributability::Unknown),
            &Judgement::Tolerating(20),
        );
    }
}

/// D24's resident-memory figure. Measured from inside the process MCF actually
/// runs, through the surface a user runs — `mcf doctor` reports its own
/// resident set, so this reads the product rather than a stand-in for it.
#[test]
#[ignore = "the budget tier is scheduled, not gating (B38): scripts/ci.sh --with-budget"]
fn resident_memory_is_within_its_ceiling() {
    let machine = Machine::read();
    let watch = Watch::start();
    let Some(resident) = reported_resident(&binary()) else {
        judge(&RESIDENT_IDLE, Verdict::NotMeasured, "unknown");
        return;
    };
    // Two readings so the figure is a measurement rather than an anecdote
    // (§3.4). Resident memory of a fresh process is near-deterministic, which
    // is why two suffice for a maximum where a percentile would need a hundred.
    let second = reported_resident(&binary()).unwrap_or(resident);
    let attributable = watch.finish();
    let measured =
        Measurement::from_samples([resident, second], conditions(&machine, &attributable))
            .expect("two readings");
    judge(
        &RESIDENT_IDLE,
        RESIDENT_IDLE.read_measurement(&measured, &attributable),
        &measured.maximum().to_string(),
    );
    // Ten per cent. A fresh process's resident set is nearly deterministic —
    // the two readings above differ by kilobytes — but it is decided by an
    // allocator whose policy is not MCF's, so the tolerance is what separates
    // "the allocator did something different" from "MCF now holds more".
    against_baseline(
        RESIDENT_IDLE.name,
        i64::try_from(measured.maximum().0).unwrap_or(i64::MAX),
        "B",
        measured.conditions(),
        &Judgement::Tolerating(100),
    );
}

/// D24's cold-start figure. Event-class: a hundred trials, read at the 99th
/// percentile, and only on a machine quiet enough to attribute the reading to
/// MCF rather than to whatever else was running (D27).
#[test]
#[ignore = "the budget tier is scheduled, not gating (B38): scripts/ci.sh --with-budget"]
fn cold_start_is_within_its_ceiling() {
    let machine = Machine::read();
    let watch = Watch::start();
    let conditions = conditions(&machine, &Attributability::Unknown);
    let measured = self_cost::cold_start(&binary(), &["--version"], EVENT_TRIALS, conditions);
    let attributable = watch.finish();
    let Some(measured) = measured else {
        judge(&COLD_START, Verdict::NotMeasured, "did not run");
        return;
    };
    let spread = measured.spread();
    judge(
        &COLD_START,
        COLD_START.read_measurement(&measured, &attributable),
        &format!(
            "p99 {} (median {}, n={})",
            COLD_START.statistic(&measured),
            spread.median,
            measured.n()
        ),
    );
    against_baseline(
        COLD_START.name,
        i64::try_from(COLD_START.statistic(&measured).as_nanos()).unwrap_or(i64::MAX),
        "ns",
        measured.conditions(),
        &Judgement::NotJudged(
            "a p99 over a hundred trials moves by a quarter between runs on an idle \
             machine, so a tolerance tight enough to catch a regression would fire on \
             the tail itself. The ceiling judges this figure; the baseline records it. \
             What B-193 fixed is the other half — a reading that went to a device is now \
             refused as unattributable, and one taken from different storage is refused \
             as incomparable",
        ),
    );
}

/// D24's added-latency figure, as far as it can honestly be read today
/// (B-035).
///
/// **What is measured.** A real `mcf serve` process on a machine of its own,
/// asked a hundred questions over its control socket from outside — which is
/// what an operator's client does, across a process boundary, through the same
/// parser and the same writer.
///
/// **What is not, and why that is said rather than assumed.** There is no
/// engine (B-320), so the half of D24's figure that waits for a first token
/// does not exist. `mcf_serve::cost` names every omission and this prints them
/// beside the reading: a number compared against a ceiling without them would
/// be claiming to be the whole of what D24 named (A21, §3.4). When B-032 gives
/// the daemon something to dispatch to, the rest of the figure arrives here.
#[test]
#[ignore = "the budget tier is scheduled, not gating (B38): scripts/ci.sh --with-budget"]
fn the_latency_mcf_interposes_is_within_its_ceiling() {
    let machine = Machine::read();
    let quarters = Quarters::new("interposed");
    let Some(mut daemon) = quarters.serve() else {
        judge(&ADDED_LATENCY, Verdict::NotMeasured, "no daemon started");
        return;
    };

    let watch = Watch::start();
    let conditions = conditions(&machine, &Attributability::Unknown);
    let measured = mcf_serve::cost::interposed(&quarters.socket(), EVENT_TRIALS, conditions);
    let attributable = watch.finish();
    let stopped = quarters.stop();
    let _reaped = daemon.wait();

    let Some(measured) = measured else {
        judge(
            &ADDED_LATENCY,
            Verdict::NotMeasured,
            "the daemon did not answer",
        );
        assert!(stopped, "the daemon under measurement would not stop");
        return;
    };

    let spread = measured.round_trip.spread();
    judge(
        &ADDED_LATENCY,
        ADDED_LATENCY.read_measurement(&measured.round_trip, &attributable),
        &format!(
            "p99 {} (median {}, n={})",
            ADDED_LATENCY.statistic(&measured.round_trip),
            spread.median,
            measured.round_trip.n()
        ),
    );
    for missing in &measured.excludes {
        println!("    this reading excludes {missing}");
    }
    against_baseline(
        ADDED_LATENCY.name,
        i64::try_from(ADDED_LATENCY.statistic(&measured.round_trip).as_nanos()).unwrap_or(i64::MAX),
        "ns",
        measured.round_trip.conditions(),
        &Judgement::NotJudged(
            "the same reason the cold start is not judged against its baseline: a p99 over a \
             hundred trials moves with the tail rather than with the code. The ceiling judges \
             this figure; the baseline records it. It is also half a figure until there is an \
             engine (B-320), and a tolerance on half a figure would be a tolerance on which \
             half",
        ),
    );
    assert!(stopped, "the daemon under measurement would not stop");
}

/// A machine of its own for a daemon to run on, and the two things a
/// measurement needs from it: somewhere to listen, and a way to stop.
struct Quarters(PathBuf);

impl Quarters {
    fn new(name: &str) -> Self {
        let root = std::env::temp_dir().join(format!("mcf-budget-{name}-{}", std::process::id()));
        let _removed = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).expect("a place for the daemon to live");
        Self(root)
    }

    fn socket(&self) -> PathBuf {
        self.0.join("mcf").join("control.sock")
    }

    /// Starts the shipped binary as a daemon, and waits for it to be reachable.
    ///
    /// Waits rather than sleeps a fixed time: a cold start is its own D24
    /// figure and this measurement is not about it, so what is wanted is *the
    /// moment it is answering*, whenever that is.
    fn serve(&self) -> Option<std::process::Child> {
        let mut command = std::process::Command::new(binary());
        command.arg("serve");
        command.env("XDG_DATA_HOME", &self.0);
        command.env("XDG_RUNTIME_DIR", &self.0);
        command.env_remove("HOME");
        let child = command
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .ok()?;

        for _ in 0..200 {
            if std::os::unix::net::UnixStream::connect(self.socket()).is_ok() {
                return Some(child);
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        None
    }

    fn stop(&self) -> bool {
        use std::io::{BufRead as _, BufReader, Write as _};
        let Ok(mut connection) = std::os::unix::net::UnixStream::connect(self.socket()) else {
            return false;
        };
        let request = mcf_serve::control::Request::Stop {
            reason: "the budget tier is finished with it".to_owned(),
        };
        if writeln!(connection, "{}", request.to_line()).is_err() {
            return false;
        }
        let mut answer = String::new();
        BufReader::new(&connection).read_line(&mut answer).is_ok()
    }
}

impl Drop for Quarters {
    fn drop(&mut self) {
        let _removed = std::fs::remove_dir_all(&self.0);
    }
}

/// The other half of D24's added-latency figure: request to the engine's
/// first token, through the daemon, on the laboratory's fixture (B-035,
/// B-034).
///
/// **On the fixture, on purpose.** D24 budgets what MCF *interposes*, and on a
/// one-block model the engine's own share of a first token is microseconds —
/// so the figure is MCF's: accept, parse, resolve, load per request, tokenize,
/// one forward pass, write back. On a real model the same path is dominated by
/// the engine, and a reading taken there would be a reading of the engine.
/// `mcf_serve::cost::to_first_token` says what it includes; this prints it.
#[test]
#[ignore = "the budget tier is scheduled, not gating (B38): scripts/ci.sh --with-budget"]
fn the_latency_to_a_first_token_is_within_its_ceiling() {
    let machine = Machine::read();
    let quarters = Quarters::new("first-token");
    let store = quarters
        .0
        .join("mcf")
        .join("models")
        .join("lab")
        .join("fixture");
    std::fs::create_dir_all(&store).expect("a store for the fixture");
    let model = store.join("a-model-that-runs.gguf");
    std::fs::write(&model, mcf_lab::fixture::a_model_that_runs()).expect("the fixture written");

    let Some(mut daemon) = quarters.serve() else {
        judge(&ADDED_LATENCY, Verdict::NotMeasured, "no daemon started");
        return;
    };

    let watch = Watch::start();
    let conditions = conditions(&machine, &Attributability::Unknown);
    let measured =
        mcf_serve::cost::to_first_token(&quarters.socket(), &model, EVENT_TRIALS, conditions);
    let attributable = watch.finish();
    let stopped = quarters.stop();
    let _reaped = daemon.wait();

    let Some(measured) = measured else {
        judge(
            &ADDED_LATENCY,
            Verdict::NotMeasured,
            "the daemon produced no first token",
        );
        assert!(stopped, "the daemon under measurement would not stop");
        return;
    };

    let spread = measured.round_trip.spread();
    judge(
        &ADDED_LATENCY,
        ADDED_LATENCY.read_measurement(&measured.round_trip, &attributable),
        &format!(
            "to a first token: p99 {} (median {}, n={})",
            ADDED_LATENCY.statistic(&measured.round_trip),
            spread.median,
            measured.round_trip.n()
        ),
    );
    for missing in &measured.excludes {
        println!("    this reading excludes {missing}");
    }
    against_baseline(
        "added_latency_to_first_token",
        i64::try_from(ADDED_LATENCY.statistic(&measured.round_trip).as_nanos()).unwrap_or(i64::MAX),
        "ns",
        measured.round_trip.conditions(),
        &Judgement::NotJudged(
            "as for the round trip: a p99 over a hundred trials moves with the tail rather \
             than with the code, so the ceiling judges and the baseline records",
        ),
    );
    assert!(stopped, "the daemon under measurement would not stop");
}

/// The tier says what it measured under, always. B20 requires a performance
/// change carry a before-and-after *under stated conditions*, and conditions
/// that are only in the record are conditions nobody reads while looking at the
/// number.
#[test]
#[ignore = "the budget tier is scheduled, not gating (B38): scripts/ci.sh --with-budget"]
fn the_tier_states_its_conditions() {
    let machine = Machine::read();
    let watch = Watch::start();
    // A moment of ordinary work, so the verdict is about something.
    let mut total = 0_u64;
    for value in 0..200_000_u64 {
        total = total.wrapping_add(value);
    }
    assert!(total > 0);
    let attributable = watch.finish();
    println!("\nbudget tier conditions");
    println!("  {}", BuildIdentity::current());
    println!("  {attributable}");
    println!("  {machine}");
    if !is_release() {
        println!(
            "\n  NOTHING IS ASSERTED in this profile. D24's ceilings are for the release\n\
             \x20 artifact, and a debug binary is a different one (§3.4)."
        );
    }
    if !attributable.permits_assertion() {
        println!(
            "\n  NOTHING IS ASSERTED on this machine right now: a timing taken under\n\
             \x20 contention measures the contention (B35), so this run is unattributable\n\
             \x20 rather than passing or failing (B24, D27). It does not refresh the\n\
             \x20 tier's age (B38)."
        );
    }
    assert!(!machine.to_string().is_empty());
}

/// Runs `mcf doctor --json --no-record` and reads back the resident figure it
/// reports about itself.
///
/// Reading the product's own report rather than measuring from outside is what
/// makes this the figure D24 is about: the resident set of a process at the
/// moment it has finished starting, which nothing outside the process can
/// observe without racing it.
fn reported_resident(binary: &Path) -> Option<Bytes> {
    let output = std::process::Command::new(binary)
        .args(["doctor", "--json", "--no-record"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8(output.stdout).ok()?;
    let value = mcf_record::json::parse(&text).ok()?;
    let bytes = value
        .get("self_cost")?
        .get("resident_bytes")?
        .as_integer()?;
    u64::try_from(bytes).ok().map(Bytes)
}

/// Where the previous readings live: beside the tier ages (B-185), one file per
/// figure so that four tests running at once cannot tear each other's writes.
///
/// `baselines/` rather than `performance/`, because `.mcf-tiers/performance` is
/// the performance tier's *stamp* — a file — and a directory of the same name
/// made `scripts/ci.sh --all` fail at the moment it went to record the tier's
/// age. Two things sharing a namespace is a collision waiting for the first run
/// that uses both, and the first run that used both was the one that found it.
fn baseline_directory() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join(".mcf-tiers")
        .join("baselines")
}

fn baseline_path(figure: &str) -> PathBuf {
    let slug: String = figure
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect();
    baseline_directory().join(format!("{slug}.json"))
}

/// One recorded reading: the number, what it is a number of, and what it was
/// taken under.
struct Reading {
    value: i64,
    unit: String,
    profile: String,
    conditions: String,
    storage: String,
}

fn read_baseline(figure: &str) -> Option<Reading> {
    let text = std::fs::read_to_string(baseline_path(figure)).ok()?;
    let value = mcf_record::json::parse(text.trim()).ok()?;
    Some(Reading {
        value: value.get("value")?.as_integer()?,
        unit: value.get("unit")?.as_text()?.to_owned(),
        profile: value.get("profile")?.as_text()?.to_owned(),
        conditions: value.get("conditions")?.as_text()?.to_owned(),
        storage: value
            .get("storage")
            .and_then(mcf_record::json::Value::as_text)
            .unwrap_or("unknown")
            .to_owned(),
    })
}

fn write_baseline(figure: &str, reading: &Reading) {
    let directory = baseline_directory();
    if std::fs::create_dir_all(&directory).is_err() {
        println!("    the baseline could not be written, so the next run has nothing to compare");
        return;
    }
    let line = mcf_record::json::Value::map([
        ("figure", mcf_record::json::Value::text(figure)),
        ("value", mcf_record::json::Value::Integer(reading.value)),
        ("unit", mcf_record::json::Value::text(&reading.unit)),
        ("profile", mcf_record::json::Value::text(&reading.profile)),
        (
            "conditions",
            mcf_record::json::Value::text(&reading.conditions),
        ),
        ("storage", mcf_record::json::Value::text(&reading.storage)),
    ])
    .to_line();
    if std::fs::write(baseline_path(figure), line).is_err() {
        println!("    the baseline could not be written, so the next run has nothing to compare");
    }
}

/// Whether a figure's change from its baseline is something this tier will
/// assert on.
enum Judgement {
    /// Judged, and this many parts per thousand of growth is tolerated.
    Tolerating(i64),
    /// Not judged, and this is why. Printed with the change so a reader sees
    /// both the number and the reason nobody is acting on it.
    NotJudged(&'static str),
}

/// Compares a figure with the last one recorded, and records this one.
///
/// The comparison is refused rather than made wrong when the two are not
/// comparable (A8): a different profile is a different artifact, and a debug
/// run has no business overwriting a release baseline.
fn against_baseline(
    figure: &str,
    value: i64,
    unit: &str,
    conditions: &Conditions,
    judgement: &Judgement,
) {
    let profile = BuildIdentity::current().profile.to_owned();
    let current = Reading {
        value,
        unit: unit.to_owned(),
        profile: profile.clone(),
        conditions: conditions.to_string(),
        storage: conditions.floor().artifact_storage.to_string(),
    };

    match read_baseline(figure) {
        None => println!("    no baseline yet; this run records one"),
        Some(previous) if previous.profile != profile => {
            println!(
                "    the baseline was taken in the {} profile and this is {profile}; \
                 not comparable (A8)",
                previous.profile
            );
        }
        Some(previous) if previous.unit != unit => {
            println!(
                "    the baseline is in {} and this is in {unit}; not comparable (A8)",
                previous.unit
            );
        }
        Some(previous) if previous.storage != current.storage => {
            // B-193, from F5: the storage an artifact is read from moves a
            // cold start by three orders of magnitude. Two readings that do not
            // share it are two measurements of different things, and A8 refuses
            // a comparison where more than one thing differed.
            println!(
                "    the baseline was taken from {} and this from {}; not comparable (A8, B-193)",
                previous.storage, current.storage
            );
        }
        Some(previous) => {
            let change = value - previous.value;
            // Per thousand rather than per cent, and computed with
            // `checked_div` because the workspace denies integer division: a
            // silently truncated quotient is a wrong number wherever it
            // appears, including in a test's report.
            let per_thousand = change
                .saturating_mul(1_000)
                .checked_div(previous.value)
                .unwrap_or(0);
            println!(
                "    before {} {}, after {value} {unit} ({}{}.{} %)",
                previous.value,
                previous.unit,
                if change < 0 { "-" } else { "+" },
                per_thousand.abs().checked_div(10).unwrap_or(0),
                per_thousand.abs() % 10
            );
            match judgement {
                Judgement::NotJudged(why) => println!("    not judged: {why}"),
                Judgement::Tolerating(tolerance) => {
                    let regressed = per_thousand > *tolerance;
                    assert!(
                        !(regressed && is_release()),
                        "{}",
                        format!(
                            "{figure} regressed: {} {} → {value} {unit}, \
                             which is {}.{} % against a tolerance of {}.{} % (B20, B-011)\n  \
                             before, under: {}\n  after, under:  {}",
                            previous.value,
                            previous.unit,
                            per_thousand.checked_div(10).unwrap_or(0),
                            per_thousand % 10,
                            tolerance.checked_div(10).unwrap_or(0),
                            tolerance % 10,
                            previous.conditions,
                            current.conditions
                        )
                    );
                    if regressed {
                        println!(
                            "    over the tolerance, and not asserted: this is a {profile} build"
                        );
                    }
                }
            }
        }
    }

    // Recorded after the comparison, so a run that fails leaves the baseline
    // it failed against rather than quietly adopting the worse number.
    write_baseline(figure, &current);
}

/// A regression is reported with a before and an after, or it is not reported.
///
/// B20: *a performance change without a before-and-after under stated
/// conditions is not a performance change; it is a guess that also increased
/// complexity.* The baseline above is the before; this asserts the other half,
/// which is that every figure carries what a comparison needs — a `Measurement`
/// with its conditions, so that two readings can be known to be comparable at
/// all (A8) rather than merely subtractable.
#[test]
#[ignore = "the budget tier is scheduled, not gating (B38): scripts/ci.sh --with-budget"]
fn every_figure_carries_what_a_comparison_would_need() {
    let machine = Machine::read();
    let conditions = conditions(&machine, &Attributability::Unknown);
    let measured: Measurement<Duration<Monotonic>> =
        self_cost::cold_start(&binary(), &["--version"], 2, conditions).expect("two trials ran");
    // The conditions travel with it, which is what a later comparison needs in
    // order to know whether the two are comparable at all (A8).
    let rendered = measured.conditions().to_string();
    assert!(rendered.contains("hardware_state="), "{rendered}");
    assert!(
        rendered.contains(BuildIdentity::current().profile),
        "{rendered}"
    );
    assert!(measured.samples().count() >= 2);
}
