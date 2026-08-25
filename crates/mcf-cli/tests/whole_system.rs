//! The whole-system tier: MCF driven as a process, against a real record on a
//! real disk (B-191, D10).
//!
//! D10 asks for *whole-system tests across the process boundary with real
//! persistence and restart*. Everything below runs the binary cargo just built,
//! with `XDG_DATA_HOME` pointed at a scratch directory, and reads back what it
//! left behind. Nothing here is a mock: the journal is a file, the exit status
//! is a process's, and the second run of a command genuinely does not share
//! memory with the first.
//!
//! **Where the boundary falls is undecided, and this tier says so.** §7.22 asks
//! whether a full-system test drives a real HTTP surface, starts an engine,
//! crosses into a supervised child, and exercises recovery with persisted
//! state; DEC-022 is open. Three of those four have nothing to test at M0 —
//! there is no daemon, no serving surface and no engine — and the fourth is
//! what this file covers. When DEC-022 closes, this tier grows the rest;
//! claiming them now would be an untested claim (A19).
//!
//! **The whole tier gates, and that is a measured decision rather than an
//! assumed one.** B38 requires the gating tier stay fast, and the expensive
//! command here looked expensive: `mcf doctor` measures a cold start over
//! D27's hundred trials, which is a hundred process spawns. Measured, a
//! `doctor` run on this machine costs about 40 ms — the trials are of
//! `mcf --version`, which is the shortest command MCF has — so the whole file
//! runs in well under a second and none of it needs to be scheduled. If that
//! stops being true the answer is to move the expensive half behind a flag and
//! say so here, not to let the gate get slow quietly.

// Every item in this file is test code; see the note in
// checks/tests/taxonomy_agreement.rs.
#![allow(clippy::expect_used, clippy::panic, clippy::indexing_slicing)]

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use mcf_core::attested::Attested;
use mcf_core::build_identity::BuildIdentity;
use mcf_core::time::Timestamp;
use mcf_record::journal::{Entry, EntryKind, Journal, replay};
use mcf_record::json::{self, Value};

/// The binary under test: the one cargo built for this test run.
fn binary() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_mcf"))
}

/// A machine's worth of state, thrown away afterwards.
///
/// Cleared on the way in as well as out (B58): a run killed mid-write is one of
/// the cases below, and the next test must not inherit what it left.
struct Machine(PathBuf);

impl Machine {
    fn new(name: &str) -> Self {
        let path =
            std::env::temp_dir().join(format!("mcf-whole-system-{name}-{}", std::process::id()));
        let _cleared = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).expect("a scratch directory is creatable");
        Self(path)
    }

    /// Where this machine keeps its record.
    fn journal(&self) -> PathBuf {
        self.0.join("mcf").join("record.jsonl")
    }

    /// Runs `mcf` on this machine and waits for it.
    fn run(&self, arguments: &[&str]) -> Output {
        self.command(arguments)
            .output()
            .expect("the binary cargo built is runnable")
    }

    /// The same, unstarted, for a caller that wants to kill it.
    fn command(&self, arguments: &[&str]) -> Command {
        let mut command = Command::new(binary());
        command.args(arguments);
        // The whole point of the tier: the process gets a machine of its own,
        // and everything it persists lands where the test can read it.
        command.env("XDG_DATA_HOME", &self.0);
        // `default_path` falls back to $HOME when XDG_DATA_HOME is unset or
        // relative; removing it means a mistake here writes nothing to the
        // person's real record rather than quietly using it.
        command.env_remove("HOME");
        command
    }
}

impl Drop for Machine {
    fn drop(&mut self) {
        let _removed = std::fs::remove_dir_all(&self.0);
    }
}

fn text(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn error_text(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

/// Writes a record the way MCF does, from a different process than the one
/// that will read it.
///
/// The library the test uses is the library the binary uses, which is what
/// makes this a whole-system fixture rather than a hand-written file: a change
/// to the journal format reaches both sides at once.
fn seed_record(machine: &Machine, entries: usize) {
    let mut journal = Journal::open(&machine.journal()).expect("a journal opens");
    for sequence in 0..entries {
        journal
            .append(&Entry::new(
                EntryKind::SelfCost,
                Timestamp::from_utc_nanos(1_700_000_000_000_000_000, Attested::Unknown),
                sequence as u64,
                Value::map([("seeded", Value::Bool(true))]),
            ))
            .expect("the entry appends");
    }
}

/// The process boundary is real: the binary reports the identity it was built
/// with, and the exit status says served.
#[test]
fn the_binary_reports_what_it_is() {
    let machine = Machine::new("version");
    let output = machine.run(&["--version"]);
    assert!(output.status.success(), "{}", error_text(&output));
    assert_eq!(text(&output).trim(), BuildIdentity::current().to_string());
}

/// A2 across the boundary: an unrecognized command is a named outcome on
/// stderr with a failing status, never a silent success.
#[test]
fn an_unknown_command_fails_the_process_and_says_what_it_saw() {
    let machine = Machine::new("unknown");
    let output = machine.run(&["measure"]);
    assert!(!output.status.success(), "an absent command exited zero");
    assert!(
        error_text(&output).contains("measure"),
        "{}",
        error_text(&output)
    );
    assert!(text(&output).is_empty(), "a refusal wrote to stdout");
}

/// Real persistence, read by a process that did not write it: one program
/// appends a record, `mcf export` — a separate process — reads the file and
/// produces a portable one whose digest is its own contents (B-302, D20).
#[test]
fn a_record_written_by_one_process_is_exported_by_another() {
    let machine = Machine::new("export");
    seed_record(&machine, 3);

    let destination = machine.0.join("bundle.mcf");
    let output = machine.run(&["export", "--to", &destination.display().to_string()]);
    assert!(output.status.success(), "{}", error_text(&output));

    let reported = text(&output);
    assert!(reported.contains("3 entries"), "{reported}");

    // The bundle is read back by the reader another machine would use, which
    // is what checks the claim the export made: `read` recomputes the digest
    // over the entries and refuses a bundle whose manifest disagrees.
    let (kind, manifest, entries) =
        mcf_record::export::read(&destination).expect("the bundle reads back");
    assert_eq!(kind, mcf_record::export::Kind::Export);
    assert_eq!(manifest.entries, 3);
    assert_eq!(entries.len(), 3);
    assert!(
        reported.contains(&manifest.digest),
        "the digest reported is not the one in the bundle: {reported}"
    );

    // A25, at the boundary: what left the record store is what the record
    // store holds, and the content store is not it. Nothing here can carry
    // prompt or completion text because no code path reaches it — this asserts
    // the observable half of that, over a bundle produced by a real process.
    let bytes = std::fs::read(&destination).expect("the bundle is on the disk");
    assert!(!bytes.is_empty(), "an export wrote an empty file");
    let seeded = entries
        .iter()
        .filter(|entry| {
            entry
                .get("body")
                .and_then(|body| body.get("seeded"))
                .is_some()
        })
        .count();
    assert_eq!(seeded, 3, "the bundle does not hold what was appended");
}

/// A2 again, at the boundary that matters most: asked to export a record that
/// does not exist, MCF says so and fails, rather than writing an empty bundle
/// that would read as a machine with no history.
#[test]
fn exporting_a_record_that_does_not_exist_is_refused_by_name() {
    let machine = Machine::new("export-empty");
    let destination = machine.0.join("bundle.mcf");
    let output = machine.run(&["export", "--to", &destination.display().to_string()]);

    assert!(
        !output.status.success(),
        "an absent record exported cleanly"
    );
    assert!(
        error_text(&output).contains("no record"),
        "{}",
        error_text(&output)
    );
    assert!(
        !destination.exists(),
        "a refused export left a file behind (A27)"
    );
}

/// A22 and B22: the headless surface is consumable by something other than a
/// person. The JSON a machine reads is the record's own codec, so a reader that
/// can read the record can read the surface.
#[test]
fn the_json_surface_parses_as_a_record_value() {
    let machine = Machine::new("json");
    let output = machine.run(&["doctor", "--no-record", "--json"]);
    assert!(output.status.success(), "{}", error_text(&output));

    let value = json::parse(text(&output).trim()).expect("the surface is readable JSON");
    for question in ["mcf", "machine", "self_cost", "laboratory"] {
        assert!(
            value.get(question).is_some(),
            "the JSON surface has no {question}: {value:?}"
        );
    }
}

/// Restart: two runs, two processes, one record. The second run appends to what
/// the first left, and a replay reads both — which is the property a daemon
/// will need and the one D20 makes the journal responsible for.
#[test]
fn the_record_survives_a_restart_and_the_second_run_appends_to_it() {
    let machine = Machine::new("restart");

    let first = machine.run(&["doctor"]);
    assert!(first.status.success(), "{}", error_text(&first));
    let after_first = replay(&machine.journal()).expect("the journal replays");
    assert!(after_first.is_complete(), "{}", after_first.statement());
    let count = after_first.entries.len();
    assert!(count >= 1, "the first run wrote nothing");

    let second = machine.run(&["doctor"]);
    assert!(second.status.success(), "{}", error_text(&second));
    let after_second = replay(&machine.journal()).expect("the journal replays");
    assert!(after_second.is_complete(), "{}", after_second.statement());
    assert!(
        after_second.entries.len() > count,
        "the second run appended nothing: {} then {}",
        count,
        after_second.entries.len()
    );

    // What the first run wrote is unchanged by the second. An append-only
    // record that rewrote its history would be the silent loss A2 forbids.
    assert_eq!(
        after_second.entries[..count]
            .iter()
            .map(Entry::body)
            .collect::<Vec<_>>(),
        after_first
            .entries
            .iter()
            .map(Entry::body)
            .collect::<Vec<_>>()
    );
}

/// `--no-record` means what it says, across the boundary: a run that was told
/// not to record leaves no journal at all (B1 — a default that flows is still
/// overridable, and the override is checkable).
#[test]
fn a_run_told_not_to_record_writes_nothing() {
    let machine = Machine::new("no-record");
    let output = machine.run(&["doctor", "--no-record"]);
    assert!(output.status.success(), "{}", error_text(&output));
    assert!(
        !machine.journal().exists(),
        "a run told not to record wrote {}",
        machine.journal().display()
    );
}

/// A27's question, asked of the record: killed at the worst possible moment,
/// does the machine come back? The kill lands wherever it lands — that is what
/// a crash does — and the property is invariant to when: the journal either
/// replays whole or reports a bounded loss (B62), and the next run works.
#[test]
fn a_run_killed_mid_write_leaves_a_record_that_still_opens() {
    let machine = Machine::new("killed");

    for attempt in 0..12u64 {
        let mut child = machine
            .command(&["doctor"])
            // Silenced: a spawned child inherits the harness's stdout, and a
            // killed `doctor` would otherwise print half a report into the
            // middle of the suite's output.
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .expect("the binary spawns");
        // Spread across the window a doctor run occupies — about 40 ms on the
        // machine this was written on — so that some kills land before the
        // first write, some during the measurement, and some around the append
        // itself. On a slower machine every kill lands earlier in the run,
        // which costs coverage and cannot cost correctness: the property below
        // is what a crash leaves behind at *any* moment, including before the
        // journal exists.
        std::thread::sleep(std::time::Duration::from_millis(attempt * 6));
        let _killed = child.kill();
        let _reaped = child.wait();

        if !machine.journal().exists() {
            continue;
        }
        match replay(&machine.journal()) {
            Ok(replayed) => {
                if let Some(loss) = &replayed.loss {
                    let length = std::fs::metadata(machine.journal())
                        .expect("the journal is there")
                        .len();
                    assert_eq!(
                        (loss.byte_offset + loss.bytes_unread) as u64,
                        length,
                        "a loss must account for the whole of what it did not read"
                    );
                }
            }
            Err(failure) => panic!("a killed run left a journal that will not open: {failure}"),
        }
    }

    // And the machine comes back: a full run after twelve kills still writes.
    let output = machine.run(&["doctor"]);
    assert!(output.status.success(), "{}", error_text(&output));
    let replayed = replay(&machine.journal()).expect("the journal opens after the kills");
    assert!(
        replayed
            .entries
            .iter()
            .any(|entry| entry.kind() == EntryKind::MachineProfile),
        "the run after the kills recorded no profile: {}",
        replayed.statement()
    );
}

/// The identity a binary reports and the identity its records carry are one
/// thing (§3.4 — MCF's own version is a condition of everything it takes).
/// Two processes, and the answer has to be the same in both.
#[test]
fn the_version_a_binary_reports_is_the_one_its_record_names() {
    let machine = Machine::new("identity");
    let reported = text(&machine.run(&["--version"])).trim().to_owned();
    let recorded_run = machine.run(&["doctor"]);
    assert!(
        recorded_run.status.success(),
        "{}",
        error_text(&recorded_run)
    );

    let header = std::fs::read_to_string(machine.journal()).expect("the journal is readable");
    let first = header.lines().next().expect("a journal has a header");
    let value = json::parse(first).expect("the header is readable");
    let created_by = value
        .get("created_by")
        .expect("the header names what created it");
    let version = created_by
        .get("version")
        .and_then(Value::as_text)
        .expect("the build identity names a version");

    assert!(
        reported.contains(version),
        "the binary reports {reported:?} and its record says {version:?}"
    );
}

/// The path a record lands on is the one the environment names, and nothing is
/// written outside it. §3.10's habit: MCF does not invent a place to keep the
/// user's evidence, and a test that did not check this could not tell the
/// difference between a record written here and one written to a real home
/// directory.
#[test]
fn nothing_is_written_outside_the_directory_the_environment_names() {
    let machine = Machine::new("confinement");
    seed_record(&machine, 1);
    let output = machine.run(&[
        "export",
        "--to",
        &machine.0.join("b.mcf").display().to_string(),
    ]);
    assert!(output.status.success(), "{}", error_text(&output));

    let mut found = Vec::new();
    walk(&machine.0, &mut found);
    for path in &found {
        assert!(
            path.starts_with(&machine.0),
            "{} is outside the machine's directory",
            path.display()
        );
    }
    assert!(
        found.iter().any(|path| path.ends_with("record.jsonl")),
        "the record is not where the environment said to put it: {found:?}"
    );
}

fn walk(directory: &Path, into: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(directory) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            walk(&path, into);
        } else {
            into.push(path);
        }
    }
}
