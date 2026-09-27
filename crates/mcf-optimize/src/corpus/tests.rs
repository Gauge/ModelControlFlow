use super::{CATEGORIES, Set, category_of, task_count};

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

/// A thousand questions with one right answer apiece, against which a setting can be
/// judged without waiting eleven thousand tokens for a model to stop thinking.
mod short_answers {
    use crate::corpus::{Kind, SHORT_FROM, Set};

    #[test]
    fn there_are_a_thousand_of_them_and_every_one_has_an_answer() {
        let sets = Set::short();
        assert_eq!(sets.len(), 40);
        let tasks: usize = sets.iter().map(|set| set.tasks.len()).sum();
        assert_eq!(tasks, 1_000);
        for set in &sets {
            assert_eq!(set.kind, Kind::ShortAnswer);
            for task in &set.tasks {
                assert!(!task.asked.trim().is_empty(), "{} asks nothing", task.name);
                assert!(
                    !task.checked.trim().is_empty(),
                    "{} has no answer, so nothing could be marked against it",
                    task.name
                );
            }
        }
    }

    /// Numbered apart from the code sets, so a set number written down in a reading names
    /// the same questions whichever corpus has grown since.
    #[test]
    fn a_set_number_names_one_corpus_and_not_the_other() {
        assert_eq!(Set::numbered(1).map(|set| set.kind), Some(Kind::Code));
        assert_eq!(
            Set::numbered(SHORT_FROM).map(|set| set.kind),
            Some(Kind::ShortAnswer)
        );
        assert!(Set::numbered(SHORT_FROM.saturating_sub(1)).is_none());
        assert!(Set::numbered(SHORT_FROM.saturating_add(40)).is_none());
    }

    #[test]
    fn the_question_asks_for_the_answer_alone() {
        let set = Set::numbered(SHORT_FROM).expect("the first short set");
        let asked = set.asked();
        assert!(asked.contains("### QUESTION 1"));
        assert!(asked.contains("### ANSWER n: value"));
        assert!(
            !asked.contains("python"),
            "nothing here wants a program written: {asked:.120}"
        );
    }

    #[test]
    fn a_question_asked_on_its_own_is_asked_alone_and_answered_on_the_line_that_is_marked() {
        let set = Set::numbered(SHORT_FROM).expect("the first short set");
        let alone = set.one_at_a_time();
        assert_eq!(alone.len(), set.tasks.len());
        let first = alone.first().expect("a first question");
        assert_eq!(first.number, set.number, "still the set it came from");
        let asked = first.asked();
        let question = &set.tasks.first().expect("a first task").asked;
        assert!(asked.contains(question.as_str()), "{asked}");
        assert!(asked.contains("### ANSWER 1: value"), "{asked}");
        assert!(
            !asked.contains("ALL") && !asked.contains("QUESTION 2"),
            "nothing here asks for more than the one: {asked}"
        );
    }

    /// Every name is its own, so a reading that says which task failed says which one.
    #[test]
    fn no_two_tasks_share_a_name() {
        let mut names: Vec<String> = Set::short()
            .into_iter()
            .flat_map(|set| set.tasks)
            .map(|task| task.name)
            .collect();
        let all = names.len();
        names.sort();
        names.dedup();
        assert_eq!(names.len(), all);
    }
}

/// Eight long programs, each asked alone and marked by running it, for the settings that
/// keep a model from repeating itself.
mod long_scripts {
    use crate::corpus::{Kind, LONG_FROM, SHORT_FROM, Set};

    /// A correct program for every task, kept beside the tasks so that a check which a
    /// correct program cannot pass is found here rather than by a model being marked down.
    const REFERENCE: [(&str, &str); 8] = [
        (
            "spreadsheet",
            include_str!("../../tasks/long/reference/spreadsheet.py"),
        ),
        (
            "markdown",
            include_str!("../../tasks/long/reference/markdown.py"),
        ),
        ("bank", include_str!("../../tasks/long/reference/bank.py")),
        (
            "planner",
            include_str!("../../tasks/long/reference/planner.py"),
        ),
        (
            "interpreter",
            include_str!("../../tasks/long/reference/interpreter.py"),
        ),
        (
            "recurrence",
            include_str!("../../tasks/long/reference/recurrence.py"),
        ),
        ("grid", include_str!("../../tasks/long/reference/grid.py")),
        ("sql", include_str!("../../tasks/long/reference/sql.py")),
    ];

    fn reply_of(reference: &str) -> String {
        format!("Here is the program.\n\n```python\n{reference}```\n")
    }

    #[test]
    fn there_are_two_sets_of_four_programs_numbered_apart_from_the_others() {
        let sets = Set::long();
        assert_eq!(sets.len(), 2);
        for (at, set) in sets.iter().enumerate() {
            assert_eq!(set.number, LONG_FROM + at);
            assert_eq!(set.kind, Kind::LongScript);
            assert_eq!(set.tasks.len(), 4);
            assert_eq!(
                Set::numbered(set.number).map(|held| held.kind),
                Some(Kind::LongScript)
            );
        }
        assert!(LONG_FROM > SHORT_FROM + Set::short().len());
        assert!(Set::numbered(LONG_FROM + 2).is_none());
    }

    #[test]
    fn every_check_makes_enough_claims_to_score_a_program_finely() {
        for task in Set::long().into_iter().flat_map(|set| set.tasks) {
            let claims = crate::marking::claims_in(&task.checked);
            assert!(claims >= 20, "{} makes only {claims} claims", task.name);
            assert!(
                !task.checked.contains("assert False"),
                "{}: a claim that only runs when something went wrong is never counted as \
                 holding, so a correct program would lose it",
                task.name
            );
        }
    }

    #[test]
    fn a_program_is_asked_for_alone_in_one_block() {
        let set = Set::numbered(LONG_FROM).expect("the first long set");
        let first = set
            .one_at_a_time()
            .into_iter()
            .next()
            .expect("a first task");
        let asked = first.asked();
        assert!(
            asked.contains("ONE fenced python code block"),
            "{asked:.300}"
        );
        assert!(asked.contains("must not read input"), "{asked:.300}");
        assert!(
            asked.contains("class `Sheet`"),
            "the task itself is in it: {asked:.300}"
        );
    }

    #[test]
    fn every_task_has_a_correct_program_beside_it() {
        let names: Vec<String> = Set::long()
            .into_iter()
            .flat_map(|set| set.tasks)
            .map(|task| task.name)
            .collect();
        let known: Vec<&str> = REFERENCE.iter().map(|(name, _)| *name).collect();
        assert_eq!(names, known);
    }

    /// A long program repeats itself in ways that are not a loop — closing brackets, lines
    /// that differ only in a name — and the watch for loops runs over every answer as it
    /// arrives. It must never cut a correct program off.
    #[test]
    fn no_correct_program_is_ever_taken_for_a_loop() {
        for (name, reference) in REFERENCE {
            let reply = reply_of(reference);
            let mut end = 1500;
            while end < reply.len() {
                while !reply.is_char_boundary(end) {
                    end += 1;
                }
                let so_far = reply.get(..end).unwrap_or(&reply);
                assert_eq!(
                    crate::looping::looping(so_far),
                    None,
                    "{name} was called a loop {end} characters in"
                );
                end += 200;
            }
            assert_eq!(crate::looping::looping(&reply), None, "{name}, whole");
        }
    }

    /// Marked the way a sweep marks them — in the container — every correct program holds
    /// every claim its check makes. Needs podman, and says so rather than passing quietly
    /// on a machine without it.
    #[test]
    fn a_correct_program_holds_every_claim_when_marked_in_the_container() {
        if crate::marking::where_podman_is().is_none() {
            eprintln!("skipped: podman is not on this machine, so nothing could be marked");
            return;
        }
        let tasks: Vec<_> = Set::long().into_iter().flat_map(|set| set.tasks).collect();
        for (task, (_, reference)) in tasks.iter().zip(REFERENCE) {
            let room = std::env::temp_dir().join(format!(
                "mcf-long-reference-{}-{}",
                std::process::id(),
                task.name
            ));
            let held = crate::marking::marked(
                &room,
                std::slice::from_ref(task),
                &reply_of(reference),
                std::time::Duration::from_secs(300),
            );
            let _swept = std::fs::remove_dir_all(&room);
            assert_eq!(held.why(), None, "{} was not marked", task.name);
            let checked = held.or_unmarked(std::slice::from_ref(task));
            let (passed, of) = checked.iter().fold((0, 0), |(passed, of), held| {
                (passed + held.passed, of + held.of)
            });
            assert_eq!(passed, of, "{}: {passed} of {of}", task.name);
            assert_eq!(
                of,
                crate::marking::claims_in(&task.checked),
                "{}",
                task.name
            );
        }
    }
}

#[test]
fn every_short_question_is_of_a_kind_and_every_kind_is_asked() {
    let mut asked = [0_usize; CATEGORIES.len()];
    for set in Set::short() {
        for task in &set.tasks {
            let Some(category) = category_of(&task.name) else {
                panic!("{} is of no kind the table knows", task.name);
            };
            if let Some(at) = CATEGORIES.iter().position(|held| held == category)
                && let Some(count) = asked.get_mut(at)
            {
                *count += 1;
            }
        }
    }
    for (category, count) in CATEGORIES.iter().zip(asked) {
        assert!(count > 0, "{} is never asked", category.key);
    }
    assert_eq!(asked.iter().sum::<usize>(), 1000);
}

#[test]
fn a_code_task_is_of_no_short_kind() {
    for set in Set::all().into_iter().chain(Set::long()) {
        for task in &set.tasks {
            assert_eq!(category_of(&task.name), None, "{}", task.name);
        }
    }
}

#[test]
fn a_kind_is_read_off_the_whole_word_before_the_number() {
    assert_eq!(
        category_of("mod-0012").map(|held| held.label),
        Some("remainders")
    );
    assert_eq!(
        category_of("powmod-0012").map(|held| held.label),
        Some("powers mod n")
    );
    assert_eq!(category_of("mod-"), None);
    assert_eq!(category_of("mod-12a"), None);
    assert_eq!(category_of("diff-patch"), None);
}

#[test]
fn the_focus_sets_are_made_the_same_every_time_and_each_run_is_its_own() {
    let sets = Set::focus();
    assert_eq!(sets.len(), crate::corpus::FOCUS_SETS);
    assert_eq!(
        sets,
        Set::focus(),
        "made from seeds, so made the same again"
    );
    let mut runs = Vec::new();
    for set in &sets {
        assert_eq!(set.kind, crate::corpus::Kind::Focus);
        assert_eq!(Set::numbered(set.number).as_ref(), Some(set));
        assert_eq!(set.tasks.len(), crate::corpus::FOCUS_RUNS);
        for task in &set.tasks {
            assert_eq!(
                crate::marking::claims_in(&task.checked),
                u32::try_from(crate::corpus::FOCUS_STEPS).unwrap_or(0),
                "a run's claims are its steps"
            );
            runs.push(task.checked.clone());
        }
    }
    let count = runs.len();
    runs.sort();
    runs.dedup();
    assert_eq!(runs.len(), count, "no two runs are the same run");
    assert_eq!(
        Set::numbered(crate::corpus::FOCUS_FROM + crate::corpus::FOCUS_SETS),
        None
    );
}

#[test]
fn a_focus_run_is_asked_with_its_rules_and_every_step() {
    let set = Set::focus().remove(0).one_at_a_time().remove(0);
    let asked = set.asked();
    assert!(asked.contains("wraps round"));
    assert!(asked.contains("Start: a="));
    assert!(asked.contains(&format!("\n{}. ", crate::corpus::FOCUS_STEPS)));
    assert!(asked.contains("OUTPUT FORMAT"));
}
