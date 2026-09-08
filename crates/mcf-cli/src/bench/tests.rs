use super::{bench_where, held_the_pin, is_a_stand_in, per_cent_of};
use mcf_core::measurement::PartsPerMillion;
use mcf_record::json::Value;

const NOWHERE: &str = "/nonexistent/mcf-bench-test/model.gguf";

#[test]
fn without_a_daemon_it_refuses_rather_than_reporting() {
    let response = bench_where(
        None,
        NOWHERE,
        NOWHERE,
        "hello",
        None,
        0,
        None,
        None,
        false,
        None,
        mcf_serve::declared::Started::default(),
    );
    assert!(!response.served);
    assert!(
        response.text.contains("there is no model at"),
        "the first thing wrong is said first: {}",
        response.text
    );
}

#[test]
fn a_missing_daemon_is_named() {
    let scratch = std::env::temp_dir().join(format!("mcf-bench-{}.gguf", std::process::id()));
    let _written = std::fs::write(&scratch, b"not a model, and never read");
    let named = scratch.display().to_string();
    let response = bench_where(
        None,
        &named,
        &named,
        "hello",
        None,
        0,
        None,
        None,
        false,
        None,
        mcf_serve::declared::Started::default(),
    );
    let _removed = std::fs::remove_file(&scratch);
    assert!(!response.served);
    assert!(
        response.text.contains("none is listening") && response.text.contains("mcf serve"),
        "{}",
        response.text
    );
}

#[test]
fn a_stand_in_is_recognized_by_name() {
    for named in [
        "MCF's own stand-in, build 0.1.0-m0",
        "stand-in",
        "STAND-IN",
        "a stand in",
    ] {
        assert!(is_a_stand_in(named), "{named} is a stand-in");
    }
    for named in ["llama.cpp @ 925e1179947e", "provisioned", "vendored engine"] {
        assert!(!is_a_stand_in(named), "{named} is not");
    }
}

#[test]
fn a_resolution_is_read_without_a_float() {
    assert_eq!(per_cent_of("5"), Some(PartsPerMillion(50_000)));
    assert_eq!(per_cent_of("2.5"), Some(PartsPerMillion(25_000)));
    assert_eq!(per_cent_of("0.1"), Some(PartsPerMillion(1_000)));
    assert_eq!(per_cent_of("100"), Some(PartsPerMillion(1_000_000)));
    assert_eq!(per_cent_of("2.55"), None, "one decimal place is the unit");
    assert_eq!(
        per_cent_of("0"),
        None,
        "a zero difference is not a question"
    );
    assert_eq!(per_cent_of("five"), None);
}

#[test]
fn asking_for_a_cold_run_is_an_argument_and_not_a_default() {
    let source = include_str!("../bench.rs");
    assert!(
        source.contains("fn vocabularies(") && source.contains("    if cold {"),
        "the flag must decide whether identifiers are sent, which is what decides whether the \
         request reaches the server that holds a model (B-376, F65)"
    );
    assert!(
        source.contains("(None, None)"),
        "and a cold run sends text, which is a fresh process per request and therefore uniform"
    );
}

#[test]
fn a_mixed_run_says_what_to_do_about_it() {
    let source = include_str!("../bench.rs");
    for said in [
        "`--cold` makes every trial load the model",
        "comparing a model with itself is uniform too",
        "resident at a time and a paired comparison alternates them",
    ] {
        assert!(
            source.contains(said),
            "the advice after a mixed run must name a way forward: `{said}` is gone"
        );
    }
}

fn rendering_of_a_run_that_could_not_decide() -> String {
    use mcf_bench::compare::{Discipline, Interleaving, UnderTest};
    use mcf_bench::warmth::Warmth;
    use mcf_core::attested::Attested;
    use mcf_core::build_identity::BuildIdentity;
    use mcf_core::hardware::{Competitor, Snapshot};
    use mcf_core::measurement::{Conditions, Floor};
    use mcf_core::time::{Duration, Monotonic};
    use mcf_core::trial::{Arm, SessionId};

    let arm = |named: &str| {
        UnderTest::new(
            Arm::new(named),
            Conditions::new(BuildIdentity::current(), Floor::nothing_known()),
        )
    };
    let named = Arm::new("one");
    let mut running = Interleaving::<Monotonic>::new(
        arm("one"),
        arm("other"),
        SessionId::new("s"),
        3,
        Discipline::Timing {
            seed: 0,
            tokens: 32,
        },
    );
    let mut round = 0_u64;
    for _ in 0..2 {
        let _ran = running.round(|which, _drew| {
            round = round.saturating_add(1);
            let held = if *which == named { 1 } else { 9 };
            Some((
                Duration::from_nanos(held * 100_000_000 * round),
                Warmth::Cold,
            ))
        });
    }
    let held = running.finish();
    let finding = held.finding(PartsPerMillion(1_000));
    assert!(
        matches!(
            finding.verdict(),
            Some(mcf_bench::enough::Verdict::NotYet { .. })
        ),
        "the fixture must be undecidable, or this is testing something else: {finding}"
    );

    let snapshot = Snapshot {
        over_millis: 214,
        competitors: vec![Competitor {
            pid: 4242,
            command: "a burner somebody left running".to_owned(),
            cores_taken: 3_500,
            is_mcf: false,
        }],
        cores_taken: 41_000,
        processor_pressure: Attested::Known(880_000),
        memory_pressure: Attested::Unknown,
        storage_pressure: Attested::Known(1_200),
        load: Attested::Unknown,
        accelerator: Attested::Unknown,
    };
    super::report(
        &finding,
        &held,
        &Err("nowhere to write".to_owned()),
        &mcf_bench::compare::MachineHeld {
            before: 4_000,
            after: 41_000,
            steady_before: Some(10_000),
            steady_after: Some(300_000),
        },
        &super::Planned {
            work: mcf_bench::planned::Work {
                trials: 200,
                arms: 2,
                tokens: 128,
            },
            expected: "no expected duration: nothing has been measured".to_owned(),
            proposal: Some("the budget buys 6 paired trial(s) — EXCLUDED, and not silently: 14 paired trial(s)".to_owned()),
            refused: None,
        },
        Some(&snapshot),
        Some(&Err("nor the snapshot".to_owned())),
        mcf_serve::declared::Started::default(),
    )
}

#[test]
fn a_run_that_could_not_decide_renders_what_competed_with_it() {
    let said = rendering_of_a_run_that_could_not_decide();

    assert!(said.contains("what was competing"), "{said}");
    assert!(
        said.contains("EXCLUDED, and not silently"),
        "B-226, §3.1: what a budget excluded is on the page beside what it ran: {said}"
    );
    assert!(
        said.contains("400 generation(s)"),
        "B-224: the work is declared in countable units: {said}"
    );
    assert!(
        said.contains("the level moved") && said.contains("not a verdict"),
        "the machine's own movement is a condition and never a judgement (B-217, DEC-007): {said}"
    );
    assert!(said.contains("a burner somebody left running"), "{said}");
    assert!(
        said.contains("does not attribute that"),
        "B24: the indecision is not attributed to the arms: {said}"
    );
    assert!(
        said.contains("THE SNAPSHOT WAS NOT RECORDED"),
        "and a snapshot that could not be kept says so rather than reading as kept (A2): {said}"
    );
}

#[test]
fn a_run_that_decided_renders_no_snapshot() {
    let source = include_str!("../bench.rs");
    assert!(
        source.contains("let competing = matches!("),
        "the snapshot must be conditional on the verdict"
    );
    assert!(
        source.contains("if let Some(snapshot) = competing {"),
        "and the section conditional on there being one"
    );
}

#[test]
fn a_trial_that_did_not_produce_what_it_pinned_is_refused() {
    let account = |tokens: i64, stopped: &str, length: &str| {
        Value::map([
            ("tokens", Value::Integer(tokens)),
            ("stopped", Value::text(stopped)),
            ("conditions", Value::map([("length", Value::text(length))])),
        ])
    };
    assert_eq!(held_the_pin(&account(128, "limit", "exactly"), 128), Ok(()));
    let short =
        held_the_pin(&account(41, "stop_token", "exactly"), 128).expect_err("41 is not 128");
    assert!(short.contains("41 of the 128 tokens pinned"), "{short}");
    assert!(short.contains("stop_token"), "{short}");
    let uncounted = held_the_pin(&account(128, "limit", "exactly_but_uncounted"), 128)
        .expect_err("a chunk count is not a token count");
    assert!(uncounted.contains("cannot be proven"), "{uncounted}");
    let unsaid = held_the_pin(&Value::map::<&str>([]), 128).expect_err("no count at all");
    assert!(unsaid.contains("an unsaid number"), "{unsaid}");
}
