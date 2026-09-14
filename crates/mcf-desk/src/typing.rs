#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Typing {
    said: String,
    caret: usize,
    anchor: usize,
    /// The column a run of up-and-down movement started from. Going down from the middle of
    /// a long line into a short one and on again comes back out at the column it set off
    /// from, rather than at the end of the short line it passed through. Anything else the
    /// caret does forgets it.
    column: Option<usize>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum By {
    Character,
    Word,
    /// To the start or the end of the line the caret is on. In text with no newlines in it
    /// that is the whole of it, which is what a one-line box wants; in text with newlines it
    /// is the line, which is what Home and End mean everywhere else.
    Line,
    /// To the same column of the line above or below.
    Row,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Way {
    Back,
    On,
}

impl Typing {
    #[must_use]
    pub fn of(said: impl Into<String>) -> Self {
        let said = said.into();
        let caret = said.len();
        Self {
            said,
            caret,
            anchor: caret,
            column: None,
        }
    }

    #[must_use]
    pub fn said(&self) -> &str {
        &self.said
    }

    #[must_use]
    pub const fn caret(&self) -> usize {
        self.caret
    }

    #[must_use]
    pub fn selection(&self) -> Option<(usize, usize)> {
        (self.caret != self.anchor)
            .then(|| (self.caret.min(self.anchor), self.caret.max(self.anchor)))
    }

    #[must_use]
    pub fn selected(&self) -> &str {
        match self.selection() {
            Some((from, to)) => self.said.get(from..to).unwrap_or(""),
            None => "",
        }
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.said.is_empty()
    }

    pub fn clear(&mut self) {
        self.column = None;
        self.said.clear();
        self.caret = 0;
        self.anchor = 0;
    }

    pub fn set(&mut self, said: impl Into<String>) {
        self.column = None;
        *self = Self::of(said);
    }

    #[must_use]
    pub fn trim(&self) -> &str {
        self.said.trim()
    }

    pub fn chars(&self) -> core::str::Chars<'_> {
        self.said.chars()
    }

    pub fn take(&mut self) -> String {
        let said = core::mem::take(&mut self.said);
        self.caret = 0;
        self.anchor = 0;
        said
    }

    fn settle(&mut self, at: usize, keeping: bool) {
        self.caret = self.round_to_a_character(at);
        if !keeping {
            self.anchor = self.caret;
        }
    }

    fn round_to_a_character(&self, at: usize) -> usize {
        let at = at.min(self.said.len());
        if self.said.is_char_boundary(at) {
            return at;
        }
        (0..=at)
            .rev()
            .find(|held| self.said.is_char_boundary(*held))
            .unwrap_or(0)
    }

    pub fn put(&mut self, text: &str, limit: usize) {
        self.column = None;
        self.rub_out_any_selection();
        let room = limit.saturating_sub(self.said.chars().count());
        let kept: String = text
            .chars()
            .filter(|held| *held != '\u{0}')
            .take(room)
            .collect();
        if kept.is_empty() {
            return;
        }
        self.said.insert_str(self.caret, &kept);
        self.settle(self.caret.saturating_add(kept.len()), false);
    }

    fn rub_out_any_selection(&mut self) -> bool {
        let Some((from, to)) = self.selection() else {
            return false;
        };
        let _gone: String = self.said.drain(from..to).collect();
        self.caret = from;
        self.anchor = from;
        true
    }

    pub fn rub(&mut self, way: Way, by: By) {
        self.column = None;
        if self.rub_out_any_selection() {
            return;
        }
        let to = self.step(way, by);
        let (from, until) = match way {
            Way::Back => (to, self.caret),
            Way::On => (self.caret, to),
        };
        if from == until {
            return;
        }
        let _gone: String = self.said.drain(from..until).collect();
        self.caret = from;
        self.anchor = from;
    }

    pub fn go(&mut self, way: Way, by: By, keeping: bool) {
        if by == By::Row {
            let column = self.column.unwrap_or_else(|| self.column_now());
            let to = self.a_row_away(way, column);
            self.settle(to, keeping);
            self.column = Some(column);
            return;
        }
        self.column = None;
        if !keeping
            && by == By::Character
            && let Some((from, to)) = self.selection()
        {
            self.settle(if way == Way::Back { from } else { to }, false);
            return;
        }
        let to = self.step(way, by);
        self.settle(to, keeping);
    }

    fn step(&self, way: Way, by: By) -> usize {
        match (way, by) {
            (Way::Back, By::Line) => self.line_began(),
            (Way::On, By::Line) => self.line_ended(),
            (way, By::Row) => self.a_row_away(way, self.column_now()),
            (Way::Back, By::Character) => self
                .said
                .get(..self.caret)
                .and_then(|before| before.char_indices().next_back().map(|(at, _)| at))
                .unwrap_or(0),
            (Way::On, By::Character) => self
                .said
                .get(self.caret..)
                .and_then(|after| {
                    after
                        .chars()
                        .next()
                        .map(|held| self.caret + held.len_utf8())
                })
                .unwrap_or(self.said.len()),
            (Way::Back, By::Word) => self.word_boundary_back(),
            (Way::On, By::Word) => self.word_boundary_on(),
        }
    }

    /// Where the line the caret is on starts: just after the newline before it, or the very
    /// beginning.
    fn line_began(&self) -> usize {
        self.said
            .get(..self.caret)
            .and_then(|before| before.rfind('\n').map(|at| at.saturating_add(1)))
            .unwrap_or(0)
    }

    /// Where it ends: at the next newline, or the very end.
    fn line_ended(&self) -> usize {
        self.said
            .get(self.caret..)
            .and_then(|after| after.find('\n').map(|at| self.caret.saturating_add(at)))
            .unwrap_or(self.said.len())
    }

    /// How far along its line the caret is, in characters rather than bytes, so that a line
    /// with anything but ASCII in it does not land the caret inside a character.
    fn column_now(&self) -> usize {
        let began = self.line_began();
        self.said
            .get(began..self.caret)
            .map_or(0, |held| held.chars().count())
    }

    /// This column, one line up or down, clamped to the end of the line it arrives at.
    fn a_row_away(&self, way: Way, column: usize) -> usize {
        let began = self.line_began();
        let wanted = match way {
            Way::Back => {
                if began == 0 {
                    return 0;
                }
                let above = self.said.get(..began.saturating_sub(1)).unwrap_or("");
                above.rfind('\n').map_or(0, |at| at.saturating_add(1))
            }
            Way::On => {
                let ended = self.line_ended();
                if ended >= self.said.len() {
                    return self.said.len();
                }
                ended.saturating_add(1)
            }
        };
        let rest = self.said.get(wanted..).unwrap_or("");
        let width = rest.find('\n').unwrap_or(rest.len());
        let line = rest.get(..width).unwrap_or("");
        let along = line.char_indices().nth(column).map_or(width, |(at, _)| at);
        wanted.saturating_add(along)
    }

    fn word_boundary_back(&self) -> usize {
        let before = self.said.get(..self.caret).unwrap_or("");
        let mut at = self.caret;
        let mut seen_a_word = false;
        for (index, held) in before.char_indices().rev() {
            if held.is_whitespace() {
                if seen_a_word {
                    break;
                }
            } else {
                seen_a_word = true;
            }
            at = index;
        }
        at
    }

    fn word_boundary_on(&self) -> usize {
        let after = self.said.get(self.caret..).unwrap_or("");
        let mut at = self.said.len();
        let mut seen_a_word = false;
        for (index, held) in after.char_indices() {
            if held.is_whitespace() {
                if seen_a_word {
                    at = self.caret.saturating_add(index);
                    return at;
                }
            } else {
                seen_a_word = true;
            }
        }
        at
    }

    pub fn all(&mut self) {
        self.anchor = 0;
        self.caret = self.said.len();
    }

    pub fn none(&mut self) {
        self.anchor = self.caret;
    }

    pub fn place(&mut self, at: usize, keeping: bool) {
        self.column = None;
        self.settle(at, keeping);
    }

    pub fn word_at(&mut self, at: usize) {
        let at = self.round_to_a_character(at);
        let before = self.said.get(..at).unwrap_or("");
        let from = before
            .char_indices()
            .rev()
            .take_while(|(_, held)| !held.is_whitespace())
            .map(|(index, _)| index)
            .last()
            .unwrap_or(at);
        let after = self.said.get(at..).unwrap_or("");
        let to = after
            .char_indices()
            .find(|(_, held)| held.is_whitespace())
            .map_or(self.said.len(), |(index, _)| at.saturating_add(index));
        self.anchor = from;
        self.caret = to;
    }

    #[must_use]
    pub fn cut(&mut self) -> Option<String> {
        let taken = self.selected().to_owned();
        if taken.is_empty() {
            return None;
        }
        let _gone = self.rub_out_any_selection();
        Some(taken)
    }

    #[must_use]
    pub fn copied(&self) -> Option<String> {
        match self.selection() {
            Some(_) => Some(self.selected().to_owned()),
            None => (!self.said.is_empty()).then(|| self.said.clone()),
        }
    }
}

#[cfg(test)]
mod tests;
