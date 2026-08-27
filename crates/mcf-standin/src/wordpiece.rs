//! The `BertNormalizer`: what happens to text before word pieces are matched
//! (B-371, A21).
//!
//! **This is a transcription, and its scope is stated.** The reference
//! normalizer lowercases, strips accents by Unicode decomposition, drops
//! control characters, and splits at whitespace, punctuation and CJK
//! characters — all against full Unicode tables. MCF carries no Unicode table
//! (a decision made in [`crate::bpe`] and kept), so each of those is
//! implemented over a stated range rather than over everything:
//!
//! * **Lowercasing** is Rust's own, which agrees with the reference's tables.
//! * **Accent stripping** decomposes the Latin-1 Supplement and Latin
//!   Extended-A letters by table below, and drops combining marks (U+0300 –
//!   U+036F) wherever they appear. A precomposed accented letter *outside*
//!   Latin passes through unchanged where the reference would decompose it —
//!   a divergence this comment states rather than hides (A21).
//! * **Punctuation** is ASCII punctuation, the General Punctuation block, and
//!   CJK symbol/fullwidth-form punctuation; symbols below `0x7F` split too.
//! * **CJK characters** split one to a word, over the ranges the reference
//!   lists — transcribed, including its own off-by-a-row oddity, because
//!   matching the reference is the point.

/// Splits text into the words pieces are matched within.
#[must_use]
pub fn words(text: &str, lowercase: bool, strip_accents: bool) -> Vec<String> {
    let mut words: Vec<String> = Vec::new();
    let mut current = String::new();

    for raw in text.chars() {
        // One precomposed Latin letter may become a base letter (kept) and a
        // dropped accent; anything else passes through this loop once.
        let decomposed = if strip_accents { fold(raw) } else { raw };

        if decomposed.is_whitespace() {
            if !current.is_empty() {
                words.push(core::mem::take(&mut current));
            }
            continue;
        }
        if decomposed == '\0' || decomposed == '\u{FFFD}' || decomposed.is_control() {
            continue;
        }
        if strip_accents && is_combining_mark(decomposed) {
            continue;
        }

        let character = if lowercase {
            // `to_lowercase` may expand (ß → ss); every produced character is
            // taken, which is what the reference's per-codepoint table does
            // for the cases this range holds.
            let mut lowered = decomposed.to_lowercase();
            let (first, second) = (lowered.next(), lowered.next());
            if let (Some(single), None) = (first, second) {
                single
            } else {
                for extra in decomposed.to_lowercase() {
                    push_split(&mut words, &mut current, extra);
                }
                continue;
            }
        } else {
            decomposed
        };

        push_split(&mut words, &mut current, character);
    }

    if !current.is_empty() {
        words.push(current);
    }
    words
}

/// Adds one character: to the current word, or as a word of its own if it is
/// the kind that stands alone.
fn push_split(words: &mut Vec<String>, current: &mut String, character: char) {
    if is_punctuation(character) || is_cjk(character) {
        if !current.is_empty() {
            words.push(core::mem::take(current));
        }
        words.push(character.to_string());
    } else {
        current.push(character);
    }
}

/// Whether a character splits as punctuation.
fn is_punctuation(character: char) -> bool {
    if character.is_ascii() {
        return character.is_ascii_punctuation()
            || (!character.is_ascii_alphanumeric()
                && !character.is_ascii_whitespace()
                && !character.is_ascii_control());
    }
    matches!(character,
        '\u{2000}'..='\u{206F}' // general punctuation
        | '\u{3000}'..='\u{303F}' // CJK symbols and punctuation
        | '\u{FF01}'..='\u{FF0F}' | '\u{FF1A}'..='\u{FF20}' // fullwidth forms
        | '\u{FF3B}'..='\u{FF40}' | '\u{FF5B}'..='\u{FF65}')
}

/// A combining mark, dropped when accents are stripped.
fn is_combining_mark(character: char) -> bool {
    matches!(character, '\u{300}'..='\u{36F}')
}

/// The CJK ranges the reference splits one character to a word.
///
/// Transcribed from `is_chinese_char`, including the range its own comment
/// calls a transcription error kept for compatibility.
fn is_cjk(character: char) -> bool {
    matches!(u32::from(character),
        0x04E00..=0x09FFF
        | 0x03400..=0x04DBF
        | 0x20000..=0x2A6DF
        | 0x2A700..=0x2B73F
        | 0x2B740..=0x2B81F
        | 0x2B920..=0x2CEAF
        | 0x0F900..=0x0FAFF
        | 0x2F800..=0x2FA1F)
}

/// A precomposed Latin letter's base letter, where decomposing it and dropping
/// the accent would leave one.
///
/// The table covers Latin-1 Supplement and Latin Extended-A. Letters whose
/// decoration is not an accent — a stroke (`ø`, `đ`, `ł`), a dotless form
/// (`ı`), a ligature (`œ`, `æ`) — do not decompose and pass through unchanged,
/// which is also the reference's behaviour: NFD leaves them whole.
#[allow(
    clippy::too_many_lines,
    reason = "the fold table is one entry per Latin letter family, and a table split across \
              functions is a table nobody can audit against the Unicode decompositions it \
              transcribes"
)]
fn fold(character: char) -> char {
    match character {
        'À'..='Å' | 'à'..='å' | 'Ā' | 'ā' | 'Ă' | 'ă' | 'Ą' | 'ą' => {
            if character.is_uppercase() { 'A' } else { 'a' }
        }
        'Ç' | 'ç' | 'Ć'..='č' => {
            if character.is_uppercase() {
                'C'
            } else {
                'c'
            }
        }
        'Ď' | 'ď' => {
            if character.is_uppercase() {
                'D'
            } else {
                'd'
            }
        }
        'È'..='Ë' | 'è'..='ë' | 'Ē'..='ě' => {
            if character.is_uppercase() {
                'E'
            } else {
                'e'
            }
        }
        'Ĝ'..='ģ' => {
            if character.is_uppercase() {
                'G'
            } else {
                'g'
            }
        }
        'Ĥ' | 'ĥ' => {
            if character.is_uppercase() {
                'H'
            } else {
                'h'
            }
        }
        'Ì'..='Ï' | 'ì'..='ï' | 'Ĩ'..='į' | 'İ' => {
            if character.is_uppercase() {
                'I'
            } else {
                'i'
            }
        }
        'Ĵ' | 'ĵ' => {
            if character.is_uppercase() {
                'J'
            } else {
                'j'
            }
        }
        'Ķ' | 'ķ' => {
            if character.is_uppercase() {
                'K'
            } else {
                'k'
            }
        }
        'Ĺ'..='ŀ' => {
            if character.is_uppercase() {
                'L'
            } else {
                'l'
            }
        }
        'Ñ' | 'ñ' | 'Ń'..='ň' => {
            if character.is_uppercase() {
                'N'
            } else {
                'n'
            }
        }
        'Ò'..='Ö' | 'ò'..='ö' | 'Ō'..='ő' => {
            if character.is_uppercase() {
                'O'
            } else {
                'o'
            }
        }
        'Ŕ'..='ř' => {
            if character.is_uppercase() {
                'R'
            } else {
                'r'
            }
        }
        'Ś'..='š' => {
            if character.is_uppercase() {
                'S'
            } else {
                's'
            }
        }
        'Ţ' | 'ţ' | 'Ť' | 'ť' => {
            if character.is_uppercase() {
                'T'
            } else {
                't'
            }
        }
        'Ù'..='Ü' | 'ù'..='ü' | 'Ũ'..='ų' => {
            if character.is_uppercase() {
                'U'
            } else {
                'u'
            }
        }
        'Ŵ' | 'ŵ' => {
            if character.is_uppercase() {
                'W'
            } else {
                'w'
            }
        }
        'Ý' | 'ý' | 'ÿ' | 'Ŷ' | 'ŷ' | 'Ÿ' => {
            if character.is_uppercase() {
                'Y'
            } else {
                'y'
            }
        }
        'Ź'..='ž' => {
            if character.is_uppercase() {
                'Z'
            } else {
                'z'
            }
        }
        other => other,
    }
}

#[cfg(test)]
mod tests;
