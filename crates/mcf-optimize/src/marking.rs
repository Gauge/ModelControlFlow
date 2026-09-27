use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::corpus::Task;

pub const IMAGE: &str = "docker.io/library/python@sha256:\
     09f7da3bc104798d0afb40bc08d23ab2da20a76130cec1f2ef170848f5d85217";

const A_TASK: Duration = Duration::from_secs(20);
const MEMORY: &str = "512m";
const PROCESSES: &str = "128";

/// What one task's check made of an answer: how many of its claims held, out of how many it
/// makes. Counting the claims rather than the task tells a solution that got most of the way
/// there from one that did nothing, which is the difference a sampling setting moves.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Checked {
    pub name: String,
    pub passed: u32,
    pub of: u32,
}

impl Checked {
    #[must_use]
    pub const fn whole(&self) -> bool {
        self.of > 0 && self.passed >= self.of
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Marked {
    By(Vec<Checked>),
    Unmarked(String),
}

impl Marked {
    #[must_use]
    pub fn or_unmarked(self, tasks: &[Task]) -> Vec<Checked> {
        match self {
            Self::By(held) => held,
            Self::Unmarked(_) => nothing_held(tasks),
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

/// Every claim a set of tasks makes, with none of them held. What a set scores when the
/// model wrote nothing worth running.
/// Mark a set of short answers by reading them.
///
/// Nothing is run: each of these questions has one right answer, short enough to write on
/// a line, so marking is comparing what was written with what was asked for. That makes a
/// graded trial cost a line of output a question instead of a program, and it needs no
/// container — a correctness sweep can be run on a machine with no podman on it at all.
///
/// What a model writes around the answer is not held against it. Models bold things, add
/// units, put a full stop at the end, and write a thousand with a comma in it; none of
/// that is a wrong answer, and a marker that called it one would be measuring formatting.
#[must_use]
pub fn marked_by_reading(tasks: &[Task], answer: &str) -> Vec<Checked> {
    let said = answers_in(answer);
    tasks
        .iter()
        .enumerate()
        .map(|(at, task)| {
            let number = at.saturating_add(1);
            let given = said.iter().find(|(held, _)| *held == number);
            let right = given.is_some_and(|(_, given)| the_same(given, &task.checked));
            Checked {
                name: task.name.clone(),
                passed: u32::from(right),
                of: 1,
            }
        })
        .collect()
}

/// What a model gave as the answer to one numbered question, as it wrote it, so a verdict
/// can show the line that was marked beside the answer that was wanted.
#[must_use]
pub fn given(answer: &str, number: usize) -> Option<String> {
    // The first, because that is the one the marker reads.
    answers_in(answer)
        .into_iter()
        .find(|(held, _)| *held == number)
        .map(|(_, given)| given)
}

/// The answer lines a model wrote, by the number each one answers.
fn answers_in(said: &str) -> Vec<(usize, String)> {
    let mut held = Vec::new();
    for line in said.lines() {
        let line = line.trim();
        let Some(rest) = line.strip_prefix("### ANSWER") else {
            continue;
        };
        let rest = rest.trim_start();
        let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
        let Ok(number) = digits.parse::<usize>() else {
            continue;
        };
        let after = rest.get(digits.len()..).unwrap_or_default();
        let after = after.trim_start().trim_start_matches([':', '-', '.']);
        held.push((number, after.trim().to_owned()));
    }
    held
}

/// Whether what was written is the answer that was wanted.
fn the_same(given: &str, wanted: &str) -> bool {
    let given = bare(given);
    let wanted = bare(wanted);
    if given == wanted {
        return true;
    }
    // A number written with separators, a sign, or a decimal tail of nothing is the same
    // number. Compared as digits rather than parsed, so a count too large for any integer
    // MCF holds is still compared exactly.
    as_a_number(&given)
        .is_some_and(|given| as_a_number(&wanted).is_some_and(|wanted| given == wanted))
}

/// What is left of an answer once the decoration is taken off.
fn bare(held: &str) -> String {
    held.trim()
        .trim_matches(|ch: char| {
            ch.is_whitespace() || matches!(ch, '*' | '`' | '"' | '\'' | '.' | ',' | ';' | ':')
        })
        .to_ascii_lowercase()
}

/// A number's digits, where what is written is one: no separators, no leading sign, no
/// trailing nothings after a point.
fn as_a_number(held: &str) -> Option<String> {
    let held: String = held.chars().filter(|ch| *ch != ',' && *ch != '_').collect();
    let (sign, digits) = match held.strip_prefix('-') {
        Some(rest) => ("-", rest),
        None => ("", held.strip_prefix('+').unwrap_or(&held)),
    };
    let digits = match digits.split_once('.') {
        Some((whole, after)) if after.chars().all(|ch| ch == '0') => whole,
        Some(_) => return None,
        None => digits,
    };
    if digits.is_empty() || !digits.chars().all(|ch| ch.is_ascii_digit()) {
        return None;
    }
    let trimmed = digits.trim_start_matches('0');
    let digits = if trimmed.is_empty() { "0" } else { trimmed };
    Some(if digits == "0" {
        digits.to_owned()
    } else {
        format!("{sign}{digits}")
    })
}

#[must_use]
pub fn nothing_held(tasks: &[Task]) -> Vec<Checked> {
    tasks
        .iter()
        .map(|task| Checked {
            name: task.name.clone(),
            passed: 0,
            of: claims_in(&task.checked),
        })
        .collect()
}

/// How many claims a check makes. Counted off the source so that a task which never ran at
/// all still has a denominator: a model that wrote nothing scored nought out of nine, not
/// nought out of nothing. A focus run's claims are its steps.
#[must_use]
pub fn claims_in(checked: &str) -> u32 {
    if let Some(run) = crate::focus::Run::from_checked(checked) {
        return u32::try_from(run.changes.len()).unwrap_or(u32::MAX).max(1);
    }
    let held = checked
        .lines()
        .filter(|line| line.trim_start().starts_with("assert "))
        .count();
    u32::try_from(held).unwrap_or(u32::MAX).max(1)
}

/// What runs inside the container: each answer, then its check, with the check's claims
/// counted one by one rather than the whole thing standing or falling on the first one that
/// does not hold.
///
/// A claim is an `assert` in the check. They are rewritten to record themselves and carry
/// on, so a solution that is wrong in one place is still marked on the rest — and where the
/// code raises instead, what held up to that point is what is reported. A claim inside a
/// loop counts once and holds only if it held every time round.
#[must_use]
pub fn the_runner() -> String {
    let seconds = A_TASK.as_secs();
    let harness = ONE_TASK.replace('\n', "\\n").replace('"', "\\\"");
    format!(
        "import subprocess, sys, pathlib\n\
         one = pathlib.Path('/tmp/one.py')\n\
         one.write_text(\"{harness}\")\n\
         for path in sorted(pathlib.Path('/work').glob('task-*.py')):\n\
        \x20   name = path.stem\n\
        \x20   check = path.with_name(name.replace('task-', 'check-') + '.py')\n\
        \x20   passed = total = 0\n\
        \x20   try:\n\
        \x20       done = subprocess.run([sys.executable, str(one), str(path), str(check)],\n\
        \x20                             capture_output=True, timeout={seconds})\n\
        \x20       for line in done.stdout.decode('utf-8', 'replace').splitlines():\n\
        \x20           if line.startswith('#MCF '):\n\
        \x20               _mark, passed, total = line.split()\n\
        \x20   except subprocess.TimeoutExpired:\n\
        \x20       pass\n\
        \x20   print(name, passed, total, flush=True)\n"
    )
}

/// The harness that marks one answer against one check, written into the container's own
/// scratch space because the work it reads is mounted read only.
const ONE_TASK: &str = r"import ast, sys
block = open(sys.argv[1]).read()
check = open(sys.argv[2]).read()
tree = ast.parse(check)
total = 0
class Count(ast.NodeTransformer):
    def visit_Assert(self, node):
        global total
        at = total
        total += 1
        return ast.copy_location(ast.Expr(ast.Call(
            func=ast.Name(id='_mcf', ctx=ast.Load()),
            args=[ast.Constant(at), node.test], keywords=[])), node)
tree = Count().visit(tree)
ast.fix_missing_locations(tree)
failed = set()
ran = set()
def _mcf(at, ok):
    ran.add(at)
    if not ok:
        failed.add(at)
room = {'_mcf': _mcf}
try:
    exec(compile(block, 'answer', 'exec'), room)
    exec(compile(tree, 'check', 'exec'), room)
except BaseException:
    pass
print('#MCF', len(ran - failed), total)
";

/// The answer and the check go into two files rather than one. The check's claims are
/// rewritten before they run so each can be counted, and rewriting the model's own code
/// along with them would be marking something nobody wrote.
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
        std::fs::write(
            room.join(format!("task-{number:02}.py")),
            format!("{block}\n"),
        )
        .map_err(|error| error.to_string())?;
        std::fs::write(
            room.join(format!("check-{number:02}.py")),
            format!("{}\n", task.checked),
        )
        .map_err(|error| error.to_string())?;
        written = written.saturating_add(1);
    }
    std::fs::write(room.join("mark.py"), the_runner()).map_err(|error| error.to_string())?;
    Ok(written)
}

#[must_use]
pub fn read_the_verdicts(said: &str, tasks: &[Task]) -> Vec<Checked> {
    let mut held = nothing_held(tasks);
    for line in said.lines() {
        let mut parts = line.split_whitespace();
        let Some(name) = parts.next() else { continue };
        let passed = parts.next().and_then(|held| held.parse::<u32>().ok());
        let of = parts.next().and_then(|held| held.parse::<u32>().ok());
        let Some(at) = name
            .strip_prefix("task-")
            .and_then(|number| number.parse::<usize>().ok())
            .and_then(|number| number.checked_sub(1))
        else {
            continue;
        };
        let Some(slot) = held.get_mut(at) else {
            continue;
        };
        // The count the container made of the claims is the better one — it saw them run,
        // where the count taken off the source only saw them written. Where nothing came
        // back, what was written still gives the task a denominator.
        if let Some(of) = of.filter(|held| *held > 0) {
            slot.of = of;
        }
        slot.passed = passed.unwrap_or(0).min(slot.of);
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
        return Marked::By(nothing_held(tasks));
    }
    let written = match laid_out(room, tasks, &blocks) {
        Ok(written) => written,
        Err(why) => return Marked::Unmarked(why),
    };
    if written == 0 {
        return Marked::By(nothing_held(tasks));
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
