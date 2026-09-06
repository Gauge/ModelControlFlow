//! Tokenizer round trip: whether text survives being read and spelled
//! back, and whether two tokenizers count it the same (B-502, D52).
//!
//! A template that eats a leading space or a run of tabs corrupts every
//! answer it frames, and it does so invisibly: the text still reads.
//! A bundled corpus — prose, code, whitespace runs, tabs, scripts that
//! are not Latin, symbols — is encoded and decoded by the engine, and
//! what came back is compared byte for byte with what went in. Beside
//! it, MCF's own tokenizer counts each text, since a prompt counted by
//! one tokenizer and served by another is charged a figure nobody
//! measured (B-441).

use mcf_record::json::Value;

use super::{Found, Site, as_integer};
use crate::served::Startup;

/// The measurement's name.
pub const NAME: &str = "tokenizer-round-trip";

/// The texts, each named for what it tries.
pub const CORPUS: [(&str, &str); 13] = [
    ("plain prose", "The keeper counted the steps to the door."),
    ("two spaces", "a  b"),
    ("tabs", "x\ty\t\tz"),
    ("leading spaces", "   indented line"),
    ("blank lines", "one\n\n\nfour"),
    ("rust", "fn main() { println!(\"{}\", 1 << 3); }"),
    ("japanese", "東京都渋谷区の灯台"),
    ("arabic", "مرحبا بالعالم"),
    ("emoji", "🙂 🏳️‍🌈 👩‍💻"),
    ("accents", "naïve café, Straße, ångström"),
    ("symbols", "∑ ≤ ∞ → ¬ ≈ π"),
    ("trailing space", "the end "),
    ("no-break space", "a\u{a0}b"),
];

/// Runs it.
#[must_use]
pub fn measure(site: &Site<'_>) -> Found {
    let engine = match site.server(&Startup {
        projector: None,
        ..site.startup()
    }) {
        Ok(engine) => engine,
        Err(why) => return Found::could_not_tell(&why),
    };
    let own = crate::probes::run::read_prefix(site.model)
        .and_then(|bytes| mcf_standin::gguf::parse(&bytes).ok())
        .and_then(|file| mcf_standin::tokenizer::Vocabulary::read(&file).ok());
    let mut lines = vec![format!(
        "  {} text(s) read by the engine and spelled back; MCF's own tokenizer counts each beside it{}",
        CORPUS.len(),
        if own.is_none() {
            " — MCF's own refuses this vocabulary, so only the engine's count is here"
        } else {
            ""
        }
    )];
    let mut rows = Vec::new();
    let mut round_tripped = 0_usize;
    let mut counts_agree = 0_usize;
    for (name, text) in CORPUS {
        if site.asker_gone() {
            return Found::could_not_tell(crate::served::CLIENT_LEFT);
        }
        let read = match engine.tokenize(text, false, false) {
            Ok(tokens) => tokens,
            Err(failure) => return Found::could_not_tell(failure.detail()),
        };
        let ids: Vec<usize> = read.iter().map(|token| token.id).collect();
        let back = match engine.detokenize(&ids) {
            Ok(back) => back,
            Err(failure) => return Found::could_not_tell(failure.detail()),
        };
        let same = back == text;
        if same {
            round_tripped = round_tripped.saturating_add(1);
        }
        let own_count = own
            .as_ref()
            .and_then(|vocabulary| vocabulary.encode(text, false).ok())
            .map(|ids| ids.len());
        let agree = own_count.is_some_and(|count| count == ids.len());
        if agree {
            counts_agree = counts_agree.saturating_add(1);
        }
        lines.push(format!(
            "  {name:<16} {:>3} token(s){}   {}",
            ids.len(),
            own_count.map_or_else(
                || "         ".to_owned(),
                |count| if count == ids.len() {
                    ", the same by MCF".to_owned()
                } else {
                    format!(", {count} by MCF")
                }
            ),
            if same {
                "round-tripped".to_owned()
            } else {
                format!("CHANGED: {}", changed(text, &back))
            }
        ));
        rows.push(Value::map([
            ("name", Value::text(name.to_owned())),
            ("engine_tokens", Value::Integer(as_integer(ids.len()))),
            (
                "own_tokens",
                own_count.map_or(Value::Null, |count| Value::Integer(as_integer(count))),
            ),
            ("round_tripped", Value::Bool(same)),
            (
                "came_back",
                if same { Value::Null } else { Value::text(back) },
            ),
        ]));
    }
    lines.push(format!(
        "  {round_tripped} of {} round-tripped; {counts_agree} counted the same by both tokenizers",
        CORPUS.len()
    ));
    Found {
        lines,
        fields: vec![
            ("texts", Value::Integer(as_integer(CORPUS.len()))),
            ("round_tripped", Value::Integer(as_integer(round_tripped))),
            ("counts_agree", Value::Integer(as_integer(counts_agree))),
            ("own_tokenizer_read", Value::Bool(own.is_some())),
            ("texts_read", Value::List(rows)),
        ],
    }
}

/// What changed between a text and what came back, in a phrase.
pub(crate) fn changed(text: &str, back: &str) -> String {
    if back.trim() == text.trim() {
        if back.len() < text.len() {
            "whitespace at an end was lost".to_owned()
        } else {
            "whitespace at an end was added".to_owned()
        }
    } else if back.split_whitespace().eq(text.split_whitespace()) {
        "whitespace inside was changed".to_owned()
    } else {
        format!("came back as {back:?}")
    }
}
