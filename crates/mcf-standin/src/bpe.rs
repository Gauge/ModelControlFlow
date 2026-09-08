use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Split {
    Gpt2,
    Gpt2DigitsApart,
    ModernThreeDigits,
    ModernOneDigit,
    ModernOneDigitSymbolsAlone,
    CasePartitionedThreeDigits,
}

#[must_use]
pub fn pieces(text: &str, split: Split) -> Vec<&str> {
    if split == Split::Gpt2DigitsApart {
        let mut out = Vec::new();
        for run in digits_apart(text) {
            if run.chars().next().is_some_and(char::is_numeric) {
                out.push(run);
            } else {
                out.extend(scan(run, Split::Gpt2));
            }
        }
        return out;
    }
    scan(text, split)
}

fn digits_apart(text: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut start = 0;
    let mut at = 0;
    while let Some(character) = text.get(at..).and_then(|tail| tail.chars().next()) {
        let next = at.saturating_add(character.len_utf8());
        if character.is_numeric() {
            if let Some(before) = text.get(start..at)
                && !before.is_empty()
            {
                out.push(before);
            }
            if let Some(digit) = text.get(at..next) {
                out.push(digit);
            }
            start = next;
        }
        at = next;
    }
    if let Some(rest) = text.get(start..)
        && !rest.is_empty()
    {
        out.push(rest);
    }
    out
}

fn scan(text: &str, split: Split) -> Vec<&str> {
    let mut out = Vec::new();
    let mut at = 0;
    while at < text.len() {
        let Some(rest) = text.get(at..) else { break };
        let taken = match split {
            Split::Gpt2 | Split::Gpt2DigitsApart => gpt2_piece(rest),
            Split::ModernThreeDigits => modern_piece(rest, &Modern::THREE_DIGITS),
            Split::ModernOneDigit => modern_piece(rest, &Modern::ONE_DIGIT),
            Split::ModernOneDigitSymbolsAlone => modern_piece(rest, &Modern::SYMBOLS_ALONE),
            Split::CasePartitionedThreeDigits => modern_piece(rest, &Modern::CASE_PARTITIONED),
        };
        let length = match taken {
            Some(length) if length > 0 => length,
            _ => rest.chars().next().map_or(1, char::len_utf8),
        };
        if let Some(piece) = rest.get(..length) {
            out.push(piece);
        }
        at = at.saturating_add(length);
    }
    out
}

fn gpt2_piece(rest: &str) -> Option<usize> {
    if let Some(length) = contraction(rest, false) {
        return Some(length);
    }
    if let Some(length) = spaced_run(rest, char::is_alphabetic) {
        return Some(length);
    }
    if let Some(length) = spaced_run(rest, char::is_numeric) {
        return Some(length);
    }
    if let Some(length) = spaced_run(rest, is_symbol) {
        return Some(length);
    }
    whitespace_run(rest)
}

struct Modern {
    digits: usize,
    trailing: &'static [char],
    case_partitioned: bool,
}

impl Modern {
    const THREE_DIGITS: Self = Self {
        digits: 3,
        trailing: &['\r', '\n'],
        case_partitioned: false,
    };
    const ONE_DIGIT: Self = Self {
        digits: 1,
        trailing: &['\r', '\n'],
        case_partitioned: false,
    };
    const SYMBOLS_ALONE: Self = Self {
        digits: 1,
        trailing: &[],
        case_partitioned: false,
    };
    const CASE_PARTITIONED: Self = Self {
        digits: 3,
        trailing: &['\r', '\n', '/'],
        case_partitioned: true,
    };
}

fn letter_not_lower(c: char) -> bool {
    c.is_alphabetic() && !c.is_ascii_lowercase()
}

fn letter_not_upper(c: char) -> bool {
    c.is_alphabetic() && !c.is_ascii_uppercase()
}

fn case_partitioned_run(rest: &str) -> Option<usize> {
    let lead = |c: char| !matches!(c, '\r' | '\n') && !c.is_alphabetic() && !c.is_numeric();
    for (first, second, first_may_be_empty) in [
        (
            letter_not_lower as fn(char) -> bool,
            letter_not_upper as fn(char) -> bool,
            true,
        ),
        (
            letter_not_lower as fn(char) -> bool,
            letter_not_upper as fn(char) -> bool,
            false,
        ),
    ] {
        let mut at = 0;
        if let Some(character) = rest.chars().next()
            && lead(character)
        {
            at = character.len_utf8();
        }
        let start = at;
        while let Some(character) = rest.get(at..).and_then(|tail| tail.chars().next()) {
            if first(character) {
                at = at.saturating_add(character.len_utf8());
            } else {
                break;
            }
        }
        if !first_may_be_empty && at == start {
            continue;
        }
        let after_first = at;
        while let Some(character) = rest.get(at..).and_then(|tail| tail.chars().next()) {
            if second(character) {
                at = at.saturating_add(character.len_utf8());
            } else {
                break;
            }
        }
        if first_may_be_empty && at == after_first {
            continue;
        }
        if at > 0 {
            if let Some(extra) = rest.get(at..).and_then(|tail| contraction(tail, true)) {
                at = at.saturating_add(extra);
            }
            return Some(at);
        }
    }
    None
}

fn modern_piece(rest: &str, shape: &Modern) -> Option<usize> {
    if let Some(length) = contraction(rest, true) {
        return Some(length);
    }
    if shape.case_partitioned {
        if let Some(length) = case_partitioned_run(rest) {
            return Some(length);
        }
    } else if let Some(length) = led_run(
        rest,
        |c| !matches!(c, '\r' | '\n') && !c.is_alphabetic() && !c.is_numeric(),
        char::is_alphabetic,
        usize::MAX,
    ) {
        return Some(length);
    }
    if let Some(length) = led_run(rest, |_| false, char::is_numeric, shape.digits) {
        return Some(length);
    }
    if let Some(length) = spaced_run(rest, is_symbol) {
        let mut end = length;
        while let Some(character) = rest.get(end..).and_then(|tail| tail.chars().next()) {
            if shape.trailing.contains(&character) {
                end = end.saturating_add(character.len_utf8());
            } else {
                break;
            }
        }
        return Some(end);
    }
    let run = run_of(rest, char::is_whitespace);
    if run > 0 {
        let mut last = None;
        let mut walk = 0;
        while walk < run {
            let Some(character) = rest.get(walk..).and_then(|tail| tail.chars().next()) else {
                break;
            };
            let next = walk.saturating_add(character.len_utf8());
            if matches!(character, '\r' | '\n') {
                last = Some(next);
            }
            walk = next;
        }
        if let Some(end) = last {
            return Some(end);
        }
    }
    whitespace_run(rest)
}

fn contraction(rest: &str, either_case: bool) -> Option<usize> {
    let tail = rest.strip_prefix('\'')?;
    let mut characters = tail.chars();
    let first = characters.next()?;
    let first = if either_case {
        first.to_ascii_lowercase()
    } else {
        first
    };
    let second = characters.next().map(|character| {
        if either_case {
            character.to_ascii_lowercase()
        } else {
            character
        }
    });
    match (first, second) {
        ('s' | 't' | 'm' | 'd', _) => Some(2),
        ('r' | 'v', Some('e')) | ('l', Some('l')) => Some(3),
        _ => None,
    }
}

fn spaced_run(rest: &str, class: fn(char) -> bool) -> Option<usize> {
    led_run(rest, |c| c == ' ', class, usize::MAX)
}

fn led_run(
    rest: &str,
    lead: fn(char) -> bool,
    class: fn(char) -> bool,
    most: usize,
) -> Option<usize> {
    let mut at = 0;
    if let Some(character) = rest.chars().next()
        && lead(character)
        && !class(character)
    {
        at = character.len_utf8();
    }
    let mut taken = 0;
    let mut end = at;
    while taken < most {
        let Some(character) = rest.get(end..).and_then(|tail| tail.chars().next()) else {
            break;
        };
        if !class(character) {
            break;
        }
        end = end.saturating_add(character.len_utf8());
        taken = taken.saturating_add(1);
    }
    if taken == 0 { None } else { Some(end) }
}

fn whitespace_run(rest: &str) -> Option<usize> {
    let run = run_of(rest, char::is_whitespace);
    if run == 0 {
        return None;
    }
    if run == rest.len() {
        return Some(run);
    }
    let last = rest
        .get(..run)
        .and_then(|blanks| blanks.chars().next_back())
        .map_or(1, char::len_utf8);
    let shorter = run.saturating_sub(last);
    if shorter == 0 {
        Some(run)
    } else {
        Some(shorter)
    }
}

fn is_symbol(character: char) -> bool {
    !character.is_whitespace() && !character.is_alphabetic() && !character.is_numeric()
}

fn run_of(rest: &str, class: fn(char) -> bool) -> usize {
    let mut end = 0;
    while let Some(character) = rest.get(end..).and_then(|tail| tail.chars().next()) {
        if !class(character) {
            break;
        }
        end = end.saturating_add(character.len_utf8());
    }
    end
}

#[must_use]
pub fn character_of(byte: u8) -> char {
    if matches!(byte, b'!'..=b'~' | 0xA1..=0xAC | 0xAE..=0xFF) {
        return char::from(byte);
    }
    let Some(moved) = MOVED.iter().position(|value| *value == byte) else {
        return '\u{FFFD}';
    };
    char::from_u32(0x100_u32.saturating_add(u32::try_from(moved).unwrap_or(0)))
        .unwrap_or('\u{FFFD}')
}

#[must_use]
pub fn byte_of(character: char) -> Option<u8> {
    let point = u32::from(character);
    if matches!(point, 0x21..=0x7E | 0xA1..=0xAC | 0xAE..=0xFF) {
        return u8::try_from(point).ok();
    }
    if (0x100..0x100 + 68).contains(&point) {
        let moved = point.saturating_sub(0x100);
        return MOVED.get(usize::try_from(moved).ok()?).copied();
    }
    None
}

const MOVED: [u8; 68] = [
    0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A, 0x0B, 0x0C, 0x0D, 0x0E, 0x0F,
    0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18, 0x19, 0x1A, 0x1B, 0x1C, 0x1D, 0x1E, 0x1F,
    0x20, 0x7F, 0x80, 0x81, 0x82, 0x83, 0x84, 0x85, 0x86, 0x87, 0x88, 0x89, 0x8A, 0x8B, 0x8C, 0x8D,
    0x8E, 0x8F, 0x90, 0x91, 0x92, 0x93, 0x94, 0x95, 0x96, 0x97, 0x98, 0x99, 0x9A, 0x9B, 0x9C, 0x9D,
    0x9E, 0x9F, 0xA0, 0xAD,
];

#[must_use]
pub fn spell(text: &str) -> String {
    text.bytes().map(character_of).collect()
}

#[derive(Debug, Clone, Default)]
pub struct Ranks(BTreeMap<String, usize>);

impl Ranks {
    #[must_use]
    pub fn read(merges: &[String]) -> Self {
        let mut ranks = BTreeMap::new();
        for (rank, merge) in merges.iter().enumerate() {
            ranks.entry(merge.clone()).or_insert(rank);
        }
        Self(ranks)
    }

    #[must_use]
    pub fn of(&self, left: &str, right: &str) -> Option<usize> {
        self.0.get(&format!("{left} {right}")).copied()
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

#[derive(Debug, Clone, Copy)]
struct Symbol {
    at: usize,
    length: usize,
    previous: i64,
    next: i64,
}

#[derive(Debug, Clone, Copy)]
struct Pair {
    left: i64,
    right: i64,
    rank: usize,
    size: usize,
}

#[must_use]
pub fn merge(piece: &str, ranks: &Ranks) -> Vec<String> {
    let mut symbols: Vec<Symbol> = Vec::new();
    for (position, character) in piece.char_indices() {
        let index = i64::try_from(symbols.len()).unwrap_or(0);
        symbols.push(Symbol {
            at: position,
            length: character.len_utf8(),
            previous: index.saturating_sub(1),
            next: index.saturating_add(1),
        });
    }
    if let Some(last) = symbols.last_mut() {
        last.next = -1;
    }

    let mut queue: Vec<Pair> = Vec::new();
    for index in 1..symbols.len() {
        let right = i64::try_from(index).unwrap_or(0);
        offer(
            piece,
            &symbols,
            right.saturating_sub(1),
            right,
            ranks,
            &mut queue,
        );
    }

    while let Some(best) = strongest(&mut queue) {
        let (Some(left), Some(right)) = (
            symbols
                .get(usize::try_from(best.left).unwrap_or(0))
                .copied(),
            symbols
                .get(usize::try_from(best.right).unwrap_or(0))
                .copied(),
        ) else {
            continue;
        };
        if left.length == 0
            || right.length == 0
            || left.length.saturating_add(right.length) != best.size
        {
            continue;
        }

        let after = right.next;
        if let Some(slot) = symbols.get_mut(usize::try_from(best.left).unwrap_or(0)) {
            slot.length = slot.length.saturating_add(right.length);
            slot.next = after;
        }
        if let Some(slot) = symbols.get_mut(usize::try_from(best.right).unwrap_or(0)) {
            slot.length = 0;
        }
        if after >= 0
            && let Some(slot) = symbols.get_mut(usize::try_from(after).unwrap_or(0))
        {
            slot.previous = best.left;
        }

        let before = symbols
            .get(usize::try_from(best.left).unwrap_or(0))
            .map_or(-1, |symbol| symbol.previous);
        offer(piece, &symbols, before, best.left, ranks, &mut queue);
        offer(piece, &symbols, best.left, after, ranks, &mut queue);
    }

    let mut out = Vec::new();
    let mut walk = 0_i64;
    while walk >= 0 {
        let Some(symbol) = symbols.get(usize::try_from(walk).unwrap_or(0)).copied() else {
            break;
        };
        if let Some(text) = piece.get(symbol.at..symbol.at.saturating_add(symbol.length)) {
            out.push(text.to_owned());
        }
        walk = symbol.next;
    }
    out
}

fn offer(
    piece: &str,
    symbols: &[Symbol],
    left: i64,
    right: i64,
    ranks: &Ranks,
    queue: &mut Vec<Pair>,
) {
    if left < 0 || right < 0 {
        return;
    }
    let (Some(first), Some(second)) = (
        symbols.get(usize::try_from(left).unwrap_or(0)),
        symbols.get(usize::try_from(right).unwrap_or(0)),
    ) else {
        return;
    };
    let (Some(head), Some(tail)) = (
        piece.get(first.at..first.at.saturating_add(first.length)),
        piece.get(second.at..second.at.saturating_add(second.length)),
    ) else {
        return;
    };
    let Some(rank) = ranks.of(head, tail) else {
        return;
    };
    queue.push(Pair {
        left,
        right,
        rank,
        size: first.length.saturating_add(second.length),
    });
}

fn strongest(queue: &mut Vec<Pair>) -> Option<Pair> {
    let mut best = 0;
    for (index, pair) in queue.iter().enumerate() {
        let current = queue.get(best)?;
        if pair.rank < current.rank || (pair.rank == current.rank && pair.left < current.left) {
            best = index;
        }
    }
    if queue.is_empty() {
        None
    } else {
        Some(queue.swap_remove(best))
    }
}

#[cfg(test)]
mod tests;
