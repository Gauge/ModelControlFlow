use mcf_core::configuration::Thousandths;
use mcf_record::json::Value;

pub use crate::generation::{Draw, Stated, Truncation, Whose};

pub const SEEDS: usize = 3;

pub const NO_INSTRUCTION: &str = "The room is quiet.";

pub const MOST_FORCED: usize = 12;

pub const MOST_CLAUSES: usize = 8;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Unit {
    Word,
    Phrase,
    Sentence,
    Paragraph,
}

impl Unit {
    pub const ALL: [Self; 4] = [Self::Word, Self::Phrase, Self::Sentence, Self::Paragraph];

    #[must_use]
    pub fn for_text(text: &str) -> Self {
        if parts_of(text, Self::Paragraph).len() > 1 {
            Self::Paragraph
        } else {
            Self::Phrase
        }
    }

    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Word => "word",
            Self::Phrase => "phrase",
            Self::Sentence => "sentence",
            Self::Paragraph => "paragraph",
        }
    }

    #[must_use]
    pub fn named(word: &str) -> Option<Self> {
        match word {
            "word" | "words" => Some(Self::Word),
            "phrase" | "phrases" => Some(Self::Phrase),
            "sentence" | "sentences" => Some(Self::Sentence),
            "paragraph" | "paragraphs" => Some(Self::Paragraph),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Part {
    pub text: String,
    pub after: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Taken<'a> {
    pub text: &'a str,
    pub by: Option<Unit>,
    pub most: Option<usize>,
    pub extras: Extras,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Extra {
    Floors,
    Alone,
    Prefixes,
    Swaps,
    Forms,
}

impl Extra {
    pub const ALL: [Self; 5] = [
        Self::Floors,
        Self::Alone,
        Self::Prefixes,
        Self::Swaps,
        Self::Forms,
    ];

    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Floors => "floors",
            Self::Alone => "alone",
            Self::Prefixes => "prefixes",
            Self::Swaps => "swaps",
            Self::Forms => "forms",
        }
    }

    #[must_use]
    pub fn named(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|extra| extra.name() == name)
    }

    #[must_use]
    pub const fn generations(self, parts: usize, removed: usize) -> usize {
        match self {
            Self::Floors => parts,
            Self::Alone => removed.saturating_add(1),
            Self::Prefixes | Self::Swaps => strict_prefixes(parts, removed),
            Self::Forms => {
                if parts > 1 {
                    Form::ALL.len()
                } else {
                    Form::OF_A_WHOLE
                }
            }
        }
    }

    #[must_use]
    pub const fn at_most(self) -> bool {
        matches!(self, Self::Forms)
    }

    #[must_use]
    pub const fn asks(self) -> &'static str {
        match self {
            Self::Floors => {
                "the control sentence at every position, so the floor is a spread rather than \
                 one draw"
            }
            Self::Alone => {
                "each part as the whole prompt in turn, to say which carries the answer on its \
                 own"
            }
            Self::Prefixes => {
                "the prompt grown a part at a time from the front, to say where the answer \
                 becomes the answer"
            }
            Self::Swaps => {
                "each part swapped with the one after it, to say whether the answer follows \
                 what the parts say or where they sit"
            }
            Self::Forms => {
                "the same parts on one line, as bullets, numbered, under headings, in tags \
                 and in capitals, to say whether the model follows the form or the words"
            }
        }
    }

    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Floors => "floor at every position",
            Self::Alone => "each part alone",
            Self::Prefixes => "prompt grown from the front",
            Self::Swaps => "neighbours swapped",
            Self::Forms => "the same words in six forms",
        }
    }

    #[must_use]
    pub const fn spent_on(self) -> &'static str {
        match self {
            Self::Floors => "for the control at every other position",
            Self::Alone => "for each part alone and the control alone",
            Self::Prefixes => "for the prompt grown a part at a time, short of the whole",
            Self::Swaps => "for each part swapped with the one after it",
            Self::Forms => "for the parts in each form the prompt is not already in",
        }
    }

    const fn bit(self) -> u8 {
        match self {
            Self::Floors => 1,
            Self::Alone => 2,
            Self::Prefixes => 4,
            Self::Swaps => 8,
            Self::Forms => 16,
        }
    }
}

const fn strict_prefixes(parts: usize, removed: usize) -> usize {
    let short = parts.saturating_sub(1);
    if removed < short { removed } else { short }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Extras(u8);

impl Extras {
    pub const NONE: Self = Self(0);

    #[must_use]
    pub const fn has(self, extra: Extra) -> bool {
        self.0 & extra.bit() != 0
    }

    #[must_use]
    pub const fn with(self, extra: Extra, asked: bool) -> Self {
        if asked {
            Self(self.0 | extra.bit())
        } else {
            Self(self.0 & !extra.bit())
        }
    }

    #[must_use]
    pub fn generations(self, parts: usize, removed: usize) -> usize {
        self.asked().fold(0, |sum, extra| {
            sum.saturating_add(extra.generations(parts, removed))
        })
    }

    pub fn asked(self) -> impl Iterator<Item = Extra> {
        Extra::ALL.into_iter().filter(move |extra| self.has(*extra))
    }

    pub fn named<'a>(names: impl IntoIterator<Item = &'a str>) -> Self {
        names
            .into_iter()
            .filter_map(Extra::named)
            .fold(Self::NONE, |set, extra| set.with(extra, true))
    }
}

impl Taken<'_> {
    #[must_use]
    pub fn unit(&self) -> (Unit, bool) {
        self.by
            .map_or_else(|| (Unit::for_text(self.text), false), |by| (by, true))
    }

    #[must_use]
    pub fn cap(&self) -> usize {
        self.most.unwrap_or(MOST_CLAUSES).max(1)
    }

    #[must_use]
    pub fn parts(&self) -> Vec<Part> {
        parts_of(self.text, self.unit().0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Held {
    pub first: Option<usize>,
    pub kept: usize,
    pub of: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Clause {
    pub text: String,
    pub without: String,
    pub changed: bool,
    pub moved: u64,
    pub held: Option<Held>,
    pub thought: Option<usize>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Settle {
    pub temperature: Thousandths,
    pub truncation: Truncation,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Settled {
    pub temperature: Thousandths,
    pub truncation: Truncation,
    pub asked: usize,
    pub distinct: usize,
    pub spread: u64,
    pub from_greedy: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FloorAt {
    pub position: usize,
    pub moved: u64,
    pub held: Option<Held>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Spread {
    pub least: u64,
    pub middle: u64,
    pub most: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reading {
    pub moved: u64,
    pub held: Option<Held>,
    pub answer: String,
    pub thought: Option<usize>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Form {
    OneLine,
    Bullets,
    Numbered,
    Headings,
    Tags,
    Capitals,
}

impl Form {
    pub const ALL: [Self; 6] = [
        Self::OneLine,
        Self::Bullets,
        Self::Numbered,
        Self::Headings,
        Self::Tags,
        Self::Capitals,
    ];

    pub const OF_A_WHOLE: usize = 2;

    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::OneLine => "one line",
            Self::Bullets => "bullets",
            Self::Numbered => "numbered",
            Self::Headings => "headings",
            Self::Tags => "tags",
            Self::Capitals => "capitals",
        }
    }

    #[must_use]
    pub fn named(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|form| form.name() == name)
    }

    #[must_use]
    pub const fn lists(self) -> bool {
        !matches!(self, Self::OneLine | Self::Capitals)
    }

    #[must_use]
    pub fn render(self, parts: &[Part]) -> Option<String> {
        if parts.is_empty() || (self.lists() && parts.len() < 2) {
            return None;
        }
        let flat = |part: &Part| part.text.split_whitespace().collect::<Vec<_>>().join(" ");
        let listed = |dressed: &dyn Fn(usize, String) -> String, between: &str| {
            parts
                .iter()
                .enumerate()
                .map(|(at, part)| dressed(at.saturating_add(1), flat(part)))
                .collect::<Vec<_>>()
                .join(between)
        };
        Some(match self {
            Self::OneLine => parts.iter().map(flat).collect::<Vec<_>>().join(" "),
            Self::Bullets => listed(&|_, text| format!("- {text}"), "\n"),
            Self::Numbered => listed(&|number, text| format!("{number}. {text}"), "\n"),
            Self::Headings => listed(&|number, text| format!("## {number}\n\n{text}"), "\n\n"),
            Self::Tags => listed(
                &|_, text| format!("<instruction>{text}</instruction>"),
                "\n",
            ),
            Self::Capitals => joined(parts).to_uppercase(),
        })
    }
}

#[must_use]
pub fn forms_asked(parts: &[Part]) -> usize {
    let written = joined(parts);
    Form::ALL
        .into_iter()
        .filter(|form| {
            form.render(parts)
                .is_some_and(|rendered| rendered != written)
        })
        .count()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Formed {
    pub form: Form,
    pub outcome: Rendering,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Rendering {
    Read(Reading),
    NotRendered(&'static str),
}

pub const AS_WRITTEN: &str = "the prompt is written this way";

pub const ONE_PART: &str = "one part is nothing to list";

struct Bench<'a, 'b> {
    baseline: &'a str,
    opening: &'a [usize],
    seed: u64,
    ask: Ask<'b>,
    force: Force<'b>,
    say: Say<'b>,
    count: usize,
    of: usize,
}

impl Bench<'_, '_> {
    fn ask_as(&mut self, what: String, prompt: &str, draw: Draw) -> Answered {
        self.count = self.count.saturating_add(1);
        (self.say)(Step {
            what,
            count: self.count,
            of: self.of,
        });
        (self.ask)(prompt, draw)
    }

    fn read(&mut self, what: String, prompt: &str) -> Reading {
        let said = self.ask_as(what, prompt, Draw::greedy(self.seed));
        let held = if self.opening.is_empty() {
            None
        } else {
            (self.force)(prompt, self.opening)
        };
        Reading {
            moved: moved_by(self.baseline.trim(), said.text.trim()),
            held,
            answer: said.text,
            thought: said.thought,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Report {
    pub floor: u64,
    pub floor_held: Option<Held>,
    pub floor_thought: Option<usize>,
    pub floors: Option<Vec<FloorAt>>,
    pub alone: Option<Vec<Reading>>,
    pub alone_floor: Option<Reading>,
    pub prefixes: Option<Vec<Reading>>,
    pub swaps: Option<Vec<Reading>>,
    pub forms: Option<Vec<Formed>>,
    pub baseline: String,
    pub baseline_thought: Option<usize>,
    pub clauses: Vec<Clause>,
    pub clauses_over_the_cap: usize,
    pub unit: Unit,
    pub unit_chosen: bool,
    pub most: usize,
    pub settled: Option<Settled>,
}

impl Report {
    #[must_use]
    pub fn floor_spread(&self) -> Option<Spread> {
        let mut drawn: Vec<u64> = self.floors.as_ref()?.iter().map(|at| at.moved).collect();
        drawn.sort_unstable();
        #[allow(
            clippy::integer_division,
            reason = "the upper median's index: a position in a list, not a figure"
        )]
        let middle = *drawn.get(drawn.len() / 2)?;
        Some(Spread {
            least: *drawn.first()?,
            middle,
            most: *drawn.last()?,
        })
    }

    #[must_use]
    pub fn unused(&self) -> Vec<&Clause> {
        self.clauses.iter().filter(|held| !held.changed).collect()
    }

    #[must_use]
    pub fn barely_moved(&self, most: u64) -> Vec<&Clause> {
        self.clauses
            .iter()
            .filter(|held| held.moved <= most)
            .collect()
    }

    #[must_use]
    pub fn at_the_floor(&self) -> Vec<&Clause> {
        self.clauses
            .iter()
            .filter(|held| held.moved <= self.floor)
            .collect()
    }
}

#[must_use]
pub fn clauses_of(prompt: &str) -> Vec<String> {
    parts_of(prompt, Unit::Sentence)
        .into_iter()
        .map(|part| part.text)
        .collect()
}

fn ends_a_sentence(character: char, next: Option<char>) -> bool {
    character == '\n'
        || (matches!(character, '.' | '?' | '!') && next.is_none_or(char::is_whitespace))
}

fn ends_a_phrase(character: char, next: Option<char>) -> bool {
    ends_a_sentence(character, next)
        || (matches!(character, ',' | ';' | ':') && next.is_none_or(char::is_whitespace))
}

#[must_use]
pub fn parts_of(text: &str, by: Unit) -> Vec<Part> {
    let mut found: Vec<Part> = Vec::new();
    let mut held = String::new();
    let mut characters = text.chars().peekable();
    let mut between = String::new();
    while let Some(character) = characters.next() {
        let ends = match by {
            Unit::Word => {
                held.push(character);
                characters.peek().is_none_or(|next| next.is_whitespace())
            }
            Unit::Phrase => {
                held.push(character);
                ends_a_phrase(character, characters.peek().copied())
            }
            Unit::Sentence => {
                held.push(character);
                ends_a_sentence(character, characters.peek().copied())
            }
            Unit::Paragraph => {
                if character == '\n' {
                    let mut ahead = characters.clone();
                    let blank = std::iter::from_fn(|| ahead.next())
                        .take_while(|next| next.is_whitespace())
                        .any(|next| next == '\n');
                    if !blank {
                        held.push(character);
                    }
                    blank
                } else {
                    held.push(character);
                    false
                }
            }
        };
        if !ends {
            continue;
        }
        let mut after = String::new();
        if by == Unit::Paragraph {
            after.push(character);
        }
        while let Some(next) = characters
            .peek()
            .copied()
            .filter(|next| next.is_whitespace())
        {
            after.push(next);
            characters.next();
        }
        let trimmed = held.trim().to_owned();
        let trailing: String = held
            .chars()
            .rev()
            .take_while(|held| held.is_whitespace())
            .collect::<Vec<char>>()
            .into_iter()
            .rev()
            .collect();
        held.clear();
        if trimmed.chars().any(char::is_alphanumeric) {
            if let Some(last) = found.last_mut() {
                last.after.push_str(&between);
            }
            between.clear();
            found.push(Part {
                text: trimmed,
                after: String::new(),
            });
            between.push_str(&trailing);
            between.push_str(&after);
        } else {
            between.push_str(&trimmed);
            between.push_str(&trailing);
            between.push_str(&after);
        }
    }
    let trimmed = held.trim().to_owned();
    if trimmed.chars().any(char::is_alphanumeric) {
        if let Some(last) = found.last_mut() {
            last.after.push_str(&between);
        }
        found.push(Part {
            text: trimmed,
            after: String::new(),
        });
    }
    if by == Unit::Phrase {
        with_short_phrases_joined(found)
    } else {
        found
    }
}

const FEWEST_WORDS_IN_A_PHRASE: usize = 3;

fn with_short_phrases_joined(found: Vec<Part>) -> Vec<Part> {
    let words_in = |text: &str| text.split_whitespace().count();
    let mut joined: Vec<Part> = Vec::new();
    for part in found {
        if let Some(last) = joined.last_mut()
            && last.text.ends_with([',', ';', ':'])
            && (words_in(&last.text) < FEWEST_WORDS_IN_A_PHRASE
                || words_in(&part.text) < FEWEST_WORDS_IN_A_PHRASE)
        {
            last.text.push_str(&last.after);
            last.text.push_str(&part.text);
            last.after = part.after;
            continue;
        }
        joined.push(part);
    }
    joined
}

#[must_use]
pub fn joined(parts: &[Part]) -> String {
    let mut text = String::new();
    for (at, part) in parts.iter().enumerate() {
        text.push_str(&part.text);
        if at.saturating_add(1) < parts.len() {
            text.push_str(&part.after);
        }
    }
    text
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Surprise {
    pub tokens: usize,
    pub first_choice: usize,
    pub past_depth: usize,
    pub no_context: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rank {
    At(usize),
    PastDepth,
    NoContext,
}

#[must_use]
pub fn surprise_by_part(parts: &[Part], ranked: &[(String, Rank)]) -> (Vec<Surprise>, usize) {
    let prompt = joined(parts);
    let mut ends = Vec::with_capacity(parts.len());
    let mut at = 0_usize;
    for (index, part) in parts.iter().enumerate() {
        at = at.saturating_add(part.text.len());
        ends.push(at);
        if index.saturating_add(1) < parts.len() {
            at = at.saturating_add(part.after.len());
        }
    }
    let mut found = vec![Surprise::default(); parts.len()];
    let mut nowhere = 0_usize;
    let mut cursor = 0_usize;
    for (text, rank) in ranked {
        let piece = text.trim();
        if piece.is_empty() {
            continue;
        }
        let rest = prompt.get(cursor..).unwrap_or_default();
        let skipped = rest.len().saturating_sub(rest.trim_start().len());
        cursor = cursor.saturating_add(skipped);
        let rest = prompt.get(cursor..).unwrap_or_default();
        if !rest.starts_with(piece) {
            nowhere = nowhere.saturating_add(1);
            continue;
        }
        let index = ends.iter().position(|end| cursor < *end);
        if let Some(surprise) = index.and_then(|index| found.get_mut(index)) {
            surprise.tokens = surprise.tokens.saturating_add(1);
            match rank {
                Rank::At(1) => surprise.first_choice = surprise.first_choice.saturating_add(1),
                Rank::At(_) => {}
                Rank::PastDepth => surprise.past_depth = surprise.past_depth.saturating_add(1),
                Rank::NoContext => surprise.no_context = surprise.no_context.saturating_add(1),
            }
        } else {
            nowhere = nowhere.saturating_add(1);
        }
        cursor = cursor.saturating_add(piece.len());
    }
    (found, nowhere)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Expected {
    pub text: String,
    pub pieces: usize,
    pub rank: Option<usize>,
    pub first_choice: usize,
    pub unread: usize,
    pub part: Option<usize>,
}

#[must_use]
pub fn expected_by_word(parts: &[Part], ranked: &[(String, Rank)]) -> (Vec<Expected>, usize) {
    let prompt = joined(parts);
    let words = parts_of(&prompt, Unit::Word);
    let ends_of = |held: &[Part]| {
        let mut ends = Vec::with_capacity(held.len());
        let mut at = 0_usize;
        for (index, part) in held.iter().enumerate() {
            at = at.saturating_add(part.text.len());
            ends.push(at);
            if index.saturating_add(1) < held.len() {
                at = at.saturating_add(part.after.len());
            }
        }
        ends
    };
    let word_ends = ends_of(&words);
    let part_ends = ends_of(parts);
    let mut found: Vec<Expected> = words
        .iter()
        .zip(&word_ends)
        .map(|(word, end)| Expected {
            text: word.text.clone(),
            pieces: 0,
            rank: None,
            first_choice: 0,
            unread: 0,
            part: part_ends
                .iter()
                .position(|part_end| end.saturating_sub(word.text.len()) < *part_end)
                .map(|index| index.saturating_add(1)),
        })
        .collect();
    let mut nowhere = 0_usize;
    let mut cursor = 0_usize;
    for (text, rank) in ranked {
        let piece = text.trim();
        if piece.is_empty() {
            continue;
        }
        let rest = prompt.get(cursor..).unwrap_or_default();
        let skipped = rest.len().saturating_sub(rest.trim_start().len());
        cursor = cursor.saturating_add(skipped);
        let rest = prompt.get(cursor..).unwrap_or_default();
        if !rest.starts_with(piece) {
            nowhere = nowhere.saturating_add(1);
            continue;
        }
        let index = word_ends.iter().position(|end| cursor < *end);
        if let Some(word) = index.and_then(|index| found.get_mut(index)) {
            if word.pieces == word.unread
                && let Rank::At(at) = rank
            {
                word.rank = Some(*at);
            }
            word.pieces = word.pieces.saturating_add(1);
            match rank {
                Rank::At(1) => word.first_choice = word.first_choice.saturating_add(1),
                Rank::At(_) | Rank::PastDepth => {}
                Rank::NoContext => word.unread = word.unread.saturating_add(1),
            }
        } else {
            nowhere = nowhere.saturating_add(1);
        }
        cursor = cursor.saturating_add(piece.len());
    }
    (found, nowhere)
}

#[must_use]
pub fn with_inert(parts: &[Part]) -> String {
    with_inert_at(parts, parts.len().saturating_sub(1))
}

#[must_use]
pub fn with_inert_at(parts: &[Part], at: usize) -> String {
    let mut all = parts.to_vec();
    let at = at.min(all.len());
    let separator = at
        .checked_sub(1)
        .and_then(|before| all.get(before))
        .map(|part| part.after.clone())
        .filter(|after| !after.is_empty())
        .or_else(|| {
            all.iter()
                .map(|part| part.after.clone())
                .find(|after| !after.is_empty())
        })
        .unwrap_or_else(|| " ".to_owned());
    if at == all.len() {
        if let Some(last) = all.last_mut() {
            last.after.clone_from(&separator);
        }
        all.push(Part {
            text: NO_INSTRUCTION.to_owned(),
            after: String::new(),
        });
    } else {
        all.insert(
            at,
            Part {
                text: NO_INSTRUCTION.to_owned(),
                after: separator,
            },
        );
    }
    joined(&all)
}

#[must_use]
pub fn without(parts: &[Part], at: usize) -> String {
    let kept: Vec<Part> = parts
        .iter()
        .enumerate()
        .filter(|(index, _)| *index != at)
        .map(|(_, held)| held.clone())
        .collect();
    joined(&kept)
}

#[must_use]
pub fn moved_by(one: &str, other: &str) -> u64 {
    let a = words_of(one);
    let b = words_of(other);
    let longest = a.len().max(b.len());
    if longest == 0 {
        return 0;
    }
    let mut row: Vec<usize> = (0..=b.len()).collect();
    for (i, left) in a.iter().enumerate() {
        let mut previous = row.first().copied().unwrap_or(0);
        if let Some(first) = row.first_mut() {
            *first = i.saturating_add(1);
        }
        for (j, right) in b.iter().enumerate() {
            let held = row.get(j.saturating_add(1)).copied().unwrap_or(0);
            let cost = usize::from(left != right);
            let candidate = previous
                .saturating_add(cost)
                .min(held.saturating_add(1))
                .min(row.get(j).copied().unwrap_or(0).saturating_add(1));
            if let Some(cell) = row.get_mut(j.saturating_add(1)) {
                *cell = candidate;
            }
            previous = held;
        }
    }
    let distance = row.last().copied().unwrap_or(0);
    u64::try_from(distance)
        .unwrap_or(0)
        .saturating_mul(1_000_000)
        .wrapping_div(u64::try_from(longest).unwrap_or(1).max(1))
}

fn words_of(text: &str) -> Vec<&str> {
    text.split_whitespace()
        .map(|word| word.trim_matches(is_sentence_punctuation))
        .filter(|word| !word.is_empty())
        .collect()
}

fn is_sentence_punctuation(c: char) -> bool {
    matches!(
        c,
        '.' | ','
            | ';'
            | ':'
            | '!'
            | '?'
            | '"'
            | '\''
            | '('
            | ')'
            | '['
            | ']'
            | '{'
            | '}'
            | '«'
            | '»'
            | '“'
            | '”'
            | '‘'
            | '’'
            | '…'
            | '*'
            | '`'
    )
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Answered {
    pub text: String,
    pub tokens: Vec<usize>,
    pub thought: Option<usize>,
}

pub type Ask<'a> = &'a mut dyn FnMut(&str, Draw) -> Answered;

pub type Force<'a> = &'a mut dyn FnMut(&str, &[usize]) -> Option<Held>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Step {
    pub what: String,
    pub count: usize,
    pub of: usize,
}

impl Step {
    #[must_use]
    pub fn to_value(&self) -> Value {
        let count = |held: usize| Value::Integer(i64::try_from(held).unwrap_or(i64::MAX));
        Value::map([
            ("what", Value::text(self.what.clone())),
            ("count", count(self.count)),
            ("of", count(self.of)),
        ])
    }
}

#[must_use]
pub fn step_said(body: &Value) -> Option<String> {
    let step = body.get("step")?;
    let figure = |key: &str| step.get(key).and_then(Value::as_integer);
    Some(format!(
        "generation {} of {}: {}",
        figure("count")?,
        figure("of")?,
        step.get("what").and_then(Value::as_text)?
    ))
}

pub type Say<'a> = &'a mut dyn FnMut(Step);

#[must_use]
pub fn planned(taken: &Taken<'_>, settling: bool) -> usize {
    let (unit, _) = taken.unit();
    let all = parts_of(taken.text, unit);
    let written = joined(&all);
    let ablated = all.len().min(taken.cap());
    let many = all.len() > 1;
    let extras = if many {
        taken
            .extras
            .with(Extra::Forms, false)
            .generations(all.len(), ablated)
    } else {
        0
    };
    let forms = if taken.extras.has(Extra::Forms) {
        Form::ALL
            .into_iter()
            .filter(|form| {
                form.render(&all)
                    .is_some_and(|rendered| rendered != written)
            })
            .count()
    } else {
        0
    };
    2_usize
        .saturating_add(if many { ablated } else { 0 })
        .saturating_add(extras)
        .saturating_add(forms)
        .saturating_add(if settling { SEEDS } else { 0 })
}

#[must_use]
pub fn measure(
    taken: &Taken<'_>,
    seed: u64,
    settle: Option<Settle>,
    ask: Ask<'_>,
    force: Force<'_>,
    say: Say<'_>,
) -> Report {
    let (unit, unit_chosen) = taken.unit();
    let all = parts_of(taken.text, unit);
    let of = planned(taken, settle.is_some());
    let prompt = joined(&all);
    say(Step {
        what: "the prompt as written".to_owned(),
        count: 1,
        of,
    });
    let Answered {
        text: baseline,
        tokens: opening,
        thought: baseline_thought,
    } = ask(&prompt, Draw::greedy(seed));
    let opening: Vec<usize> = opening.into_iter().take(MOST_FORCED).collect();
    let mut bench = Bench {
        baseline: &baseline,
        opening: &opening,
        seed,
        ask,
        force,
        say,
        count: 1,
        of,
    };

    let most = taken.cap();
    let ablated = all.len().min(most);
    let mut clauses = Vec::with_capacity(ablated);
    if all.len() > 1 {
        for at in 0..ablated {
            let shortened = without(&all, at);
            let read = bench.read(
                format!("without part {} of {}", at.saturating_add(1), all.len()),
                &shortened,
            );
            clauses.push(Clause {
                changed: read.answer.trim() != baseline.trim(),
                moved: read.moved,
                text: all
                    .get(at)
                    .map(|part| part.text.clone())
                    .unwrap_or_default(),
                without: read.answer,
                held: read.held,
                thought: read.thought,
            });
        }
    }

    let read = bench.read("the control sentence added".to_owned(), &with_inert(&all));
    let floor = read.moved;
    let floor_thought = read.thought;
    let floor_held = if all.len() <= 1 { None } else { read.held };
    let floors = (taken.extras.has(Extra::Floors) && all.len() > 1)
        .then(|| floors_of(&mut bench, &all, (floor, floor_held)));
    let (alone, alone_floor) = if taken.extras.has(Extra::Alone) && all.len() > 1 {
        let (alone, floor) = alone_of(&mut bench, &all, ablated);
        (Some(alone), Some(floor))
    } else {
        (None, None)
    };
    let prefixes = (taken.extras.has(Extra::Prefixes) && all.len() > 1)
        .then(|| prefixes_of(&mut bench, &all, ablated));
    let swaps = (taken.extras.has(Extra::Swaps) && all.len() > 1)
        .then(|| swaps_of(&mut bench, &all, ablated));
    let forms = taken
        .extras
        .has(Extra::Forms)
        .then(|| forms_of(&mut bench, &all, &prompt));

    let settled = settle.map(|under| settled(&prompt, seed, under, &baseline, &mut bench));

    Report {
        floor,
        floor_held,
        floor_thought,
        floors,
        alone,
        alone_floor,
        prefixes,
        swaps,
        forms,
        baseline,
        baseline_thought,
        clauses,
        clauses_over_the_cap: all.len().saturating_sub(ablated),
        unit,
        unit_chosen,
        most,
        settled,
    }
}

fn floors_of(bench: &mut Bench<'_, '_>, all: &[Part], drawn: (u64, Option<Held>)) -> Vec<FloorAt> {
    let drawn_at = all.len().saturating_sub(1);
    (0..=all.len())
        .map(|position| {
            if position == drawn_at {
                return FloorAt {
                    position,
                    moved: drawn.0,
                    held: drawn.1,
                };
            }
            let read = bench.read(
                format!(
                    "the control sentence at position {} of {}",
                    position,
                    all.len()
                ),
                &with_inert_at(all, position),
            );
            FloorAt {
                position,
                moved: read.moved,
                held: read.held,
            }
        })
        .collect()
}

fn alone_of(bench: &mut Bench<'_, '_>, all: &[Part], first: usize) -> (Vec<Reading>, Reading) {
    let alone = all
        .iter()
        .take(first)
        .enumerate()
        .map(|(at, part)| {
            bench.read(
                format!("part {} of {} alone", at.saturating_add(1), all.len()),
                part.text.trim(),
            )
        })
        .collect();
    (
        alone,
        bench.read("the control sentence alone".to_owned(), NO_INSTRUCTION),
    )
}

fn prefixes_of(bench: &mut Bench<'_, '_>, all: &[Part], most: usize) -> Vec<Reading> {
    (1..=strict_prefixes(all.len(), most))
        .map(|kept| {
            let what = if kept == 1 {
                "the first part alone".to_owned()
            } else {
                format!("the first {kept} parts")
            };
            bench.read(what, &joined(all.get(..kept).unwrap_or_default()))
        })
        .collect()
}

fn swaps_of(bench: &mut Bench<'_, '_>, all: &[Part], most: usize) -> Vec<Reading> {
    (0..strict_prefixes(all.len(), most))
        .map(|at| {
            bench.read(
                format!(
                    "parts {} and {} swapped",
                    at.saturating_add(1),
                    at.saturating_add(2)
                ),
                &swapped(all, at),
            )
        })
        .collect()
}

#[must_use]
pub fn swapped(parts: &[Part], at: usize) -> String {
    let mut all = parts.to_vec();
    let after = at.saturating_add(1);
    if after < all.len() {
        let separators: Vec<String> = all.iter().map(|part| part.after.clone()).collect();
        all.swap(at, after);
        for (part, separator) in all.iter_mut().zip(separators) {
            part.after = separator;
        }
    }
    joined(&all)
}

fn forms_of(bench: &mut Bench<'_, '_>, all: &[Part], written: &str) -> Vec<Formed> {
    Form::ALL
        .into_iter()
        .map(|form| Formed {
            form,
            outcome: match form.render(all) {
                None => Rendering::NotRendered(ONE_PART),
                Some(rendered) if rendered == written => Rendering::NotRendered(AS_WRITTEN),
                Some(rendered) => {
                    Rendering::Read(bench.read(format!("as {}", form.name()), &rendered))
                }
            },
        })
        .collect()
}

fn settled(
    prompt: &str,
    seed: u64,
    under: Settle,
    baseline: &str,
    bench: &mut Bench<'_, '_>,
) -> Settled {
    let answers: Vec<String> = (0..SEEDS)
        .map(|extra| {
            let draw = Draw {
                seed: seed.wrapping_add(u64::try_from(extra).unwrap_or(u64::MAX)),
                temperature: under.temperature,
                truncation: under.truncation,
            };
            let what = format!(
                "seed {} of {SEEDS} at temperature {}",
                extra.saturating_add(1),
                under.temperature
            );
            bench.ask_as(what, prompt, draw).text.trim().to_owned()
        })
        .collect();
    let mut distinct = answers.clone();
    distinct.sort();
    distinct.dedup();
    let mut spread = 0_u64;
    for (at, one) in answers.iter().enumerate() {
        for other in answers.iter().skip(at.saturating_add(1)) {
            spread = spread.max(moved_by(one, other));
        }
    }
    let from_greedy = answers
        .iter()
        .map(|answer| moved_by(baseline.trim(), answer))
        .max()
        .unwrap_or(0);
    Settled {
        temperature: under.temperature,
        truncation: under.truncation,
        asked: SEEDS,
        distinct: distinct.len(),
        spread,
        from_greedy,
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod splitting_tests {
    #![allow(clippy::panic, clippy::expect_used)]

    use super::clauses_of;

    #[test]
    fn a_decimal_does_not_end_a_sentence() {
        let held = clauses_of("Results must be accurate to 0.001 tolerance. Use doubles.");
        assert_eq!(
            held,
            vec![
                "Results must be accurate to 0.001 tolerance.".to_owned(),
                "Use doubles.".to_owned()
            ],
            "a prompt about precision was ablated on half a number"
        );
    }

    #[test]
    fn a_dotted_name_does_not_end_a_sentence() {
        assert_eq!(
            clauses_of("Call arr.Length to get the size. Return an int."),
            vec![
                "Call arr.Length to get the size.".to_owned(),
                "Return an int.".to_owned()
            ]
        );
        assert_eq!(clauses_of("Use System.Numerics.").len(), 1);
    }

    #[test]
    fn a_line_ending_is_a_break() {
        assert_eq!(clauses_of("One line.\nAnother line.").len(), 2);
        assert_eq!(clauses_of("no punctuation\nand more").len(), 2);
    }

    #[test]
    fn an_abbreviation_is_still_read_as_an_ending() {
        assert_eq!(
            clauses_of("Use a library, e.g. System.Numerics. Keep it simple.").len(),
            3
        );
    }

    #[test]
    fn a_prompt_is_split_where_a_person_would_split_it() {
        let held = clauses_of(
            "write a class in c#. the class handles all the basic math functions. each \
             function should output extremely accurate results.",
        );
        assert_eq!(held.len(), 3, "{held:?}");
        assert!(held[0].ends_with("c#."), "{held:?}");
    }
}
