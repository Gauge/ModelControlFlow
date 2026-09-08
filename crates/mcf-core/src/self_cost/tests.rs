use super::{
    Budget, COLD_START, CORE_BINARY, EVENT_TRIALS, Kind, RECORD_WRITE, RESIDENT_IDLE, Verdict,
    artifact_bytes, cold_start, resident_bytes,
};
use crate::attested::Attested;
use crate::build_identity::BuildIdentity;
use crate::hardware::Attributability;
use crate::measurement::{Bytes, Conditions, Floor, Measurement};

fn conditions() -> Conditions {
    Conditions::new(BuildIdentity::current(), Floor::nothing_known())
}

#[test]
fn the_resident_reading_is_of_a_running_process() {
    if let Attested::Known(rss) = resident_bytes() {
        assert!(rss > Bytes(0), "a running process reports no memory");
        assert!(
            rss < Bytes(64 * 1024 * 1024 * 1024),
            "a test binary reports {rss}, which is not plausible"
        );
    }
}

#[test]
fn the_resident_reading_is_in_bytes_and_lands_on_a_page_boundary() {
    let Attested::Known(rss) = resident_bytes() else {
        println!("  resident memory is not readable here, so this is not checked");
        return;
    };
    let Some(page) = page_size() else {
        println!("  the page size is not readable here, so this is not checked");
        return;
    };

    assert_eq!(
        rss.0 % page,
        0,
        "{rss} is not a whole number of {page}-byte pages, so the reading is not in bytes"
    );

    for attempt in 0..8 {
        let (Some(before), Attested::Known(bytes), Some(after)) =
            (resident_pages(), resident_bytes(), resident_pages())
        else {
            println!("  the second route is not readable here, so it is not compared");
            return;
        };
        let (smaller, larger) = (
            before.saturating_mul(page).min(bytes.0),
            before.saturating_mul(page).max(bytes.0),
        );
        assert!(
            larger <= smaller.saturating_mul(2),
            "the two routes are not describing the same quantity on attempt {attempt}: \
             {before} pages by statm is {} B, against {bytes} by status — a disagreement that \
             size is a wrong multiplier rather than a moving resident set",
            before.saturating_mul(page)
        );
        if before != after || before.saturating_mul(page) != bytes.0 {
            continue;
        }
        return;
    }
    println!(
        "  the resident set never held still across two reads, so the routes are not compared"
    );
}

fn page_size() -> Option<u64> {
    let output = std::process::Command::new("getconf")
        .arg("PAGESIZE")
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    String::from_utf8(output.stdout).ok()?.trim().parse().ok()
}

fn resident_pages() -> Option<u64> {
    let statm = std::fs::read_to_string("/proc/self/statm").ok()?;
    statm.split_whitespace().nth(1)?.parse().ok()
}

#[test]
fn an_absent_artifact_is_unknown_and_not_zero() {
    let absent = artifact_bytes(std::path::Path::new("/nonexistent/mcf-not-here"));
    assert_eq!(absent, Attested::Unknown);
    assert_eq!(CORE_BINARY.read(absent), Verdict::NotMeasured);
}

#[test]
fn not_measured_is_not_within() {
    assert_eq!(RESIDENT_IDLE.read(Attested::Unknown), Verdict::NotMeasured);
    assert_ne!(RESIDENT_IDLE.read(Attested::Unknown), Verdict::Within);
    assert_eq!(Verdict::NotMeasured.to_string(), "not measured");
}

#[test]
fn the_ceiling_is_inclusive_and_one_past_it_is_over() {
    let budget = Budget {
        name: "a ceiling",
        ceiling: Bytes(100),
        kind: Kind::CeilingOnState,
    };
    assert_eq!(budget.read(Attested::Known(Bytes(99))), Verdict::Within);
    assert_eq!(budget.read(Attested::Known(Bytes(100))), Verdict::Within);
    assert_eq!(budget.read(Attested::Known(Bytes(101))), Verdict::Over);
}

#[test]
fn the_budgets_are_the_figures_d24_states() {
    assert_eq!(RESIDENT_IDLE.ceiling, Bytes(20 * 1024 * 1024));
    assert_eq!(CORE_BINARY.ceiling, Bytes(40 * 1024 * 1024));
    assert_eq!(COLD_START.ceiling.as_nanos(), 100_000_000);
    assert_eq!(RECORD_WRITE.ceiling.as_nanos(), 2_000_000);
    for budget in [RESIDENT_IDLE, CORE_BINARY] {
        assert_eq!(budget.kind, Kind::CeilingOnState, "{}", budget.name);
    }
    for budget in [COLD_START, RECORD_WRITE] {
        assert_eq!(budget.kind, Kind::CeilingOnEvent, "{}", budget.name);
    }
}

#[test]
fn a_command_that_cannot_run_yields_no_measurement() {
    let nothing = cold_start(
        std::path::Path::new("/nonexistent/mcf-not-here"),
        &[],
        5,
        conditions(),
    );
    assert!(nothing.is_none());
}

#[test]
fn a_command_that_runs_yields_a_measurement_with_its_trials() {
    let program = std::path::Path::new("/bin/true");
    if !program.exists() {
        return;
    }
    let measured = cold_start(program, &[], 3, conditions()).expect("three trials ran");
    assert_eq!(measured.n(), 3);
    assert!(measured.spread().minimum <= measured.spread().median);
    assert!(!measured.spread().median.is_simulated());
}

fn quiet() -> Attributability {
    Attributability::Attributable { delay_ppm: 189 }
}

fn busy() -> Attributability {
    Attributability::Unattributable {
        delay_ppm: 111_248,
        tolerated_ppm: 10_000,
    }
}

fn samples(values: impl IntoIterator<Item = u64>) -> Measurement<Bytes> {
    Measurement::from_samples(values.into_iter().map(Bytes), conditions())
        .expect("at least two samples")
}

#[test]
fn a_busy_machine_makes_a_run_neither_a_pass_nor_a_failure() {
    let over = samples([1_000, 999_999_999]);
    assert_eq!(
        RESIDENT_IDLE.read_measurement(&over, &busy()),
        Verdict::Unattributable
    );
    assert_ne!(
        RESIDENT_IDLE.read_measurement(&over, &busy()),
        Verdict::Within
    );
    assert_ne!(
        RESIDENT_IDLE.read_measurement(&over, &busy()),
        Verdict::Over
    );
}

#[test]
fn unknown_attributability_is_not_permission() {
    let within = samples([1_000, 2_000]);
    assert_eq!(
        RESIDENT_IDLE.read_measurement(&within, &Attributability::Unknown),
        Verdict::Unattributable
    );
    assert!(!Attributability::Unknown.permits_assertion());
    assert!(quiet().permits_assertion());
}

#[test]
fn an_event_class_figure_refuses_too_few_trials() {
    let twenty = Measurement::from_samples(
        (0..20).map(|i| crate::time::Duration::from_nanos(i * 1000)),
        conditions(),
    )
    .expect("twenty samples");
    assert_eq!(
        COLD_START.read_measurement(&twenty, &quiet()),
        Verdict::TooFewTrials {
            had: 20,
            needs: EVENT_TRIALS
        }
    );
}

#[test]
fn a_state_class_figure_is_read_at_the_maximum() {
    let mut values: Vec<u64> = (0..99).map(|_| 1_000).collect();
    values.push(999_999_999);
    let measured = samples(values);
    assert_eq!(
        RESIDENT_IDLE.statistic(&measured).value(),
        Bytes(999_999_999)
    );
    assert_eq!(
        RESIDENT_IDLE.read_measurement(&measured, &quiet()),
        Verdict::Over
    );
}

#[test]
fn an_event_class_figure_is_read_at_the_ninety_ninth_percentile() {
    use crate::time::Duration;
    let mut values: Vec<Duration<crate::time::Monotonic>> =
        (0..99).map(|_| Duration::from_nanos(1_000_000)).collect();
    values.push(Duration::from_nanos(9_000_000_000));
    let measured = Measurement::from_samples(values, conditions()).expect("a hundred samples");
    assert_eq!(
        COLD_START.statistic(&measured).value().as_nanos(),
        1_000_000
    );
    assert_eq!(
        COLD_START.read_measurement(&measured, &quiet()),
        Verdict::Within
    );
}

#[test]
fn a_prohibition_admits_no_margin() {
    use crate::measurement::Count;
    let prohibition = Budget {
        name: "timer wakeups while idle",
        ceiling: Count(0),
        kind: Kind::Prohibition,
    };
    let none = Measurement::from_samples([Count(0), Count(0)], conditions()).expect("two");
    let one = Measurement::from_samples([Count(0), Count(1)], conditions()).expect("two");
    assert_eq!(
        prohibition.read_measurement(&none, &quiet()),
        Verdict::Within
    );
    assert_eq!(prohibition.read_measurement(&one, &quiet()), Verdict::Over);
}

#[test]
fn every_verdict_renders_distinctly() {
    let rendered = [
        Verdict::Within.to_string(),
        Verdict::Over.to_string(),
        Verdict::NotMeasured.to_string(),
        Verdict::Unattributable.to_string(),
        Verdict::TooFewTrials {
            had: 20,
            needs: 100,
        }
        .to_string(),
    ];
    for text in &rendered {
        assert!(!text.is_empty());
    }
    let mut unique: Vec<String> = rendered.to_vec();
    unique.sort();
    unique.dedup();
    assert_eq!(unique.len(), rendered.len(), "two verdicts read the same");
}
