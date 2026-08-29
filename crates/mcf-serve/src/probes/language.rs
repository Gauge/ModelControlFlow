//! What a language costs this model's vocabulary — and what that is not
//! (B-057, B-379, F81, D42, §X, A6, A7).
//!
//! **The one modality question answerable today without an engine, a rater or
//! a judgement.** How many tokens a sentence spends is the tokenizer's answer,
//! it is exact, it is deterministic, and it arrives in milliseconds. Every
//! other multilingual question — whether a model *speaks* a language well — is
//! a graded task and belongs to a laboratory (§XIII).
//!
//! **Why it earns a place under D42's test.** *A probe earns its place when a
//! wrong answer to it would corrupt a measurement or a served answer.* This one
//! does both: tokens are the unit of the context budget, of the token budget, of
//! the time a turn takes and of what a comparison holds still. A model that
//! spends two and a half times as many tokens on the same meaning is being asked
//! a different question at the same budget, which is §3.8's measurement error
//! wearing a different hat.
//!
//! **What it must never be read as** (F81's care, kept here because the sentence
//! is where it is lost): a cost is a fact about a *file's vocabulary*. It says
//! nothing about fluency, correctness or grasp. A model can be excellent at a
//! language its vocabulary spells expensively, and a model whose vocabulary is
//! cheap on a language can be useless in it. The rendering says so every time,
//! not once at the top of a manual.
//!
//! **The comparison is of vocabularies, not of sentences**, so the same meaning
//! is used in every language and the English count is the baseline every other
//! is expressed against — a ratio, which travels, beside the counts, which do
//! not (§3.27).

use std::path::Path;

use mcf_core::probe::{Method, Outcome, Probed};
use mcf_standin::gguf;
use mcf_standin::tokenizer::Vocabulary;

/// The same meaning in each language, so that what differs between two counts
/// is the vocabulary rather than the sentence.
///
/// Six, and each one a plain declarative sentence about the same scene. A
/// longer or more idiomatic sample would measure the sample.
pub const SAMPLES: [(&str, &str); 6] = [
    (
        "English",
        "The quick brown fox jumps over the lazy dog near the riverbank at dawn.",
    ),
    (
        "German",
        "Der schnelle braune Fuchs springt über den faulen Hund am Flussufer.",
    ),
    (
        "French",
        "Le rapide renard brun saute par-dessus le chien paresseux près de la rivière.",
    ),
    (
        "Spanish",
        "El rápido zorro marrón salta sobre el perro perezoso cerca del río.",
    ),
    (
        "Japanese",
        "素早い茶色のキツネが川辺で怠け者の犬を飛び越えます。",
    ),
    (
        "Arabic",
        "الثعلب البني السريع يقفز فوق الكلب الكسول بالقرب من النهر.",
    ),
];

/// What one language cost.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cost {
    /// The language.
    pub language: &'static str,
    /// How many identifiers this vocabulary spends on the sample.
    pub tokens: usize,
    /// How many characters the sample has, so a reader can see that the two
    /// are different questions.
    pub characters: usize,
    /// The cost against the English sample's, in parts per million.
    ///
    /// A ratio because a ratio is the durable half (§3.27): another machine
    /// with another build will spend the same identifiers on the same file,
    /// and the ratio is what survives a comparison between two files.
    pub against_english_ppm: u64,
}

/// What the language-cost probe observed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Spend {
    /// Each language, in the order they were asked.
    pub costs: Vec<Cost>,
    /// The language this vocabulary spends most on, and the one it spends
    /// least on — the two ends of the answer.
    pub dearest: &'static str,
    /// The cheapest.
    pub cheapest: &'static str,
    /// Any sample the vocabulary could not encode at all, which is a fact
    /// about the file rather than a failure of the probe (A7).
    pub unencodable: Vec<&'static str>,
}

/// The method.
pub const LANGUAGE_COST: Method = Method {
    name: "language-cost",
    asks: "the same sentence in six languages of this model's own vocabulary, and counts the \
           identifiers each one spends — no generation, no engine, no rater, and no judgement",
    decides: "how much of a context budget, a token budget and a turn each language costs on \
              this file — and nothing whatever about how well the model speaks it, which is a \
              graded task and belongs to a laboratory (D42, §XIII)",
};

/// Runs the language-cost probe.
///
/// **It takes no engine**, which is unusual enough to be worth saying in the
/// result: D42 makes the engine a condition of a probe because a probe asks a
/// model to do something. This one asks the *file*, so its answer holds for
/// every engine that reads that file, and the conditions say so rather than
/// naming an engine that did not participate (A7).
///
/// # Errors
///
/// Never: a probe that cannot decide reports `Inconclusive` with its reason
/// (D42's third state).
#[must_use]
pub fn language_cost(model: &Path, bytes: &[u8]) -> Probed<Spend> {
    let conditions = super::conditions(&LANGUAGE_COST, model, NO_ENGINE);
    let Ok(file) = gguf::parse(bytes) else {
        return Probed::inconclusive(
            LANGUAGE_COST,
            "the file could not be read as a model",
            0,
            0,
            conditions,
        );
    };
    let Ok(vocabulary) = Vocabulary::read(&file) else {
        return Probed::inconclusive(
            LANGUAGE_COST,
            "the vocabulary could not be read, and the cost of a language is a fact about the \
             vocabulary",
            0,
            0,
            conditions,
        );
    };

    let mut counted: Vec<(&'static str, usize, usize)> = Vec::new();
    let mut unencodable = Vec::new();
    for (language, sample) in SAMPLES {
        // Without the beginning-of-text marker: it is a property of the turn,
        // not of the language, and counting it would add one to every sample
        // and change every ratio.
        match vocabulary.encode(sample, false) {
            Ok(identifiers) => counted.push((language, identifiers.len(), sample.chars().count())),
            Err(_) => unencodable.push(language),
        }
    }

    let Some((_, english, _)) = counted
        .iter()
        .find(|(language, _, _)| *language == "English")
    else {
        return Probed::inconclusive(
            LANGUAGE_COST,
            "the English sample could not be encoded, so there is no baseline to express the \
             others against",
            counted.len(),
            0,
            conditions,
        );
    };
    let english = *english;
    if english == 0 {
        return Probed::inconclusive(
            LANGUAGE_COST,
            "the English sample encoded to nothing, which is a vocabulary this probe cannot \
             read rather than a language that is free",
            counted.len(),
            0,
            conditions,
        );
    }

    let costs: Vec<Cost> = counted
        .iter()
        .map(|(language, tokens, characters)| Cost {
            language,
            tokens: *tokens,
            characters: *characters,
            against_english_ppm: u64::try_from(*tokens)
                .unwrap_or(u64::MAX)
                .saturating_mul(1_000_000)
                .checked_div(u64::try_from(english).unwrap_or(1))
                .unwrap_or(0),
        })
        .collect();

    let dearest = costs
        .iter()
        .max_by_key(|cost| cost.tokens)
        .map_or("", |cost| cost.language);
    let cheapest = costs
        .iter()
        .min_by_key(|cost| cost.tokens)
        .map_or("", |cost| cost.language);

    let trials = costs.len();
    Probed {
        method: LANGUAGE_COST,
        outcome: Outcome::Observed(Spend {
            costs,
            dearest,
            cheapest,
            unencodable,
        }),
        trials,
        // No tokens were generated. Nought is the true figure and the surface
        // says why, rather than leaving a reader to wonder what a probe that
        // spent nothing did.
        tokens: 0,
        conditions,
    }
}

/// What stands where an engine's name would be.
///
/// Spelled out rather than left empty: a condition set with a blank in it reads
/// as a condition nobody recorded, and this one is a condition nobody *needed*
/// (A7).
pub const NO_ENGINE: &str = "none — this asks the file's vocabulary, not a model";

#[cfg(test)]
mod tests;
