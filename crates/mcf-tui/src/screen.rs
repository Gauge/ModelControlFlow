//! A screen built in memory and written once.
//!
//! Everything is drawn into a grid of cells and the whole grid is sent in a
//! single write. Not for speed — a terminal is not a demanding target — but
//! because a screen assembled in pieces and written in pieces is a screen that
//! can be caught half-drawn, and because one write means one place where the
//! escape sequences are produced.
//!
//! Colour is by role rather than by name. A screen that says `Red` has decided
//! how something should look; a screen that says [`Ink::Refusal`] has said what
//! it is, and the palette is one edit away from being right everywhere.

use std::fmt::Write as _;

/// What a piece of text is, which decides how it looks.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Ink {
    /// Ordinary text.
    #[default]
    Plain,
    /// Structure: rules, frames, labels that are not the content.
    Quiet,
    /// A heading.
    Heading,
    /// A measured value.
    Figure,
    /// Something MCF could not do, or would not.
    Refusal,
    /// Something that held.
    Held,
    /// The row the operator is on.
    Selected,
}

impl Ink {
    /// The escape sequence that turns this on.
    fn sequence(self) -> &'static str {
        match self {
            // 256-colour, because every terminal in use understands it and the
            // sixteen-colour palette is whatever the operator's theme says it
            // is — which would make MCF's refusals a colour somebody chose for
            // something else.
            Self::Plain => "\x1b[0m",
            Self::Quiet => "\x1b[38;5;244m",
            Self::Heading => "\x1b[1;38;5;180m",
            Self::Figure => "\x1b[38;5;74m",
            Self::Refusal => "\x1b[38;5;174m",
            Self::Held => "\x1b[38;5;108m",
            Self::Selected => "\x1b[7m",
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct Cell {
    what: char,
    ink: Ink,
}

impl Default for Cell {
    fn default() -> Self {
        Self {
            what: ' ',
            ink: Ink::Plain,
        }
    }
}

/// A screen, sized to the terminal it will be written to.
#[derive(Debug)]
pub struct Screen {
    width: usize,
    height: usize,
    cells: Vec<Cell>,
}

impl Screen {
    /// An empty screen of this size.
    #[must_use]
    pub fn new(width: u16, height: u16) -> Self {
        let (width, height) = (width as usize, height as usize);
        Self {
            width,
            height,
            cells: vec![Cell::default(); width * height],
        }
    }

    /// How many columns.
    #[must_use]
    pub const fn width(&self) -> usize {
        self.width
    }

    /// How many rows.
    #[must_use]
    pub const fn height(&self) -> usize {
        self.height
    }

    /// Writes text at a position, clipped to the screen.
    ///
    /// Clipping rather than wrapping: a value that does not fit is a value the
    /// operator should see truncated, not one that silently pushes the rest of
    /// the row onto the next line and makes a table stop being one.
    pub fn put(&mut self, column: usize, row: usize, text: &str, ink: Ink) {
        if row >= self.height {
            return;
        }
        for (offset, what) in text.chars().enumerate() {
            let at = column + offset;
            if at >= self.width {
                return;
            }
            if let Some(cell) = self.cells.get_mut(row * self.width + at) {
                *cell = Cell { what, ink };
            }
        }
    }

    /// Writes text right-aligned so that it ends at `column`.
    ///
    /// Figures line up on their last digit or they do not line up.
    pub fn put_right(&mut self, column: usize, row: usize, text: &str, ink: Ink) {
        let width = text.chars().count();
        let start = column.saturating_sub(width);
        self.put(start, row, text, ink);
    }

    /// Fills a row with one character, for rules.
    pub fn rule(&mut self, column: usize, row: usize, width: usize, ink: Ink) {
        let line: String =
            std::iter::repeat_n('─', width.min(self.width.saturating_sub(column))).collect();
        self.put(column, row, &line, ink);
    }

    /// Marks a whole row as selected, so the highlight runs to the edge rather
    /// than stopping at the end of the text.
    pub fn select_row(&mut self, row: usize) {
        if row >= self.height {
            return;
        }
        for column in 0..self.width {
            if let Some(cell) = self.cells.get_mut(row * self.width + column) {
                cell.ink = Ink::Selected;
            }
        }
    }

    /// One row as plain text, with no escape sequences in it.
    ///
    /// What the operator would read if the colours were taken away. Tests
    /// compare against this rather than against [`Self::rendered`], where a
    /// column position is a byte offset that moves whenever a style changes —
    /// which is a property of the encoding, not of the layout.
    #[must_use]
    pub fn line(&self, row: usize) -> String {
        if row >= self.height {
            return String::new();
        }
        (0..self.width)
            .filter_map(|column| self.cells.get(row * self.width + column))
            .map(|cell| cell.what)
            .collect()
    }

    /// The whole screen as one string of bytes to write.
    #[must_use]
    pub fn rendered(&self) -> String {
        // Home the cursor rather than clearing: clearing first shows the
        // operator an empty screen between frames.
        let mut out = String::with_capacity(self.cells.len() * 2 + 64);
        out.push_str("\x1b[H");
        let mut ink = None;
        for row in 0..self.height {
            if row > 0 {
                out.push_str("\r\n");
            }
            for column in 0..self.width {
                let cell = self
                    .cells
                    .get(row * self.width + column)
                    .copied()
                    .unwrap_or_default();
                if ink != Some(cell.ink) {
                    // Reset before each change, so an attribute like reverse
                    // video cannot leak into the cell after it.
                    out.push_str("\x1b[0m");
                    out.push_str(cell.ink.sequence());
                    ink = Some(cell.ink);
                }
                out.push(cell.what);
            }
        }
        let _ended = write!(out, "\x1b[0m");
        out
    }
}

#[cfg(test)]
mod tests;
