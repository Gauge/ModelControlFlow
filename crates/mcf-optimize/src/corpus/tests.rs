use super::{task_count, Set};

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
            assert!(seen.insert(task.name.clone()), "{} appears twice", task.name);
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
