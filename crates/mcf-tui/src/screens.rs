pub mod host;
pub mod monitor;

use crate::screen::{Ink, Screen};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Where {
    Monitor,
    Models,
    Components,
    Settings,
    Exit,
}

impl Where {
    pub const ALL: [Self; 4] = [Self::Monitor, Self::Models, Self::Components, Self::Exit];

    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Monitor => "System",
            Self::Models => "Models",
            Self::Components => "Engines",
            Self::Settings => "Settings",
            Self::Exit => "Exit",
        }
    }

    #[must_use]
    pub const fn title(self) -> &'static str {
        match self {
            Self::Monitor => "This system",
            Self::Models => "Models",
            Self::Components => "Engines: what MCF has built and can build",
            Self::Settings => "Settings",
            Self::Exit => "Exit",
        }
    }
}

pub fn frame(into: &mut Screen, at: Where, cursor: usize, state: Option<(&str, Ink)>) -> usize {
    let width = into.width();
    into.put(0, 0, "┌─ ", Ink::Quiet);
    into.put(3, 0, at.title(), Ink::Heading);
    let after = 3 + at.title().chars().count() + 1;
    into.rule(after, 0, width.saturating_sub(after + 1), Ink::Quiet);
    into.put(0, 0, "┌", Ink::Quiet);
    into.put(width.saturating_sub(1), 0, "┐", Ink::Quiet);

    let mut column = 1;
    for (index, item) in Where::ALL.iter().enumerate() {
        let text = format!(" {} ", item.label());
        let ink = if index == cursor {
            Ink::Selected
        } else {
            Ink::Quiet
        };
        into.put(column, 1, &text, ink);
        column += text.chars().count() + 1;
    }
    if let Some((word, ink)) = state {
        into.put_right(width.saturating_sub(1), 1, word, ink);
    }
    into.put(0, 1, "│", Ink::Quiet);
    into.put(width.saturating_sub(1), 1, "│", Ink::Quiet);

    let heavy: String = std::iter::repeat_n('═', width.saturating_sub(2)).collect();
    into.put(0, 2, "╞", Ink::Quiet);
    into.put(1, 2, &heavy, Ink::Quiet);
    into.put(width.saturating_sub(1), 2, "╡", Ink::Quiet);
    3
}

pub fn close(into: &mut Screen, from: usize) {
    let width = into.width();
    let last = into.height().saturating_sub(1);
    for row in from..last {
        if into.at(0, row) == ' ' {
            into.put(0, row, "│", Ink::Quiet);
        }
        if into.at(width.saturating_sub(1), row) == ' ' {
            into.put(width.saturating_sub(1), row, "│", Ink::Quiet);
        }
    }
    into.put(0, last, "└", Ink::Quiet);
    into.rule(1, last, width.saturating_sub(2), Ink::Quiet);
    into.put(width.saturating_sub(1), last, "┘", Ink::Quiet);
}

pub fn columns(into: &mut Screen, at: usize, row: usize, cells: &[(&str, usize, bool, Ink)]) {
    let mut column = at;
    for (text, width, right, ink) in cells {
        let room = width.saturating_sub(1);
        let clipped: String = if text.chars().count() > room {
            text.chars().take(room).collect()
        } else {
            (*text).to_owned()
        };
        if *right {
            into.put_right(column + width, row, &clipped, *ink);
        } else {
            into.put(column, row, &clipped, *ink);
        }
        column += width;
    }
}

#[must_use]
pub fn gigabytes(bytes: u64) -> String {
    #[allow(
        clippy::integer_division,
        reason = "exact: a float would round a byte count before printing it"
    )]
    {
        let whole = bytes / 1_000_000_000;
        let hundredths = (bytes % 1_000_000_000) / 10_000_000;
        format!("{whole}.{hundredths:02} GB")
    }
}

#[must_use]
pub fn grouped(number: u64) -> String {
    let digits = number.to_string();
    let mut out = String::new();
    let total = digits.len();
    for (at, digit) in digits.chars().enumerate() {
        if at > 0 && (total - at).is_multiple_of(3) {
            out.push(',');
        }
        out.push(digit);
    }
    out
}

pub const UNKNOWN: &str = "—";

#[must_use]
pub fn or_unknown<T: std::fmt::Display>(held: Option<T>, suffix: &str) -> String {
    held.map_or_else(|| UNKNOWN.to_owned(), |value| format!("{value}{suffix}"))
}

#[cfg(test)]
mod tests;
