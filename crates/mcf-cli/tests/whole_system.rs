#![allow(clippy::expect_used, clippy::panic, clippy::indexing_slicing)]

mod serving;

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use mcf_core::attested::Attested;
use mcf_core::build_identity::BuildIdentity;
use mcf_core::time::Timestamp;
use mcf_record::journal::{Entry, EntryKind, Journal, replay};
use mcf_record::json::{self, Value};

fn binary() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_mcf"))
}

struct Machine(PathBuf);

impl Machine {
    fn new(name: &str) -> Self {
        let path =
            std::env::temp_dir().join(format!("mcf-whole-system-{name}-{}", std::process::id()));
        let _cleared = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).expect("a scratch directory is creatable");
        Self(path)
    }

    fn journal(&self) -> PathBuf {
        self.0.join("mcf").join("record.jsonl")
    }

    fn run(&self, arguments: &[&str]) -> Output {
        self.command(arguments)
            .output()
            .expect("the binary cargo built is runnable")
    }

    fn command(&self, arguments: &[&str]) -> Command {
        let mut command = Command::new(binary());
        command.args(arguments);
        command.env("XDG_DATA_HOME", &self.0);
        command.env("XDG_RUNTIME_DIR", &self.0);
        command.env_remove("HOME");
        command
    }
}

impl Drop for Machine {
    fn drop(&mut self) {
        if self.0.join("mcf").join("control.sock").exists() {
            let _asked = self
                .command(&["stop", "--because", "the test that started it finished"])
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .status();
        }
        let _removed = std::fs::remove_dir_all(&self.0);
    }
}

fn text(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn error_text(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

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

#[test]
fn the_binary_reports_what_it_is() {
    let machine = Machine::new("version");
    let output = machine.run(&["--version"]);
    assert!(output.status.success(), "{}", error_text(&output));
    assert_eq!(text(&output).trim(), BuildIdentity::current().to_string());
}

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

    assert_eq!(text(&machine.run(&["license"])), stated);
}

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

#[test]
fn a_record_written_by_one_process_is_exported_by_another() {
    let machine = Machine::new("export");
    seed_record(&machine, 3);

    let destination = machine.0.join("bundle.mcf");
    let output = machine.run(&["export", "--to", &destination.display().to_string()]);
    assert!(output.status.success(), "{}", error_text(&output));

    let reported = text(&output);
    assert!(reported.contains("3 entries"), "{reported}");

    let (kind, manifest, entries) =
        mcf_record::export::read(&destination).expect("the bundle reads back");
    assert_eq!(kind, mcf_record::export::Kind::Export);
    assert_eq!(manifest.entries, 3);
    assert_eq!(entries.len(), 3);
    assert!(
        reported.contains(&manifest.digest),
        "the digest reported is not the one in the bundle: {reported}"
    );

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

#[test]
fn the_json_surface_parses_as_a_record_value() {
    let machine = Machine::new("json");
    let output = machine.run(&["doctor", "--no-record", "--json"]);
    assert!(output.status.success(), "{}", error_text(&output));

    let value = json::parse(text(&output).trim()).expect("the surface is readable JSON");
    for question in ["mcf", "machine", "self_cost"] {
        assert!(
            value.get(question).is_some(),
            "the JSON surface has no {question}: {value:?}"
        );
    }
}

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

#[test]
fn a_run_killed_mid_write_leaves_a_record_that_still_opens() {
    let machine = Machine::new("killed");

    for attempt in 0..12u64 {
        let mut child = machine
            .command(&["doctor"])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .expect("the binary spawns");
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

fn a_model(machine: &Machine, name: &str, bytes: usize, with_provenance: bool) -> PathBuf {
    let path = machine.0.join("mcf").join("models").join(name);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("the model directory is creatable");
    }
    std::fs::write(&path, vec![b'w'; bytes]).expect("a model file is writable");
    if with_provenance {
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

#[test]
fn listing_an_empty_machine_says_it_is_empty() {
    let machine = Machine::new("list-empty");
    let output = machine.run(&["list"]);
    assert!(output.status.success(), "{}", error_text(&output));
    let text = text(&output);
    assert!(text.contains("no models"), "{text}");
}

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

    let written = std::fs::read_to_string(machine.journal()).expect("the record was written");
    assert!(written.contains("artifact_removed"), "{written}");
    assert!(written.contains("superseded"), "{written}");
    assert!(written.contains("model.gguf"), "{written}");

    let mut shelved = Vec::new();
    walk(&machine.0.join("mcf").join("removed"), &mut shelved);
    assert_eq!(shelved.len(), 2, "{shelved:?}");
}

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

fn a_hub_serving(weights: &str, digest: &str) -> serving::Serving {
    use serving::answer;
    let revision = "50968a4468ef4233ed78cd7c3de230dd1d61a56b";
    serving::Serving::answering(std::collections::BTreeMap::from([
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

#[test]
fn a_model_is_acquired_listed_and_removed() {
    let machine = Machine::new("pull-lifecycle");
    let weights = "GGUF the weights";
    let digest = mcf_core::digest::sha256(weights.as_bytes()).hex();
    let serving = a_hub_serving(weights, &digest);

    let offered = machine.run(&["pull", "owner/model", "--from", &serving.base()]);
    assert!(offered.status.success(), "{}", error_text(&offered));
    let offered = text(&offered);
    assert!(offered.contains("model.gguf"), "{offered}");
    assert!(offered.contains("nothing was acquired"), "{offered}");
    assert!(offered.contains("apache-2.0"), "{offered}");

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

    let record = std::fs::read_to_string(machine.journal()).expect("a record");
    assert!(record.contains("artifact_acquired"), "{record}");
    assert!(
        record.contains("50968a4468ef4233ed78cd7c3de230dd1d61a56b"),
        "{record}"
    );

    let listed = text(&machine.run(&["list"]));
    assert!(listed.contains("model.gguf"), "{listed}");
    assert!(listed.contains("owner/model"), "{listed}");
    assert!(!listed.contains("origin unknown"), "{listed}");
    assert!(listed.contains("licence: apache-2.0"), "{listed}");
    assert!(listed.contains("permissive"), "{listed}");

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

#[test]
fn what_was_acquired_is_checked_against_what_the_hub_says_now() {
    let machine = Machine::new("check");
    let weights = "GGUF the weights";
    let digest = mcf_core::digest::sha256(weights.as_bytes()).hex();
    let serving = a_hub_serving(weights, &digest);

    let pulled = machine.run(&["pull", "owner/model:model.gguf", "--from", &serving.base()]);
    assert!(pulled.status.success(), "{}", error_text(&pulled));

    let here = machine.run(&["check", "--here"]);
    assert!(here.status.success(), "{}", error_text(&here));
    let said = text(&here);
    assert!(
        said.contains("the bytes here are the bytes that arrived"),
        "{said}"
    );
    assert!(said.contains("still matches it"), "{said}");

    let checked = machine.run(&["check", "--from", &serving.base()]);
    assert!(checked.status.success(), "{}", error_text(&checked));
    let said = text(&checked);
    assert!(said.contains("nothing MCF compared has changed"), "{said}");
    assert!(said.contains("nothing there has changed"), "{said}");

    let relicensed = a_hub_declaring(weights, &digest, "cc-by-nc-4.0");
    let after = machine.run(&["check", "--from", &relicensed.base()]);
    assert!(after.status.success(), "{}", error_text(&after));
    let said = text(&after);
    assert!(
        said.contains("was apache-2.0 and is now cc-by-nc-4.0"),
        "{said}"
    );
    assert!(said.contains("Nothing is invalidated"), "{said}");
    let listed = text(&machine.run(&["list"]));
    assert!(listed.contains("model.gguf"), "{listed}");

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

#[test]
fn bytes_that_changed_on_this_disk_are_found_by_checking() {
    let machine = Machine::new("check-bytes");
    let weights = "GGUF the weights";
    let digest = mcf_core::digest::sha256(weights.as_bytes()).hex();
    let serving = a_hub_serving(weights, &digest);

    let pulled = machine.run(&["pull", "owner/model:model.gguf", "--from", &serving.base()]);
    assert!(pulled.status.success(), "{}", error_text(&pulled));

    let held = machine.0.join("mcf/models/owner/model/model.gguf");
    std::fs::write(&held, "GGUF the weights, altered").expect("the bytes change");

    let checked = machine.run(&["check", "--here"]);
    assert!(checked.status.success(), "{}", error_text(&checked));
    let said = text(&checked);
    assert!(said.contains("no longer match the digest"), "{said}");
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

#[test]
fn models_go_where_the_operator_says_and_every_surface_looks_there() {
    let machine = Machine::new("stores");
    let weights = "GGUF the weights";
    let digest = mcf_core::digest::sha256(weights.as_bytes()).hex();
    let serving = a_hub_serving(weights, &digest);

    let first = machine.0.join("store-one");
    let second = machine.0.join("store-two");
    std::fs::create_dir_all(&first).expect("a store");
    std::fs::create_dir_all(&second).expect("a second store");
    let listed = format!("{}:{}", first.display(), second.display());

    let pulled = machine
        .command(&["pull", "owner/model:model.gguf", "--from", &serving.base()])
        .env("MCF_MODELS", &listed)
        .output()
        .expect("the binary runs");
    assert!(pulled.status.success(), "{}", error_text(&pulled));
    assert!(
        first.join("owner/model/model.gguf").is_file(),
        "the model did not go in the first store"
    );

    let elsewhere = machine.0.join("somewhere-else");
    let pulled = machine
        .command(&[
            "pull",
            "owner/model:model.gguf",
            "--from",
            &serving.base(),
            "--into",
            elsewhere.to_str().unwrap_or_default(),
        ])
        .env("MCF_MODELS", &listed)
        .output()
        .expect("the binary runs");
    assert!(pulled.status.success(), "{}", error_text(&pulled));
    assert!(
        elsewhere.join("owner/model/model.gguf").is_file(),
        "--into was ignored: {}",
        text(&pulled)
    );

    let refused = machine
        .command(&[
            "pull",
            "owner/model:model.gguf",
            "--from",
            &serving.base(),
            "--into",
            "somewhere/relative",
        ])
        .env("MCF_MODELS", &listed)
        .output()
        .expect("the binary runs");
    assert!(!refused.status.success());
    assert!(
        error_text(&refused).contains("absolute"),
        "{}",
        error_text(&refused)
    );

    std::fs::create_dir_all(second.join("other/model")).expect("a second model");
    std::fs::write(second.join("other/model/model.gguf"), weights).expect("a file");
    let listing = machine
        .command(&["list"])
        .env("MCF_MODELS", &listed)
        .output()
        .expect("the binary runs");
    let said = text(&listing);
    assert!(said.contains("store-one"), "{said}");
    assert!(said.contains("store-two"), "{said}");
    assert!(said.contains("other/model/model.gguf"), "{said}");
}

#[test]
fn a_model_held_in_two_stores_is_ambiguous_rather_than_first_wins() {
    let machine = Machine::new("stores-ambiguous");
    let first = machine.0.join("store-one");
    let second = machine.0.join("store-two");
    for store in [&first, &second] {
        std::fs::create_dir_all(store.join("owner/model")).expect("a store");
        std::fs::write(
            store.join("owner/model/model.gguf"),
            mcf_standin::fixture::a_model_that_runs(),
        )
        .expect("a model");
    }
    let listed = format!("{}:{}", first.display(), second.display());

    let refused = machine
        .command(&["ask", "owner/model:model.gguf", "--prompt", "yes"])
        .env("MCF_MODELS", &listed)
        .output()
        .expect("the binary runs");
    assert!(!refused.status.success(), "{}", text(&refused));
    let said = error_text(&refused);
    assert!(said.contains("names 2 files"), "{said}");
    assert!(said.contains("store-one"), "{said}");
    assert!(said.contains("store-two"), "{said}");
}

#[test]
fn a_hub_that_cannot_be_reached_is_not_reported_as_unchanged() {
    let machine = Machine::new("check-unreachable");
    let weights = "GGUF the weights";
    let digest = mcf_core::digest::sha256(weights.as_bytes()).hex();
    let serving = a_hub_serving(weights, &digest);
    let pulled = machine.run(&["pull", "owner/model:model.gguf", "--from", &serving.base()]);
    assert!(pulled.status.success(), "{}", error_text(&pulled));
    drop(serving);

    let checked = machine.run(&["check", "--from", "http://127.0.0.1:9/"]);
    assert!(checked.status.success(), "{}", error_text(&checked));
    let said = text(&checked);
    assert!(said.contains("no answer"), "{said}");
    assert!(
        !said.contains("nothing there has changed."),
        "an unreachable hub was reported as unchanged: {said}"
    );
    assert!(
        !said.contains("never existed"),
        "a network failure claimed the repository might be private: {said}"
    );
}

fn a_hub_declaring(weights: &str, digest: &str, licence: &str) -> serving::Serving {
    use serving::answer;
    let revision = "50968a4468ef4233ed78cd7c3de230dd1d61a56b";
    serving::Serving::answering(std::collections::BTreeMap::from([
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

#[test]
fn a_repository_of_variants_is_planned_before_anything_is_downloaded() {
    use serving::answer;
    let machine = Machine::new("pull-plan");
    let revision = "50968a4468ef4233ed78cd7c3de230dd1d61a56b";
    let configuration = r#"{"num_hidden_layers":28,"num_key_value_heads":8,"head_dim":128}"#;
    let serving = serving::Serving::answering(std::collections::BTreeMap::from([
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

    let asked = serving.asked();
    assert_eq!(asked.len(), 3, "{asked:?}");
    assert!(
        !asked.iter().any(|request| request.contains(".gguf")),
        "a plan downloaded weights: {asked:?}"
    );
}

#[test]
fn a_requantization_records_the_weights_it_was_made_from() {
    use serving::answer;
    let machine = Machine::new("pull-chain");
    let weights = "GGUF a requantization";
    let digest = mcf_core::digest::sha256(weights.as_bytes()).hex();
    let revision = "50968a4468ef4233ed78cd7c3de230dd1d61a56b";
    let serving = serving::Serving::answering(std::collections::BTreeMap::from([
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

#[test]
fn what_arrived_is_planned_again_now_that_it_is_here() {
    use serving::answer;
    let machine = Machine::new("pull-replan");
    let weights = "GGUF the weights";
    let digest = mcf_core::digest::sha256(weights.as_bytes()).hex();
    let revision = "50968a4468ef4233ed78cd7c3de230dd1d61a56b";
    let configuration = r#"{"num_hidden_layers":4,"num_key_value_heads":2,"head_dim":64}"#;
    let serving = serving::Serving::answering(std::collections::BTreeMap::from([
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

#[test]
fn an_artifact_nobody_could_check_is_reported_as_held() {
    let machine = Machine::new("pull-unverified");
    let weights = "GGUF the weights";
    let revision = "50968a4468ef4233ed78cd7c3de230dd1d61a56b";
    let serving = serving::Serving::answering(std::collections::BTreeMap::from([
        (
            "/api/models/owner/model".to_owned(),
            serving::answer(&format!(r#"{{"sha":"{revision}"}}"#)),
        ),
        (
            format!("/api/models/owner/model/tree/{revision}?recursive=true"),
            serving::answer(&format!(
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
    assert!(pulled.contains("licence: unknown"), "{pulled}");
}

#[test]
fn a_private_repository_says_what_is_missing_and_what_was_not_used() {
    let machine = Machine::new("pull-private");
    let serving = serving::Serving::answering(std::collections::BTreeMap::from([(
        "/api/models/owner/model".to_owned(),
        serving::status(401, "Unauthorized"),
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
    assert!(said.contains("withdrawn"), "{said}");
    assert!(
        said.contains("looked") && said.contains("nothing"),
        "the refusal does not say what MCF found and did not use: {said}"
    );
}

#[test]
fn a_credential_is_read_where_it_is_named_and_not_sent_in_the_clear() {
    let machine = Machine::new("pull-credential");
    let serving = serving::Serving::answering(std::collections::BTreeMap::from([(
        "/api/models/owner/model".to_owned(),
        serving::status(401, "Unauthorized"),
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

#[test]
fn a_generation_puts_its_text_in_the_content_store_and_not_in_the_record() {
    let machine = Machine::new("content-separation");
    let model = machine.0.join("fixture.gguf");
    std::fs::write(&model, mcf_standin::fixture::a_model_that_runs()).expect("the fixture writes");

    let mut serving = machine
        .command(&["serve"])
        .stdout(std::process::Stdio::piped())
        .spawn()
        .expect("the daemon starts");
    {
        use std::io::BufRead as _;
        let stdout = serving.stdout.as_mut().expect("it says where it is");
        let mut reader = std::io::BufReader::new(stdout);
        let mut line = String::new();
        let _read = reader.read_line(&mut line);
        assert!(line.contains("mcf is up on"), "{line}");
    }

    let ran = machine.run(&[
        "ask",
        model.to_str().expect("a fixture path is text"),
        "--prompt",
        "a",
        "--limit",
        "4",
        "--engine",
        "stand-in",
    ]);
    let said = text(&ran);
    let _stopped = machine.run(&["stop", "--because", "the content-separation test is done"]);
    let _waited = serving.wait();
    assert!(
        said.contains("served   by the daemon"),
        "the daemon did not serve it, so this tests the wrong path: {said}"
    );

    let record = std::fs::read_to_string(machine.journal()).expect("the record is there");
    let generated: Vec<mcf_record::json::Value> = record
        .lines()
        .filter_map(|line| mcf_record::json::parse(line).ok())
        .filter(|entry| {
            entry.get("kind").and_then(mcf_record::json::Value::as_text) == Some("generated")
        })
        .collect();
    assert_eq!(generated.len(), 1, "one generation, one entry: {record}");
    let entry = generated.first().expect("one entry");
    let body = entry.get("body").expect("an entry has a body");

    for content in ["text", "prompt", "produced_tokens"] {
        assert!(
            body.get(content).is_none(),
            "the record holds `{content}`, which is what a person wrote or a model said \
             (A25, F105): {}",
            body.to_line()
        );
    }
    let bytes = body
        .get("text_bytes")
        .and_then(mcf_record::json::Value::as_integer)
        .expect("the record says how much was said, which is a measurement about content");
    assert!(bytes > 0, "the model said nothing, so this proves nothing");

    let id = entry
        .get("id")
        .and_then(mcf_record::json::Value::as_text)
        .expect("a written entry has an identifier");
    let store = mcf_record::content::ContentStore::open(
        &mcf_record::content::ContentStore::beside(&machine.journal()),
    )
    .expect("the content store opens");
    let kept = store
        .disclose_kept(id)
        .expect("the store answers")
        .expect("what the model said was kept, because A1 forbids losing it to fix a leak");
    assert_eq!(
        i64::try_from(kept.length_bytes()).unwrap_or(i64::MAX),
        bytes,
        "the record's length and the content store's content disagree"
    );
    assert!(
        said.contains(kept.disclose()),
        "what was filed is not what the caller was told: {said}"
    );
}

#[test]
fn the_daemon_starts_stays_up_and_stops_when_asked() {
    let machine = Machine::new("daemon");
    let mut serving = machine
        .command(&["serve"])
        .stdout(std::process::Stdio::piped())
        .spawn()
        .expect("the daemon starts");

    let mut said = String::new();
    {
        use std::io::BufRead as _;
        let stdout = serving.stdout.as_mut().expect("it prints where it is");
        let mut reader = std::io::BufReader::new(stdout);
        for _ in 0..5 {
            let mut line = String::new();
            if reader.read_line(&mut line).unwrap_or(0) == 0 {
                break;
            }
            said.push_str(&line);
        }
    }
    assert!(said.contains("mcf is up on"), "{said}");
    assert!(
        said.contains("memory cap") && said.contains("when memory runs out"),
        "{said}"
    );
    assert!(said.contains("no engine is installed yet"), "{said}");
    assert!(said.contains("provision"), "{said}");
    assert!(said.contains("costs nothing"), "{said}");
    for cited in ["B-320", "D38", "B65", "§3.13"] {
        assert!(!said.contains(cited), "a citation reached a person: {said}");
    }

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

#[test]
fn a_daemon_sent_a_termination_signal_stops_and_says_why() {
    use std::io::Read as _;

    let machine = Machine::new("daemon-terminated");
    let mut daemon = machine
        .command(&["serve"])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
        .expect("the daemon spawns");

    let mut answered = false;
    for _ in 0..300 {
        if machine.run(&["status"]).status.success() {
            answered = true;
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    assert!(answered, "the daemon never answered");

    let signalled = Command::new("kill")
        .arg("-TERM")
        .arg(daemon.id().to_string())
        .status()
        .expect("kill runs");
    assert!(signalled.success(), "kill could not signal the daemon");

    let mut exited = None;
    for _ in 0..600 {
        if let Ok(Some(status)) = daemon.try_wait() {
            exited = Some(status);
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    let Some(exited) = exited else {
        let _killed = daemon.kill();
        panic!("the daemon did not stop within six seconds of SIGTERM");
    };
    assert!(exited.success(), "the daemon left with {exited}");

    let mut said = String::new();
    daemon
        .stdout
        .take()
        .expect("stdout was piped")
        .read_to_string(&mut said)
        .expect("stdout is read");
    assert!(
        said.contains("mcf stopped, because: the process received SIGTERM"),
        "the daemon did not say why it stopped:\n{said}"
    );

    let record = std::fs::read_to_string(machine.journal()).expect("the record is there");
    let stopped = record
        .lines()
        .find(|line| line.contains("daemon_stopped"))
        .expect("the record carries the stop");
    assert!(
        stopped.contains("the process received SIGTERM"),
        "the stop does not name the signal: {stopped}"
    );
}

#[test]
fn a_daemon_whose_socket_is_gone_still_stops_when_signalled() {
    let machine = Machine::new("daemon-socket-gone");
    let mut daemon = Reaped(
        machine
            .command(&["serve"])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .expect("the daemon spawns"),
    );

    let mut answered = false;
    for _ in 0..300 {
        if machine.run(&["status"]).status.success() {
            answered = true;
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    assert!(answered, "the daemon never answered");

    let socket = machine.0.join("mcf").join("control.sock");
    assert!(socket.exists(), "no socket at {}", socket.display());
    std::fs::remove_file(&socket).expect("the socket goes out from under the daemon");

    let signalled = Command::new("kill")
        .arg("-TERM")
        .arg(daemon.0.id().to_string())
        .status()
        .expect("kill runs");
    assert!(signalled.success(), "kill could not signal the daemon");

    let mut exited = None;
    for _ in 0..600 {
        if let Ok(Some(status)) = daemon.0.try_wait() {
            exited = Some(status);
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    assert!(
        exited.is_some(),
        "the daemon is asked to stop through its own socket, so a socket that is gone \
         left it sitting in accept ignoring SIGTERM, and only SIGKILL could reach it"
    );
}

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

        std::thread::sleep(std::time::Duration::from_millis(attempt * 4));
        if attempt >= 6 {
            let _asked = machine.run(&["status"]);
        }
        let _killed = daemon.kill();
        let _reaped = daemon.wait();

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

#[test]
fn stopping_nothing_says_so_and_says_what_would_start_one() {
    let machine = Machine::new("stop-nothing");
    let refused = machine.run(&["stop"]);
    assert!(!refused.status.success());
    let said = error_text(&refused);
    assert!(said.contains("nothing is listening"), "{said}");
    assert!(said.contains("mcf serve"), "{said}");
}

#[test]
fn a_model_on_this_machine_answers_something_and_the_answer_is_marked() {
    let machine = Machine::new("ask");
    let model = machine.0.join("mcf/models/owner/model/model.gguf");
    std::fs::create_dir_all(model.parent().expect("a parent")).expect("a directory");
    std::fs::write(&model, mcf_standin::fixture::a_model_that_runs()).expect("a model file");

    let answered = machine.run(&[
        "ask",
        "owner/model:model.gguf",
        "--prompt",
        "yes",
        "--limit",
        "3",
    ]);
    assert!(answered.status.success(), "{}", error_text(&answered));
    let said = text(&answered);

    assert!(said.contains("yes yes yes"), "{said}");

    assert!(said.contains("what produced it"), "{said}");
    assert!(said.contains("greedy, seed 0"), "{said}");
    assert!(said.contains("3 token(s)"), "{said}");

    let by_path = machine.run(&["ask", &model.display().to_string(), "--prompt", "no"]);
    assert!(by_path.status.success(), "{}", error_text(&by_path));
    assert!(text(&by_path).contains("no"), "{}", text(&by_path));
}

#[test]
fn the_record_reads_back_and_says_what_it_could_not_read() {
    let machine = Machine::new("log");
    seed_record(&machine, 2);

    let read = machine.run(&["log"]);
    assert!(read.status.success(), "{}", error_text(&read));
    let said = text(&read);
    assert!(said.contains("2 entries"), "{said}");
    assert!(said.contains("self_cost"), "{said}");

    let wrong = machine.run(&["log", "--kind", "not_a_kind"]);
    assert!(!wrong.status.success());
    assert!(
        error_text(&wrong).contains("artifact_acquired"),
        "{}",
        error_text(&wrong)
    );

    let filtered = text(&machine.run(&["log", "--kind", "self_cost"]));
    assert!(filtered.contains("2 self_cost entries"), "{filtered}");

    let full = text(&machine.run(&["log", "--full"]));
    assert!(full.contains("\"kind\":\"self_cost\""), "{full}");

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

#[test]
fn a_models_defaults_are_visible_with_their_sources() {
    let machine = Machine::new("explain");
    let model = machine.0.join("mcf/models/owner/model/model.gguf");
    std::fs::create_dir_all(model.parent().expect("a parent")).expect("a directory");
    std::fs::write(&model, mcf_standin::fixture::a_model_that_runs()).expect("a model file");

    let explained = machine.run(&["explain", "owner/model:model.gguf"]);
    assert!(explained.status.success(), "{}", error_text(&explained));
    let said = text(&explained);

    assert!(said.contains("WHAT THE FILE DECLARES"), "{said}");
    assert!(said.contains("WHAT MCF READ FROM THE BYTES"), "{said}");
    assert!(said.contains("WHAT MCF WOULD CHOOSE"), "{said}");
    assert!(said.contains("nothing beside it says"), "{said}");
    assert!(said.contains("WHAT MCF CANNOT TELL YOU"), "{said}");
    assert!(said.contains("Which quantization should I run?"), "{said}");
}

#[test]
fn running_something_that_is_not_a_model_is_refused_legibly() {
    let machine = Machine::new("run-refused");
    let not_a_model = machine.0.join("not-a-model.gguf");
    std::fs::write(&not_a_model, b"ONNX or something").expect("a file");

    let refused = machine.run(&[
        "ask",
        &not_a_model.display().to_string(),
        "--prompt",
        "anything",
    ]);
    assert!(!refused.status.success());
    let said = error_text(&refused);
    assert!(said.contains("did not run"), "{said}");
    assert!(said.contains("provision"), "{said}");
    assert!(
        !said.contains("B-320"),
        "a citation reached a person: {said}"
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

fn pinned_llama() -> &'static mcf_core::component::Component {
    mcf_core::component::COMPONENTS
        .iter()
        .find(|component| component.name == "llama.cpp")
        .expect("llama.cpp is a component MCF knows how to provision")
}

fn pinned_llama_prefix() -> String {
    let commit = pinned_llama().commit;
    format!("llama.cpp@{}", commit.get(..12).unwrap_or(commit))
}

fn seed_provisioned(machine: &Machine) -> PathBuf {
    let prefix = machine
        .0
        .join("mcf")
        .join("provisioned")
        .join(pinned_llama_prefix());
    std::fs::create_dir_all(prefix.join("build").join("bin")).expect("a prefix");
    std::fs::write(
        prefix.join("mcf-provenance.json"),
        format!(
            "{{\"component\":\"llama.cpp\",\"commit\":\"{}\"}}\n",
            pinned_llama().commit
        ),
    )
    .expect("provenance written");
    prefix
}

#[test]
fn provisioning_lists_what_it_knows_and_what_is_present() {
    let machine = Machine::new("provision-list");
    let before = machine.run(&["provision", "--list"]);
    assert!(before.status.success(), "{}", error_text(&before));
    let listed = text(&before);
    assert!(listed.contains(&pinned_llama_prefix()), "{listed}");
    assert!(listed.contains("not provisioned"), "{listed}");
    assert!(
        listed.contains("sha256:"),
        "the image digest is what pins: {listed}"
    );

    seed_provisioned(&machine);
    let after = text(&machine.run(&["provision", "--list"]));
    assert!(after.contains("—  provisioned"), "{after}");
}

#[test]
fn a_removal_is_reasoned_recorded_and_complete() {
    let machine = Machine::new("provision-remove");
    let prefix = seed_provisioned(&machine);

    let unreasoned = machine.run(&["provision", "--remove", "llama.cpp"]);
    assert!(!unreasoned.status.success());
    assert!(prefix.exists(), "a refusal removes nothing");
    assert!(
        error_text(&unreasoned).contains("--because"),
        "{}",
        error_text(&unreasoned)
    );

    let removed = machine.run(&[
        "provision",
        "--remove",
        "llama.cpp",
        "--because",
        "the whole-system tier asked",
    ]);
    assert!(removed.status.success(), "{}", error_text(&removed));
    assert!(!prefix.exists(), "the prefix is gone");
    assert!(
        text(&removed).contains("the whole-system tier asked"),
        "{}",
        text(&removed)
    );

    let log = text(&machine.run(&["log", "--kind", "component_removed"]));
    assert!(
        log.contains("removed the provisioned llama.cpp, because: the whole-system tier asked"),
        "{log}"
    );

    let again = machine.run(&["provision", "--remove", "llama.cpp", "--because", "again"]);
    assert!(again.status.success());
    assert!(
        text(&again).contains("is not provisioned"),
        "{}",
        text(&again)
    );
}

#[test]
fn an_unknown_component_is_refused_with_the_table() {
    let machine = Machine::new("provision-unknown");
    let output = machine.run(&["provision", "not-a-component"]);
    assert!(!output.status.success());
    let said = error_text(&output);
    assert!(
        said.contains("does not know how to provision not-a-component"),
        "{said}"
    );
    assert!(said.contains("llama.cpp"), "{said}");
    assert!(
        !machine.journal().exists(),
        "a refusal before anything happened records nothing"
    );
}

#[test]
fn an_already_provisioned_component_is_left_alone() {
    let machine = Machine::new("provision-twice");
    let prefix = seed_provisioned(&machine);
    let marker = prefix.join("build").join("bin").join("keep");
    std::fs::write(&marker, b"present").expect("a marker");

    let output = machine.run(&["provision", "llama.cpp"]);
    assert!(output.status.success(), "{}", error_text(&output));
    assert!(
        text(&output).contains("already provisioned"),
        "{}",
        text(&output)
    );
    assert!(marker.exists(), "an existing build is untouched");
}

#[test]
fn a_running_daemon_serves_a_generation_and_records_its_account() {
    let machine = Machine::new("served");
    let models = machine
        .0
        .join("mcf")
        .join("models")
        .join("lab")
        .join("fixture");
    std::fs::create_dir_all(&models).expect("a store");
    let model = models.join("a-model-that-runs.gguf");
    std::fs::write(&model, mcf_standin::fixture::a_model_that_runs()).expect("a model file");

    let mut serving = Reaped(
        machine
            .command(&["serve"])
            .stdout(std::process::Stdio::piped())
            .spawn()
            .expect("the daemon starts"),
    );
    {
        use std::io::BufRead as _;
        let stdout = serving.0.stdout.as_mut().expect("it prints where it is");
        let mut line = String::new();
        std::io::BufReader::new(stdout)
            .read_line(&mut line)
            .expect("the first line");
        assert!(line.contains("mcf is up on"), "{line}");
    }

    let ran = machine.run(&[
        "ask",
        "lab/fixture:a-model-that-runs.gguf",
        "--prompt",
        "yes",
        "--limit",
        "4",
    ]);
    assert!(ran.status.success(), "{}", error_text(&ran));
    let said = text(&ran);
    assert!(said.contains("served   by the daemon at"), "{said}");
    assert!(
        said.contains("MARKED"),
        "the mark travels with the answer: {said}"
    );
    assert!(
        said.contains("model loaded loaded"),
        "the first request loads: {said}"
    );

    let again = text(&machine.run(&[
        "ask",
        "lab/fixture:a-model-that-runs.gguf",
        "--prompt",
        "yes",
        "--limit",
        "2",
    ]));
    assert!(again.contains("model loaded resident"), "{again}");
    let status = text(&machine.run(&["status"]));
    assert!(
        status.contains("resident: ") && status.contains("a-model-that-runs.gguf"),
        "{status}"
    );

    let log = text(&machine.run(&["log", "--kind", "generated"]));
    assert!(log.contains("generated"), "{log}");
    assert!(log.contains("MARKED degraded"), "{log}");

    let missing = machine.run(&["ask", "lab/fixture:no-such.gguf", "--prompt", "x"]);
    assert!(!missing.status.success());

    let stopped = machine.run(&["stop", "--because", "the whole-system test is done"]);
    assert!(stopped.status.success(), "{}", error_text(&stopped));
    assert!(serving.0.wait().expect("the daemon exits").success());
}

#[test]
fn a_daemon_killed_mid_generation_leaves_a_client_that_says_so_and_a_record_that_opens() {
    use std::io::Read as _;

    let machine = Machine::new("killed-mid-stream");
    let models = machine
        .0
        .join("mcf")
        .join("models")
        .join("lab")
        .join("fixture");
    std::fs::create_dir_all(&models).expect("a store");
    std::fs::write(
        models.join("a-model-that-runs.gguf"),
        mcf_standin::fixture::a_model_that_runs(),
    )
    .expect("a model file");

    let mut daemon = machine
        .command(&["serve"])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
        .expect("the daemon starts");
    {
        use std::io::BufRead as _;
        let stdout = daemon.stdout.as_mut().expect("it prints where it is");
        let mut line = String::new();
        let _read = std::io::BufReader::new(stdout).read_line(&mut line);
        assert!(line.contains("mcf is up"), "{line}");
    }

    let mut client = machine
        .command(&[
            "ask",
            "lab/fixture:a-model-that-runs.gguf",
            "--prompt",
            "yes",
            "--limit",
            "1000000",
        ])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("the client starts");
    let mut stdout = client.stdout.take().expect("piped");
    let mut first = [0_u8; 1];
    let _got = stdout.read(&mut first);
    let _killed = daemon.kill();
    let _reaped = daemon.wait();

    let mut rest = Vec::new();
    let _read = stdout.read_to_end(&mut rest);
    let mut err = String::new();
    let _read = client
        .stderr
        .take()
        .expect("piped")
        .read_to_string(&mut err);
    let status = client.wait().expect("the client exits");
    assert!(
        !status.success(),
        "a stream without its account is not served"
    );
    assert!(
        err.contains("the stream ended before its account"),
        "the client did not say what happened:\n{err}"
    );
    assert!(
        err.contains("token(s) were received"),
        "what was received is counted, not dropped (A4):\n{err}"
    );

    if machine.journal().exists() {
        let _replayed = replay(&machine.journal()).expect("the record opens after the kill");
    }

    let mut next = machine
        .command(&["serve"])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .expect("a second daemon spawns");
    let mut answered = false;
    for _ in 0..200 {
        if machine.run(&["status"]).status.success() {
            answered = true;
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    assert!(answered, "no daemon answered after the kill");
    let stopped = machine.run(&["stop", "--because", "the kill test is done"]);
    assert!(stopped.status.success(), "{}", error_text(&stopped));
    assert!(next.wait().expect("the daemon exits").success());
}

/// A port nothing is on, asked of the kernel rather than picked and hoped for:
/// a test that fails and leaves an engine behind must not take the next run
/// down with it.
fn anything_answering_on(port: u16) -> bool {
    use std::io::Write as _;
    let Ok(mut connection) = std::net::TcpStream::connect(("127.0.0.1", port)) else {
        return false;
    };
    write!(
        connection,
        "GET /health HTTP/1.1\r\nHost: localhost\r\n\r\n"
    )
    .is_ok()
}

fn a_free_port() -> u16 {
    std::net::TcpListener::bind(("127.0.0.1", 0))
        .expect("the kernel has a port")
        .local_addr()
        .expect("it has an address")
        .port()
}

struct Reaped(std::process::Child);

impl Drop for Reaped {
    fn drop(&mut self) {
        let _killed = self.0.kill();
        let _waited = self.0.wait();
    }
}

/// A machine with two models it can hold, an engine that answers, and a
/// daemon up and waiting.
fn a_machine_ready_to_host(named: &str) -> (Machine, Reaped) {
    let machine = Machine::new(named);
    let models = machine
        .0
        .join("mcf")
        .join("models")
        .join("lab")
        .join("fixture");
    std::fs::create_dir_all(&models).expect("a store");
    for name in ["first.gguf", "second.gguf"] {
        std::fs::write(
            models.join(name),
            mcf_standin::fixture::a_model_that_can_be_hosted(),
        )
        .expect("a model file");
    }
    let _engine = fake_provisioned_engine(&machine, "answers");
    let mut serving = Reaped(
        machine
            .command(&["serve"])
            .stdout(std::process::Stdio::piped())
            .spawn()
            .expect("the daemon starts"),
    );
    {
        use std::io::BufRead as _;
        let stdout = serving.0.stdout.as_mut().expect("it prints where it is");
        let mut line = String::new();
        let _read = std::io::BufReader::new(stdout).read_line(&mut line);
        assert!(line.contains("mcf is up on"), "{line}");
    }
    (machine, serving)
}

/// `mcf host` end to end: a model held on a port, asked a question through
/// the endpoint, and let go — with the record showing what it did.
#[test]
fn a_model_is_held_asked_and_let_go() {
    let (machine, mut serving) = a_machine_ready_to_host("held-asked-let-go");

    let first_port = a_free_port().to_string();
    let held = machine.run(&["host", "lab/fixture:first.gguf", "--port", &first_port]);
    assert!(held.status.success(), "{}", error_text(&held));
    let said = text(&held);
    assert!(said.contains("first"), "what it holds is not named: {said}");

    let hosted = text(&machine.run(&["hosted"]));
    assert!(hosted.contains("first"), "the model is not held: {hosted}");
    assert!(
        hosted.contains(&first_port),
        "the port is not named: {hosted}"
    );

    // Asked through the hold rather than by naming a file: the point of
    // hosting is that a question goes to whatever is up.
    let answered = machine.run(&["ask", "--prompt", "where"]);
    assert!(answered.status.success(), "{}", error_text(&answered));
    let answer = text(&answered);
    assert!(
        answer.starts_with(STAND_IN_ANSWER),
        "the held engine did not answer: {answer}"
    );
    assert!(
        answer.contains("served   by the daemon at"),
        "the answer does not say who served it: {answer}"
    );

    // One model at a time: holding the second lets the first go.
    let second_port = a_free_port().to_string();
    let second = machine.run(&["host", "lab/fixture:second.gguf", "--port", &second_port]);
    assert!(second.status.success(), "{}", error_text(&second));
    let hosted = text(&machine.run(&["hosted"]));
    assert!(
        hosted.contains("second"),
        "the second is not held: {hosted}"
    );
    assert!(
        !hosted.contains("first.gguf"),
        "both are held, and one runs at a time: {hosted}"
    );
    // The daemon's own view saying one thing is held proves nothing on its
    // own: what proves the first was let go is that its engine is gone from
    // the port it was on.
    let first_port: u16 = first_port.parse().expect("a port");
    let mut freed = false;
    for _ in 0..40 {
        if !anything_answering_on(first_port) {
            freed = true;
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(250));
    }
    assert!(
        freed,
        "the first model's engine is still answering on {first_port}, so hosting the second \
         did not let it go"
    );

    let freed = machine.run(&["unhost"]);
    assert!(freed.status.success(), "{}", error_text(&freed));
    let hosted = text(&machine.run(&["hosted"]));
    assert!(
        hosted.contains("nothing is being hosted"),
        "unhosting left it held: {hosted}"
    );

    let _stopped = machine.run(&["stop", "--because", "the hosting test is done"]);
    let _waited = serving.0.wait();

    // The record says what was held and what the hold did.
    let log = text(&machine.run(&["log", "--full"]));
    assert!(
        log.contains("first.gguf"),
        "the hold is not recorded: {log}"
    );
    assert!(
        log.contains("tokens_predicted_total") || log.contains("generated_tokens"),
        "what the hold produced is not recorded: {log}"
    );
}

fn fake_provisioned_engine(machine: &Machine, does: &str) -> PathBuf {
    use std::os::unix::fs::PermissionsExt as _;
    let prefix = machine
        .0
        .join("mcf")
        .join("provisioned")
        .join("llama.cpp@fakefakefake");
    let bin = prefix.join("build").join("bin");
    std::fs::create_dir_all(&bin).expect("a prefix");
    std::fs::write(
        prefix.join("mcf-provenance.json"),
        "{\"component\":\"llama.cpp\",\"commit\":\"fakefakefakefakefakefakefakefakefakefake\"}\n",
    )
    .expect("provenance");
    let me = std::env::current_exe().expect("this test binary has a path");
    let script = format!(
        "#!/bin/sh\nsock=\nport=\nwhile [ $# -gt 0 ]; do if [ \"$1\" = --host ]; then \
         case \"$2\" in /*) sock=\"$2\";; esac; fi; if [ \"$1\" = --port ]; then port=\"$2\"; fi; \
         if [ \"$1\" = --list-devices ]; then echo 'Available devices:'; exit 0; fi; \
         shift; done\nMCF_FAKE_LLAMA_SERVER_SOCKET=\"$sock\" \
         MCF_FAKE_LLAMA_SERVER_PORT=\"$port\" MCF_FAKE_LLAMA_SERVER_DOES={does} \
         exec \"{}\" --exact fake_llama_server --nocapture --test-threads 1\n",
        me.display()
    );
    let tool = bin.join("llama-server");
    std::fs::write(&tool, script).expect("the engine written");
    std::fs::set_permissions(&tool, std::fs::Permissions::from_mode(0o755)).expect("executable");
    std::fs::write(bin.join("llama-completion"), "#!/bin/sh\nexit 1\n").expect("the tool");
    prefix
}

fn dies_part_way(connection: &mut dyn std::io::Write) -> ! {
    let mut body = String::new();
    for piece in ["Paris is ", "the capital"] {
        let event = format!("data: {{\"content\":\"{piece}\",\"tokens\":[1]}}\n\n");
        let framed = format!("{:x}\r\n{event}\r\n", event.len());
        body.push_str(&framed);
    }
    let _written = write!(
        connection,
        "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\n\
         Transfer-Encoding: chunked\r\n\r\n{body}"
    );
    let _flushed = connection.flush();
    eprintln!("segmentation fault, or thereabouts");
    #[allow(
        clippy::exit,
        reason = "the stand-in engine dies here, which is what it is for"
    )]
    std::process::exit(139);
}

const STAND_IN_ANSWER: &str = "Paris is the capital.";

/// What the stand-in says it has done, in the shape llama.cpp publishes.
const ENGINE_METRICS: &str = "\
# HELP llamacpp:prompt_tokens_total Number of prompt tokens processed.
# TYPE llamacpp:prompt_tokens_total counter
llamacpp:prompt_tokens_total 12
# HELP llamacpp:tokens_predicted_total Number of generation tokens processed.
# TYPE llamacpp:tokens_predicted_total counter
llamacpp:tokens_predicted_total 34
# HELP llamacpp:requests_processing Number of requests processing.
# TYPE llamacpp:requests_processing gauge
llamacpp:requests_processing 0
";

/// Where the stand-in listens. `mcf host` puts an engine on a port and asks
/// it for `/health`; everything else reaches it over a socket in the runtime
/// directory. One server answers both, because what it answers is the same.
enum Listening {
    Socket(std::os::unix::net::UnixListener),
    Port(std::net::TcpListener),
}

impl Listening {
    fn accept(&self) -> Option<Box<dyn ReadWrite>> {
        match self {
            Self::Socket(held) => held.accept().ok().map(|(held, _)| {
                let held: Box<dyn ReadWrite> = Box::new(held);
                held
            }),
            Self::Port(held) => held.accept().ok().map(|(held, _)| {
                let held: Box<dyn ReadWrite> = Box::new(held);
                held
            }),
        }
    }
}

trait ReadWrite: std::io::Read + std::io::Write {}
impl<T: std::io::Read + std::io::Write> ReadWrite for T {}

#[test]
fn fake_llama_server() {
    use std::io::{Read as _, Write as _};
    let socket = std::env::var("MCF_FAKE_LLAMA_SERVER_SOCKET").unwrap_or_default();
    let port: Option<u16> = std::env::var("MCF_FAKE_LLAMA_SERVER_PORT")
        .ok()
        .and_then(|held| held.trim().parse().ok());
    let listening = match (port, socket.is_empty()) {
        (Some(port), _) => Listening::Port(
            std::net::TcpListener::bind(("127.0.0.1", port)).expect("the port binds"),
        ),
        (None, false) => {
            assert!(
                socket.starts_with('/'),
                "a stand-in binds a socket only at an absolute path, never in the source tree: {socket}"
            );
            let _gone = std::fs::remove_file(&socket);
            Listening::Socket(
                std::os::unix::net::UnixListener::bind(&socket).expect("the socket binds"),
            )
        }
        (None, true) => return,
    };
    let does = std::env::var("MCF_FAKE_LLAMA_SERVER_DOES").unwrap_or_default();
    loop {
        let Some(mut connection) = listening.accept() else {
            continue;
        };
        let mut raw = Vec::new();
        let mut chunk = [0_u8; 4096];
        let (head, body) = loop {
            let Ok(read) = connection.read(&mut chunk) else {
                break (String::new(), String::new());
            };
            raw.extend_from_slice(chunk.get(..read).unwrap_or_default());
            let text = String::from_utf8_lossy(&raw).into_owned();
            if let Some((head, body)) = text.split_once("\r\n\r\n") {
                let length: usize = head
                    .lines()
                    .find_map(|line| line.strip_prefix("Content-Length: "))
                    .and_then(|held| held.trim().parse().ok())
                    .unwrap_or(0);
                if body.len() >= length || read == 0 {
                    break (head.to_owned(), body.to_owned());
                }
            }
            if read == 0 {
                break (text, String::new());
            }
        };
        let path = head.split_whitespace().nth(1).unwrap_or("");
        if path == "/completion" && does == "dies_part_way" {
            dies_part_way(&mut *connection);
        }
        if path == "/metrics" {
            let _written = write!(
                connection,
                "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: {}\r\n\
                 Connection: close\r\n\r\n{ENGINE_METRICS}",
                ENGINE_METRICS.len()
            );
            continue;
        }
        let answer = stand_in_answer(path, &body, &does);
        let _written = write!(
            connection,
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\
             Connection: close\r\n\r\n{answer}",
            answer.len()
        );
    }
}

fn stand_in_answer(path: &str, body: &str, does: &str) -> String {
    match path {
        "/health" => "{\"status\":\"ok\"}".to_owned(),
        "/tokenize" => {
            let content = body
                .split_once("\"content\":\"")
                .and_then(|(_, rest)| rest.split_once('"'))
                .map_or("", |(content, _)| content);
            let tokens: Vec<String> = content
                .bytes()
                .map(|byte| format!("{{\"id\":{byte},\"piece\":\"{}\"}}", char::from(byte)))
                .collect();
            format!("{{\"tokens\":[{}]}}", tokens.join(","))
        }
        "/completion" if does == "dies" => {
            eprintln!("segmentation fault, or thereabouts");
            #[allow(
                clippy::exit,
                reason = "the stand-in engine dies here, which is what it is for"
            )]
            std::process::exit(139);
        }
        "/completion" => {
            let sent = body.matches(',').count();
            let said: Vec<String> = STAND_IN_ANSWER
                .bytes()
                .map(|byte| byte.to_string())
                .chain(std::iter::once("0".to_owned()))
                .collect();
            format!(
                "{{\"content\":\"{STAND_IN_ANSWER}\",\"tokens_predicted\":{},\
                     \"tokens_evaluated\":{sent},\"stop_type\":\"eos\",\"tokens\":[{}]}}",
                said.len(),
                said.join(",")
            )
        }
        "/detokenize" => {
            let content: String = body
                .split_once("\"tokens\":[")
                .and_then(|(_, rest)| rest.split_once(']'))
                .map_or("", |(tokens, _)| tokens)
                .split(',')
                .filter_map(|token| token.trim().parse::<u8>().ok())
                .filter(|byte| *byte != 0)
                .map(char::from)
                .collect();
            format!("{{\"content\":\"{content}\"}}")
        }
        _ => "{\"error\":{\"message\":\"the stand-in does not answer that\"}}".to_owned(),
    }
}

#[test]
fn a_provisioned_engine_is_chosen_streamed_and_named() {
    let machine = Machine::new("provisioned-engine");
    let models = machine
        .0
        .join("mcf")
        .join("models")
        .join("lab")
        .join("fixture");
    std::fs::create_dir_all(&models).expect("a store");
    std::fs::write(
        models.join("a-model-that-runs.gguf"),
        mcf_standin::fixture::a_model_that_runs(),
    )
    .expect("a file");
    fake_provisioned_engine(&machine, "answers");

    let mut serving = Reaped(
        machine
            .command(&["serve"])
            .stdout(std::process::Stdio::piped())
            .spawn()
            .expect("the daemon starts"),
    );
    {
        use std::io::BufRead as _;
        let stdout = serving.0.stdout.as_mut().expect("it prints where it is");
        let mut line = String::new();
        let _read = std::io::BufReader::new(stdout).read_line(&mut line);
        assert!(line.contains("mcf is up on"), "{line}");
    }

    let ran = machine.run(&[
        "ask",
        "lab/fixture:a-model-that-runs.gguf",
        "--prompt",
        "yes",
    ]);
    assert!(ran.status.success(), "{}", error_text(&ran));
    let said = text(&ran);
    assert!(said.starts_with("Paris is the capital."), "{said}");
    assert!(
        said.contains("provisioned llama.cpp server @fakefakefake"),
        "{said}"
    );
    assert!(said.contains("a real engine"), "{said}");
    assert!(
        !said.contains("MARKED"),
        "a real engine is not marked degraded: {said}"
    );

    let own = machine.run(&[
        "ask",
        "lab/fixture:a-model-that-runs.gguf",
        "--prompt",
        "yes",
        "--engine",
        "stand-in",
    ]);
    assert!(own.status.success(), "{}", error_text(&own));
    assert!(text(&own).contains("stand-in"), "{}", text(&own));

    let stopped = machine.run(&["stop", "--because", "done"]);
    assert!(stopped.status.success(), "{}", error_text(&stopped));
    assert!(serving.0.wait().expect("the daemon exits").success());
}

#[test]
fn what_the_engine_wrote_before_it_died_is_on_the_page() {
    let machine = Machine::new("provisioned-dies-part-way");
    let models = machine
        .0
        .join("mcf")
        .join("models")
        .join("lab")
        .join("fixture");
    std::fs::create_dir_all(&models).expect("a store");
    std::fs::write(
        models.join("m.gguf"),
        mcf_standin::fixture::a_model_that_runs(),
    )
    .expect("a file");
    fake_provisioned_engine(&machine, "dies_part_way");

    let mut serving = Reaped(
        machine
            .command(&["serve"])
            .stdout(std::process::Stdio::piped())
            .spawn()
            .expect("the daemon starts"),
    );
    {
        use std::io::BufRead as _;
        let stdout = serving.0.stdout.as_mut().expect("it prints where it is");
        let mut line = String::new();
        let _read = std::io::BufReader::new(stdout).read_line(&mut line);
        assert!(line.contains("mcf is up on"), "{line}");
    }

    let ran = machine.run(&["ask", "lab/fixture:m.gguf", "--prompt", "yes"]);
    assert!(!ran.status.success());
    let said = text(&ran);
    assert!(said.starts_with("Paris is "), "what arrived: {said:?}");
    let out = format!("{said}{}", error_text(&ran));
    assert!(out.contains("engine.exit.midstream"), "{out}");
    assert!(
        out.contains("segmentation fault, or thereabouts"),
        "the engine's own words: {out}"
    );

    let status = machine.run(&["status"]);
    assert!(status.status.success(), "the daemon died with its engine");
    let stopped = machine.run(&["stop", "--because", "done"]);
    assert!(stopped.status.success(), "{}", error_text(&stopped));
    assert!(serving.0.wait().expect("the daemon exits").success());
}

#[test]
fn a_provisioned_engine_that_dies_mid_answer_leaves_a_partial_answer_and_a_daemon() {
    let machine = Machine::new("provisioned-dies");
    let models = machine
        .0
        .join("mcf")
        .join("models")
        .join("lab")
        .join("fixture");
    std::fs::create_dir_all(&models).expect("a store");
    std::fs::write(
        models.join("m.gguf"),
        mcf_standin::fixture::a_model_that_runs(),
    )
    .expect("a file");
    fake_provisioned_engine(&machine, "dies");

    let mut serving = Reaped(
        machine
            .command(&["serve"])
            .stdout(std::process::Stdio::piped())
            .spawn()
            .expect("the daemon starts"),
    );
    {
        use std::io::BufRead as _;
        let stdout = serving.0.stdout.as_mut().expect("it prints where it is");
        let mut line = String::new();
        let _read = std::io::BufReader::new(stdout).read_line(&mut line);
        assert!(line.contains("mcf is up on"), "{line}");
    }

    let ran = machine.run(&["ask", "lab/fixture:m.gguf", "--prompt", "yes"]);
    assert!(!ran.status.success());
    let out = format!("{}{}", text(&ran), error_text(&ran));
    assert!(out.contains("engine.exit.midstream"), "{out}");
    assert!(
        out.contains("segmentation fault, or thereabouts"),
        "the engine's own words: {out}"
    );

    let status = machine.run(&["status"]);
    assert!(status.status.success(), "the daemon died with its engine");
    let log = text(&machine.run(&["log", "--kind", "generated"]));
    assert!(log.contains("generated"), "{log}");

    let stopped = machine.run(&["stop", "--because", "done"]);
    assert!(stopped.status.success(), "{}", error_text(&stopped));
    assert!(serving.0.wait().expect("the daemon exits").success());
}
