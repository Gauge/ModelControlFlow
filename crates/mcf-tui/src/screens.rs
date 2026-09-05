//! The screens, and the drawing they have in common.
//!
//! **Columns, because a figure is read against the one above it.** Everything
//! here exists so that a temperature sits under a temperature and a size under
//! a size: labels left, figures right, and the widths fixed by the table rather
//! than by the longest thing that happened to be in it today. A bar would say
//! *roughly*, and roughly is not what an instrument is for.
//!
//! **Eighty by twenty-four.** The layouts in `../layout/` generate and check the
//! same screens at that size, which is the smallest any terminal has ever been.
//! A larger terminal gets more rows of models and more measured depths — never
//! the same picture stretched.

pub mod diagnostics;
pub mod host;
pub mod monitor;

use crate::screen::{Ink, Screen};

/// Which screen is shown, and what the menu offers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Where {
    /// The machine, live.
    Monitor,
    /// Setting up a measurement.
    Diagnostics,
    /// Everything held.
    Models,
    /// What MCF can build, and what it has.
    Components,
    /// What a prompt does to a model.
    ///
    /// Not a column entry: the row holds every item at once and has four
    /// columns of slack where a seventh needs nine. It is reached from
    /// Diagnostics, where a model is already chosen, and the window reaches it
    /// the same way (B-072).
    Prompt,
    /// How MCF is set up.
    Settings,
    /// Leave.
    Exit,
}

impl Where {
    /// Every item, in the order they sit along the top.
    pub const ALL: [Self; 5] = [
        Self::Monitor,
        Self::Models,
        Self::Diagnostics,
        Self::Components,
        Self::Exit,
    ];

    /// The word in the menu.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Monitor => "Machine",
            Self::Diagnostics => "Diagnostics",
            Self::Models => "Models",
            Self::Components => "Engines",
            Self::Prompt => "Prompt",
            Self::Settings => "Settings",
            Self::Exit => "Exit",
        }
    }

    /// The title in the frame, which is how a person knows where they are —
    /// the menu highlight is the cursor, not the place.
    #[must_use]
    pub const fn title(self) -> &'static str {
        match self {
            Self::Monitor => "This machine",
            Self::Diagnostics => "Diagnostics",
            Self::Models => "Models",
            Self::Components => "Engines: what MCF has built and can build",
            Self::Prompt => "What a prompt does",
            Self::Settings => "Settings",
            Self::Exit => "Exit",
        }
    }
}

/// Draws the frame and the menu, and returns the first row inside.
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

/// Draws the sides and the foot, once the body is in.
pub fn close(into: &mut Screen, from: usize) {
    let width = into.width();
    let last = into.height().saturating_sub(1);
    for row in from..last {
        // A screen that drew its own divider owns both ends of that row: a
        // side painted over it would leave a rule running into a wall.
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

/// A row of columns: `(text, width, right-aligned, ink)`.
///
/// The width is the table's, not the text's, which is the whole point — a
/// column that resized itself to its contents would move under the reader
/// every time a figure changed.
pub fn columns(into: &mut Screen, at: usize, row: usize, cells: &[(&str, usize, bool, Ink)]) {
    let mut column = at;
    for (text, width, right, ink) in cells {
        // Clipped to the CELL, not to the screen. A label longer than its
        // column used to run into the next one and be half-overwritten by the
        // figure there — `a much longe100000l` — which is worse than a
        // truncation because it reads as a value.
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

/// A size, exact and without a float.
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

/// A count with its thousands separated: `40,960`.
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

/// What a reading MCF could not take looks like: a dash, never a zero (A7).
pub const UNKNOWN: &str = "—";

/// A number, or the dash.
#[must_use]
pub fn or_unknown<T: std::fmt::Display>(held: Option<T>, suffix: &str) -> String {
    held.map_or_else(|| UNKNOWN.to_owned(), |value| format!("{value}{suffix}"))
}

#[cfg(test)]
mod tests;
