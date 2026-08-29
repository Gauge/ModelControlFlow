//! What a model file says about how to sample it (B-281, B60, D18, A21).
//!
//! **B60 has MCF adopt the artifact's own recommendation rather than imposing a
//! house style.** That requires a recommendation to exist, and the first thing
//! to establish about it is where MCF would find one. There are exactly two
//! places, and this module is the first: the GGUF's own metadata, which travels
//! with the weights and is therefore the only source an offline machine has
//! ([`mcf_hub`]'s generation configuration is the second, and needs the
//! repository).
//!
//! **The format defines keys for it and files rarely carry them.** GGUF's
//! general specification admits `<arch>.temperature`, `<arch>.top_p` and their
//! neighbours; six of six model files examined for
//! [findings.md](../../../doc/findings.md) F63 carry none. So the honest
//! outcome of this module is usually *nothing declared*, which is a state to
//! report rather than a hole to fill (A7).
//!
//! **Read, never believed** (A21). A value here is what the file says, and
//! saying so is all this does: whether it is a good setting for this machine
//! is a measurement nobody has taken, and [`Chosen::DeclaredByArtifact`] is the
//! mark that says as much.
//!
//! [`Chosen::DeclaredByArtifact`]: mcf_core::configuration::Chosen::DeclaredByArtifact
//! [`mcf_hub`]: https://docs.rs/mcf-hub

use mcf_core::attested::Attested;
use mcf_core::configuration::{Sampling, Thousandths};

use crate::gguf::{Model, Value};

/// What a model file recommends, and whether it recommended anything.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Recommendation {
    /// The file states at least one sampling parameter.
    Declared {
        /// What it states. Parameters it does not state stay `Unknown`: a file
        /// that names a temperature and no `top_p` has recommended a
        /// temperature, and inventing the rest would be A7's forbidden
        /// substitution.
        sampling: Sampling,
        /// Which keys were read, so the claim is checkable against the file.
        keys: Vec<String>,
    },
    /// The file states none.
    ///
    /// **The ordinary case**, and the reason this is a variant rather than an
    /// empty `Sampling`: *no recommendation* and *a recommendation that sets
    /// nothing* are different facts, and only the first justifies MCF choosing
    /// for itself (B60).
    NoneDeclared,
}

impl Recommendation {
    /// What the file recommends, where it recommends anything.
    #[must_use]
    pub const fn sampling(&self) -> Option<&Sampling> {
        match self {
            Self::Declared { sampling, .. } => Some(sampling),
            Self::NoneDeclared => None,
        }
    }
}

/// Reads a model file's sampling recommendation.
///
/// The keys are looked for under the architecture the file declares, which is
/// how GGUF namespaces everything else it says about a model. A file that
/// declares no architecture has no namespace to look in and therefore
/// recommends nothing — which is the same answer as a file that declares one
/// and says nothing in it, and is right for the same reason.
#[must_use]
pub fn read(file: &Model) -> Recommendation {
    let Some(architecture) = file.get("general.architecture").and_then(Value::as_text) else {
        return Recommendation::NoneDeclared;
    };

    let mut keys = Vec::new();
    let mut fraction = |name: &str| -> Attested<Thousandths> {
        let key = format!("{architecture}.{name}");
        match file.get(&key).and_then(thousandths) {
            Some(held) => {
                keys.push(key);
                Attested::Known(held)
            }
            None => Attested::Unknown,
        }
    };
    let temperature = fraction("temperature");
    let top_p = fraction("top_p");
    let repetition_penalty = fraction("repetition_penalty");

    let mut whole = |name: &str| -> Attested<u32> {
        let key = format!("{architecture}.{name}");
        match file.get(&key).and_then(count) {
            Some(held) => {
                keys.push(key);
                Attested::Known(held)
            }
            None => Attested::Unknown,
        }
    };
    let top_k = whole("top_k");
    let max_output_tokens = whole("max_output_tokens");

    if keys.is_empty() {
        return Recommendation::NoneDeclared;
    }
    Recommendation::Declared {
        sampling: Sampling {
            temperature,
            top_p,
            top_k,
            repetition_penalty,
            max_output_tokens,
        },
        keys,
    }
}

/// A fraction, in thousandths.
///
/// The file may write it as a float or as an integer, and both are read. A
/// value outside what a sampler parameter can be — negative, or beyond a
/// thousand times what any of these ever is — is *not read* rather than
/// clamped: a clamped value is a number MCF chose, and a file stating one is
/// a file MCF should not be adopting from (A7, §3.7).
fn thousandths(value: &Value) -> Option<Thousandths> {
    let held = match value {
        Value::Float(held) => {
            if !held.is_finite() || *held < 0.0 || *held > 1_000.0 {
                return None;
            }
            // Thousandths of the stated value, rounded to nearest, in the one
            // place a float is unavoidable: the file's own number arrives as
            // one, and this is the boundary where it stops being one. The
            // range was checked above, so the conversion is bounded by the
            // check rather than by the cast.
            #[expect(
                clippy::cast_possible_truncation,
                reason = "the value is finite and within [0, 1000], asserted immediately above"
            )]
            let scaled = (held * 1_000.0).round() as i64;
            u32::try_from(scaled).ok()?
        }
        Value::Integer(held) => u32::try_from(*held).ok()?.checked_mul(1_000)?,
        _ => return None,
    };
    Some(Thousandths(held))
}

/// A whole number, where the file states one.
fn count(value: &Value) -> Option<u32> {
    match value {
        Value::Integer(held) => u32::try_from(*held).ok(),
        _ => None,
    }
}

#[cfg(test)]
mod tests;
