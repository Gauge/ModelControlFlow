use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::corpus::Task;

pub const IMAGE: &str = "docker.io/library/python@sha256:\
     09f7da3bc104798d0afb40bc08d23ab2da20a76130cec1f2ef170848f5d85217";

const A_TASK: Duration = Duration::from_secs(20);
const MEMORY: &str = "512m";
const PROCESSES: &str = "128";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Marked {
    By(Vec<(String, bool)>),
    Unmarked(String),
}

impl Marked {
    #[must_use]
    pub fn or_unmarked(self, tasks: &[Task]) -> Vec<(String, bool)> {
        match self {
            Self::By(held) => held,
            Self::Unmarked(_) => tasks
                .iter()
                .map(|task| (task.name.clone(), false))
                .collect(),
        }
    }

    #[must_use]
    pub fn why(&self) -> Option<&str> {
        match self {
            Self::By(_) => None,
            Self::Unmarked(why) => Some(why),
        }
    }
}

#[must_use]
pub fn where_podman_is() -> Option<PathBuf> {
    for held in ["/usr/bin/podman", "/usr/local/bin/podman", "/bin/podman"] {
        let path = PathBuf::from(held);
        if path.is_file() {
            return Some(path);
        }
    }
    None
}

#[must_use]
pub fn the_runner() -> String {
    let seconds = A_TASK.as_secs();
    format!(
        "import subprocess, sys, pathlib\n\
         for path in sorted(pathlib.Path('/work').glob('task-*.py')):\n\
        \x20   name = path.stem\n\
        \x20   try:\n\
        \x20       done = subprocess.run([sys.executable, str(path)], capture_output=True,\n\
        \x20                             timeout={seconds})\n\
        \x20       print(name, 'PASS' if done.returncode == 0 else 'FAIL', flush=True)\n\
        \x20   except subprocess.TimeoutExpired:\n\
        \x20       print(name, 'FAIL', flush=True)\n"
    )
}

#[must_use]
pub fn a_task_file(block: &str, checked: &str) -> String {
    format!("{block}\n\n{checked}\n")
}

pub fn laid_out(room: &Path, tasks: &[Task], blocks: &[String]) -> Result<usize, String> {
    std::fs::create_dir_all(room).map_err(|error| error.to_string())?;
    let mut written: usize = 0;
    for (at, task) in tasks.iter().enumerate() {
        let Some(block) = blocks.get(at) else {
            continue;
        };
        if block.trim().is_empty() {
            continue;
        }
        let number = at.saturating_add(1);
        let path = room.join(format!("task-{number:02}.py"));
        std::fs::write(&path, a_task_file(block, &task.checked))
            .map_err(|error| error.to_string())?;
        written = written.saturating_add(1);
    }
    std::fs::write(room.join("mark.py"), the_runner()).map_err(|error| error.to_string())?;
    Ok(written)
}

#[must_use]
pub fn read_the_verdicts(said: &str, tasks: &[Task]) -> Vec<(String, bool)> {
    let mut held: Vec<(String, bool)> = tasks
        .iter()
        .map(|task| (task.name.clone(), false))
        .collect();
    for line in said.lines() {
        let mut parts = line.split_whitespace();
        let Some(name) = parts.next() else { continue };
        let passed = parts.next() == Some("PASS");
        let Some(number) = name.strip_prefix("task-") else {
            continue;
        };
        let Ok(number) = number.parse::<usize>() else {
            continue;
        };
        let Some(at) = number.checked_sub(1) else {
            continue;
        };
        if let Some(slot) = held.get_mut(at) {
            slot.1 = passed;
        }
    }
    held
}

#[must_use]
pub fn arguments(room: &Path) -> Vec<String> {
    vec![
        "run".to_owned(),
        "--rm".to_owned(),
        "--network=none".to_owned(),
        format!("--memory={MEMORY}"),
        format!("--pids-limit={PROCESSES}"),
        "--read-only".to_owned(),
        "--tmpfs".to_owned(),
        "/tmp:size=64m".to_owned(),
        "--security-opt".to_owned(),
        "no-new-privileges".to_owned(),
        "--cap-drop=ALL".to_owned(),
        "-v".to_owned(),
        format!("{}:/work:ro,z", room.display()),
        IMAGE.to_owned(),
        "python3".to_owned(),
        "/work/mark.py".to_owned(),
    ]
}

#[must_use]
pub fn marked(room: &Path, tasks: &[Task], answer: &str, patience: Duration) -> Marked {
    let Some(podman) = where_podman_is() else {
        return Marked::Unmarked(
            "podman is not installed, and a model's own code is only ever run inside a \
             container"
                .to_owned(),
        );
    };
    let blocks = crate::trial::blocks(answer);
    if blocks.is_empty() {
        return Marked::By(
            tasks
                .iter()
                .map(|task| (task.name.clone(), false))
                .collect(),
        );
    }
    let written = match laid_out(room, tasks, &blocks) {
        Ok(written) => written,
        Err(why) => return Marked::Unmarked(why),
    };
    if written == 0 {
        return Marked::By(
            tasks
                .iter()
                .map(|task| (task.name.clone(), false))
                .collect(),
        );
    }
    let mut child = match std::process::Command::new(&podman)
        .env_remove("XDG_DATA_HOME")
        .args(arguments(room))
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
    {
        Ok(child) => child,
        Err(error) => return Marked::Unmarked(format!("podman could not be started: {error}")),
    };
    let said = waited_for(&mut child, patience);
    match said {
        Ok(said) => Marked::By(read_the_verdicts(&said, tasks)),
        Err(why) => Marked::Unmarked(why),
    }
}

fn waited_for(child: &mut std::process::Child, patience: Duration) -> Result<String, String> {
    use std::io::Read as _;
    let mut said = String::new();
    if let Some(mut output) = child.stdout.take() {
        let (send, heard) = std::sync::mpsc::channel();
        let _reader = std::thread::spawn(move || {
            let mut held = String::new();
            let _read = output.read_to_string(&mut held);
            let _sent = send.send(held);
        });
        let Ok(held) = heard.recv_timeout(patience) else {
            let _killed = child.kill();
            let _reaped = child.wait();
            return Err("the container did not finish in the time allowed".to_owned());
        };
        said = held;
    }
    let _status = child.wait();
    Ok(said)
}

#[cfg(test)]
mod tests;
