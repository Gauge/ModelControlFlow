#![allow(clippy::panic, clippy::expect_used)]

use std::path::{Path, PathBuf};
use std::process::Command;

const PRIVILEGED: &[&str] = &["mcf-helper", "sudo", "pkexec", "doas", "nvidia-smi", "su"];

fn binary() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_mcf"))
}

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

    fn started(&self) -> String {
        std::fs::read_to_string(self.witness()).unwrap_or_default()
    }

    fn run(&self, arguments: &[&str]) -> std::process::Output {
        let mut command = Command::new(binary());
        command.args(arguments);
        command.env("XDG_DATA_HOME", &self.root);
        command.env("XDG_RUNTIME_DIR", &self.root);
        command.env_remove("HOME");
        command.env("PATH", self.root.join("bin"));
        command.output().expect("the binary runs")
    }

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

#[test]
fn a_hostile_model_reaches_no_privileged_program() {
    let watched = Watched::new("model");
    let held = watched.models().join("owner/model");
    std::fs::create_dir_all(&held).expect("a store");
    let model = held.join("model.gguf");
    std::fs::write(&model, a_hostile_model()).expect("a hostile model");

    let listed = watched.run(&["list"]);
    let explained = watched.run(&["explain", "owner/model:model.gguf"]);
    let ran = watched.run(&["ask", "owner/model:model.gguf", "--prompt", "what is this"]);
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
