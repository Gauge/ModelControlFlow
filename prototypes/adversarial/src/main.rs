//! The §7.19 adversarial prototype (B-002).
//!
//! D4 chose Rust and then declined to assume it was right: *"§3.13 requires
//! optimizing what is measured rather than what is imagined, so this decision
//! is confirmed by a small adversarial prototype: probe an accelerator,
//! supervise a child runtime deliberately made to die badly, record both under
//! §3.1, and measure the result against §7.16. If that goes badly, this entry
//! is amended rather than defended."*
//!
//! This is that prototype. It is not MCF: nothing here is on a path any user
//! reaches, and every part of it is superseded by a real item — B-013 for the
//! profiler, B-033 for supervision, B-011 for the budget suite. It exists to
//! produce evidence for DEC-019, and for DEC-008 and DEC-004 alongside it.
//!
//! It prints a report and exits zero whatever it finds. A machine with no
//! accelerator is a valid subject: the interesting question is whether MCF
//! says so correctly, and B19 requires the answer be obtainable on a laptop.

mod accelerator;
mod footprint;
mod nvml;
mod supervise;

use std::process::ExitCode;

use mcf_core::attested::Attested;
use mcf_core::failure::Failure;
use mcf_core::measurement::{Bytes, Quantity};
use mcf_core::time::{Duration, Monotonic};

use accelerator::Probe;

fn main() -> ExitCode {
    println!("{}", mcf_core::build_identity::BuildIdentity::current());
    println!(
        "\nThe §7.19 adversarial prototype (B-002). Evidence, not a product.\n\
         Every figure below is a reading taken on this machine, now.\n"
    );

    report_accelerator();
    report_supervision();
    report_footprint();

    println!(
        "\nWhat this run establishes is written up in D4 and in DEC-019;\n\
         nothing here is a measurement of a model, and nothing here is shipped."
    );
    ExitCode::SUCCESS
}

fn report_accelerator() {
    println!("ACCELERATOR — two routes to the same device");
    for probe in [accelerator::by_file(), accelerator::by_vendor_library()] {
        match probe {
            Probe::Found(reading) => {
                println!(
                    "\n  route {:<16} answered {} of 5",
                    reading.route,
                    reading.answered()
                );
                for (question, value) in reading.entries() {
                    println!("    {question:<14} {value}");
                }
            }
            Probe::Nothing(failure) => {
                println!("\n  nothing found");
                render_failure(&failure);
            }
        }
    }
}

fn report_supervision() {
    println!("\nSUPERVISION — four ways a runtime dies badly");
    let deadline = Duration::<Monotonic>::from_nanos(2_000_000_000);
    let scenarios: [(&str, &str, Vec<&str>); 4] = [
        (
            "binary absent at spawn",
            "/nonexistent/mcf-prototype-no-such-runtime",
            vec![],
        ),
        ("exits before any output", "/bin/sh", vec!["-c", "exit 3"]),
        (
            "partial output, then killed",
            "/bin/sh",
            vec!["-c", "printf 'two of five chunks'; kill -9 $$"],
        ),
        (
            "alive and silent past the deadline",
            "/bin/sh",
            vec!["-c", "sleep 30"],
        ),
    ];

    for (name, program, arguments) in scenarios {
        let outcome = supervise::supervise(program, &arguments, deadline);
        println!("\n  {name}");
        println!(
            "    waited         {} ({})",
            outcome.waited,
            if outcome.waited <= deadline {
                "within the deadline"
            } else {
                "past the deadline"
            }
        );
        println!(
            "    partial output {}",
            if outcome.preserved_partial_output() {
                format!(
                    "{} bytes preserved: {:?}",
                    outcome.output.len(),
                    outcome.output
                )
            } else {
                "none produced".to_owned()
            }
        );
        match &outcome.failure {
            Some(failure) => render_failure(failure),
            None => println!("    outcome        the runtime completed"),
        }
    }
    println!("\n  the supervisor is still running, which is A3's whole claim");
}

fn report_footprint() {
    println!("\nCOST — what this prototype costs on this machine");

    let conditions = footprint::conditions("the adversarial prototype, release profile");
    let binary = std::env::current_exe().unwrap_or_default();

    match footprint::artifact_bytes(&binary) {
        Attested::Known(size) => println!(
            "    artifact       {size}  (D24 budgets the core binary at ≤ 40 MiB: {})",
            footprint::against_budget(size, Bytes(40 * 1024 * 1024))
        ),
        Attested::Unknown => println!("    artifact       unknown"),
    }

    match footprint::resident_bytes() {
        Attested::Known(rss) => println!(
            "    resident       {rss}  (D24 budgets ≤ 20 MiB with nothing loaded: {})",
            footprint::against_budget(rss, Bytes(20 * 1024 * 1024))
        ),
        Attested::Unknown => println!("    resident       unknown on this platform"),
    }

    // The subject is `mcf`, not this prototype. D24's figure is *cold start to
    // first command response*, and `mcf --version` is the shortest complete
    // one MCF has. Measuring this binary instead would have measured this
    // binary re-running the whole report — which is what the first draft did,
    // and it did not terminate. Recorded because it is the kind of mistake a
    // prototype exists to make before a suite makes it (B18).
    let subject = binary.with_file_name("mcf");
    match footprint::cold_start(&subject, &["--version"], 20, conditions) {
        Some(measurement) => {
            const BUDGET: Duration<Monotonic> = Duration::from_nanos(100_000_000);
            let spread = measurement.spread();
            // Both statistics, because D24 does not say which one the cold-start
            // budget is about — and on a machine that is not quiet they
            // disagree by more than an order of magnitude. That disagreement is
            // a finding about the budget, not a number to pick from.
            println!(
                "    cold start     median {} over n={} — {}",
                spread.median,
                measurement.n(),
                footprint::against_budget(spread.median, BUDGET)
            );
            println!(
                "                   p95    {} — {}",
                spread.p95,
                footprint::against_budget(spread.p95, BUDGET)
            );
            println!("                   spread {}", render_spread(&measurement));
            println!(
                "                   D24 states no statistic for this budget, and these\n\
                 \x20                  two disagree. Recorded as void §7.50 / DEC-050, not\n\
                 \x20                  resolved here. Nor was this machine quiet: B35\n\
                 \x20                  makes a timing taken under contention a\n\
                 \x20                  measurement of the contention."
            );
        }
        None => println!(
            "    cold start     not measurable: {} did not run twice",
            subject.display()
        ),
    }

    println!(
        "    idle CPU       not measurable at this stage — D24's figure is for a\n\
         \x20                  daemon, and there is no daemon until M2 (B-030)"
    );
    println!(
        "    added latency  not measurable at this stage — there is no serving\n\
         \x20                  path until M2 (B-035)"
    );
}

fn render_spread(measurement: &mcf_core::measurement::Measurement<Duration<Monotonic>>) -> String {
    let spread = measurement.spread();
    format!(
        "min {} · p5 {} · p95 {} · max {} ({})",
        spread.minimum.as_nanos(),
        spread.p5.as_nanos(),
        spread.p95.as_nanos(),
        spread.maximum.as_nanos(),
        <Duration<Monotonic> as Quantity>::UNIT,
    )
}

fn render_failure(failure: &Failure) {
    println!("    category       {}", failure.category());
    println!("    meaning        {}", failure.category().meaning());
    println!("    attribution    {}", failure.attribution());
    println!("    disposition    {}", failure.disposition());
    println!("    subsystem      {}", failure.subsystem());
    println!("    detail         {}", failure.detail());
    for entry in failure.context() {
        println!("    · {:<12} {}", entry.key, entry.value);
    }
}
