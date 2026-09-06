//! Vocabulary coverage: how much of a mixed corpus the tokenizer spells
//! with byte fallbacks or the unknown token, text by text (B-541, D55,
//! B-502).
//!
//! A vocabulary that has no piece for a script spells it one byte at a
//! time, three or four tokens a character, and a model reads and writes
//! that script at a third of its speed and with less of its window; a
//! vocabulary with no byte fallback loses the character to its unknown
//! token. Each text of the corpus is tokenized and its tokens counted
//! three ways — all, byte fallbacks, unknowns — beside its length in
//! bytes and in characters, so a person can see what a script costs on
//! this model before they send it one.

use mcf_record::json::Value;

use super::{Found, Reading, Site, as_integer, ppm};
use crate::served::Startup;

/// The measurement's name.
pub const NAME: &str = "vocabulary";

/// The corpus: the round-trip texts and longer passages in several
/// scripts, code and symbols.
pub const CORPUS: [(&str, &str); 12] = [
    (
        "english",
        "The keeper counted the steps to the door and found one more than yesterday, which he wrote in the book beside the date.",
    ),
    (
        "french",
        "Le gardien compta les marches jusqu'à la porte et en trouva une de plus qu'hier, ce qu'il nota dans le registre à côté de la date.",
    ),
    (
        "german",
        "Der Wärter zählte die Stufen bis zur Tür und fand eine mehr als gestern, was er neben dem Datum ins Buch schrieb.",
    ),
    (
        "russian",
        "Смотритель пересчитал ступени до двери и нашёл на одну больше, чем вчера, что записал в журнал рядом с датой.",
    ),
    (
        "greek",
        "Ο φύλακας μέτρησε τα σκαλιά ως την πόρτα και βρήκε ένα παραπάνω από χθες, το οποίο σημείωσε στο βιβλίο δίπλα στην ημερομηνία.",
    ),
    (
        "japanese",
        "番人は扉までの段を数え、昨日より一段多いことに気づき、日付の横の帳簿にそれを書き留めた。",
    ),
    (
        "chinese",
        "看守数了通往门口的台阶，发现比昨天多了一级，便把它记在日期旁边的簿子里。",
    ),
    (
        "korean",
        "관리인은 문까지의 계단을 세어 어제보다 하나 더 많다는 것을 알고 날짜 옆 장부에 적었다.",
    ),
    (
        "arabic",
        "عدّ الحارس الدرجات حتى الباب فوجد درجة زائدة عن الأمس، فدوّن ذلك في السجل بجانب التاريخ.",
    ),
    (
        "hindi",
        "रखवाले ने दरवाज़े तक की सीढ़ियाँ गिनीं और कल से एक अधिक पाई, जिसे उसने तारीख़ के पास बही में लिख लिया।",
    ),
    (
        "rust",
        "fn count(steps: &[u32]) -> u32 { steps.iter().filter(|s| **s > 0).count() as u32 }\n",
    ),
    ("symbols", "∑ ≤ ∞ → ¬ ≈ π ∀x∈ℝ: x² ≥ 0 — 🙂 🏳️‍🌈 👩‍💻 ✓ ✗"),
];

/// Runs it.
#[must_use]
#[allow(
    clippy::too_many_lines,
    reason = "one measurement read straight through: each text tokenized, its counts a row each"
)]
pub fn measure(site: &Site<'_>) -> Found {
    let engine = match site.server(&Startup {
        projector: None,
        ..site.startup()
    }) {
        Ok(engine) => engine,
        Err(why) => return Found::could_not_tell(&why),
    };
    let unknown_id = std::fs::read(site.model)
        .ok()
        .and_then(|bytes| mcf_standin::gguf::parse(&bytes).ok())
        .and_then(|file| {
            file.get("tokenizer.ggml.unknown_token_id")
                .and_then(mcf_standin::gguf::Value::as_integer)
                .and_then(|id| usize::try_from(id).ok())
        });
    let mut rows = Vec::new();
    let mut lines = vec![format!(
        "  {} text(s) tokenized; a byte fallback is a piece the engine writes as <0x..>, an \
         unknown is the token the file names as such{}",
        CORPUS.len(),
        unknown_id.map_or(" (this file names none)".to_owned(), |id| format!(
            " ({id})"
        ))
    )];
    let (mut all_tokens, mut all_bytes, mut all_fallbacks, mut all_unknown) =
        (0_usize, 0_usize, 0_usize, 0_usize);
    for (at, (name, text)) in CORPUS.into_iter().enumerate() {
        site.progress(at, CORPUS.len(), name);
        let tokens = match engine.tokenize(text, false, false) {
            Ok(tokens) => tokens,
            Err(failure) => return Found::could_not_tell(failure.detail()),
        };
        let fallbacks = tokens
            .iter()
            .filter(|token| is_byte_piece(&token.piece))
            .count();
        let unknown = tokens
            .iter()
            .filter(|token| unknown_id == Some(token.id))
            .count();
        let characters = text.chars().count();
        all_tokens = all_tokens.saturating_add(tokens.len());
        all_bytes = all_bytes.saturating_add(text.len());
        all_fallbacks = all_fallbacks.saturating_add(fallbacks);
        all_unknown = all_unknown.saturating_add(unknown);
        let dims = [("text", Value::text(name))];
        rows.push(Reading::new(
            &dims,
            "bytes",
            as_integer(text.len()),
            "bytes",
        ));
        rows.push(Reading::new(
            &dims,
            "characters",
            as_integer(characters),
            "count",
        ));
        rows.push(Reading::new(
            &dims,
            "tokens",
            as_integer(tokens.len()),
            "tokens",
        ));
        rows.push(Reading::new(
            &dims,
            "byte_fallbacks",
            as_integer(fallbacks),
            "tokens",
        ));
        rows.push(Reading::new(
            &dims,
            "unknown",
            as_integer(unknown),
            "tokens",
        ));
        lines.push(format!(
            "  {name:<10} {:>4} character(s) in {:>4} token(s); {fallbacks} byte fallback(s), \
             {unknown} unknown; {} tokens a thousand characters",
            characters,
            tokens.len(),
            tokens
                .len()
                .saturating_mul(1000)
                .checked_div(characters.max(1))
                .unwrap_or(0)
        ));
    }
    lines.push(format!(
        "  {all_tokens} token(s) over {all_bytes} byte(s); {all_fallbacks} byte fallback(s), \
         {all_unknown} unknown — {} fallbacks a million tokens",
        ppm(
            u64::try_from(all_fallbacks).unwrap_or(u64::MAX),
            u64::try_from(all_tokens).unwrap_or(u64::MAX)
        )
    ));
    Found {
        lines,
        fields: vec![
            ("texts", Value::Integer(as_integer(CORPUS.len()))),
            ("tokens", Value::Integer(as_integer(all_tokens))),
            ("bytes", Value::Integer(as_integer(all_bytes))),
            ("byte_fallbacks", Value::Integer(as_integer(all_fallbacks))),
            ("unknown", Value::Integer(as_integer(all_unknown))),
            (
                "unknown_id",
                unknown_id.map_or(Value::Null, |id| Value::Integer(as_integer(id))),
            ),
        ],
        rows,
    }
}

/// Whether a piece is a byte the vocabulary could not spell, as the
/// engine writes one: `<0xE2>`.
#[must_use]
pub fn is_byte_piece(piece: &str) -> bool {
    piece.len() == 6
        && piece.starts_with("<0x")
        && piece.ends_with('>')
        && piece
            .get(3..5)
            .is_some_and(|hex| hex.chars().all(|c| c.is_ascii_hexdigit()))
}

#[cfg(test)]
mod tests {
    use super::{CORPUS, is_byte_piece};

    #[test]
    fn a_byte_piece_is_told_from_a_word() {
        assert!(is_byte_piece("<0xE2>"));
        assert!(is_byte_piece("<0x0a>"));
        assert!(!is_byte_piece("<0xE2"));
        assert!(!is_byte_piece("hello"));
        assert!(!is_byte_piece("<0xZZ>"));
    }

    #[test]
    fn the_corpus_is_named_and_non_empty() {
        for (name, text) in CORPUS {
            assert!(!name.is_empty() && !text.is_empty());
        }
    }
}
