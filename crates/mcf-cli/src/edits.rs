//! Edit tasks: a whole file given, one change asked, hidden cases run on
//! the result, and the parts not asked about compared byte for byte
//! (B-522, D55).
//!
//! Writing a function from nothing is one skill; changing a file without
//! disturbing the rest of it is the one an editor is used for. Each task
//! is a small Python file of three or four functions and a change to one
//! of them, or one to add. The model is asked for the whole file back.
//! The cases cover the changed function and the untouched ones; and every
//! untouched function's source is looked for in the answer as it was
//! given, byte for byte. Two counts then stand beside each other: what
//! held, and what was changed that nobody asked to change.

use std::path::Path;

use mcf_bench::eval::{Case, Ran, Task};
use mcf_record::json::Value;
use mcf_serve::examine::Reading;

/// How many times each edit is asked for.
pub(crate) const ATTEMPTS: usize = 2;

/// The token budget for a whole file back.
const BUDGET: usize = 700;

/// One edit task: the file as given, the change asked, and the checks.
pub(crate) struct Edit {
    /// The file, whole, as the model is given it.
    pub file: &'static str,
    /// The change asked, in words.
    pub asks: &'static str,
    /// The name of the function the change is to, or to add; every other
    /// function must come back as it was.
    pub changes: &'static str,
    /// The cases on the result, which the checker runs.
    pub task: Task,
}

/// The edit tasks.
pub(crate) const EDITS: &[Edit] = &[
    Edit {
        file: "def total(items):\n    return sum(price for _, price in items)\n\n\ndef cheapest(items):\n    return min(items, key=lambda item: item[1])[0]\n\n\ndef restock(items, threshold):\n    return [name for name, price in items if price < threshold]\n",
        asks: "Change `cheapest` so that it returns None for an empty list instead of raising.",
        changes: "cheapest",
        task: Task {
            name: "cheapest-of-none",
            function: "cheapest",
            asks: "",
            cases: &[
                Case {
                    call: "cheapest([])",
                    expects: "None",
                },
                Case {
                    call: "cheapest([('pen', 3), ('cap', 1), ('ink', 2)])",
                    expects: "'cap'",
                },
                Case {
                    call: "total([('pen', 3), ('cap', 1)])",
                    expects: "4",
                },
                Case {
                    call: "restock([('pen', 3), ('cap', 1), ('ink', 2)], 3)",
                    expects: "['cap', 'ink']",
                },
            ],
        },
    },
    Edit {
        file: "def slug(title):\n    return '-'.join(title.lower().split())\n\n\ndef truncate(text, n):\n    return text[:n]\n\n\ndef initials(name):\n    return ''.join(part[0].upper() for part in name.split())\n",
        asks: "Change `truncate` so that when the text is longer than n it is cut and ends with \
               '...', the result being at most n characters including the dots; text that fits \
               is returned unchanged.",
        changes: "truncate",
        task: Task {
            name: "truncate-with-dots",
            function: "truncate",
            asks: "",
            cases: &[
                Case {
                    call: "truncate('hello world', 8)",
                    expects: "'hello...'",
                },
                Case {
                    call: "truncate('hi', 8)",
                    expects: "'hi'",
                },
                Case {
                    call: "truncate('exactly8', 8)",
                    expects: "'exactly8'",
                },
                Case {
                    call: "slug('Hello  World')",
                    expects: "'hello-world'",
                },
                Case {
                    call: "initials('ada byron lovelace')",
                    expects: "'ABL'",
                },
            ],
        },
    },
    Edit {
        file: "def deposit(balance, amount):\n    return balance + amount\n\n\ndef withdraw(balance, amount):\n    return balance - amount\n\n\ndef interest(balance, rate):\n    return round(balance * rate, 2)\n",
        asks: "Change `withdraw` so that it returns None when the amount is more than the balance, \
               and the new balance otherwise.",
        changes: "withdraw",
        task: Task {
            name: "withdraw-no-overdraft",
            function: "withdraw",
            asks: "",
            cases: &[
                Case {
                    call: "withdraw(10, 3)",
                    expects: "7",
                },
                Case {
                    call: "withdraw(3, 10)",
                    expects: "None",
                },
                Case {
                    call: "withdraw(5, 5)",
                    expects: "0",
                },
                Case {
                    call: "deposit(10, 3)",
                    expects: "13",
                },
                Case {
                    call: "interest(200, 0.015)",
                    expects: "3.0",
                },
            ],
        },
    },
    Edit {
        file: "def is_leap(year):\n    return year % 4 == 0 and (year % 100 != 0 or year % 400 == 0)\n\n\ndef days_in_month(year, month):\n    if month == 2:\n        return 28\n    if month in (4, 6, 9, 11):\n        return 30\n    return 31\n\n\ndef day_of_year(year, month, day):\n    return sum(days_in_month(year, m) for m in range(1, month)) + day\n",
        asks: "`days_in_month` is wrong for February in a leap year. Fix it using `is_leap`.",
        changes: "days_in_month",
        task: Task {
            name: "february-in-a-leap-year",
            function: "days_in_month",
            asks: "",
            cases: &[
                Case {
                    call: "days_in_month(2024, 2)",
                    expects: "29",
                },
                Case {
                    call: "days_in_month(2023, 2)",
                    expects: "28",
                },
                Case {
                    call: "days_in_month(1900, 2)",
                    expects: "28",
                },
                Case {
                    call: "days_in_month(2024, 4)",
                    expects: "30",
                },
                Case {
                    call: "is_leap(2000)",
                    expects: "True",
                },
                Case {
                    call: "day_of_year(2024, 3, 1)",
                    expects: "61",
                },
            ],
        },
    },
    Edit {
        file: "def push(queue, item):\n    queue.append(item)\n    return queue\n\n\ndef pop(queue):\n    return queue.pop(0)\n\n\ndef peek(queue):\n    return queue[0] if queue else None\n",
        asks: "Add a function `size(queue)` that returns how many items the queue holds. Change \
               nothing else.",
        changes: "size",
        task: Task {
            name: "add-size",
            function: "size",
            asks: "",
            cases: &[
                Case {
                    call: "size([1, 2, 3])",
                    expects: "3",
                },
                Case {
                    call: "size([])",
                    expects: "0",
                },
                Case {
                    call: "push([1], 2)",
                    expects: "[1, 2]",
                },
                Case {
                    call: "pop([4, 5])",
                    expects: "4",
                },
                Case {
                    call: "peek([])",
                    expects: "None",
                },
            ],
        },
    },
];

/// What the model is asked: the file, the change, and the whole file back.
#[must_use]
pub(crate) fn prompt_for(edit: &Edit) -> String {
    format!(
        "Here is a Python file:\n```python\n{}```\n\n{} Reply with the whole file, changed \
         only where asked, and nothing else.",
        edit.file, edit.asks
    )
}

/// The file's top-level functions as `(name, source)`, each block running
/// from its `def` to the next top-level `def` or the end, trailing blank
/// lines dropped.
#[must_use]
pub(crate) fn functions_of(file: &str) -> Vec<(&str, &str)> {
    let starts: Vec<usize> = file
        .match_indices("def ")
        .filter(|(at, _)| *at == 0 || file.as_bytes().get(at.wrapping_sub(1)) == Some(&b'\n'))
        .map(|(at, _)| at)
        .collect();
    starts
        .iter()
        .enumerate()
        .filter_map(|(which, &at)| {
            let end = starts
                .get(which.wrapping_add(1))
                .copied()
                .unwrap_or(file.len());
            let block = file.get(at..end)?.trim_end();
            let name = block
                .get(4..)?
                .split(|character: char| character == '(' || character.is_whitespace())
                .next()?;
            Some((name, block))
        })
        .collect()
}

/// How the untouched functions came back: how many there were, how many
/// are in the answer byte for byte, their bytes, and the bytes of those
/// that are not.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Outside {
    pub untouched: usize,
    pub kept: usize,
    pub bytes: usize,
    pub changed_bytes: usize,
}

/// Compares the parts nobody asked about with the answer.
#[must_use]
pub(crate) fn outside(edit: &Edit, answer: &str) -> Outside {
    let mut held = Outside {
        untouched: 0,
        kept: 0,
        bytes: 0,
        changed_bytes: 0,
    };
    for (name, block) in functions_of(edit.file) {
        if name == edit.changes {
            continue;
        }
        held.untouched = held.untouched.saturating_add(1);
        held.bytes = held.bytes.saturating_add(block.len());
        if answer.contains(block) {
            held.kept = held.kept.saturating_add(1);
        } else {
            held.changed_bytes = held.changed_bytes.saturating_add(block.len());
        }
    }
    held
}

/// One attempt's rows: the cases, the outside, the size and the asking.
#[must_use]
pub(crate) fn rows_of(
    edit: &Edit,
    attempt: usize,
    ran: &Ran,
    answer: &str,
    ask_ns: u64,
) -> Vec<Reading> {
    let whole = |count: usize| i64::try_from(count).unwrap_or(i64::MAX);
    let dims = [
        ("task", Value::text(edit.task.name.to_owned())),
        ("attempt", Value::Integer(whole(attempt))),
    ];
    let mut rows = vec![
        Reading::new(
            &dims,
            "ask_ns",
            i64::try_from(ask_ns).unwrap_or(i64::MAX),
            "ns",
        ),
        Reading::new(&dims, "answer_bytes", whole(answer.len()), "bytes"),
        Reading::new(&dims, "file_bytes", whole(edit.file.len()), "bytes"),
    ];
    match ran {
        Ran::Checked { passed, of } => {
            rows.push(Reading::new(&dims, "wrote", 1, "bool"));
            rows.push(Reading::new(&dims, "ran", 1, "bool"));
            rows.push(Reading::new(&dims, "cases_held", whole(*passed), "count"));
            rows.push(Reading::new(&dims, "cases", whole(*of), "count"));
            rows.push(Reading::new(
                &dims,
                "whole",
                i64::from(passed == of),
                "bool",
            ));
        }
        Ran::Refused {
            wrote_something, ..
        } => {
            rows.push(Reading::new(
                &dims,
                "wrote",
                i64::from(*wrote_something),
                "bool",
            ));
            rows.push(Reading::new(&dims, "ran", 0, "bool"));
        }
    }
    if !answer.trim().is_empty() {
        let kept = outside(edit, answer);
        rows.push(Reading::new(
            &dims,
            "untouched",
            whole(kept.untouched),
            "count",
        ));
        rows.push(Reading::new(
            &dims,
            "untouched_kept",
            whole(kept.kept),
            "count",
        ));
        rows.push(Reading::new(
            &dims,
            "outside_bytes",
            whole(kept.bytes),
            "bytes",
        ));
        rows.push(Reading::new(
            &dims,
            "outside_changed_bytes",
            whole(kept.changed_bytes),
            "bytes",
        ));
    }
    rows
}

/// Runs every edit against the model through the daemon, each answer in
/// the container, and returns the lines said and the rows to record.
pub(crate) fn run(
    socket: &Path,
    named: &str,
    podman: &Path,
    scratch: &Path,
) -> (Vec<String>, Vec<Reading>, Option<String>) {
    let mut lines = vec![
        format!(
            "  {} edit task(s), {ATTEMPTS} attempt(s) each: a whole file given, one change \
             asked, the whole file back; the untouched functions compared byte for byte",
            EDITS.len()
        ),
        String::new(),
    ];
    let mut rows = Vec::new();
    let mut engine_ran = None;
    for edit in EDITS {
        lines.push(format!("  {}", edit.task.name));
        for attempt in 0..ATTEMPTS {
            let began = std::time::Instant::now();
            let spoken = mcf_serve::probes::spoken(
                socket,
                Path::new(named),
                &prompt_for(edit),
                None,
                BUDGET,
                None,
            );
            let ask_ns = u64::try_from(began.elapsed().as_nanos()).unwrap_or(u64::MAX);
            if engine_ran.is_none() {
                engine_ran.clone_from(&spoken.engine_ran);
            }
            let answer = crate::eval::code_in(&spoken.text);
            let ran = crate::eval::run_in_container(podman, scratch, &edit.task, &answer);
            let kept = outside(edit, &answer);
            lines.push(match &ran {
                Ran::Checked { passed, of } => format!(
                    "      attempt {}: {passed} of {of} case(s) held; {} of {} untouched \
                     function(s) back byte for byte",
                    attempt.wrapping_add(1),
                    kept.kept,
                    kept.untouched
                ),
                Ran::Refused { because, .. } => format!(
                    "      attempt {}: did not run — {because}",
                    attempt.wrapping_add(1)
                ),
            });
            rows.extend(rows_of(edit, attempt, &ran, &answer, ask_ns));
        }
        lines.push(String::new());
    }
    (lines, rows, engine_ran)
}

#[cfg(test)]
mod tests {
    use super::{EDITS, Outside, functions_of, outside, prompt_for};

    #[test]
    fn a_file_splits_into_its_functions() {
        let held = functions_of(EDITS[0].file);
        let names: Vec<&str> = held.iter().map(|(name, _)| *name).collect();
        assert_eq!(names, ["total", "cheapest", "restock"]);
        assert!(
            held[1].1.starts_with("def cheapest(items):"),
            "{:?}",
            held[1]
        );
        assert!(!held[1].1.ends_with('\n'));
    }

    #[test]
    fn the_untouched_functions_are_looked_for_byte_for_byte() {
        let edit = &EDITS[0];
        let answer = edit.file.replace(
            "    return min(items, key=lambda item: item[1])[0]",
            "    if not items:\n        return None\n    return min(items, key=lambda item: item[1])[0]",
        );
        assert_eq!(
            outside(edit, &answer),
            Outside {
                untouched: 2,
                kept: 2,
                bytes: outside(edit, edit.file).bytes,
                changed_bytes: 0
            }
        );
        let reformatted = answer.replace(
            "return sum(price for _, price in items)",
            "return sum(p for _, p in items)",
        );
        let held = outside(edit, &reformatted);
        assert_eq!((held.untouched, held.kept), (2, 1));
        assert!(held.changed_bytes > 0);
    }

    #[test]
    fn every_edit_names_a_function_and_the_added_one_is_not_in_the_file() {
        for edit in EDITS {
            let names: Vec<&str> = functions_of(edit.file)
                .iter()
                .map(|(name, _)| *name)
                .collect();
            assert!(!names.is_empty(), "{}", edit.task.name);
            assert!(
                names.contains(&edit.changes) || edit.changes == "size",
                "{}: {names:?}",
                edit.task.name
            );
            assert!(prompt_for(edit).contains(edit.file));
        }
    }
}
