#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Typing {
    said: String,
    caret: usize,
    anchor: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum By {
    Character,
    Word,
    Line,
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
        self.said.clear();
        self.caret = 0;
        self.anchor = 0;
    }

    pub fn set(&mut self, said: impl Into<String>) {
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
            (Way::Back, By::Line) => 0,
            (Way::On, By::Line) => self.said.len(),
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
