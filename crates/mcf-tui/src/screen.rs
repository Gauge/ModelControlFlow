use std::fmt::Write as _;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Ink {
    #[default]
    Plain,
    Quiet,
    Heading,
    Figure,
    Refusal,
    Held,
    Selected,
}

impl Ink {
    fn sequence(self) -> &'static str {
        match self {
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

#[derive(Debug)]
pub struct Screen {
    width: usize,
    height: usize,
    cells: Vec<Cell>,
}

impl Screen {
    #[must_use]
    pub fn new(width: u16, height: u16) -> Self {
        let (width, height) = (width as usize, height as usize);
        Self {
            width,
            height,
            cells: vec![Cell::default(); width * height],
        }
    }

    #[must_use]
    pub const fn width(&self) -> usize {
        self.width
    }

    #[must_use]
    pub const fn height(&self) -> usize {
        self.height
    }

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

    pub fn put_right(&mut self, column: usize, row: usize, text: &str, ink: Ink) {
        let width = text.chars().count();
        let start = column.saturating_sub(width);
        self.put(start, row, text, ink);
    }

    pub fn rule(&mut self, column: usize, row: usize, width: usize, ink: Ink) {
        let line: String =
            std::iter::repeat_n('─', width.min(self.width.saturating_sub(column))).collect();
        self.put(column, row, &line, ink);
    }

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

    #[must_use]
    pub fn at(&self, column: usize, row: usize) -> char {
        self.cells
            .get(row * self.width + column)
            .map_or(' ', |cell| cell.what)
    }

    #[must_use]
    pub fn ink(&self, column: usize, row: usize) -> Ink {
        self.cells
            .get(row * self.width + column)
            .map_or(Ink::Plain, |cell| cell.ink)
    }

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

    #[must_use]
    pub fn rendered(&self) -> String {
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
