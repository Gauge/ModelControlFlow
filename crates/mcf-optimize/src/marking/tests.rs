use super::{
    IMAGE, Marked, a_task_file, arguments, laid_out, marked, read_the_verdicts, the_runner,
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

#[test]
fn a_task_file_is_the_model_s_code_and_then_the_check_nobody_else_wrote() {
    let held = a_task_file("def add(a, b):\n    return a + b", "assert add(2,2)==4");
    assert!(held.starts_with("def add"));
    assert!(held.trim_end().ends_with("assert add(2,2)==4"));
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
    let held = read_the_verdicts("task-01 PASS\ntask-02 FAIL\n", &tasks());
    assert_eq!(
        held,
        vec![("adds".to_owned(), true), ("doubles".to_owned(), false)]
    );
}

#[test]
fn a_task_the_container_said_nothing_about_counts_as_failed() {
    let held = read_the_verdicts("task-02 PASS\n", &tasks());
    assert_eq!(
        held,
        vec![("adds".to_owned(), false), ("doubles".to_owned(), true)],
        "silence is never a pass"
    );
}

#[test]
fn rubbish_on_the_output_does_not_turn_into_a_pass() {
    let held = read_the_verdicts("hello\ntask-01\ntask-99 PASS\ntask-01 PASS", &tasks());
    assert_eq!(held.first().map(|held| held.1), Some(true));
    assert_eq!(held.get(1).map(|held| held.1), Some(false));
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
    assert_eq!(
        held,
        Marked::By(vec![
            ("adds".to_owned(), false),
            ("doubles".to_owned(), false)
        ])
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
    assert_eq!(
        held.clone().or_unmarked(&tasks()),
        vec![("adds".to_owned(), false), ("doubles".to_owned(), false)]
    );
    assert!(held.why().is_some(), "and it says why it could not mark");
}

#[test]
#[ignore = "marks a saved reply with the real container"]
fn which_task_failed() {
    let sp = "/tmp/claude-1001/-home-gauge/de864044-8c77-447c-816d-275ac51a12fb/scratchpad";
    let content = std::fs::read_to_string(format!("{sp}/content.txt")).unwrap_or_default();
    let set = crate::corpus::Set::numbered(2).expect("set 2");
    let room = std::path::PathBuf::from(format!("{sp}/room2"));
    let _swept = std::fs::remove_dir_all(&room);
    let held = super::marked(
        &room,
        &set.tasks,
        &content,
        std::time::Duration::from_secs(600),
    );
    for (name, ok) in held.or_unmarked(&set.tasks) {
        println!("  {} {name}", if ok { "PASS" } else { "FAIL" });
    }
    println!("  (files left in {})", room.display());
}
