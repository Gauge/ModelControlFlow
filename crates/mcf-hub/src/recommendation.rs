//! What a repository says about how to sample the model it publishes (B-281,
//! B60, D18, A21).
//!
//! **The second of the two places a recommendation can be**, and the one that
//! needs a network. `mcf_standin::recommended` reads the model file's own
//! metadata, which travels with the weights; this reads the repository's
//! `generation_config.json`, which does not.
//!
//! **The finding that shapes this module** ([findings.md](../../../doc/findings.md)
//! F63): the recommendation is usually **in a different repository from the
//! weights**. A GGUF conversion repository publishes the quantized files and,
//! at most, the architecture's `config.json`; the sampling recommendation lives
//! in the base repository the conversion came from, which MCF was never asked
//! to fetch and cannot identify from the conversion alone. Six repositories
//! were examined and none published one.
//!
//! So the ordinary answer here is [`Recommendation::NoneDeclared`], and that is
//! a state to report rather than a hole to fill. B60 has MCF adopt what the
//! artifact recommends *where it recommends anything*, and name its own choice
//! as its own where it does not.
//!
//! **Read, never believed** (A21), and every byte untrusted (§3.7): the file is
//! a repository's, so a value that is not a sampler parameter is refused rather
//! than repaired.

use mcf_core::attested::Attested;
use mcf_core::configuration::{Sampling, Thousandths};
use mcf_core::failure::{Category, Result};

use crate::client::Hub;
use crate::source::Listing;
use mcf_record::json::Value;

/// The file a repository states its sampling recommendation in.
///
/// Named here so that the one place MCF looks is one line to find, and so
/// that a report can say *MCF looked here* rather than *MCF found nothing*.
pub const WHERE: &str = "generation_config.json";

/// What a repository recommends, and whether it recommended anything.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Recommendation {
    /// The repository states at least one sampling parameter.
    Declared {
        /// What it states. Parameters it does not state stay `Unknown`.
        sampling: Sampling,
        /// Which fields were read, so the claim is checkable against the file.
        fields: Vec<String>,
    },
    /// The repository publishes the file and states nothing MCF reads as a
    /// sampler parameter.
    ///
    /// Distinct from the file being absent, because the two are different
    /// facts about the publisher: one looked and said nothing, the other did
    /// not look.
    NothingStated,
    /// The repository publishes no such file.
    ///
    /// **The ordinary case for a GGUF repository** (F63).
    NoneDeclared,
}

impl Recommendation {
    /// What the repository recommends, where it recommends anything.
    #[must_use]
    pub const fn sampling(&self) -> Option<&Sampling> {
        match self {
            Self::Declared { sampling, .. } => Some(sampling),
            Self::NothingStated | Self::NoneDeclared => None,
        }
    }

    /// One sentence a surface can print, which always says where MCF looked.
    #[must_use]
    pub fn describe(&self) -> String {
        match self {
            Self::Declared { sampling, fields } => format!(
                "{sampling} — declared in {WHERE} ({}), unverified here (A21)",
                fields.join(", ")
            ),
            Self::NothingStated => format!(
                "{WHERE} is published and states no sampler parameter MCF reads — the publisher \
                 looked and said nothing"
            ),
            Self::NoneDeclared => format!(
                "no {WHERE} in this repository, so it recommends nothing MCF can adopt; a \
                 conversion repository usually publishes none, and the recommendation is in the \
                 base repository it came from (F63)"
            ),
        }
    }
}

/// Reads a repository's sampling recommendation.
///
/// # Errors
///
/// Whatever asking the hub fails with, except a plain absence — a repository
/// that publishes no such file is [`Recommendation::NoneDeclared`] rather than
/// a failure, for the reason [`Hub::configuration`] gives about `config.json`.
pub fn read(hub: &Hub, listing: &Listing) -> Result<Recommendation> {
    let Some(body) = hub.metadata_file(listing, WHERE)? else {
        return Ok(Recommendation::NoneDeclared);
    };
    Ok(from_json(&body))
}

/// The same, from bytes already in hand.
///
/// Separated so that the reading is testable without a hub, which is what B19
/// asks of anything the gating tier examines.
#[must_use]
pub fn from_json(body: &Value) -> Recommendation {
    let mut fields = Vec::new();
    let mut fraction = |name: &str| -> Attested<Thousandths> {
        match body.get(name).and_then(thousandths) {
            Some(held) => {
                fields.push(name.to_owned());
                Attested::Known(held)
            }
            None => Attested::Unknown,
        }
    };
    let temperature = fraction("temperature");
    let top_p = fraction("top_p");
    let repetition_penalty = fraction("repetition_penalty");

    let mut whole = |name: &str| -> Attested<u32> {
        match body.get(name).and_then(Value::as_integer) {
            Some(held) => match u32::try_from(held) {
                Ok(held) => {
                    fields.push(name.to_owned());
                    Attested::Known(held)
                }
                Err(_) => Attested::Unknown,
            },
            None => Attested::Unknown,
        }
    };
    let top_k = whole("top_k");
    let max_output_tokens = whole("max_new_tokens");

    if fields.is_empty() {
        return Recommendation::NothingStated;
    }
    Recommendation::Declared {
        sampling: Sampling {
            temperature,
            top_p,
            top_k,
            repetition_penalty,
            max_output_tokens,
        },
        fields,
    }
}

/// A fraction, in thousandths, from an untrusted document.
///
/// The record's `Value` has no float, so a repository writing `0.7` arrives
/// here as text or as an integer. Both are read; anything outside what a
/// sampler parameter can be is *not read* rather than clamped, because a
/// clamped value is a number MCF chose (A7, §3.7).
fn thousandths(value: &Value) -> Option<Thousandths> {
    match value {
        Value::Integer(held) => u32::try_from(*held)
            .ok()?
            .checked_mul(1_000)
            .map(Thousandths),
        Value::Text(held) => decimal(held),
        _ => None,
    }
}

/// A decimal written as text, in thousandths, without a float.
///
/// `0.7` is 700, `1` is 1000, `0.9500` is 950. More than three decimal places
/// is refused rather than rounded: the unit is thousandths, and silently
/// dropping a digit is deciding a value the publisher stated (A7).
fn decimal(written: &str) -> Option<Thousandths> {
    let written = written.trim();
    let (whole, rest) = match written.split_once('.') {
        Some((whole, rest)) => (whole, rest),
        None => (written, ""),
    };
    let whole: u32 = whole.parse().ok()?;
    if rest.len() > 3
        && rest
            .get(3..)
            .is_some_and(|tail| !tail.chars().all(|d| d == '0'))
    {
        return None;
    }
    let mut thousandths = 0_u32;
    for at in 0..3 {
        let digit = rest
            .chars()
            .nth(at)
            .map_or(Some(0), |held| held.to_digit(10))?;
        thousandths = thousandths.checked_mul(10)?.checked_add(digit)?;
    }
    whole
        .checked_mul(1_000)?
        .checked_add(thousandths)
        .map(Thousandths)
}

/// So that a hub failure that is a plain absence is not one.
#[allow(dead_code, reason = "read by the doc comment on `read`")]
const ABSENCE: Category = Category::HubRefNotFound;

#[cfg(test)]
mod tests;
