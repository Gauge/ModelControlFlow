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

fn binary() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_mcf"))
}

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

fn is_release() -> bool {
    BuildIdentity::current().profile == "release"
}

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
        Verdict::Within
        | Verdict::NotMeasured
        | Verdict::Unattributable
        | Verdict::TooFewTrials { .. } => {}
    }
}

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
        against_baseline(
            CORE_BINARY.name,
            i64::try_from(size).unwrap_or(i64::MAX),
            "B",
            &conditions(&Machine::read_through(&[]), &Attributability::Unknown),
            &Judgement::Tolerating(20),
        );
    }
}

#[test]
#[ignore = "the budget tier is scheduled, not gating (B38): scripts/ci.sh --with-budget"]
fn resident_memory_is_within_its_ceiling() {
    let machine = Machine::read();
    let watch = Watch::start();
    let Some(resident) = reported_resident(&binary()) else {
        judge(&RESIDENT_IDLE, Verdict::NotMeasured, "unknown");
        return;
    };
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
    against_baseline(
        RESIDENT_IDLE.name,
        i64::try_from(measured.maximum().0).unwrap_or(i64::MAX),
        "B",
        measured.conditions(),
        &Judgement::Tolerating(100),
    );
}

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
        i64::try_from(COLD_START.statistic(&measured).value().as_nanos()).unwrap_or(i64::MAX),
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
        i64::try_from(
            ADDED_LATENCY
                .statistic(&measured.round_trip)
                .value()
                .as_nanos(),
        )
        .unwrap_or(i64::MAX),
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
    std::fs::write(&model, mcf_standin::fixture::a_model_that_runs()).expect("the fixture written");

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
        i64::try_from(
            ADDED_LATENCY
                .statistic(&measured.round_trip)
                .value()
                .as_nanos(),
        )
        .unwrap_or(i64::MAX),
        "ns",
        measured.round_trip.conditions(),
        &Judgement::NotJudged(
            "as for the round trip: a p99 over a hundred trials moves with the tail rather \
             than with the code, so the ceiling judges and the baseline records",
        ),
    );
    assert!(stopped, "the daemon under measurement would not stop");
}

#[test]
#[ignore = "the budget tier is scheduled, not gating (B38): scripts/ci.sh --with-budget"]
fn the_tier_states_its_conditions() {
    let machine = Machine::read();
    let watch = Watch::start();
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

enum Judgement {
    Tolerating(i64),
    NotJudged(&'static str),
}

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
            println!(
                "    the baseline was taken from {} and this from {}; not comparable (A8, B-193)",
                previous.storage, current.storage
            );
        }
        Some(previous) => {
            let change = value - previous.value;
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

    write_baseline(figure, &current);
}

#[test]
#[ignore = "the budget tier is scheduled, not gating (B38): scripts/ci.sh --with-budget"]
fn every_figure_carries_what_a_comparison_would_need() {
    let machine = Machine::read();
    let conditions = conditions(&machine, &Attributability::Unknown);
    let measured: Measurement<Duration<Monotonic>> =
        self_cost::cold_start(&binary(), &["--version"], 2, conditions).expect("two trials ran");
    let rendered = measured.conditions().to_string();
    assert!(rendered.contains("hardware_state="), "{rendered}");
    assert!(
        rendered.contains(BuildIdentity::current().profile),
        "{rendered}"
    );
    assert!(measured.samples().count() >= 2);
}
