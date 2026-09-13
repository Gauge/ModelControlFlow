use core::fmt::Write as _;

use mcf_record::json::{Value, parse};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Task {
    pub name: String,
    pub asked: String,
    pub checked: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Set {
    pub number: usize,
    pub tasks: Vec<Task>,
}

const SETS: [&str; 8] = [
    include_str!("../tasks/set1.json"),
    include_str!("../tasks/set2.json"),
    include_str!("../tasks/set3.json"),
    include_str!("../tasks/set4.json"),
    include_str!("../tasks/set5.json"),
    include_str!("../tasks/set6.json"),
    include_str!("../tasks/set7.json"),
    include_str!("../tasks/set8.json"),
];

impl Set {
    #[must_use]
    pub fn all() -> Vec<Self> {
        SETS.iter()
            .enumerate()
            .filter_map(|(at, text)| {
                Some(Self {
                    number: at.checked_add(1)?,
                    tasks: tasks_in(text)?,
                })
            })
            .collect()
    }

    #[must_use]
    pub fn numbered(number: usize) -> Option<Self> {
        let text = SETS.get(number.checked_sub(1)?)?;
        Some(Self {
            number,
            tasks: tasks_in(text)?,
        })
    }

    #[must_use]
    pub fn asked(&self) -> String {
        let mut said = format!(
            "You will solve ALL of the following {} Python problems, IN ORDER.\n\n",
            self.tasks.len()
        );
        for (at, task) in self.tasks.iter().enumerate() {
            let number = at.saturating_add(1);
            let _written = writeln!(said, "### TASK {number} ({})\n{}\n", task.name, task.asked);
        }
        said.push_str(
            "---\nOUTPUT FORMAT, follow exactly:\nFor each task output a header line \
             '### SOLUTION n' followed by ONE fenced python code block containing the complete \
             self-contained solution. No commentary.",
        );
        said
    }
}

fn tasks_in(text: &str) -> Option<Vec<Task>> {
    let Ok(Value::List(entries)) = parse(text) else {
        return None;
    };
    entries
        .iter()
        .map(|entry| {
            let field = |key: &str| entry.get(key).and_then(Value::as_text).map(str::to_owned);
            Some(Task {
                name: field("n")?,
                asked: field("p")?,
                checked: field("t")?,
            })
        })
        .collect()
}

#[must_use]
pub fn task_count() -> usize {
    Set::all().iter().map(|set| set.tasks.len()).sum()
}

#[cfg(test)]
mod tests;
