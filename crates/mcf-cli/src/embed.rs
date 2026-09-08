use std::fmt::Write as _;
use std::path::Path;

use mcf_core::build_identity::BuildIdentity;
use mcf_core::failure::Failure;
use mcf_standin::bert::{self, Pooling};
use mcf_standin::gguf;
use mcf_standin::tokenizer::Vocabulary;

use crate::Response;
use crate::run::{ambiguous, resolve};

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

    if let Err(failure) = examine_for_embedding(&path) {
        return Response {
            text: refused(&path, &failure),
            served: false,
        };
    }

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

fn examine_for_embedding(path: &std::path::Path) -> Result<(), Failure> {
    use std::io::Read as _;
    let held = std::fs::metadata(path).map_or(0, |meta| meta.len());
    for cap in [16_u64 << 20, 256 << 20, u64::MAX] {
        let take = cap.min(held);
        let mut prefix = Vec::new();
        if std::fs::File::open(path)
            .and_then(|handle| handle.take(take).read_to_end(&mut prefix))
            .is_err()
        {
            return Ok(());
        }
        match gguf::parse(&prefix) {
            Ok(file) => return crate::run::fits_in_memory(&file),
            Err(failure) => {
                if take >= held {
                    return Err(failure);
                }
            }
        }
    }
    Ok(())
}

fn answer(bytes: &[u8], text: &str) -> Result<(bert::Embedding, String), Failure> {
    let file = gguf::parse(bytes)?;
    let vocabulary = Vocabulary::read(&file)?;
    let model =
        bert::load(&file, bytes)?.across(mcf_standin::threads::Threads::what_the_machine_reports());
    let tokens = vocabulary.encode(text, true)?;
    let build = BuildIdentity::current().version.to_owned();
    let marked = bert::embed(&model, &build, &tokens)?;
    let mark = marked.degradation().to_string();
    let embedding = marked.value().observed().clone();
    Ok((embedding, mark))
}

fn render(path: &Path, embedding: &bert::Embedding, mark: &str) -> String {
    let mut vector = String::from("[");
    for (index, value) in embedding.vector.iter().enumerate() {
        if index > 0 {
            vector.push(',');
        }
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

fn refused(path: &Path, failure: &Failure) -> String {
    format!(
        "{}\n  `mcf embed` is for embedding models — the bert family — and `mcf run` is for \
         the ones that produce text; each refuses the other's kind by name (DEC-055)",
        crate::say::refusal(&format!("{} did not embed", path.display()), failure)
    )
}
