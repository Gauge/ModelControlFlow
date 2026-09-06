//! Bits per byte: the log-likelihood the model gives a fixed text, per
//! byte so that vocabularies compare (B-499, D52).
//!
//! **Per byte, and the reading says why.** A token is whatever a
//! vocabulary makes of the text, so bits a token compare two files of one
//! model and nothing else; bytes are the text's own, and two models read
//! the same bytes. The text is MCF's, bundled, so that every reading is
//! of the same question — and it is plain prose and a little code, so
//! that no model is measured on a text it is likely to have seen whole.
//!
//! **Bounded, not lost.** The engine ranks so many candidates at each
//! position; a token past them is counted at the last one listed, which
//! makes the figure a floor by that much, and the reading says how many
//! positions were bounded.

use mcf_record::json::Value;

use super::{Found, Reading, Site, as_integer, whole};
use crate::served::Startup;

/// The measurement's name.
pub const NAME: &str = "bits-per-byte";

/// How many ranked candidates are asked for at each position.
pub const RANKED: usize = 128;

/// The text every reading is of.
pub const TEXT: &str = "The lighthouse stood at the end of a long spit of shingle, and the keeper \
walked out to it each evening with a lamp in one hand and a tin of tea in the other. The wind \
came off the water in gusts that flattened the grass and then let it rise again, as though the \
land were breathing. She counted the steps from the last cottage to the door: four hundred and \
twelve, the same as yesterday, the same as every day since she had taken the post. Inside, the \
stair wound upward past the store rooms, the oil, the log book with its columns of weather, and \
at the top the great lens turned on its bath of mercury with a sound like a held breath.\n\n\
fn steps_between(from: u32, to: u32) -> u32 {\n    if to > from { to - from } else { from - to }\n}\n";

/// Millibits over a count, to the nearest millibit.
#[expect(
    clippy::integer_division,
    reason = "millibits over a count; the remainder is under a millibit"
)]
pub(crate) fn per(millibits: i64, count: usize) -> i64 {
    millibits / i64::try_from(count).unwrap_or(1).max(1)
}

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
    let said = |failure: mcf_core::Failure| failure.detail().to_owned();
    let ids: Vec<usize> = match engine.tokenize(TEXT, true, false) {
        Ok(tokens) => tokens.into_iter().map(|token| token.id).collect(),
        Err(failure) => return Found::could_not_tell(&said(failure)),
    };
    if ids.len() < 2 {
        return Found::could_not_tell("the text segmented to under two tokens");
    }
    let mut spent: i64 = 0;
    let mut bounded = 0_usize;
    let mut read = 0_usize;
    let mut rows = Vec::new();
    for at in 1..ids.len() {
        if at % 16 == 0 {
            site.progress(at, ids.len(), "position");
        }
        if site.asker_gone() {
            return Found::could_not_tell(crate::served::CLIENT_LEFT);
        }
        let (Some(prefix), Some(wanted)) = (ids.get(..at), ids.get(at)) else {
            break;
        };
        let ranked = match engine.distribution_at(prefix, RANKED) {
            Ok(ranked) => ranked,
            Err(failure) => return Found::could_not_tell(&said(failure)),
        };
        read = read.saturating_add(1);
        let (millibits, past) = if let Some((_, mb)) = ranked.iter().find(|(id, _)| id == wanted) {
            (-mb, false)
        } else {
            bounded = bounded.saturating_add(1);
            (ranked.last().map_or(0, |(_, mb)| -mb), true)
        };
        spent = spent.saturating_add(millibits);
        let dims = [("position", Value::Integer(as_integer(at)))];
        rows.push(Reading::new(&dims, "millibits", millibits, "millibits"));
        rows.push(Reading::new(&dims, "past_ranked", i64::from(past), "bool"));
    }
    let bytes = TEXT.len();
    let per_byte = per(spent, bytes);
    let per_token = per(spent, read);
    Found {
        lines: vec![
            format!(
                "  {read} position(s) read over {bytes} bytes of MCF's own text, the first token given"
            ),
            format!(
                "  {} bits a byte ({} a token){}",
                super::fidelity::millibits_said(per_byte),
                super::fidelity::millibits_said(per_token),
                if bounded > 0 {
                    format!(
                        "; {bounded} position(s) past the {RANKED} ranked, counted at the bound, so \
                         the figure is a floor by that much"
                    )
                } else {
                    String::new()
                }
            ),
            "  bits a byte compare across models; bits a token compare only across files of \
             one model, since a token is the vocabulary's"
                .to_owned(),
        ],
        fields: vec![
            ("tokens", Value::Integer(as_integer(read))),
            ("bytes", Value::Integer(as_integer(bytes))),
            ("millibits_per_byte", Value::Integer(per_byte)),
            ("millibits_per_token", Value::Integer(per_token)),
            ("bounded", Value::Integer(as_integer(bounded))),
            ("ranked_to", whole(RANKED as u64)),
        ],
        rows,
    }
}
