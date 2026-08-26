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
//! **Where the boundary falls is decided, and this tier is where it is drawn.**
//! D36 answers §7.22's four questions, and every answer is demonstrated below
//! rather than asserted: a real protocol against a peer the laboratory can make
//! hostile (`mcf pull` over loopback HTTP), the real engine MCF ships (`mcf
//! run` through the stand-in, D31), the shipped binary across every process
//! boundary MCF has (the daemon, started, asked, killed at eight moments and
//! started again), and recovery from the disk in every scenario that has state.
//!
//! **Nothing on MCF's side is mocked.** That is the rule the four answers
//! share. A simulated component appears only to produce a failure that is hard
//! to cause on purpose (D26), never to stand in for one that works.
//!
//! **The outer edge is what keeps this tier gating**: no network beyond
//! loopback, no accelerator, no large model, no credential. A claim that cannot
//! be tested inside those bounds belongs to a scheduled tier that says so — the
//! online tier speaks TLS to the real hub — rather than to a mock here (A19).
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
        // Including its control socket: without this the daemon under test
        // would listen where the *operator's* daemon listens, and a test that
        // reaches outside its machine is not a test of one (B19).
        command.env("XDG_RUNTIME_DIR", &self.0);
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
    for _entry in 0..entries {
        journal
            .append(&Entry::new(
                EntryKind::SelfCost,
                Timestamp::from_utc_nanos(1_700_000_000_000_000_000, Attested::Unknown),
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

/// The artifact states its own terms, across the process boundary (B-330).
///
/// A redistributor has a binary, not a repository. GPL-3.0 §4 asks whoever
/// conveys a copy to hand on a copy of the License, so the whole text is
/// compiled in — and this is the test that the person with the obligation can
/// actually get at it.
#[test]
fn the_binary_states_its_licence_and_carries_the_text() {
    let machine = Machine::new("licence");

    let short = machine.run(&["licence"]);
    assert!(short.status.success(), "{}", error_text(&short));
    let stated = text(&short);
    assert!(stated.contains("GPL-3.0-only"), "{stated}");
    assert!(stated.contains("NO WARRANTY"), "{stated}");
    assert!(stated.contains("section 6"), "{stated}");

    let full = machine.run(&["licence", "--full"]);
    assert!(full.status.success(), "{}", error_text(&full));
    let whole = text(&full);
    assert!(
        whole.contains("GNU GENERAL PUBLIC LICENSE") && whole.contains("TERMS AND CONDITIONS"),
        "the binary does not carry the licence text it is conveyed under"
    );
    assert!(whole.len() > stated.len() + 30_000, "{}", whole.len());

    // Both spellings reach it: the SPDX identifier and this project's documents
    // disagree, and a redistributor should not have to guess.
    assert_eq!(text(&machine.run(&["license"])), stated);
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

/// A model file on this machine, with a provenance beside it if asked for.
fn a_model(machine: &Machine, name: &str, bytes: usize, with_provenance: bool) -> PathBuf {
    let path = machine.0.join("mcf").join("models").join(name);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("the model directory is creatable");
    }
    std::fs::write(&path, vec![b'w'; bytes]).expect("a model file is writable");
    if with_provenance {
        // The library the binary uses, so a change to the sidecar's shape
        // reaches both sides at once.
        mcf_hub::store::record_provenance(
            &path,
            &mcf_core::provenance::Provenance::acquired(
                mcf_core::provenance::Origin::hub(
                    mcf_core::provenance::Repository::new("owner/model"),
                    Some(mcf_core::provenance::Revision::new("abc123")),
                ),
                Timestamp::now(),
            ),
        )
        .expect("the provenance is writable");
    }
    path
}

/// `list` on a machine that has acquired nothing says so, rather than failing
/// or inventing a directory.
#[test]
fn listing_an_empty_machine_says_it_is_empty() {
    let machine = Machine::new("list-empty");
    let output = machine.run(&["list"]);
    assert!(output.status.success(), "{}", error_text(&output));
    let text = text(&output);
    assert!(text.contains("no models"), "{text}");
}

/// And on a machine holding models it names each one, with where it came from
/// — or with the fact that nothing says (A7).
#[test]
fn listing_names_every_model_and_what_is_known_about_it() {
    let machine = Machine::new("list-models");
    a_model(&machine, "accounted.gguf", 32, true);
    a_model(&machine, "by-hand.gguf", 16, false);

    let output = machine.run(&["list"]);
    assert!(output.status.success(), "{}", error_text(&output));
    let text = text(&output);
    assert!(text.contains("accounted.gguf"), "{text}");
    assert!(text.contains("owner/model"), "{text}");
    assert!(text.contains("by-hand.gguf"), "{text}");
    assert!(
        text.contains("origin unknown"),
        "a model nothing accounts for was listed as though it were accounted for: {text}"
    );
    assert!(
        !text.contains("mcf-provenance"),
        "the listing shows MCF's own bookkeeping as though it were a model: {text}"
    );
}

/// `rm` without a reason previews and removes nothing. This is the whole of
/// §3.11 at the surface: an operator who typed the wrong name finds out by
/// reading rather than by losing a model.
#[test]
fn removing_without_a_reason_previews_and_removes_nothing() {
    let machine = Machine::new("rm-preview");
    let model = a_model(&machine, "model.gguf", 64, true);

    let output = machine.run(&["rm", "model.gguf"]);
    assert!(output.status.success(), "{}", error_text(&output));
    let text = text(&output);
    assert!(text.contains("model.gguf"), "{text}");
    assert!(text.contains("--because"), "{text}");
    assert!(text.contains("nothing was removed"), "{text}");
    assert!(model.exists(), "the preview removed the model");
}

/// With a reason it moves the model to a shelf, records the removal, and
/// deletes nothing.
#[test]
fn removing_with_a_reason_shelves_the_model_and_records_it() {
    let machine = Machine::new("rm-authorized");
    let model = a_model(&machine, "model.gguf", 64, true);

    let output = machine.run(&["rm", "model.gguf", "--because", "superseded"]);
    assert!(output.status.success(), "{}", error_text(&output));
    let text = text(&output);
    assert!(text.contains("superseded"), "{text}");
    assert!(!model.exists(), "the model is still where it was");
    assert!(
        machine.0.join("mcf").join("removed").exists(),
        "nothing was shelved: {text}"
    );

    // The record goes first, and it is still there afterwards.
    let written = std::fs::read_to_string(machine.journal()).expect("the record was written");
    assert!(written.contains("artifact_removed"), "{written}");
    assert!(written.contains("superseded"), "{written}");
    assert!(written.contains("model.gguf"), "{written}");

    // And the model is still on the disk, on the shelf, sidecar and all.
    let mut shelved = Vec::new();
    walk(&machine.0.join("mcf").join("removed"), &mut shelved);
    assert_eq!(shelved.len(), 2, "{shelved:?}");
}

/// `--purge` is the only thing that destroys a model, and it takes the same
/// authorization a removal does.
#[test]
fn purging_needs_the_same_authorization_and_says_what_it_did() {
    let machine = Machine::new("rm-purge");
    a_model(&machine, "model.gguf", 64, true);

    let previewed = machine.run(&["rm", "model.gguf", "--purge"]);
    assert!(previewed.status.success(), "{}", error_text(&previewed));
    assert!(
        text(&previewed).contains("nothing was removed"),
        "a purge without a reason removed something: {}",
        text(&previewed)
    );

    let output = machine.run(&["rm", "model.gguf", "--purge", "--because", "done with it"]);
    assert!(output.status.success(), "{}", error_text(&output));
    let text = text(&output);
    assert!(text.contains("purged"), "{text}");
    assert!(text.contains("cannot be brought back"), "{text}");

    let mut left = Vec::new();
    walk(&machine.0.join("mcf").join("removed"), &mut left);
    assert!(left.is_empty(), "a purge left something behind: {left:?}");
}

/// A name that is not there is an outcome that says so, and the store is
/// untouched.
#[test]
fn removing_something_that_is_not_there_says_so() {
    let machine = Machine::new("rm-absent");
    let model = a_model(&machine, "model.gguf", 8, false);

    let output = machine.run(&["rm", "not-a-model.gguf", "--because", "cleaning up"]);
    assert!(
        !output.status.success(),
        "a removal of nothing reported success"
    );
    let text = error_text(&output);
    assert!(text.contains("not-a-model.gguf"), "{text}");
    assert!(text.contains("nothing was removed"), "{text}");
    assert!(model.exists(), "the model that was there is gone");
}

/// The two answers a listing needs and the file itself, in the hub's own
/// shapes (F9).
fn a_hub_serving(weights: &str, digest: &str) -> mcf_lab::serving::Serving {
    use mcf_lab::serving::answer;
    let revision = "50968a4468ef4233ed78cd7c3de230dd1d61a56b";
    mcf_lab::serving::Serving::answering(std::collections::BTreeMap::from([
        (
            "/api/models/owner/model".to_owned(),
            answer(&format!(
                r#"{{"sha":"{revision}","tags":["gguf","license:apache-2.0"],"cardData":{{"license":"apache-2.0"}}}}"#
            )),
        ),
        (
            format!("/api/models/owner/model/tree/{revision}?recursive=true"),
            answer(&format!(
                r#"[{{"type":"file","size":{},"lfs":{{"oid":"{digest}","size":{}}},"path":"model.gguf"}}]"#,
                weights.len(),
                weights.len()
            )),
        ),
        (
            format!("/owner/model/resolve/{revision}/model.gguf"),
            format!(
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n{weights}",
                weights.len()
            ),
        ),
    ]))
    .expect("a loopback port")
}

/// The M1 claim, as processes: a model enters this machine, is listed with its
/// provenance, and leaves deliberately — none of it against a network (B-029,
/// B19).
#[test]
fn a_model_is_acquired_listed_and_removed() {
    let machine = Machine::new("pull-lifecycle");
    let weights = "GGUF the weights";
    let digest = mcf_core::digest::sha256(weights.as_bytes()).hex();
    let serving = a_hub_serving(weights, &digest);

    // Nothing is chosen for the operator: without a file, the repository's
    // contents are offered and nothing is acquired.
    let offered = machine.run(&["pull", "owner/model", "--from", &serving.base()]);
    assert!(offered.status.success(), "{}", error_text(&offered));
    let offered = text(&offered);
    assert!(offered.contains("model.gguf"), "{offered}");
    assert!(offered.contains("nothing was acquired"), "{offered}");
    assert!(offered.contains("apache-2.0"), "{offered}");

    // Named, it arrives, verified against the digest the hub declared.
    let pulled = machine.run(&["pull", "owner/model:model.gguf", "--from", &serving.base()]);
    assert!(pulled.status.success(), "{}", error_text(&pulled));
    let pulled = text(&pulled);
    assert!(pulled.contains("acquired"), "{pulled}");
    assert!(pulled.contains(&digest), "{pulled}");
    assert!(pulled.contains("verified against"), "{pulled}");

    let held = machine.0.join("mcf/models/owner/model/model.gguf");
    assert_eq!(
        std::fs::read(&held).expect("the model is on the disk"),
        weights.as_bytes()
    );

    // The acquisition is in the record, and the provenance is beside the file.
    let record = std::fs::read_to_string(machine.journal()).expect("a record");
    assert!(record.contains("artifact_acquired"), "{record}");
    assert!(
        record.contains("50968a4468ef4233ed78cd7c3de230dd1d61a56b"),
        "{record}"
    );

    // Listed, with where it came from and what its terms are — §III asks that
    // a licence be legible *before use*, and this is where an operator sees
    // what they hold (B-023).
    let listed = text(&machine.run(&["list"]));
    assert!(listed.contains("model.gguf"), "{listed}");
    assert!(listed.contains("owner/model"), "{listed}");
    assert!(!listed.contains("origin unknown"), "{listed}");
    assert!(listed.contains("licence: apache-2.0"), "{listed}");
    assert!(listed.contains("permissive"), "{listed}");

    // `mcf explain` says the same sentence about the same model, and the unit
    // test beside that surface is where it is asserted: the fixture here is
    // four words of text rather than a readable model, because what this test
    // is about is the transfer rather than the format.

    // And removed, deliberately.
    let removed = machine.run(&[
        "rm",
        "owner/model/model.gguf",
        "--because",
        "the lifecycle test is done with it",
    ]);
    assert!(removed.status.success(), "{}", error_text(&removed));
    assert!(!held.exists(), "the model is still there");
    let record = std::fs::read_to_string(machine.journal()).expect("a record");
    assert!(record.contains("artifact_removed"), "{record}");
}

/// Looking upstream at something already acquired, and finding it changed
/// (B-331, D37, §7.38).
///
/// Two hubs and one artifact: the first is where it came from, the second says
/// something different about the same repository. That is exactly what a
/// relicensing looks like from here — a changed field beside a success, with
/// nothing refusing (F17) — and the only way to find it is to compare against
/// what was written down at acquisition.
#[test]
fn what_was_acquired_is_checked_against_what_the_hub_says_now() {
    let machine = Machine::new("check");
    let weights = "GGUF the weights";
    let digest = mcf_core::digest::sha256(weights.as_bytes()).hex();
    let serving = a_hub_serving(weights, &digest);

    let pulled = machine.run(&["pull", "owner/model:model.gguf", "--from", &serving.base()]);
    assert!(pulled.status.success(), "{}", error_text(&pulled));

    // The bytes here, with no network at all: the half of the question that is
    // about this disk (B-301, §7.49).
    let here = machine.run(&["check", "--here"]);
    assert!(here.status.success(), "{}", error_text(&here));
    let said = text(&here);
    assert!(
        said.contains("the bytes here are the bytes that arrived"),
        "{said}"
    );
    assert!(said.contains("still matches it"), "{said}");

    // Nothing has changed, and *checked and unchanged* is a finding rather than
    // silence (A1).
    let checked = machine.run(&["check", "--from", &serving.base()]);
    assert!(checked.status.success(), "{}", error_text(&checked));
    let said = text(&checked);
    assert!(said.contains("nothing MCF compared has changed"), "{said}");
    assert!(said.contains("nothing there has changed"), "{said}");

    // The same repository, relicensed under the pin.
    let relicensed = a_hub_declaring(weights, &digest, "cc-by-nc-4.0");
    let after = machine.run(&["check", "--from", &relicensed.base()]);
    assert!(after.status.success(), "{}", error_text(&after));
    let said = text(&after);
    assert!(
        said.contains("was apache-2.0 and is now cc-by-nc-4.0"),
        "{said}"
    );
    // And nothing is withdrawn by it: the artifact is still here, still listed.
    assert!(said.contains("Nothing is invalidated"), "{said}");
    let listed = text(&machine.run(&["list"]));
    assert!(listed.contains("model.gguf"), "{listed}");

    // The finding is written down twice, and neither is a correction: beside
    // the artifact and in the record (D37, D20).
    let sidecar = machine
        .0
        .join("mcf/models/owner/model/model.gguf.mcf-provenance.json");
    let beside = std::fs::read_to_string(&sidecar).expect("the provenance is beside it");
    assert!(beside.contains("relicensed"), "{beside}");
    assert!(
        beside.contains("apache-2.0"),
        "the provenance lost what was true at acquisition: {beside}"
    );
    let record = std::fs::read_to_string(machine.journal()).expect("a record");
    assert!(record.contains("artifact_checked"), "{record}");
    assert!(record.contains("relicensed"), "{record}");
}

/// A machine that has never acquired anything answers the same way to every
/// surface that looks at the store (A6).
///
/// `mcf check` used to call it *the model store could not be read* while `mcf
/// list` called it *does not exist yet* — two answers to one situation, and the
/// failing one was the wrong one: a store nobody has created is not a store
/// that cannot be read.
#[test]
fn a_machine_holding_nothing_says_the_same_thing_to_every_surface() {
    let machine = Machine::new("check-empty");
    for arguments in [vec!["list"], vec!["check"], vec!["check", "--here"]] {
        let output = machine.run(&arguments);
        assert!(
            output.status.success(),
            "{arguments:?} failed on a machine holding nothing: {}",
            error_text(&output)
        );
        let said = text(&output);
        assert!(
            said.contains("does not exist yet"),
            "{arguments:?} said something else: {said}"
        );
    }
}

/// Corruption on this disk is found by re-reading, and is a different fact
/// from anything the hub says (B-301, §7.49, §3.8).
///
/// The failure this exists to catch is the quiet one: bytes that rot between
/// acquisition and a measurement taken months later, which without a check
/// shows up as a strange result rather than as a bad file.
#[test]
fn bytes_that_changed_on_this_disk_are_found_by_checking() {
    let machine = Machine::new("check-bytes");
    let weights = "GGUF the weights";
    let digest = mcf_core::digest::sha256(weights.as_bytes()).hex();
    let serving = a_hub_serving(weights, &digest);

    let pulled = machine.run(&["pull", "owner/model:model.gguf", "--from", &serving.base()]);
    assert!(pulled.status.success(), "{}", error_text(&pulled));

    // The disk rots under it. Built rather than caused (D26): what MCF observes
    // is bytes that do not match, however they came to differ.
    let held = machine.0.join("mcf/models/owner/model/model.gguf");
    std::fs::write(&held, "GGUF the weights, altered").expect("the bytes change");

    let checked = machine.run(&["check", "--here"]);
    assert!(checked.status.success(), "{}", error_text(&checked));
    let said = text(&checked);
    assert!(said.contains("no longer match the digest"), "{said}");
    // And the finding is in the record. A check that verified bytes and left no
    // account could not answer *when was this last known to be fine*, which is
    // the question D37 exists for.
    let record = std::fs::read_to_string(machine.journal()).expect("a record");
    assert!(record.contains("artifact_checked"), "{record}");
    assert!(record.contains("changed"), "{record}");
    assert!(
        said.contains(&digest),
        "the reading does not say what was expected: {said}"
    );
    assert!(
        said.contains("fact about this disk"),
        "the verdict does not say whose fault it is: {said}"
    );
}

/// A hub nobody can reach is not a clean bill of health (A7, F17).
///
/// The failure this guards against is a run of unanswered questions reading as
/// *nothing has changed* — which is what it said before, because an unreachable
/// hub was counted among the checked and the sentence about privacy and
/// withdrawal was borrowed for a refused connection.
#[test]
fn a_hub_that_cannot_be_reached_is_not_reported_as_unchanged() {
    let machine = Machine::new("check-unreachable");
    let weights = "GGUF the weights";
    let digest = mcf_core::digest::sha256(weights.as_bytes()).hex();
    let serving = a_hub_serving(weights, &digest);
    let pulled = machine.run(&["pull", "owner/model:model.gguf", "--from", &serving.base()]);
    assert!(pulled.status.success(), "{}", error_text(&pulled));
    drop(serving);

    // Port 9 is discard: nothing listens on it, on any machine.
    let checked = machine.run(&["check", "--from", "http://127.0.0.1:9/"]);
    assert!(checked.status.success(), "{}", error_text(&checked));
    let said = text(&checked);
    assert!(said.contains("no answer"), "{said}");
    assert!(
        !said.contains("nothing there has changed."),
        "an unreachable hub was reported as unchanged: {said}"
    );
    // And the sentence F17 earned is not borrowed for a refused connection.
    assert!(
        !said.contains("never existed"),
        "a network failure claimed the repository might be private: {said}"
    );
}

/// A hub that publishes the same file under a licence of its choosing.
fn a_hub_declaring(weights: &str, digest: &str, licence: &str) -> mcf_lab::serving::Serving {
    use mcf_lab::serving::answer;
    let revision = "50968a4468ef4233ed78cd7c3de230dd1d61a56b";
    mcf_lab::serving::Serving::answering(std::collections::BTreeMap::from([
        (
            "/api/models/owner/model".to_owned(),
            answer(&format!(
                r#"{{"sha":"{revision}","tags":["gguf","license:{licence}"],"cardData":{{"license":"{licence}"}}}}"#
            )),
        ),
        (
            format!("/api/models/owner/model/tree/{revision}?recursive=true"),
            answer(&format!(
                r#"[{{"type":"file","size":{},"lfs":{{"oid":"{digest}","size":{}}},"path":"model.gguf"}}]"#,
                weights.len(),
                weights.len()
            )),
        ),
    ]))
    .expect("a loopback port")
}

/// A second acquisition of a verified artifact costs nothing: the bytes are
/// already here and they are still what they should be.
#[test]
fn acquiring_something_already_held_does_not_fetch_it_again() {
    let machine = Machine::new("pull-again");
    let weights = "GGUF the weights";
    let digest = mcf_core::digest::sha256(weights.as_bytes()).hex();
    let serving = a_hub_serving(weights, &digest);

    let first = machine.run(&["pull", "owner/model:model.gguf", "--from", &serving.base()]);
    assert!(first.status.success(), "{}", error_text(&first));
    let asked_after_first = serving.asked().len();

    let again = machine.run(&["pull", "owner/model:model.gguf", "--from", &serving.base()]);
    assert!(again.status.success(), "{}", error_text(&again));

    let downloads = serving
        .asked()
        .iter()
        .filter(|request| request.contains("GET /owner/model/resolve/"))
        .count();
    assert_eq!(
        downloads,
        1,
        "the weights were fetched twice: {:?}",
        serving.asked()
    );
    assert!(
        serving.asked().len() > asked_after_first,
        "nothing was asked at all"
    );
}

/// The question an operator is actually asking when they name a repository and
/// no file: which of these will run here (PR3, B-213).
#[test]
fn a_repository_of_variants_is_planned_before_anything_is_downloaded() {
    use mcf_lab::serving::answer;
    let machine = Machine::new("pull-plan");
    let revision = "50968a4468ef4233ed78cd7c3de230dd1d61a56b";
    let configuration = r#"{"num_hidden_layers":28,"num_key_value_heads":8,"head_dim":128}"#;
    let serving = mcf_lab::serving::Serving::answering(std::collections::BTreeMap::from([
        (
            "/api/models/owner/model".to_owned(),
            answer(&format!(
                r#"{{"sha":"{revision}","cardData":{{"license":"apache-2.0"}}}}"#
            )),
        ),
        (
            format!("/api/models/owner/model/tree/{revision}?recursive=true"),
            answer(&format!(
                r#"[{{"type":"file","size":{},"path":"config.json"}},
                    {{"type":"file","size":8000,"path":"small.gguf"}},
                    {{"type":"file","size":900000000000000,"path":"enormous.gguf"}}]"#,
                configuration.len()
            )),
        ),
        (
            format!("/owner/model/resolve/{revision}/config.json"),
            answer(configuration),
        ),
    ]))
    .expect("a loopback port");

    let offered = machine.run(&["pull", "owner/model", "--from", &serving.base()]);
    assert!(offered.status.success(), "{}", error_text(&offered));
    let offered = text(&offered);

    assert!(offered.contains("4096 tokens of context"), "{offered}");
    let small = offered
        .lines()
        .find(|line| line.contains("small.gguf") && line.contains("fits"))
        .unwrap_or_default();
    assert!(small.contains("fits"), "{offered}");
    let enormous = offered
        .lines()
        .find(|line| line.contains("enormous.gguf") && line.contains("NOT fit"))
        .unwrap_or_default();
    assert!(enormous.contains("more than this machine has"), "{offered}");

    // The plan cost three cheap questions and no weights.
    let asked = serving.asked();
    assert_eq!(asked.len(), 3, "{asked:?}");
    assert!(
        !asked.iter().any(|request| request.contains(".gguf")),
        "a plan downloaded weights: {asked:?}"
    );
}

/// §XII's hard case end to end: a requantization of somebody else's weights,
/// where the provenance that matters is the other repository's (B-019).
#[test]
fn a_requantization_records_the_weights_it_was_made_from() {
    use mcf_lab::serving::answer;
    let machine = Machine::new("pull-chain");
    let weights = "GGUF a requantization";
    let digest = mcf_core::digest::sha256(weights.as_bytes()).hex();
    let revision = "50968a4468ef4233ed78cd7c3de230dd1d61a56b";
    let serving = mcf_lab::serving::Serving::answering(std::collections::BTreeMap::from([
        (
            "/api/models/somebody/model-GGUF".to_owned(),
            answer(&format!(
                r#"{{"sha":"{revision}","tags":["gguf","license:apache-2.0",
                    "base_model:original/weights","base_model:quantized:original/weights"]}}"#
            )),
        ),
        (
            format!("/api/models/somebody/model-GGUF/tree/{revision}?recursive=true"),
            answer(&format!(
                r#"[{{"type":"file","size":{},"lfs":{{"oid":"{digest}","size":{}}},"path":"model.gguf"}}]"#,
                weights.len(),
                weights.len()
            )),
        ),
        (
            format!("/somebody/model-GGUF/resolve/{revision}/model.gguf"),
            format!(
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n{weights}",
                weights.len()
            ),
        ),
    ]))
    .expect("a loopback port");

    let pulled = machine.run(&[
        "pull",
        "somebody/model-GGUF:model.gguf",
        "--from",
        &serving.base(),
    ]);
    assert!(pulled.status.success(), "{}", error_text(&pulled));
    let said = text(&pulled);
    assert!(said.contains("made from"), "{said}");
    assert!(said.contains("original/weights"), "{said}");
    assert!(said.contains("cannot vouch for"), "{said}");

    // The chain is in the sidecar, and it traverses: this artifact, the
    // transformation, and the weights it was made from.
    let sidecar = machine
        .0
        .join("mcf/models/somebody/model-GGUF/model.gguf.mcf-provenance.json");
    let written = std::fs::read_to_string(&sidecar).expect("a sidecar");
    let value = json::parse(&written).expect("it is JSON");
    let provenance = mcf_record::decode::provenance(&value).expect("it reads back");

    assert_eq!(provenance.depth(), 2, "the chain does not reach the base");
    let base = provenance.source().expect("a base");
    assert!(
        base.origin().to_string().contains("original/weights"),
        "{:?}",
        base.origin()
    );
    assert!(
        base.retrieved_at().known().is_none(),
        "MCF claimed to have fetched weights it never fetched"
    );
    assert_eq!(
        provenance.transformations().len(),
        1,
        "the publisher's own word for what they did was not kept"
    );
}

/// The plan is re-asked once the model is here, because it was made from what a
/// hub declared and what is true now is a different question (PR3, B-213).
#[test]
fn what_arrived_is_planned_again_now_that_it_is_here() {
    use mcf_lab::serving::answer;
    let machine = Machine::new("pull-replan");
    let weights = "GGUF the weights";
    let digest = mcf_core::digest::sha256(weights.as_bytes()).hex();
    let revision = "50968a4468ef4233ed78cd7c3de230dd1d61a56b";
    let configuration = r#"{"num_hidden_layers":4,"num_key_value_heads":2,"head_dim":64}"#;
    let serving = mcf_lab::serving::Serving::answering(std::collections::BTreeMap::from([
        (
            "/api/models/owner/model".to_owned(),
            answer(&format!(r#"{{"sha":"{revision}"}}"#)),
        ),
        (
            format!("/api/models/owner/model/tree/{revision}?recursive=true"),
            answer(&format!(
                r#"[{{"type":"file","size":{},"path":"config.json"}},
                    {{"type":"file","size":{},"lfs":{{"oid":"{digest}","size":{}}},"path":"model.gguf"}}]"#,
                configuration.len(),
                weights.len(),
                weights.len()
            )),
        ),
        (
            format!("/owner/model/resolve/{revision}/config.json"),
            answer(configuration),
        ),
        (
            format!("/owner/model/resolve/{revision}/model.gguf"),
            format!(
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n{weights}",
                weights.len()
            ),
        ),
    ]))
    .expect("a loopback port");

    let pulled = machine.run(&["pull", "owner/model:model.gguf", "--from", &serving.base()]);
    assert!(pulled.status.success(), "{}", error_text(&pulled));
    let said = text(&pulled);
    assert!(said.contains("now that it is here"), "{said}");
    assert!(said.contains("model.gguf — fits"), "{said}");
}

/// A repository that publishes no configuration cannot be planned for, and MCF
/// says that rather than showing a plan it guessed (A7).
#[test]
fn a_repository_with_no_configuration_is_said_to_be_unplannable() {
    let machine = Machine::new("pull-no-plan");
    let weights = "GGUF the weights";
    let digest = mcf_core::digest::sha256(weights.as_bytes()).hex();
    let serving = a_hub_serving(weights, &digest);

    let offered = text(&machine.run(&["pull", "owner/model", "--from", &serving.base()]));
    assert!(
        offered.contains("cannot say which of these would run here"),
        "{offered}"
    );
}

/// A hub that publishes no digest leaves the artifact *held* rather than
/// verified, and the surface says so in as many words (A21).
#[test]
fn an_artifact_nobody_could_check_is_reported_as_held() {
    let machine = Machine::new("pull-unverified");
    let weights = "GGUF the weights";
    let revision = "50968a4468ef4233ed78cd7c3de230dd1d61a56b";
    let serving = mcf_lab::serving::Serving::answering(std::collections::BTreeMap::from([
        (
            "/api/models/owner/model".to_owned(),
            mcf_lab::serving::answer(&format!(r#"{{"sha":"{revision}"}}"#)),
        ),
        (
            format!("/api/models/owner/model/tree/{revision}?recursive=true"),
            mcf_lab::serving::answer(&format!(
                r#"[{{"type":"file","size":{},"path":"model.gguf"}}]"#,
                weights.len()
            )),
        ),
        (
            format!("/owner/model/resolve/{revision}/model.gguf"),
            format!(
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n{weights}",
                weights.len()
            ),
        ),
    ]))
    .expect("a loopback port");

    let pulled = machine.run(&["pull", "owner/model:model.gguf", "--from", &serving.base()]);
    assert!(pulled.status.success(), "{}", error_text(&pulled));
    let pulled = text(&pulled);
    assert!(pulled.contains("HELD, NOT VERIFIED"), "{pulled}");
    assert!(
        pulled.contains("declared no digest"),
        "the surface does not say why: {pulled}"
    );
    // And the terms nobody declared are reported as unknown rather than filled
    // in (A7, B-023).
    assert!(pulled.contains("licence: unknown"), "{pulled}");
}

/// A private repository says which credential is missing, and tells the
/// operator what MCF has looked at and not used (B-024).
#[test]
fn a_private_repository_says_what_is_missing_and_what_was_not_used() {
    let machine = Machine::new("pull-private");
    let serving = mcf_lab::serving::Serving::answering(std::collections::BTreeMap::from([(
        "/api/models/owner/model".to_owned(),
        mcf_lab::serving::status(401, "Unauthorized"),
    )]))
    .expect("a loopback port");

    let refused = machine.run(&["pull", "owner/model", "--from", &serving.base()]);
    assert!(!refused.status.success());
    let said = error_text(&refused);
    assert!(
        said.contains("would not say whether this repository exists"),
        "{said}"
    );
    assert!(said.contains("owner/model"), "{said}");
    assert!(said.contains("--token-from-env"), "{said}");
    // And it names the ambiguity the hub imposes: withdrawn, private and never
    // existed are one answer, and advising a credential as though it must work
    // would be advice for one of three cases (F17, D37).
    assert!(said.contains("withdrawn"), "{said}");
    assert!(
        said.contains("looked") && said.contains("nothing"),
        "the refusal does not say what MCF found and did not use: {said}"
    );
}

/// A credential is read only from where the operator names, and never sent over
/// a connection that cannot keep it (B-024).
#[test]
fn a_credential_is_read_where_it_is_named_and_not_sent_in_the_clear() {
    let machine = Machine::new("pull-credential");
    let serving = mcf_lab::serving::Serving::answering(std::collections::BTreeMap::from([(
        "/api/models/owner/model".to_owned(),
        mcf_lab::serving::status(401, "Unauthorized"),
    )]))
    .expect("a loopback port");

    let token = machine.0.join("token");
    std::fs::write(&token, "hf_a_real_looking_token\n").expect("a token file");

    let refused = machine.run(&[
        "pull",
        "owner/model",
        "--from",
        &serving.base(),
        "--token-from",
        &token.display().to_string(),
    ]);
    assert!(!refused.status.success());
    let said = error_text(&refused);
    assert!(said.contains("cannot keep it"), "{said}");
    assert!(
        !said.contains("hf_a_real_looking_token"),
        "the token is in the output: {said}"
    );
    assert!(
        serving.asked().is_empty(),
        "a request went out carrying a credential: {:?}",
        serving.asked()
    );
}

/// The daemon, as processes: one starts and stays up, another asks it to stop,
/// and it says why it stopped (B-030, B-210, D1).
#[test]
fn the_daemon_starts_stays_up_and_stops_when_asked() {
    let machine = Machine::new("daemon");
    let mut serving = machine
        .command(&["serve"])
        .stdout(std::process::Stdio::piped())
        .spawn()
        .expect("the daemon starts");

    // It says it is up before it blocks, so a client knows when to connect.
    let mut said = String::new();
    {
        use std::io::BufRead as _;
        let stdout = serving.stdout.as_mut().expect("it prints where it is");
        let mut reader = std::io::BufReader::new(stdout);
        for _ in 0..4 {
            let mut line = String::new();
            if reader.read_line(&mut line).unwrap_or(0) == 0 {
                break;
            }
            said.push_str(&line);
        }
    }
    assert!(said.contains("mcf is up on"), "{said}");
    assert!(said.contains("cannot serve a model yet"), "{said}");
    assert!(said.contains("idle costs nothing"), "{said}");

    // A second daemon refuses rather than sharing the record.
    let second = machine.run(&["serve"]);
    assert!(!second.status.success(), "two daemons started");
    assert!(
        error_text(&second).contains("already listening"),
        "{}",
        error_text(&second)
    );

    let stopped = machine.run(&["stop", "--because", "the whole-system test is done"]);
    assert!(stopped.status.success(), "{}", error_text(&stopped));
    assert!(
        text(&stopped).contains("it said it is stopping"),
        "{}",
        text(&stopped)
    );

    let ended = serving.wait().expect("the daemon exits");
    assert!(ended.success(), "the daemon exited badly: {ended:?}");
}

/// A daemon killed at every stage of its life comes back (B-030, A27, §3.1).
///
/// **What B-030's condition asks.** *The lab kills the daemon at every
/// lifecycle stage and it recovers to a coherent, queryable state each time.*
/// The stages a daemon has today are the ones it can be killed *in*: starting
/// up, recording that it started, idle in `accept`, and answering a request.
/// Each is reached by killing at a different moment after the spawn, the way
/// `a_run_killed_mid_write_leaves_a_record_that_still_opens` reaches the
/// moments of an ordinary run.
///
/// **What must be true afterwards, whichever moment the kill landed on.** A
/// dead daemon leaves a socket file behind — the kernel does not remove one for
/// a process that did not get to — and the next daemon must take it over rather
/// than refuse to start beside a corpse. The record must open, whole or with a
/// bounded loss it reports (B62). And the machine must be *queryable*: `mcf
/// status` gets an answer from the new daemon.
///
/// **The kill is `SIGKILL`.** A28's shape: what is asserted is what survives
/// the worst interruption, not what a polite shutdown manages to clean up.
#[test]
fn a_daemon_killed_at_any_stage_comes_back() {
    let machine = Machine::new("daemon-killed");

    for attempt in 0..8_u64 {
        let mut daemon = machine
            .command(&["serve"])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .expect("the daemon spawns");

        // Spread across the window a daemon's start occupies — spawn, recover
        // the record and the store, bind, record that it started — so that
        // some kills land before the socket exists, some during the append
        // that says it is up, and some while it is idle in `accept`. The last
        // two attempts talk to it first, so the kill lands on a daemon that
        // has answered.
        std::thread::sleep(std::time::Duration::from_millis(attempt * 4));
        if attempt >= 6 {
            let _asked = machine.run(&["status"]);
        }
        let _killed = daemon.kill();
        let _reaped = daemon.wait();

        // The record either replays whole or says exactly what it lost. A
        // daemon killed mid-append is the one thing that can tear a line, and
        // the loss must be the tail rather than the history (A4, B62).
        if machine.journal().exists() {
            let replayed = replay(&machine.journal()).expect("the record opens after a kill");
            if let Some(loss) = &replayed.loss {
                let length = std::fs::metadata(machine.journal())
                    .expect("the journal is there")
                    .len();
                assert!(
                    u64::try_from(loss.byte_offset).unwrap_or(u64::MAX) < length,
                    "attempt {attempt}: the loss is not inside the file: {loss}"
                );
            }
        }

        // And the machine is queryable again: a new daemon starts, over the
        // socket the dead one left, and answers.
        let mut next = machine
            .command(&["serve"])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .expect("a second daemon spawns");

        let mut answered = None;
        for _ in 0..200 {
            let asked = machine.run(&["status"]);
            if asked.status.success() {
                answered = Some(text(&asked));
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        let answered = answered
            .unwrap_or_else(|| panic!("attempt {attempt}: no daemon answered after the kill"));
        assert!(
            answered.contains("mcf is up on"),
            "attempt {attempt}: {answered}"
        );
        // Coherent as well as answering: what it recovered is what the disk
        // holds, and it says so rather than starting with an empty history
        // (B-030, B62).
        assert!(
            answered.contains("recovered"),
            "attempt {attempt}: the daemon did not say what it recovered: {answered}"
        );

        let stopped = machine.run(&["stop", "--because", "the next attempt needs the socket"]);
        assert!(
            stopped.status.success(),
            "attempt {attempt}: {}",
            error_text(&stopped)
        );
        let ended = next.wait().expect("the daemon exits");
        assert!(
            ended.success(),
            "attempt {attempt}: the daemon exited badly: {ended:?}"
        );
    }
}

/// Asking a daemon that is not there to stop says so, and says what would start
/// one — rather than failing with a socket error nobody can act on.
#[test]
fn stopping_nothing_says_so_and_says_what_would_start_one() {
    let machine = Machine::new("stop-nothing");
    let refused = machine.run(&["stop"]);
    assert!(!refused.status.success());
    let said = error_text(&refused);
    assert!(said.contains("nothing is listening"), "{said}");
    assert!(said.contains("mcf serve"), "{said}");
}

/// §VI's bar, as far as MCF can honestly reach it today: having a model and
/// using a model are one command apart (B-040, D31, B65).
#[test]
fn a_model_on_this_machine_answers_something_and_the_answer_is_marked() {
    let machine = Machine::new("run");
    let model = machine.0.join("mcf/models/owner/model/model.gguf");
    std::fs::create_dir_all(model.parent().expect("a parent")).expect("a directory");
    std::fs::write(&model, mcf_lab::fixture::a_model_that_runs()).expect("a model file");

    let answered = machine.run(&[
        "run",
        "owner/model:model.gguf",
        "--prompt",
        "yes",
        "--limit",
        "3",
    ]);
    assert!(answered.status.success(), "{}", error_text(&answered));
    let said = text(&answered);

    // The answer this model gives, which a person can state in advance: its
    // embedding table is one-hot, so greedy decoding repeats what it is given.
    assert!(said.contains("yes yes yes"), "{said}");

    // And the conditions, beside it rather than under it.
    assert!(said.contains("what produced it"), "{said}");
    assert!(said.contains("greedy, seed 0"), "{said}");
    assert!(said.contains("3 token(s)"), "{said}");
    assert!(said.contains("MARKED"), "{said}");
    assert!(
        said.contains("can never be a speed"),
        "the answer does not say what it cannot be (B65): {said}"
    );

    // A path works as well as a name, because both are things somebody types.
    let by_path = machine.run(&["run", &model.display().to_string(), "--prompt", "no"]);
    assert!(by_path.status.success(), "{}", error_text(&by_path));
    assert!(text(&by_path).contains("no"), "{}", text(&by_path));
}

/// The record can be read back from a command, and what it could not read is
/// said rather than skipped (B-363, A22, B62).
#[test]
fn the_record_reads_back_and_says_what_it_could_not_read() {
    let machine = Machine::new("log");
    seed_record(&machine, 2);

    let read = machine.run(&["log"]);
    assert!(read.status.success(), "{}", error_text(&read));
    let said = text(&read);
    assert!(said.contains("2 entries"), "{said}");
    assert!(said.contains("self_cost"), "{said}");

    // A kind MCF does not have is refused with the list it does, rather than
    // silently showing nothing.
    let wrong = machine.run(&["log", "--kind", "not_a_kind"]);
    assert!(!wrong.status.success());
    assert!(
        error_text(&wrong).contains("artifact_acquired"),
        "{}",
        error_text(&wrong)
    );

    // A filter says what it counted, so it cannot be mistaken for the whole.
    let filtered = text(&machine.run(&["log", "--kind", "self_cost"]));
    assert!(filtered.contains("2 self_cost entries"), "{filtered}");

    // And the record's own JSON is what a script gets.
    let full = text(&machine.run(&["log", "--full"]));
    assert!(full.contains("\"kind\":\"self_cost\""), "{full}");

    // A crash mid-append leaves a torn last line. It is reported, and what came
    // before it is still read.
    {
        use std::io::Write as _;
        let mut torn = std::fs::OpenOptions::new()
            .append(true)
            .open(machine.journal())
            .expect("the record opens");
        torn.write_all(b"{\"id\":\"half a line").expect("it writes");
    }
    let damaged = machine.run(&["log"]);
    assert!(damaged.status.success(), "{}", error_text(&damaged));
    let said = text(&damaged);
    assert!(
        said.contains("2 entries"),
        "what was whole was not read: {said}"
    );
    assert!(
        said.contains("PART OF THE RECORD COULD NOT BE READ"),
        "the log read past a torn line without saying so: {said}"
    );
}

/// What MCF chose is visible with its source, and what it cannot say is said
/// (B-038, §3.15, §6.5).
#[test]
fn a_models_defaults_are_visible_with_their_sources() {
    let machine = Machine::new("explain");
    let model = machine.0.join("mcf/models/owner/model/model.gguf");
    std::fs::create_dir_all(model.parent().expect("a parent")).expect("a directory");
    std::fs::write(&model, mcf_lab::fixture::a_model_that_runs()).expect("a model file");

    let explained = machine.run(&["explain", "owner/model:model.gguf"]);
    assert!(explained.status.success(), "{}", error_text(&explained));
    let said = text(&explained);

    // Three columns, each saying which kind of thing it is.
    assert!(said.contains("WHAT THE FILE DECLARES"), "{said}");
    assert!(said.contains("WHAT MCF READ FROM THE BYTES"), "{said}");
    assert!(said.contains("WHAT MCF WOULD CHOOSE"), "{said}");
    // A model with no provenance beside it says so rather than leaving a blank.
    assert!(said.contains("nothing beside it says"), "{said}");
    // And the questions MCF has no basis to answer are named as unanswered.
    assert!(said.contains("WHAT MCF CANNOT TELL YOU"), "{said}");
    assert!(said.contains("Which quantization should I run?"), "{said}");
}

/// A run of a model that is not one is refused legibly, and says why MCF's own
/// reader is strict.
#[test]
fn running_something_that_is_not_a_model_is_refused_legibly() {
    let machine = Machine::new("run-refused");
    let not_a_model = machine.0.join("not-a-model.gguf");
    std::fs::write(&not_a_model, b"ONNX or something").expect("a file");

    let refused = machine.run(&[
        "run",
        &not_a_model.display().to_string(),
        "--prompt",
        "anything",
    ]);
    assert!(!refused.status.success());
    let said = error_text(&refused);
    assert!(said.contains("did not run"), "{said}");
    assert!(said.contains("no vendored engine"), "{said}");
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
