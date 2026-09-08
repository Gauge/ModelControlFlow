#![allow(clippy::panic)]

#[test]
fn the_benchmark_runner_returns_what_it_produced() {
    let source = code_only(&read("crates/mcf-cli/src/bench.rs"));
    assert!(
        source.contains(") -> Comparison<Monotonic> {"),
        "the runner must return the comparison it built, not a `Result` that discards it on the \
         first failed trial (A4)"
    );
    assert!(
        !source.contains(") -> Result<Comparison<Monotonic>, String> {"),
        "an all-or-nothing return type on something that can partially succeed is A4's own \
         description of its violation"
    );
    assert!(
        source.contains("running.stopped_short("),
        "and what stopped it must be recorded rather than returned in place of the trials"
    );
}

#[test]
fn a_comparison_says_why_it_stopped_or_says_nothing() {
    let source = code_only(&read("crates/mcf-bench/src/compare.rs"));
    assert!(
        source.contains("cut_short: Option<String>,"),
        "a comparison must be able to say why it stopped early (A4, B-087)"
    );
    assert!(
        source.contains("pub fn stopped_short("),
        "and the runner must be able to record it"
    );
    for forbidden in ["cut_short: bool", "was_interrupted: bool"] {
        assert!(
            !source.contains(forbidden),
            "`{forbidden}` loses the reason, and a partial result without its reason is a smaller \
             number nobody can explain (A2)"
        );
    }
}

#[test]
fn the_interruption_is_written_down() {
    let source = code_only(&read("crates/mcf-bench/src/record.rs"));
    assert!(
        source.contains("\"cut_short\","),
        "the record must carry why a run stopped (A4, B-086)"
    );
    assert!(
        source.contains("map_or(Value::Null, Value::text)"),
        "and `null` must mean *it finished* rather than *nobody recorded it*"
    );
}

#[test]
fn a_generation_that_ended_early_keeps_its_tokens() {
    let source = code_only(&read("crates/mcf-cli/src/run.rs"));
    assert!(
        source.contains("token(s) were received from the daemon at"),
        "tokens that arrived before a stream ended must be kept and reported (A4)"
    );
    assert!(
        source.contains("produced"),
        "and counted, so a reader knows how much of the answer they have"
    );
}

fn code_only(source: &str) -> String {
    source
        .lines()
        .filter(|line| {
            let trimmed = line.trim_start();
            !trimmed.starts_with("//!") && !trimmed.starts_with("///") && !trimmed.starts_with("//")
        })
        .collect::<Vec<&str>>()
        .join("\n")
}

fn read(relative: &str) -> String {
    let path = mcf_checks::workspace::root().join(relative);
    std::fs::read_to_string(&path).unwrap_or_else(|error| {
        panic!("{} is readable: {error}", path.display());
    })
}

#[test]
fn the_record_keeps_the_verdict_the_operator_was_shown() {
    let source = code_only(&read("crates/mcf-cli/src/bench.rs"));
    assert!(
        source.contains("let body = record::comparison(held, finding, method, machine);"),
        "the record must be written from the finding the operator saw (A6)"
    );
    assert!(
        !source.contains("record::comparison(held, &held.finding(RESOLVING))"),
        "recomputing the finding here takes the default resolution rather than the one asked \
         about, which puts a different verdict in the record from the one on the screen"
    );
}
