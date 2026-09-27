//! Whether a model keeps its place: five registers of one digit each, a long list of small
//! changes to them, and a line of the registers after every change.
//!
//! Nothing in it needs knowing anything or working anything out. Each step is as easy as
//! the one before it, so what goes wrong is the model losing hold of where it is — which
//! is the thing a sampling setting damages first — and not a question it could never have
//! answered. And every step is a mark of its own: one request is a hundred and fifty of
//! them, where a short question is one.
//!
//! Each run is made from a seed, so the same number is the same run every time, on any
//! machine, and no model has seen it before.

use core::fmt::Write as _;

use crate::corpus::Task;
use crate::marking::Checked;

/// What the registers are called, in the order a line writes them.
pub const REGISTERS: [char; 5] = ['a', 'b', 'c', 'd', 'e'];

/// What every register holds, in the order `REGISTERS` names them.
pub type State = [u8; 5];

/// The first line of a run written down as its check, so a check is known for one.
const HEADING: &str = "focus";

/// One change to the registers. Every value is a digit and every sum wraps round within
/// nought to nine, so no step needs arithmetic past what a child can do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Change {
    Add(usize, u8),
    Take(usize, u8),
    Copy { into: usize, from: usize },
    Set(usize, u8),
    Swap(usize, usize),
}

// There is no rotation of all five among the changes, though there was. Asked of a capable
// model, it was got wrong nineteen times in twenty while every other change was got right
// ninety-nine times in a hundred: the model read it the other way round. A change a model
// misunderstands every time measures the words it is put in, not whether the model kept
// its place.

impl Change {
    #[must_use]
    pub fn applied(self, state: State) -> State {
        let mut held = state;
        let at = |place: usize| state.get(place).copied().unwrap_or(0);
        match self {
            Self::Add(place, by) => put(&mut held, place, at(place).saturating_add(by) % 10),
            Self::Take(place, by) => {
                put(
                    &mut held,
                    place,
                    at(place).saturating_add(10 - by % 10) % 10,
                );
            }
            Self::Copy { into, from } => put(&mut held, into, at(from)),
            Self::Set(place, to) => put(&mut held, place, to % 10),
            Self::Swap(one, other) => held.swap(one.min(4), other.min(4)),
        }
        held
    }

    /// The change as the question writes it, and as a check holds it.
    #[must_use]
    pub fn said(self) -> String {
        let name = |place: usize| REGISTERS.get(place).copied().unwrap_or('?');
        match self {
            Self::Add(place, by) => format!("{} += {by}", name(place)),
            Self::Take(place, by) => format!("{} -= {by}", name(place)),
            Self::Copy { into, from } => format!("{} = {}", name(into), name(from)),
            Self::Set(place, to) => format!("{} = {to}", name(place)),
            Self::Swap(one, other) => format!("swap {} {}", name(one), name(other)),
        }
    }

    /// A change read back from how it is said.
    #[must_use]
    pub fn read(said: &str) -> Option<Self> {
        let words: Vec<&str> = said.split_whitespace().collect();
        match words.as_slice() {
            ["swap", one, other] => Some(Self::Swap(register(one)?, register(other)?)),
            [place, "+=", by] => Some(Self::Add(register(place)?, digit(by)?)),
            [place, "-=", by] => Some(Self::Take(register(place)?, digit(by)?)),
            [place, "=", what] => {
                let place = register(place)?;
                match digit(what) {
                    Some(to) => Some(Self::Set(place, to)),
                    None => Some(Self::Copy {
                        into: place,
                        from: register(what)?,
                    }),
                }
            }
            _ => None,
        }
    }
}

fn put(state: &mut State, place: usize, value: u8) {
    if let Some(slot) = state.get_mut(place) {
        *slot = value;
    }
}

fn register(said: &str) -> Option<usize> {
    let mut chars = said.chars();
    let only = chars.next()?;
    if chars.next().is_some() {
        return None;
    }
    REGISTERS.iter().position(|held| *held == only)
}

fn digit(said: &str) -> Option<u8> {
    match said.as_bytes() {
        [only] if only.is_ascii_digit() => Some(only.saturating_sub(b'0')),
        _ => None,
    }
}

/// A state as a line writes it: the digits, a to e, a space between.
#[must_use]
pub fn said(state: State) -> String {
    state
        .iter()
        .map(u8::to_string)
        .collect::<Vec<_>>()
        .join(" ")
}

/// One run: where the registers start, and every change made to them in order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Run {
    pub start: State,
    pub changes: Vec<Change>,
}

impl Run {
    /// The run a seed makes. The same seed and length make the same run on any machine.
    #[must_use]
    pub fn seeded(seed: u64, length: usize) -> Self {
        let mut dice = Dice(seed);
        let mut start = [0_u8; 5];
        for held in &mut start {
            *held = dice.digit();
        }
        let changes = (0..length).map(|_| dice.change()).collect();
        Self { start, changes }
    }

    /// Where the registers stand after each change, in order.
    #[must_use]
    pub fn states(&self) -> Vec<State> {
        let mut at = self.start;
        self.changes
            .iter()
            .map(|change| {
                at = change.applied(at);
                at
            })
            .collect()
    }

    /// What the question shows: where the registers start, and the changes numbered.
    #[must_use]
    pub fn asked(&self) -> String {
        let mut said_so = String::from("Start:");
        for (name, value) in REGISTERS.iter().zip(self.start) {
            let _wrote = write!(said_so, " {name}={value}");
        }
        said_so.push_str("\n\nSteps:\n");
        for (at, change) in self.changes.iter().enumerate() {
            let _wrote = writeln!(said_so, "{}. {}", at.saturating_add(1), change.said());
        }
        said_so
    }

    /// The run as its check holds it: a heading, the start, then a change to a line.
    #[must_use]
    pub fn checked(&self) -> String {
        let mut held = format!("{HEADING}\n{}\n", said(self.start));
        for change in &self.changes {
            held.push_str(&change.said());
            held.push('\n');
        }
        held
    }

    /// A run read back from its check. Nothing for a check that is not one.
    #[must_use]
    pub fn from_checked(checked: &str) -> Option<Self> {
        let mut lines = checked.lines();
        if lines.next()?.trim() != HEADING {
            return None;
        }
        let start = state_in(lines.next()?)?;
        let changes = lines
            .filter(|line| !line.trim().is_empty())
            .map(Change::read)
            .collect::<Option<Vec<_>>>()?;
        Some(Self { start, changes })
    }
}

/// The first step a model got wrong: what it wrote there, if it wrote anything, and what
/// following the step from its own line before would have given.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Slip {
    pub step: usize,
    pub wrote: Option<State>,
    pub wanted: State,
}

impl Slip {
    /// The slip, said by the registers that were wrong, since two rows of five digits side
    /// by side make a reader find the difference for themselves.
    #[must_use]
    pub fn said(&self) -> String {
        let Some(wrote) = self.wrote else {
            return format!("first slip at step {}: no line for it", self.step);
        };
        let wrong = |state: State| {
            REGISTERS
                .iter()
                .zip(state.iter().zip(wrote.iter().zip(self.wanted)))
                .filter(|(_, (_, (wrote, wanted)))| *wrote != wanted)
                .map(|(name, (value, _))| format!("{name}={value}"))
                .collect::<Vec<_>>()
                .join(" ")
        };
        format!(
            "first slip at step {}: wrote {}, wanted {}",
            self.step,
            wrong(wrote),
            wrong(self.wanted)
        )
    }
}

/// What a run's answer came to: the steps it held, of how many, and where it first slipped.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Followed {
    pub held: u32,
    pub of: u32,
    pub first_slip: Option<Slip>,
}

/// Mark an answer against a run, step by step.
///
/// Each step is marked against what following it from the model's own line before would
/// give, not against where the registers truly stand. One slip is then one step wrong,
/// rather than every step after it: the count is of the times the model lost its place,
/// which is what this is here to see, and not of how early the first of them came. A step
/// with no line is a slip, and the line wanted for it stands in for the next one, so a
/// model that stops half way has every step it never wrote counted against it.
#[must_use]
pub fn followed(run: &Run, answer: &str) -> Followed {
    let written = lines_in(answer);
    let mut before = run.start;
    let mut held = 0_u32;
    let mut first_slip = None;
    for (at, change) in run.changes.iter().enumerate() {
        let step = at.saturating_add(1);
        let wanted = change.applied(before);
        let wrote = written
            .iter()
            .find(|(number, _)| *number == step)
            .map(|(_, state)| *state);
        if wrote == Some(wanted) {
            held = held.saturating_add(1);
        } else if first_slip.is_none() {
            first_slip = Some(Slip {
                step,
                wrote,
                wanted,
            });
        }
        before = wrote.unwrap_or(wanted);
    }
    Followed {
        held,
        of: u32::try_from(run.changes.len()).unwrap_or(u32::MAX),
        first_slip,
    }
}

/// Mark each run of a set by reading the lines its answer wrote.
#[must_use]
pub fn marked(tasks: &[Task], answer: &str) -> Vec<Checked> {
    tasks
        .iter()
        .map(|task| {
            let (passed, of) = Run::from_checked(&task.checked).map_or((0, 1), |run| {
                let came = followed(&run, answer);
                (came.held, came.of)
            });
            Checked {
                name: task.name.clone(),
                passed,
                of,
            }
        })
        .collect()
}

/// Where a run's answer first went wrong, said for the window. Nothing where it never did.
#[must_use]
pub fn first_slip(tasks: &[Task], answer: &str) -> Option<String> {
    let run = tasks
        .first()
        .and_then(|task| Run::from_checked(&task.checked))?;
    followed(&run, answer).first_slip.map(|slip| slip.said())
}

/// Every numbered line of registers in an answer. The first line for a step is the one
/// marked, as it is for a short answer.
///
/// A line is asked for as `4: swap d b -> a=5 b=1 c=3 d=0 e=8`: the step said again, then
/// the registers by name. Saying the step keeps a model on the step it is at, and naming
/// the registers keeps it from counting along a row of five digits for the one it wants —
/// asked for the digits alone, a capable model slipped on nearly half its steps, and
/// almost every slip was the right change made to the wrong register.
///
/// What a model puts around a line is not held against it: it may bold it, put it in a
/// list, write `Step 4:` for `4:`, leave the step out, or leave the names off. What it may
/// not do is write other than a digit for each of the five registers.
fn lines_in(answer: &str) -> Vec<(usize, State)> {
    answer.lines().filter_map(a_line).collect()
}

fn a_line(line: &str) -> Option<(usize, State)> {
    let decoration =
        |ch: char| ch.is_whitespace() || matches!(ch, '*' | '-' | '`' | '#' | '>' | '|');
    let mut rest = line.trim_start_matches(decoration);
    if let Some(after) = rest
        .get(..4)
        .filter(|held| held.eq_ignore_ascii_case("step"))
        .and_then(|_| rest.get(4..))
    {
        rest = after.trim_start();
    }
    let digits = rest.chars().take_while(char::is_ascii_digit).count();
    let number = rest.get(..digits)?.parse::<usize>().ok()?;
    let rest = rest
        .get(digits..)?
        .trim_start_matches(|ch: char| ch.is_whitespace() || matches!(ch, '*' | '`'));
    let rest = rest.strip_prefix([':', '.', ')'])?;
    let registers = rest.split_once("->").map_or(rest, |(_, after)| after);
    Some((number, state_in(registers)?))
}

/// Five registers read off what follows a line's number: by name where every one is
/// named, and in order where they are not.
fn state_in(said: &str) -> Option<State> {
    let mut held = Vec::with_capacity(5);
    let mut named = Vec::with_capacity(5);
    for word in said.split(|ch: char| ch.is_whitespace() || ch == ',') {
        let word = word.trim_matches(|ch: char| matches!(ch, '*' | '`' | '[' | ']' | '(' | ')'));
        if word.is_empty() {
            continue;
        }
        let name = word
            .split_once(['=', ':'])
            .and_then(|(name, value)| Some((register(name)?, value)));
        let value = name.map_or(word, |(_, value)| value);
        match digit(value) {
            Some(value) if held.len() < 5 => {
                held.push(value);
                named.push(name.map(|(place, _)| place));
            }
            // Anything after the fifth register — a note, a comment — is let be. Anything
            // before it that is not a register makes this no line of registers at all.
            _ if held.len() == 5 => break,
            _ => return None,
        }
    }
    let mut state: State = held.try_into().ok()?;
    let places: Vec<usize> = named
        .iter()
        .copied()
        .collect::<Option<_>>()
        .unwrap_or_default();
    let mut sorted = places.clone();
    sorted.sort_unstable();
    sorted.dedup();
    if sorted.len() == 5 {
        let by_name = state;
        for (place, value) in places.iter().zip(by_name) {
            put(&mut state, *place, value);
        }
    }
    Some(state)
}

/// A small generator of numbers that is the same everywhere: splitmix64.
struct Dice(u64);

impl Dice {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut held = self.0;
        held = (held ^ (held >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        held = (held ^ (held >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        held ^ (held >> 31)
    }

    fn below(&mut self, bound: u64) -> u64 {
        self.next().checked_rem(bound).unwrap_or(0)
    }

    fn digit(&mut self) -> u8 {
        u8::try_from(self.below(10)).unwrap_or(0)
    }

    fn register(&mut self) -> usize {
        usize::try_from(self.below(5)).unwrap_or(0)
    }

    /// Two registers that are not the same one.
    fn pair(&mut self) -> (usize, usize) {
        let one = self.register();
        let other = usize::try_from(self.below(4)).unwrap_or(0);
        (
            one,
            if other >= one {
                other.saturating_add(1)
            } else {
                other
            },
        )
    }

    fn by(&mut self) -> u8 {
        u8::try_from(self.below(9)).unwrap_or(0).saturating_add(1)
    }

    /// A change, drawn so that adding and taking away, which touch one register, come up
    /// most, and copying and swapping, which read one register to change another, next.
    fn change(&mut self) -> Change {
        match self.below(20) {
            0..=5 => Change::Add(self.register(), self.by()),
            6..=9 => Change::Take(self.register(), self.by()),
            10..=13 => {
                let (into, from) = self.pair();
                Change::Copy { into, from }
            }
            14..=15 => Change::Set(self.register(), self.digit()),
            _ => {
                let (one, other) = self.pair();
                Change::Swap(one, other)
            }
        }
    }
}

#[cfg(test)]
mod tests;
