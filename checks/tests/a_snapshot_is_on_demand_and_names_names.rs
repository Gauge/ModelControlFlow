#![allow(clippy::panic)]

#[test]
fn there_is_no_monitor() {
    let source = code_only(&read("crates/mcf-core/src/hardware/contention.rs"));
    for forbidden in ["thread::spawn", "loop {", "interval", "every(", "spawn("] {
        assert!(
            !source.contains(forbidden),
            "`{forbidden}` in a contention snapshot is the first step to a monitor (B4, D5)"
        );
    }
    assert!(
        source.contains("pub fn sample() -> Snapshot"),
        "it must be a function somebody calls, not a thing that runs"
    );
}

#[test]
fn only_an_undecided_run_takes_one() {
    let source = code_only(&read("crates/mcf-cli/src/bench.rs"));
    assert!(
        source.contains("Some(mcf_bench::enough::Verdict::NotYet { .. })"),
        "the trigger must be a run that could not decide (B24, PR5)"
    );
    assert!(
        source.contains(".then(mcf_core::hardware::contention)"),
        "and nothing else may call for one from here"
    );
    assert_eq!(
        source.matches("hardware::contention").count(),
        1,
        "one place takes a snapshot; a second would be the beginning of a habit (B4)"
    );
}

#[test]
fn it_is_taken_after_the_run() {
    let source = read("crates/mcf-cli/src/bench.rs");
    let Some((before, _)) = source.split_once("let competing = matches!(") else {
        panic!("the snapshot is taken somewhere");
    };
    assert!(
        before.contains("let held = interleave("),
        "the run must be over before the machine is sampled (§3.8, B3)"
    );
}

#[test]
fn it_names_what_was_competing_including_mcf() {
    let source = code_only(&read("crates/mcf-core/src/hardware/contention.rs"));
    assert!(
        source.contains("pub command: String,"),
        "a competitor must be named, or the snapshot is a number (PR5)"
    );
    assert!(
        source.contains("pub is_mcf: bool,"),
        "and MCF's own process marked rather than hidden"
    );
    assert!(
        !source.contains("filter(|held| !held.is_mcf)"),
        "hiding MCF would hide the one process the reader can do something about"
    );
}

#[test]
fn an_unreadable_accelerator_is_not_reported_as_idle() {
    let source = code_only(&read("crates/mcf-core/src/hardware/contention.rs"));
    assert!(
        source.contains("pub accelerator: Attested<String>"),
        "per-process accelerator occupancy must be attested, because MCF may not be able to read \
         it (D25)"
    );
    assert!(
        source.contains("accelerator: Attested::Unknown"),
        "and unknown where it cannot — *unknown* is not *no contention* (A7)"
    );
}

#[test]
fn the_snapshot_is_written_down() {
    let source = code_only(&read("crates/mcf-cli/src/bench.rs"));
    assert!(
        source.contains("EntryKind::ContentionSnapshot"),
        "a finding printed and not written down does not survive the terminal (§3.1)"
    );
    let kinds = code_only(&read("crates/mcf-record/src/journal/entry.rs"));
    assert!(
        kinds.contains("ContentionSnapshot,") && kinds.contains(r#""contention_snapshot""#),
        "and the kind must exist with a name stable for life (C5)"
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
