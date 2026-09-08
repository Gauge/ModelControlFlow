use std::path::Path;

use mcf_core::probe::{Method, Outcome, Probed};

#[must_use]
pub fn in_process() -> String {
    format!(
        "MCF's own engine, in this process — `mcf embed` consults no daemon; build {}",
        mcf_core::build_identity::identifier()
    )
}

const TEXT: &str = "A short sentence to embed.";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Embeds {
    pub declared: Declared,
    pub width: usize,
    pub identical_twice: bool,
    pub tokens: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Declared {
    pub architecture: String,
    pub width: Option<usize>,
}

pub const EMBEDDING: Method = Method {
    name: "embedding",
    asks: "for the vector of one short text, twice, and reports the width that came back and \
           whether the two were identical to the last bit",
    decides: "whether MCF may ask this artifact for embeddings and what width to expect — and \
              nothing about how good they are, which needs a graded task (D42, §XIII)",
};

pub type Ask<'a> = &'a mut dyn FnMut(&str) -> Option<(usize, String, usize)>;

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
        tokens: 0,
        conditions,
    }
}

#[cfg(test)]
mod tests;

#[must_use]
pub fn measured(path: &std::path::Path, text: &str) -> Option<(usize, String, usize)> {
    use mcf_standin::bert;
    use mcf_standin::tokenizer::Vocabulary;
    let bytes = std::fs::read(path).ok()?;
    let file = mcf_standin::gguf::parse(&bytes).ok()?;
    let vocabulary = Vocabulary::read(&file).ok()?;
    let tokens = vocabulary.encode(text, true).ok()?;
    let model = bert::load(&file, &bytes)
        .ok()?
        .across(mcf_standin::threads::Threads::what_the_machine_reports());
    let build = mcf_core::build_identity::BuildIdentity::current()
        .version
        .to_owned();
    let marked = bert::embed(&model, &build, &tokens).ok()?;
    let embedding = marked.value().observed().clone();
    let mut digest = mcf_core::digest::Sha256::new();
    for value in &embedding.vector {
        digest.update(&value.to_le_bytes());
    }
    Some((embedding.vector.len(), digest.finish().hex(), tokens.len()))
}

#[must_use]
pub fn similarities(path: &std::path::Path, triples: &[[&str; 3]]) -> Option<Vec<(i64, i64)>> {
    use mcf_standin::bert;
    use mcf_standin::tokenizer::Vocabulary;
    let bytes = std::fs::read(path).ok()?;
    let file = mcf_standin::gguf::parse(&bytes).ok()?;
    let vocabulary = Vocabulary::read(&file).ok()?;
    let model = bert::load(&file, &bytes)
        .ok()?
        .across(mcf_standin::threads::Threads::what_the_machine_reports());
    let build = mcf_core::build_identity::BuildIdentity::current()
        .version
        .to_owned();
    let embed = |text: &str| {
        let tokens = vocabulary.encode(text, true).ok()?;
        let marked = bert::embed(&model, &build, &tokens).ok()?;
        Some(marked.value().observed().clone())
    };
    let mut out = Vec::with_capacity(triples.len());
    for [first, second, third] in triples {
        let (a, b, c) = (embed(first)?, embed(second)?, embed(third)?);
        out.push((
            bert::similarity_millionths(&a, &b),
            bert::similarity_millionths(&a, &c),
        ));
    }
    Some(out)
}
