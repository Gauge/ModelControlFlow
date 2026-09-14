use super::{Set, task_count};

#[test]
fn the_corpus_is_sixty_four_tasks_in_eight_sets() {
    let sets = Set::all();
    assert_eq!(sets.len(), 8, "eight sets are compiled in");
    assert_eq!(task_count(), 64, "sixty-four tasks in total");
    for set in &sets {
        assert_eq!(set.tasks.len(), 8, "set {} holds eight tasks", set.number);
    }
}

#[test]
fn every_task_carries_a_name_a_question_and_a_check() {
    for set in Set::all() {
        for task in &set.tasks {
            assert!(!task.name.is_empty(), "a task is named");
            assert!(
                task.asked.len() > 80,
                "{} asks something substantial",
                task.name
            );
            assert!(
                task.checked.contains("assert"),
                "{} is checked by assertions",
                task.name
            );
        }
    }
}

#[test]
fn no_two_tasks_share_a_name() {
    let mut seen = std::collections::BTreeSet::new();
    for set in Set::all() {
        for task in &set.tasks {
            assert!(
                seen.insert(task.name.clone()),
                "{} appears twice",
                task.name
            );
        }
    }
    assert_eq!(seen.len(), 64);
}

#[test]
fn a_set_renders_one_prompt_naming_every_task_in_order() {
    let Some(set) = Set::numbered(3) else {
        panic!("set three is compiled in");
    };
    let asked = set.asked();
    let mut at = 0;
    for (index, task) in set.tasks.iter().enumerate() {
        let number = index + 1;
        let header = format!("### TASK {number} ({})", task.name);
        let Some(found) = asked.get(at..).and_then(|rest| rest.find(&header)) else {
            panic!("{header} is absent or out of order");
        };
        at += found + header.len();
    }
    assert!(asked.contains("### SOLUTION n"), "the format is stated");
}

#[test]
fn a_set_beyond_the_corpus_is_absent_rather_than_empty() {
    assert!(Set::numbered(9).is_none());
    assert!(Set::numbered(0).is_none());
}

/// A task has to ask for everything its check insists on. The sudoku check hands the solver
/// a grid that is already full and already wrong, and requires it to answer that it cannot
/// be solved — which a solver only does if it looks at the values it was given before it
/// starts filling blanks, and only does that if it was told to.
#[test]
fn a_task_asks_for_everything_its_check_insists_on() {
    let set = super::Set::numbered(2).expect("set 2");
    let held = set
        .tasks
        .iter()
        .find(|task| task.name == "sudoku")
        .expect("set 2 has it");
    assert!(
        held.checked.contains("assert solve_sudoku(bad) is None"),
        "this test is reading the wrong check"
    );
    assert!(
        held.asked.contains("already breaks the rules"),
        "the check rejects a grid whose givens conflict; the task has to say so, or it marks \
         a model down for doing exactly what it was asked and no more: {}",
        held.asked
    );
}
