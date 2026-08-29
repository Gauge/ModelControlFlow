//! Whether this artifact *produces* an embedding, of what width, and whether it
//! produces the same one twice (B-057, D42, §X, A21, A19).
//!
//! **The declaration and the observation are different things** (A21). A file
//! says it is an embedding model by its architecture and its pooling metadata;
//! what MCF can say is that it asked for a vector and got one, of a stated
//! width, and got the same one when it asked again. Those are three mechanical
//! questions a reader can check, and none of them is *how good the embeddings
//! are* — which is a graded task and a laboratory's (§XIII).
//!
//! **Why it earns a place under D42's test.** A wrong answer corrupts a served
//! answer directly: a caller that assumes a width builds an index that cannot
//! be queried, and a caller that assumes an ordinary text model will embed gets
//! a refusal at the point of use rather than at the point of choosing. Both are
//! §3.8's measurement error in the shape a user meets.
//!
//! **Determinism is the third question and the interesting one.** An embedding
//! that differs between two identical calls cannot be compared with anything —
//! not with itself yesterday, not with another model's, not across a corpus.
//! MCF's own engine is deterministic by construction, so a *difference* here
//! would be a defect in MCF rather than a property of the model, and the probe
//! says which it would be rather than reporting a number.

use std::path::Path;

use mcf_core::probe::{Method, Outcome, Probed};

/// What stands where a daemon's engine name would be.
///
/// **`mcf embed` loads in-process and never consults a daemon** — that was
/// established rather than assumed when F103 audited which surfaces inherit an
/// engine. So naming the daemon's engine here would record a condition that did
/// not participate, which is F102's defect in a new place: the first run of this
/// probe did exactly that, and the conditions said *provisioned llama.cpp,
/// through the daemon* about arithmetic MCF performed itself (A21, §3.4).
///
/// The build is named because it is the part that can change: MCF's own
/// embedding path is this binary's, and F104 established that a version string
/// does not identify one.
#[must_use]
pub fn in_process() -> String {
    format!(
        "MCF's own engine, in this process — `mcf embed` consults no daemon (F103); build {}",
        mcf_core::build_identity::identifier()
    )
}

/// The text every trial embeds.
///
/// One short text, twice. Short because the question is about the shape of the
/// answer rather than about how much the model can read, and the *same* text
/// because that is the whole of the determinism question.
const TEXT: &str = "A short sentence to embed.";

/// What the embedding probe observed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Embeds {
    /// What the file's own metadata says it is.
    pub declared: Declared,
    /// The width of the vector that came back.
    pub width: usize,
    /// Whether asking twice produced the same vector, to the last bit.
    ///
    /// Not *near enough*: MCF's own engine performs the same arithmetic in the
    /// same order, so anything but equality is a defect rather than noise, and
    /// a tolerance here would hide it (A19).
    pub identical_twice: bool,
    /// How many identifiers the text spent, which is the condition the width
    /// was measured under.
    pub tokens: usize,
}

/// What the artifact claims, read and never believed (A21).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Declared {
    /// The architecture its metadata names.
    pub architecture: String,
    /// The embedding width its metadata declares, where it declares one.
    pub width: Option<usize>,
}

/// The method.
pub const EMBEDDING: Method = Method {
    name: "embedding",
    asks: "for the vector of one short text, twice, and reports the width that came back and \
           whether the two were identical to the last bit",
    decides: "whether MCF may ask this artifact for embeddings and what width to expect — and \
              nothing about how good they are, which needs a graded task (D42, §XIII)",
};

/// How a caller produces one embedding: a text in, a width and a digest of the
/// vector out.
///
/// A digest rather than the vector, because the probe's question is *the same
/// one twice* and comparing digests answers it without the probe holding a
/// thousand floats it has no other use for — and because a vector on its way
/// through a probe is a vector something might record (A25's shape).
pub type Ask<'a> = &'a mut dyn FnMut(&str) -> Option<(usize, String, usize)>;

/// Runs the embedding probe.
///
/// # Errors
///
/// Never: a probe that cannot decide reports `Inconclusive` with its reason
/// (D42's third state).
#[must_use]
pub fn embedding(model: &Path, bytes: &[u8], engine: &str, ask: Ask<'_>) -> Probed<Embeds> {
    let conditions = super::conditions(&EMBEDDING, model, engine);
    let Ok(file) = mcf_standin::gguf::parse(bytes) else {
        return Probed::inconclusive(
            EMBEDDING,
            "the file could not be read as a model",
            0,
            0,
            conditions,
        );
    };
    let architecture = file
        .get("general.architecture")
        .and_then(mcf_standin::gguf::Value::as_text)
        .unwrap_or("unstated")
        .to_owned();
    let width = file
        .get(&format!("{architecture}.embedding_length"))
        .and_then(mcf_standin::gguf::Value::as_integer)
        .and_then(|held| usize::try_from(held).ok());
    let declared = Declared {
        architecture,
        width,
    };

    let Some((first_width, first, tokens)) = ask(TEXT) else {
        return Probed::inconclusive(
            EMBEDDING,
            "this artifact did not produce an embedding, which is what an artifact that is not \
             an embedding model does — MCF asked and reports that it did not, rather than \
             reporting that it cannot (A7)",
            1,
            0,
            conditions,
        );
    };
    let Some((second_width, second, _)) = ask(TEXT) else {
        return Probed::inconclusive(
            EMBEDDING,
            "the first call produced an embedding and the second did not, so whether this \
             artifact embeds is not a question with one answer here",
            2,
            0,
            conditions,
        );
    };

    Probed {
        method: EMBEDDING,
        outcome: Outcome::Observed(Embeds {
            declared,
            width: first_width,
            identical_twice: first == second && first_width == second_width,
            tokens,
        }),
        trials: 2,
        // Nothing was generated: an embedding is a forward pass and no
        // sampling. Nought is the figure, and it is not a gap in the record.
        tokens: 0,
        conditions,
    }
}

#[cfg(test)]
mod tests;
