use super::{
    Checked, IMAGE, Marked, arguments, claims_in, laid_out, marked, read_the_verdicts, the_runner,
};
use crate::corpus::Task;
use std::path::PathBuf;

struct Scratch {
    path: PathBuf,
}

impl Scratch {
    fn new(name: &str) -> Self {
        let path = std::env::temp_dir().join(format!(
            "mcf-marking-{name}-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _fresh = std::fs::remove_dir_all(&path);
        Self { path }
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _removed = std::fs::remove_dir_all(&self.path);
    }
}

fn tasks() -> Vec<Task> {
    vec![
        Task {
            name: "adds".to_owned(),
            asked: "write add".to_owned(),
            checked: "assert add(2,2)==4".to_owned(),
        },
        Task {
            name: "doubles".to_owned(),
            asked: "write double".to_owned(),
            checked: "assert double(3)==6".to_owned(),
        },
    ]
}

#[test]
fn the_container_reaches_no_network_and_cannot_write_where_it_was_given_the_code() {
    let held = arguments(std::path::Path::new("/somewhere"));
    assert!(held.contains(&"--network=none".to_owned()), "{held:?}");
    assert!(held.contains(&"--read-only".to_owned()), "{held:?}");
    assert!(held.contains(&"--cap-drop=ALL".to_owned()), "{held:?}");
    assert!(
        held.iter().any(|held| held.ends_with(":/work:ro,z")),
        "the code a model wrote is mounted read-only: {held:?}"
    );
    assert!(
        held.iter().any(|held| held.starts_with("--memory=")),
        "{held:?}"
    );
    assert!(
        held.iter().any(|held| held.starts_with("--pids-limit=")),
        "a fork bomb is a thing a model can write: {held:?}"
    );
}

#[test]
fn the_image_is_pinned_to_a_digest_rather_than_a_tag() {
    assert!(
        IMAGE.contains("@sha256:"),
        "a tag can be moved under us: {IMAGE}"
    );
    assert!(!IMAGE.contains(":latest"), "{IMAGE}");
}

/// The answer and the check go into separate files. The check's claims are rewritten before
/// they run so that each can be counted, and rewriting the model's own code along with them
/// would be marking something nobody wrote.
#[test]
fn the_model_s_code_and_the_check_nobody_else_wrote_are_kept_apart() {
    let scratch = Scratch::new("apart");
    let _written = laid_out(
        &scratch.path,
        &tasks(),
        &["def add(a, b):\n    return a + b".to_owned()],
    )
    .expect("laid out");
    let answer = std::fs::read_to_string(scratch.path.join("task-01.py")).expect("the answer");
    let check = std::fs::read_to_string(scratch.path.join("check-01.py")).expect("the check");
    assert!(answer.starts_with("def add"), "{answer:?}");
    assert!(
        !answer.contains("assert"),
        "the check is not in with it: {answer:?}"
    );
    assert!(
        check.trim_end().ends_with("assert add(2,2)==4"),
        "{check:?}"
    );
}

#[test]
fn the_claims_a_check_makes_are_counted_off_its_source() {
    assert_eq!(claims_in("assert a\nassert b\nassert c"), 3);
    assert_eq!(
        claims_in("for i in x:\n    assert i\nassert done"),
        2,
        "a claim inside a loop is one claim, however many times it runs"
    );
    assert_eq!(
        claims_in("x = 'assert nothing'"),
        1,
        "a task with no claims at all still has a denominator, or it is worth nothing to \
         score it"
    );
}

#[test]
fn one_file_is_laid_out_for_each_answer_a_model_actually_gave() {
    let scratch = Scratch::new("laid-out");
    let written = laid_out(
        &scratch.path,
        &tasks(),
        &["def add(a,b): return a+b".to_owned()],
    )
    .expect("laid out");
    assert_eq!(written, 1, "one block was offered, so one file is written");
    assert!(scratch.path.join("task-01.py").is_file());
    assert!(
        !scratch.path.join("task-02.py").is_file(),
        "a task the model never answered is not invented"
    );
    assert!(scratch.path.join("mark.py").is_file());
}

#[test]
fn an_empty_block_is_not_written_as_an_answer() {
    let scratch = Scratch::new("empty-block");
    let written =
        laid_out(&scratch.path, &tasks(), &["   ".to_owned(), String::new()]).expect("laid out");
    assert_eq!(written, 0);
}

#[test]
fn a_verdict_line_is_read_back_onto_the_task_it_belongs_to() {
    let held = read_the_verdicts("task-01 1 1\ntask-02 0 1\n", &tasks());
    assert_eq!(
        held,
        vec![
            Checked {
                name: "adds".to_owned(),
                passed: 1,
                of: 1
            },
            Checked {
                name: "doubles".to_owned(),
                passed: 0,
                of: 1
            }
        ]
    );
}

/// Most of the way there and nowhere at all are different answers, and the whole point of
/// counting claims is that they read differently.
#[test]
fn a_task_half_right_is_marked_half_right() {
    let held = read_the_verdicts("task-01 5 9\n", &tasks());
    let first = held.first().expect("a task");
    assert_eq!((first.passed, first.of), (5, 9));
    assert!(!first.whole(), "five of nine is not the whole of it");
    assert!(
        Checked {
            name: "x".to_owned(),
            passed: 9,
            of: 9
        }
        .whole()
    );
}

#[test]
fn a_task_the_container_said_nothing_about_holds_none_of_its_claims() {
    let held = read_the_verdicts("task-02 1 1\n", &tasks());
    assert_eq!(
        held.first().map(|held| held.passed),
        Some(0),
        "silence is never a pass"
    );
    assert_eq!(
        held.first().map(|held| held.of),
        Some(1),
        "and it still has a denominator, taken off what the check was written to claim"
    );
    assert_eq!(held.get(1).map(|held| held.passed), Some(1));
}

#[test]
fn rubbish_on_the_output_does_not_turn_into_a_pass() {
    let held = read_the_verdicts("hello\ntask-01\ntask-99 1 1\ntask-01 1 1", &tasks());
    assert_eq!(held.first().map(|held| held.passed), Some(1));
    assert_eq!(held.get(1).map(|held| held.passed), Some(0));
}

#[test]
fn a_count_of_claims_never_runs_past_the_claims_there_are() {
    let held = read_the_verdicts("task-01 900 1\n", &tasks());
    assert_eq!(
        held.first().map(|held| held.passed),
        Some(1),
        "a container that said something impossible does not get to score it"
    );
}

#[test]
fn an_answer_with_no_code_in_it_fails_every_task_without_starting_a_container() {
    let scratch = Scratch::new("no-code");
    let held = marked(
        &scratch.path,
        &tasks(),
        "I would rather not.",
        std::time::Duration::from_secs(1),
    );
    assert_eq!(held, Marked::By(super::nothing_held(&tasks())));
    let none = held.or_unmarked(&tasks());
    assert!(
        none.iter().all(|held| held.passed == 0 && held.of > 0),
        "no claim held, and every task still says how many it makes: {none:?}"
    );
}

#[test]
fn the_runner_gives_every_task_a_deadline_of_its_own() {
    let held = the_runner();
    assert!(held.contains("timeout="), "{held}");
    assert!(
        held.contains("TimeoutExpired"),
        "a task that hangs is a failed task, not a hung sweep: {held}"
    );
}

#[test]
fn unmarked_answers_count_as_failed_rather_than_as_passed() {
    let held = Marked::Unmarked("podman is not installed".to_owned());
    assert!(
        held.clone()
            .or_unmarked(&tasks())
            .iter()
            .all(|held| held.passed == 0),
        "nothing was marked, so nothing held"
    );
    assert!(held.why().is_some(), "and it says why it could not mark");
}

/// The whole reason for counting claims, run for real. A solution that gets one of two
/// claims right reads as one of two; under a pass-or-fail mark it read the same as one that
/// got neither, and a set of eight tasks was worth eighteen points either way as a reading.
#[test]
fn a_solution_part_of_the_way_there_is_marked_part_of_the_way_there() {
    if super::where_podman_is().is_none() {
        eprintln!("skipped: podman is not installed here");
        return;
    }
    let scratch = Scratch::new("partly");
    let held = marked(
        &scratch.path,
        &[Task {
            name: "adds".to_owned(),
            asked: String::new(),
            checked: "assert add(2, 2) == 4\nassert add(1, 5) == 6\nassert add(0, 0) == 0"
                .to_owned(),
        }],
        "```python\ndef add(a, b):\n    return a * b\n```",
        std::time::Duration::from_secs(120),
    );
    let verdicts = held.or_unmarked(&[Task {
        name: "adds".to_owned(),
        asked: String::new(),
        checked: String::new(),
    }]);
    let first = verdicts.first().expect("the one task");
    assert_eq!(
        (first.passed, first.of),
        (2, 3),
        "multiplication gets two of those three right by luck: {verdicts:?}"
    );
    assert!(!first.whole());
}
