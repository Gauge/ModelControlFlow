//! Nothing a repository sends can reach a privileged operation (B-180, A26,
//! §6.20, §6.4).
//!
//! **What B-180 asks for, and why it is a scenario rather than a policy.** *An
//! adversarial model and hostile repository code both fail to touch a
//! privileged operation.* A rule saying MCF does not do that is worth nothing:
//! the question is whether a hostile input can *cause* it, and the only honest
//! way to ask is to make the privileged things observable and then be hostile.
//!
//! **How the observation is built** (D26 — the observable, not its cause). A
//! directory is put first on `PATH` holding executables named for everything a
//! program reaches for when it wants rights it was not given: `mcf-helper`,
//! `sudo`, `pkexec`, `doas`, `nvidia-smi`. Each of them does one thing — append
//! its name and its arguments to a witness file — and then exits. If anything
//! MCF does while handling a hostile artifact starts one of them, the witness
//! says so, with the arguments that reached it.
//!
//! **What is hostile here.** A model whose metadata is a shell command, a path
//! traversal, a format specifier and a governor name; a repository whose file
//! names are arguments to the privileged helper; and a reference that tries to
//! climb out of the store. Then every surface that touches an artifact is run
//! against them: `pull`, `list`, `explain`, `run`, `rm`.
//!
//! **What this asserts today and what it will assert later.** Today MCF starts
//! none of those programs at all, so the witness is empty by construction —
//! which is a stronger claim than B-180 asks for and the right one to hold
//! while it is true (`nothing_acquired_is_ever_run` holds the other half). When
//! the daemon starts the helper for a legitimate reason, this test is where the
//! distinction is kept: the helper may be started, and never with anything an
//! artifact could influence.

#![allow(clippy::panic, clippy::expect_used)]

use std::path::{Path, PathBuf};
use std::process::Command;

/// Everything a program reaches for when it wants rights it was not started
/// with, plus the vendor's tool the one privileged operation uses.
const PRIVILEGED: &[&str] = &["mcf-helper", "sudo", "pkexec", "doas", "nvidia-smi", "su"];

fn binary() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_mcf"))
}

/// A machine of its own, with a witnessed `PATH`.
struct Watched {
    root: PathBuf,
}

impl Watched {
    fn new(name: &str) -> Self {
        let root = std::env::temp_dir().join(format!("mcf-elevate-{name}-{}", std::process::id()));
        let _removed = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("bin")).expect("a place for the sentinels");
        let watched = Self { root };
        for program in PRIVILEGED {
            watched.sentinel(program);
        }
        watched
    }

    /// An executable that records being started and does nothing else.
    fn sentinel(&self, name: &str) {
        let path = self.root.join("bin").join(name);
        let witness = self.witness();
        std::fs::write(
            &path,
            format!(
                "#!/bin/sh\nprintf '%s %s\\n' \"{name}\" \"$*\" >> {}\nexit 0\n",
                witness.display()
            ),
        )
        .expect("a sentinel");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt as _;
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755))
                .expect("an executable sentinel");
        }
    }

    fn witness(&self) -> PathBuf {
        self.root.join("witness")
    }

    /// What was started, if anything was.
    fn started(&self) -> String {
        std::fs::read_to_string(self.witness()).unwrap_or_default()
    }

    fn run(&self, arguments: &[&str]) -> std::process::Output {
        let mut command = Command::new(binary());
        command.args(arguments);
        command.env("XDG_DATA_HOME", &self.root);
        command.env("XDG_RUNTIME_DIR", &self.root);
        command.env_remove("HOME");
        // The sentinels first, so that anything MCF starts by name is one of
        // them rather than the machine's own.
        command.env("PATH", self.root.join("bin"));
        command.output().expect("the binary runs")
    }

    /// Where a sentinel lives, for a test that starts one deliberately.
    fn sentinel_path(&self, name: &str) -> PathBuf {
        self.root.join("bin").join(name)
    }

    fn models(&self) -> PathBuf {
        self.root.join("mcf/models")
    }
}

impl Drop for Watched {
    fn drop(&mut self) {
        let _removed = std::fs::remove_dir_all(&self.root);
    }
}

/// A GGUF whose metadata is every hostile string worth trying.
fn a_hostile_model() -> Vec<u8> {
    fn push_string(into: &mut Vec<u8>, text: &str) {
        into.extend_from_slice(&(text.len() as u64).to_le_bytes());
        into.extend_from_slice(text.as_bytes());
    }
    let fields: &[(&str, &str)] = &[
        ("general.architecture", "llama"),
        ("general.name", "; sudo mcf-helper governor performance #"),
        (
            "general.description",
            "$(pkexec nvidia-smi -c EXCLUSIVE_PROCESS)",
        ),
        ("general.author", "../../../../usr/bin/sudo"),
        ("general.licence", "`doas sh -c 'echo owned'`"),
        ("general.url", "--under / governor performance"),
    ];

    let mut bytes = b"GGUF".to_vec();
    bytes.extend_from_slice(&3_u32.to_le_bytes());
    bytes.extend_from_slice(&0_u64.to_le_bytes());
    bytes.extend_from_slice(&(fields.len() as u64).to_le_bytes());
    for (key, value) in fields {
        push_string(&mut bytes, key);
        bytes.extend_from_slice(&8_u32.to_le_bytes());
        push_string(&mut bytes, value);
    }
    bytes
}

/// Every surface that touches an artifact, against a hostile one, with every
/// privileged program watched.
#[test]
fn a_hostile_model_reaches_no_privileged_program() {
    let watched = Watched::new("model");
    let held = watched.models().join("owner/model");
    std::fs::create_dir_all(&held).expect("a store");
    let model = held.join("model.gguf");
    std::fs::write(&model, a_hostile_model()).expect("a hostile model");

    // Everything an operator can do to a model on this machine.
    let listed = watched.run(&["list"]);
    let explained = watched.run(&["explain", "owner/model:model.gguf"]);
    let ran = watched.run(&["run", "owner/model:model.gguf", "what is this"]);
    let removed = watched.run(&[
        "rm",
        "owner/model/model.gguf",
        "--because",
        "the test is done",
    ]);

    assert_eq!(
        watched.started(),
        "",
        "handling a hostile model started a privileged program"
    );

    // And the hostile strings came back as strings: what is in a model file is
    // data, whatever it looks like (§3.7).
    let said = String::from_utf8_lossy(&explained.stdout).into_owned();
    assert!(
        said.contains("; sudo mcf-helper governor performance #"),
        "the metadata was not shown as written: {said}"
    );
    for output in [&listed, &ran, &removed] {
        let text = String::from_utf8_lossy(&output.stdout);
        assert!(
            !text.contains("EXCLUSIVE_PROCESS"),
            "a surface interpolated a hostile string: {text}"
        );
    }
}

/// A repository whose *file names* are arguments to the privileged helper.
///
/// The hostile input MCF is most likely to hand to something else: a name it
/// got from a hub, used to build a path, a message or — one day — a command
/// line. Nothing is started, and the names come back as names.
#[test]
fn a_repository_of_hostile_names_reaches_no_privileged_program() {
    let watched = Watched::new("names");
    let held = watched.models().join("owner/model");
    std::fs::create_dir_all(&held).expect("a store");
    for name in [
        "governor performance.gguf",
        "--under=slash.gguf",
        "; sudo -s .gguf",
        "$(nvidia-smi).gguf",
    ] {
        std::fs::write(held.join(name), b"not a model").expect("a hostile name");
    }

    let listed = watched.run(&["list"]);
    assert!(listed.status.success(), "listing a hostile store failed");
    assert_eq!(
        watched.started(),
        "",
        "listing a store of hostile names started a privileged program"
    );

    let said = String::from_utf8_lossy(&listed.stdout);
    assert!(
        said.contains("; sudo -s .gguf"),
        "a name was not shown as written: {said}"
    );
}

/// A reference that tries to climb out of the store, through the surfaces that
/// resolve one.
#[test]
fn a_reference_that_climbs_out_reaches_no_privileged_program() {
    let watched = Watched::new("climb");
    std::fs::create_dir_all(watched.models()).expect("a store");

    for reference in [
        "../../../../etc/shadow",
        "owner/model:../../../../usr/bin/sudo",
        "/usr/bin/pkexec",
        "owner/../../model:model.gguf",
    ] {
        for surface in ["explain", "run"] {
            let output = watched.run(&[surface, reference, "hello"]);
            assert!(
                !output.status.success(),
                "{surface} accepted {reference}: {}",
                String::from_utf8_lossy(&output.stdout)
            );
        }
    }
    assert_eq!(
        watched.started(),
        "",
        "a reference that climbs out of the store started a privileged program"
    );
}

/// Runs a freshly-written executable, retrying `ETXTBSY`.
///
/// Not a defect in MCF and not a flake to paper over: a program that writes an
/// executable and runs it is racing every other thread in its own process,
/// because a `fork` between the write and the close hands the child a writable
/// descriptor and `exec` refuses it. The tests here run in parallel and write
/// sentinels, so they hit it. Retrying is the fix that does not serialize the
/// suite.
fn started(program: &Path, arguments: &[&str]) -> std::process::Output {
    for _ in 0..50 {
        match Command::new(program).args(arguments).output() {
            Ok(outcome) => return outcome,
            Err(error) if error.kind() == std::io::ErrorKind::ExecutableFileBusy => {
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
            Err(error) => panic!("the sentinel would not run: {error}"),
        }
    }
    panic!("the sentinel was busy for half a second")
}

/// The sentinels work.
///
/// A test whose evidence is *nothing happened* is worthless if nothing could
/// have happened: this starts one of the sentinels deliberately and asserts the
/// witness catches it. Without this, every assertion above would pass on a
/// broken harness (A19).
#[test]
fn the_witness_catches_a_privileged_program_that_is_started() {
    let watched = Watched::new("witness");
    let sentinel = watched.sentinel_path("sudo");
    let outcome = started(&sentinel, &["mcf-helper", "governor", "performance"]);
    assert!(outcome.status.success());

    let started = watched.started();
    assert!(started.contains("sudo"), "the witness recorded nothing");
    assert!(
        started.contains("governor performance"),
        "the witness lost the arguments: {started}"
    );
}
