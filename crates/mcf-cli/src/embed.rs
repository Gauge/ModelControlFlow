//! `mcf embed`: a model turns text into a vector (B-371, DEC-055, D31, B65).
//!
//! **The other verb, for the other kind of model.** `mcf run` asks what a model
//! says next, and an embedding model has no answer — no output head, nothing to
//! sample. What it does is read a whole text at once and produce one vector,
//! and this surface asks for exactly that.
//!
//! **The vector is the first line, whole, as JSON.** §3.3 ranks
//! machine-readability first: the thing a caller does with an embedding is
//! compute with it, and 384 numbers are not for reading. Legibility is second
//! and not omitted — the conditions block underneath says what the vector is,
//! how it was pooled, that it is unit length, and what produced it. A script
//! takes line one; a person reads the rest.
//!
//! **The conditions travel with the answer** (§3.4): the model, the token
//! count, the pooling the file declared, the normalization applied, and the
//! engine — with the same mark every stand-in answer carries, because an
//! embedding from an engine written to be read is as much a behaviour answer
//! as a sentence is, and can no more become a speed (B65).

use std::fmt::Write as _;
use std::path::Path;

use mcf_core::build_identity::BuildIdentity;
use mcf_core::failure::Failure;
use mcf_standin::bert::{self, Pooling};
use mcf_standin::gguf;
use mcf_standin::tokenizer::Vocabulary;

use crate::Response;
use crate::run::{ambiguous, resolve};

/// Embeds a text and prints the vector.
pub(crate) fn run(model: &str, text: &str) -> Response {
    let path = match resolve(model) {
        Ok(Some(path)) => path,
        Ok(None) => {
            return Response {
                text: format!(
                    "mcf: there is no model at {model}\n  `mcf list` says what this machine is \
                     holding; a path to a file works too"
                ),
                served: false,
            };
        }
        Err(found) => {
            return Response {
                text: ambiguous(model, &found),
                served: false,
            };
        }
    };

    let bytes = match std::fs::read(&path) {
        Ok(bytes) => bytes,
        Err(error) => {
            return Response {
                text: format!("mcf: {} could not be read\n  {error}", path.display()),
                served: false,
            };
        }
    };

    match answer(&bytes, text) {
        Ok((embedding, mark)) => Response {
            text: render(&path, &embedding, &mark),
            served: true,
        },
        Err(failure) => Response {
            text: refused(&path, &failure),
            served: false,
        },
    }
}

/// Reads the model, embeds, and keeps the mark.
fn answer(bytes: &[u8], text: &str) -> Result<(bert::Embedding, String), Failure> {
    let file = gguf::parse(bytes)?;
    let vocabulary = Vocabulary::read(&file)?;
    let model = bert::load(&file, bytes)?;
    let tokens = vocabulary.encode(text, true)?;
    let build = BuildIdentity::current().version.to_owned();
    let marked = bert::embed(&model, &build, &tokens)?;
    let mark = marked.degradation().to_string();
    let embedding = marked.value().observed().clone();
    Ok((embedding, mark))
}

/// The answer first and machine-readable, the conditions after and legible.
fn render(path: &Path, embedding: &bert::Embedding, mark: &str) -> String {
    let mut vector = String::from("[");
    for (index, value) in embedding.vector.iter().enumerate() {
        if index > 0 {
            vector.push(',');
        }
        // Shortest round-trip formatting: what is printed reads back as
        // exactly the `f32` that was computed (A1 for numbers). The write
        // cannot fail: the target is a `String`.
        let _written = write!(vector, "{value}");
    }
    vector.push(']');

    format!(
        "{{\"width\":{},\"embedding\":{vector}}}\n\n\
         ── what produced it ─────────────────────────────────────────\n\
         \x20 model    {}\n\
         \x20 text     {} token(s), brackets included\n\
         \x20 pooled   {}\n\
         \x20 length   normalized to 1 (euclidean), stated because a vector\n\
         \x20          only compares at a stated length\n\
         \x20 engine   MCF's own stand-in\n\
         \x20 MARKED   {mark}\n\
         \x20 This is a behaviour answer and can never be a speed (B65, D31).",
        embedding.vector.len(),
        path.display(),
        embedding.tokens,
        match embedding.pooling {
            Pooling::Mean => "the mean over every position, as the file declares",
            Pooling::First => "the first position alone, as the file declares",
        },
    )
}

/// A refusal, said the shared way.
fn refused(path: &Path, failure: &Failure) -> String {
    format!(
        "{}\n  `mcf embed` is for embedding models — the bert family — and `mcf run` is for \
         the ones that produce text; each refuses the other's kind by name (DEC-055)",
        crate::say::refusal(&format!("{} did not embed", path.display()), failure)
    )
}
