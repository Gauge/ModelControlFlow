use core::fmt::Write as _;

use mcf_record::json::{Value, parse};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Task {
    pub name: String,
    pub asked: String,
    pub checked: String,
}

/// What a set asks for, and therefore how an answer to it is marked.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// Write the program. Marked by running the model's own code against the claims the
    /// task makes, inside a container.
    Code,
    /// Say the answer. Marked by reading it, because there is exactly one right answer
    /// and it is short enough to write on one line.
    ShortAnswer,
    /// Write one long program. Marked like a code task, by running it against the claims
    /// its check makes, inside a container — but asked alone, because a program of several
    /// hundred lines is the whole of an answer, and what a setting does to a model writing
    /// at that length is what this kind is here to measure.
    LongScript,
    /// Follow a long list of small changes to five registers, and write the registers after
    /// each. Marked by reading, a step at a time: a run is as many marks as it has steps,
    /// and every step is as easy as the last, so a step gone wrong is the model losing its
    /// place and nothing else.
    Focus,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Set {
    pub number: usize,
    pub kind: Kind,
    pub tasks: Vec<Task>,
}

/// Where the short-answer sets start counting.
///
/// Held apart from the code sets rather than continuing from them, so that a set number
/// written down in a reading a year ago still names the same set after either corpus has
/// grown.
pub const SHORT_FROM: usize = 101;

/// Where the long-script sets start counting, apart from both of the others for the same
/// reason they are apart from each other.
pub const LONG_FROM: usize = 201;

/// Where the focus sets start counting, apart from the others for the same reason.
pub const FOCUS_FROM: usize = 301;

/// How many focus sets there are, how many runs are in each, and how many steps a run is.
///
/// Four runs to a set because four are asked at once, and a set whose runs are the same
/// length finishes them together rather than leaving three slots idle behind the longest.
/// A hundred and fifty steps is long enough that losing one's place comes into it, and
/// short enough that a run is a couple of thousand tokens.
pub const FOCUS_SETS: usize = 8;
pub const FOCUS_RUNS: usize = 4;
pub const FOCUS_STEPS: usize = 150;

const LONG: [&str; 2] = [
    include_str!("../tasks/long/set1.json"),
    include_str!("../tasks/long/set2.json"),
];

const SHORT: [&str; 40] = [
    include_str!("../tasks/short/set1.json"),
    include_str!("../tasks/short/set2.json"),
    include_str!("../tasks/short/set3.json"),
    include_str!("../tasks/short/set4.json"),
    include_str!("../tasks/short/set5.json"),
    include_str!("../tasks/short/set6.json"),
    include_str!("../tasks/short/set7.json"),
    include_str!("../tasks/short/set8.json"),
    include_str!("../tasks/short/set9.json"),
    include_str!("../tasks/short/set10.json"),
    include_str!("../tasks/short/set11.json"),
    include_str!("../tasks/short/set12.json"),
    include_str!("../tasks/short/set13.json"),
    include_str!("../tasks/short/set14.json"),
    include_str!("../tasks/short/set15.json"),
    include_str!("../tasks/short/set16.json"),
    include_str!("../tasks/short/set17.json"),
    include_str!("../tasks/short/set18.json"),
    include_str!("../tasks/short/set19.json"),
    include_str!("../tasks/short/set20.json"),
    include_str!("../tasks/short/set21.json"),
    include_str!("../tasks/short/set22.json"),
    include_str!("../tasks/short/set23.json"),
    include_str!("../tasks/short/set24.json"),
    include_str!("../tasks/short/set25.json"),
    include_str!("../tasks/short/set26.json"),
    include_str!("../tasks/short/set27.json"),
    include_str!("../tasks/short/set28.json"),
    include_str!("../tasks/short/set29.json"),
    include_str!("../tasks/short/set30.json"),
    include_str!("../tasks/short/set31.json"),
    include_str!("../tasks/short/set32.json"),
    include_str!("../tasks/short/set33.json"),
    include_str!("../tasks/short/set34.json"),
    include_str!("../tasks/short/set35.json"),
    include_str!("../tasks/short/set36.json"),
    include_str!("../tasks/short/set37.json"),
    include_str!("../tasks/short/set38.json"),
    include_str!("../tasks/short/set39.json"),
    include_str!("../tasks/short/set40.json"),
];

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
    /// Every short-answer set: a thousand questions with one right answer apiece, twenty-five
    /// to a set, and each one asked on its own.
    ///
    /// They are here because the code corpus is eight sets of hard problems, and a run of
    /// it is slow, coarse and expensive: a model may spend eleven thousand tokens thinking
    /// before it writes a line, and sixty-four tasks is a score that moves in lumps. A
    /// thousand short ones move a score smoothly, cost a line of output each, and need no
    /// container to mark.
    #[must_use]
    pub fn short() -> Vec<Self> {
        SHORT
            .iter()
            .enumerate()
            .filter_map(|(at, text)| {
                Some(Self {
                    number: SHORT_FROM.checked_add(at)?,
                    kind: Kind::ShortAnswer,
                    tasks: tasks_in(text)?,
                })
            })
            .collect()
    }

    /// The long-script sets: eight programs of several hundred lines apiece, each asked on
    /// its own and marked by running it against a check of twenty to forty claims.
    ///
    /// They are for the settings a short answer cannot show: the ones that keep a model
    /// from repeating itself. A model answering a sum in one line never loops, so a
    /// penalty against looping shows only what it costs there. Over a long program it
    /// shows both what it costs and what it saves.
    #[must_use]
    pub fn long() -> Vec<Self> {
        LONG.iter()
            .enumerate()
            .filter_map(|(at, text)| {
                Some(Self {
                    number: LONG_FROM.checked_add(at)?,
                    kind: Kind::LongScript,
                    tasks: tasks_in(text)?,
                })
            })
            .collect()
    }

    /// The focus sets: runs of small changes to follow, made from their seeds rather than
    /// read from a file, marked by reading a line a step.
    ///
    /// These are what a marked sweep asks. A short question is one mark for however long a
    /// model takes over it, and how hard it is varies from one to the next, so a score over
    /// them moves with which questions a setting happened to get right. A run is a hundred
    /// and fifty marks of the same difficulty, and what it marks wrong is a slip.
    #[must_use]
    pub fn focus() -> Vec<Self> {
        (0..FOCUS_SETS)
            .filter_map(|at| Self::numbered(FOCUS_FROM.checked_add(at)?))
            .collect()
    }

    /// A focus set by its number. Its runs are seeded by where they stand in the corpus, so
    /// a run is the same run for as long as the number means the same set.
    fn focus_numbered(number: usize) -> Option<Self> {
        let at = number.checked_sub(FOCUS_FROM)?;
        if at >= FOCUS_SETS {
            return None;
        }
        let tasks = (0..FOCUS_RUNS)
            .map(|run| {
                let place = at.saturating_mul(FOCUS_RUNS).saturating_add(run);
                let seed = u64::try_from(place).unwrap_or(0);
                let made = crate::focus::Run::seeded(seed, FOCUS_STEPS);
                Task {
                    name: format!("focus-{:04}", place.saturating_add(1)),
                    asked: made.asked(),
                    checked: made.checked(),
                }
            })
            .collect();
        Some(Self {
            number,
            kind: Kind::Focus,
            tasks,
        })
    }

    /// The code sets: sixty-four programs to write, marked by running them.
    ///
    /// Retired as the measure of correctness — the short questions are what a marked
    /// sweep asks now — and kept, because they are a harder test than the short ones and
    /// readings taken against them are in the record. A timed trial still reads and
    /// writes against the first of them, where what is asked matters only in being the
    /// same every time.
    #[must_use]
    pub fn all() -> Vec<Self> {
        SETS.iter()
            .enumerate()
            .filter_map(|(at, text)| {
                Some(Self {
                    number: at.checked_add(1)?,
                    kind: Kind::Code,
                    tasks: tasks_in(text)?,
                })
            })
            .collect()
    }

    #[must_use]
    pub fn numbered(number: usize) -> Option<Self> {
        if number >= FOCUS_FROM {
            return Self::focus_numbered(number);
        }
        if let Some(at) = number.checked_sub(LONG_FROM) {
            let text = LONG.get(at)?;
            return Some(Self {
                number,
                kind: Kind::LongScript,
                tasks: tasks_in(text)?,
            });
        }
        if let Some(at) = number.checked_sub(SHORT_FROM) {
            let text = SHORT.get(at)?;
            return Some(Self {
                number,
                kind: Kind::ShortAnswer,
                tasks: tasks_in(text)?,
            });
        }
        let text = SETS.get(number.checked_sub(1)?)?;
        Some(Self {
            number,
            kind: Kind::Code,
            tasks: tasks_in(text)?,
        })
    }

    /// Each question of this set as a set of its own, numbered as this one is, so that a
    /// trial can ask them one at a time and still write down one reading for the set.
    ///
    /// Asked twenty-five to a request, a model reasoned about all of them together before it
    /// wrote the first answer, and a set of sums took as long as a set of hard problems. Asked
    /// one at a time, an easy question is answered as quickly as it is easy.
    #[must_use]
    pub fn one_at_a_time(&self) -> Vec<Self> {
        self.tasks
            .iter()
            .map(|task| Self {
                number: self.number,
                kind: self.kind,
                tasks: vec![task.clone()],
            })
            .collect()
    }

    #[must_use]
    pub fn asked(&self) -> String {
        if self.kind == Kind::ShortAnswer {
            return self.asked_for_short_answers();
        }
        if let (Kind::Focus, [task]) = (self.kind, self.tasks.as_slice()) {
            return format!(
                "Five registers, a to e, each hold one digit. Apply the steps below in order.\n\n\
                 - `x += n` adds n to x and `x -= n` takes n from it. Every sum wraps round \
                 within 0 to 9: 8 + 5 is 3, and 2 - 4 is 8.\n\
                 - `x = y` copies y into x. `x = n` sets x to n.\n\
                 - `swap x y` swaps the two.\n\n{}\n\
                 ---\nOUTPUT FORMAT, follow exactly:\nFor every step, in order, write one \
                 line: the step's number, a colon, the step itself, an arrow, then all five \
                 registers after that step — for example `1: c += 4 -> a=3 b=7 c=4 d=5 e=2`. \
                 Write each line as you reach its step, one line for every step to the last, \
                 and nothing else.",
                task.asked
            );
        }
        if let (Kind::LongScript, [task]) = (self.kind, self.tasks.as_slice()) {
            return format!(
                "Write one complete, self-contained Python 3 program for the task below.\n\n{}\n\n\
                 ---\nOUTPUT FORMAT, follow exactly:\nOutput ONE fenced python code block holding \
                 the whole program, and nothing after it. Define the functions and classes the \
                 task names. The program is loaded and then its functions are called, so it must \
                 not read input or run anything when it is loaded. Use only the standard library.",
                task.asked
            );
        }
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

impl Set {
    /// One line of working is allowed and then the answer, because a model told to answer
    /// and nothing else will sometimes reason in the answer line and put the wrong thing
    /// in it. The marker reads the answer line and ignores the rest.
    fn asked_for_short_answers(&self) -> String {
        if let [task] = self.tasks.as_slice() {
            return format!(
                "Answer the following question.\n\n{}\n\n---\nOUTPUT FORMAT, follow exactly:\n\
                 End with one line '### ANSWER 1: value'. The value is the answer alone — a \
                 number with no units and no thousands separators, or a single word. Give an \
                 answer even if you are unsure.",
                task.asked
            );
        }
        let mut said = format!(
            "Answer ALL of the following {} questions, IN ORDER.\n\n",
            self.tasks.len()
        );
        for (at, task) in self.tasks.iter().enumerate() {
            let number = at.saturating_add(1);
            let _written = writeln!(said, "### QUESTION {number}\n{}\n", task.asked);
        }
        said.push_str(
            "---\nOUTPUT FORMAT, follow exactly:\nFor each question output one line \
             '### ANSWER n: value' and nothing else on that line. The value is the answer \
             alone — a number with no units and no thousands separators, or a single word. \
             Give every answer, in order, even if you are unsure.",
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
                // `t` on a code task is the claims its answer has to satisfy; `a` on a
                // short-answer task is the answer itself.
                checked: field("t").or_else(|| field("a"))?,
            })
        })
        .collect()
}

/// One kind of short question: the word its name starts with, and what to call it.
#[derive(Debug, PartialEq, Eq)]
pub struct Category {
    pub key: &'static str,
    pub label: &'static str,
}

/// Every kind of short question, in the order the generator makes them.
///
/// A score over a thousand questions says how a setting did, and nothing about where. A
/// setting that makes a model better at sums and worse at reading code can come out level
/// on the whole, and only the questions taken apart by kind show it did anything at all.
pub const CATEGORIES: [Category; 26] = [
    Category {
        key: "arith",
        label: "arithmetic",
    },
    Category {
        key: "prec",
        label: "precedence",
    },
    Category {
        key: "pct",
        label: "percentages",
    },
    Category {
        key: "gcd",
        label: "gcd and lcm",
    },
    Category {
        key: "prime",
        label: "counting primes",
    },
    Category {
        key: "mod",
        label: "remainders",
    },
    Category {
        key: "powmod",
        label: "powers mod n",
    },
    Category {
        key: "base",
        label: "into a base",
    },
    Category {
        key: "frombase",
        label: "out of a base",
    },
    Category {
        key: "seqterm",
        label: "sequence terms",
    },
    Category {
        key: "seqsum",
        label: "sequence sums",
    },
    Category {
        key: "solve",
        label: "equations",
    },
    Category {
        key: "comb",
        label: "combinatorics",
    },
    Category {
        key: "rate",
        label: "rates",
    },
    Category {
        key: "work",
        label: "working together",
    },
    Category {
        key: "digits",
        label: "digit sums",
    },
    Category {
        key: "revnum",
        label: "reversed digits",
    },
    Category {
        key: "bits",
        label: "bitwise",
    },
    Category {
        key: "geom",
        label: "geometry",
    },
    Category {
        key: "sets",
        label: "sets",
    },
    Category {
        key: "text",
        label: "letters and words",
    },
    Category {
        key: "list",
        label: "lists",
    },
    Category {
        key: "code",
        label: "reading code",
    },
    Category {
        key: "order",
        label: "ordering",
    },
    Category {
        key: "weeks",
        label: "days and weeks",
    },
    Category {
        key: "units",
        label: "units",
    },
];

/// The kind of short question a task is, read off its name: `pct-0002` is a percentage.
///
/// Nothing for a task that is not a short question. A code task is named for what it
/// builds, with no number after it, and is a kind of its own.
#[must_use]
pub fn category_of(name: &str) -> Option<&'static Category> {
    let (key, number) = name.rsplit_once('-')?;
    if number.is_empty() || !number.chars().all(|ch| ch.is_ascii_digit()) {
        return None;
    }
    CATEGORIES.iter().find(|category| category.key == key)
}

#[must_use]
pub fn task_count() -> usize {
    Set::all().iter().map(|set| set.tasks.len()).sum()
}

#[cfg(test)]
mod tests;
