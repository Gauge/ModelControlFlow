//! Byte-pair encoding, the way GGUF's `gpt2` vocabularies mean it (B-365).
//!
//! **This is the other tokenizer.** A unigram vocabulary carries a score per
//! token and is segmented by choosing the highest-scoring split; a byte-pair
//! vocabulary carries an ordered list of *merges* and is segmented by applying
//! them in the order they are listed. The two produce different identifiers
//! from the same text, which is why the file says which it carries and MCF
//! refuses rather than guesses.
//!
//! **Three parts, in this order.** Text becomes bytes; the bytes become
//! characters of a printable alphabet; the characters are split into pieces by
//! a pre-tokenizer; each piece is merged separately. Nothing merges across a
//! piece boundary, which is what makes the pre-tokenizer part of the algorithm
//! rather than a tidying step: a different split is a different tokenization
//! and therefore a different answer from the same model.
//!
//! **The alphabet is why nothing can fail to be representable.** Every one of
//! the 256 byte values has a character, and every one of those characters is a
//! token in any real byte-level vocabulary, so the fallback is total by
//! construction — there is no `<0xNN>` here because there does not need to be.
//!
//! **The pre-tokenizer is a hand-written scanner, and it is stated which one.**
//! The upstream patterns are regular expressions over Unicode classes; this
//! reads them as an ordered alternation and tries each alternative at each
//! position, taking the first that matches, which is what an ordered
//! alternation means. `\p{L}` is read as `char::is_alphabetic` and `\p{N}` as
//! `char::is_numeric` — exact for every ASCII text, and for a handful of
//! combining marks Rust calls alphabetic where the regular expression would
//! not, a divergence stated here because MCF does not carry a Unicode table of
//! its own (A21: what is declared is not what is verified).

use std::collections::BTreeMap;

/// How a pre-tokenizer splits text before anything is merged.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Split {
    /// The original GPT-2 expression: contractions, then runs of letters, of
    /// digits, of symbols, each optionally preceded by one space.
    Gpt2,
    /// The expression llama.cpp calls `llama-bpe` and `qwen2` — they are the
    /// same expression — which differs from GPT-2's in three ways that matter:
    /// digits come in groups of at most three, a run of symbols swallows the
    /// newlines after it, and a run of whitespace ending in a newline is one
    /// piece.
    Modern,
}

/// Splits text into the pieces the merges are applied within.
///
/// The pieces are returned as byte ranges into `text` and cover it completely
/// and without overlap — a scanner that could drop a character would be losing
/// information silently (A1), so the walk advances by one character when no
/// alternative matches rather than skipping ahead.
#[must_use]
pub fn pieces(text: &str, split: Split) -> Vec<&str> {
    let mut out = Vec::new();
    let mut at = 0;
    while at < text.len() {
        let Some(rest) = text.get(at..) else { break };
        let taken = match split {
            Split::Gpt2 => gpt2_piece(rest),
            Split::Modern => modern_piece(rest),
        };
        let length = match taken {
            Some(length) if length > 0 => length,
            // No alternative matched. Every real alternation ends in `\s+` or a
            // catch-all, so this is unreachable for the two implemented here —
            // and it advances rather than looping, because a tokenizer that
            // hung on an unusual character would be a worse failure than one
            // that emitted it alone.
            _ => rest.chars().next().map_or(1, char::len_utf8),
        };
        if let Some(piece) = rest.get(..length) {
            out.push(piece);
        }
        at = at.saturating_add(length);
    }
    out
}

/// `'s|'t|'re|'ve|'m|'ll|'d| ?\p{L}+| ?\p{N}+| ?[^\s\p{L}\p{N}]+|\s+(?!\S)|\s+`
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

/// The expression `llama-bpe` and `qwen2` share, alternative by alternative.
fn modern_piece(rest: &str) -> Option<usize> {
    // `(?:'[sS]|'[tT]|…)` — the same contractions, either case.
    if let Some(length) = contraction(rest, true) {
        return Some(length);
    }
    // `[^\r\n\p{L}\p{N}]?\p{L}+` — one optional lead character that is not a
    // newline, a letter or a digit, then letters. Note that the lead is *any*
    // such character and not only a space, which is the difference that makes
    // `(hello` one piece here and two under GPT-2's.
    if let Some(length) = led_run(
        rest,
        |c| !matches!(c, '\r' | '\n') && !c.is_alphabetic() && !c.is_numeric(),
        char::is_alphabetic,
        usize::MAX,
    ) {
        return Some(length);
    }
    // `\p{N}{1,3}` — at most three digits, so that a long number is several
    // pieces and the merges cannot span them.
    if let Some(length) = led_run(rest, |_| false, char::is_numeric, 3) {
        return Some(length);
    }
    // ` ?[^\s\p{L}\p{N}]+[\r\n]*`
    if let Some(length) = spaced_run(rest, is_symbol) {
        let mut end = length;
        while let Some(character) = rest.get(end..).and_then(|tail| tail.chars().next()) {
            if matches!(character, '\r' | '\n') {
                end = end.saturating_add(character.len_utf8());
            } else {
                break;
            }
        }
        return Some(end);
    }
    // `\s*[\r\n]+` — greedy whitespace that backtracks to end on a newline,
    // which comes to: the run up to and including its last newline.
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

/// `'s`, `'t`, `'re`, `'ve`, `'m`, `'ll`, `'d`, optionally either case.
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

/// ` ?<class>+`: an optional single space, then one or more of a class.
fn spaced_run(rest: &str, class: fn(char) -> bool) -> Option<usize> {
    led_run(rest, |c| c == ' ', class, usize::MAX)
}

/// `<lead>?<class>{1,most}`, the shape four of the alternatives share.
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

/// `\s+(?!\S)` then `\s+`: a run of whitespace, less its last character when
/// something non-blank follows it — which is how the space before a word ends
/// up attached to the word rather than to the blanks before it.
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

/// Not whitespace, not a letter, not a digit.
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

/// The character a byte is written as, in the printable alphabet these
/// vocabularies are spelled in.
///
/// The construction is the one GPT-2 shipped: bytes that are already printable
/// and unambiguous stand for themselves, and the remaining 68 are moved up into
/// the block starting at `U+0100`, in order. It is a bijection — that is the
/// point of it — so `byte_of` inverts it exactly.
#[must_use]
pub fn character_of(byte: u8) -> char {
    if matches!(byte, b'!'..=b'~' | 0xA1..=0xAC | 0xAE..=0xFF) {
        return char::from(byte);
    }
    // Where it sits among the moved bytes decides where it lands, and that
    // position is read from the same table `byte_of` reads — so the two cannot
    // disagree by sharing an arithmetic mistake.
    let Some(moved) = MOVED.iter().position(|value| *value == byte) else {
        return '\u{FFFD}';
    };
    char::from_u32(0x100_u32.saturating_add(u32::try_from(moved).unwrap_or(0)))
        .unwrap_or('\u{FFFD}')
}

/// The byte a character of the alphabet stands for.
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

/// The 68 byte values that are not printable in the alphabet, in order.
///
/// Written out rather than computed so that `character_of` and `byte_of` are
/// checked against each other by a test rather than by sharing an arithmetic
/// mistake.
const MOVED: [u8; 68] = [
    0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A, 0x0B, 0x0C, 0x0D, 0x0E, 0x0F,
    0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18, 0x19, 0x1A, 0x1B, 0x1C, 0x1D, 0x1E, 0x1F,
    0x20, 0x7F, 0x80, 0x81, 0x82, 0x83, 0x84, 0x85, 0x86, 0x87, 0x88, 0x89, 0x8A, 0x8B, 0x8C, 0x8D,
    0x8E, 0x8F, 0x90, 0x91, 0x92, 0x93, 0x94, 0x95, 0x96, 0x97, 0x98, 0x99, 0x9A, 0x9B, 0x9C, 0x9D,
    0x9E, 0x9F, 0xA0, 0xAD,
];

/// Text as the alphabet spells it.
#[must_use]
pub fn spell(text: &str) -> String {
    text.bytes().map(character_of).collect()
}

/// The merges a file carries, by what they join.
///
/// Keyed by the two halves with a space between them, which is exactly how the
/// file writes them — and unambiguous, because a space is not a character of
/// the alphabet (it is written `Ġ`).
#[derive(Debug, Clone, Default)]
pub struct Ranks(BTreeMap<String, usize>);

impl Ranks {
    /// Reads a merge list, in the order it was written: earlier is stronger.
    #[must_use]
    pub fn read(merges: &[String]) -> Self {
        let mut ranks = BTreeMap::new();
        for (rank, merge) in merges.iter().enumerate() {
            // First wins: a merge listed twice is strongest where it first
            // appears, which is what applying the list in order would do.
            ranks.entry(merge.clone()).or_insert(rank);
        }
        Self(ranks)
    }

    /// How strong the merge of two pieces is, where it is a merge at all.
    #[must_use]
    pub fn of(&self, left: &str, right: &str) -> Option<usize> {
        self.0.get(&format!("{left} {right}")).copied()
    }

    /// How many merges it holds.
    #[must_use]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Whether it holds none, which no byte-pair vocabulary does.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

/// One run of the spelled text, in a chain that merges.
#[derive(Debug, Clone, Copy)]
struct Symbol {
    at: usize,
    /// Bytes covered. Zero means it was merged away.
    length: usize,
    previous: i64,
    next: i64,
}

/// A pending merge of two adjacent symbols.
#[derive(Debug, Clone, Copy)]
struct Pair {
    left: i64,
    right: i64,
    rank: usize,
    /// What the pair spelled when it was offered, so that a pair whose symbols
    /// have since grown can be recognised as stale rather than applied to the
    /// wrong text.
    size: usize,
}

/// Applies the merges to one pre-tokenized piece, and returns what it became.
///
/// The piece is already spelled in the alphabet. The result is the pieces in
/// order — every one of them a substring of the input, together covering it
/// exactly.
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

/// The merge to apply next: the earliest in the list, and the leftmost of
/// equals.
///
/// The tie-break is not decoration. Two implementations that break ties
/// differently produce different identifiers from the same text and therefore
/// different answers from the same model, which is exactly the kind of
/// difference nobody would think to look for (§3.12).
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
