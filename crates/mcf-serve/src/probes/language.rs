//! What a language costs this model's vocabulary — and what that is not
//! (B-057, B-379, F81, D42, §X, A6, A7).
//!
//! **The one modality question answerable today without a generation, a rater
//! or a judgement.** How many tokens a sentence spends is the tokenizer's
//! answer, it is exact, it is deterministic, and it arrives in milliseconds.
//! Every other multilingual question — whether a model *speaks* a language
//! well — is a graded task and belongs to a laboratory (§XIII).
//!
//! **Whose tokenizer.** The one that generates for this model: the count a
//! turn is charged is that tokenizer's reading, and a vocabulary MCF's own
//! tokenizer refuses is still read by the engine that answers for it — so the
//! counting goes through the daemon, which reads with whichever engine the
//! model resolves to, and the result names it (B-442, B-441, F158).
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

use super::Counted;

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
    /// Any sample the tokenizer could not read at all, and why in its own
    /// words — a fact about the file rather than a failure of the probe (A7).
    pub unencodable: Vec<(&'static str, String)>,
    /// The tokenizer that counted, in the daemon's words: a count is a
    /// reading, and a reading has a reader (B-442).
    pub read_by: String,
}

/// The method.
pub const LANGUAGE_COST: Method = Method {
    name: "language-cost",
    asks: "the same sentence in six languages of the tokenizer that generates for this model, \
           and counts the identifiers each one spends — no generation, no rater, and no \
           judgement",
    decides: "how much of a context budget, a token budget and a turn each language costs on \
              this file — and nothing whatever about how well the model speaks it, which is a \
              graded task and belongs to a laboratory (D42, §XIII)",
};

/// Runs the language-cost probe.
///
/// `count` is how a sample is counted: the text in, how many identifiers it
/// cost and who counted out, or why it could not be. Passing it in keeps the
/// probe independent of which tokenizer read — the engine is a condition and
/// the caller states it (D42) — and the caller's counter is the daemon's,
/// which reads with the engine that generates for this model (B-442). No
/// generation runs; the engine is named because its tokenizer took part.
///
/// # Errors
///
/// Never: a probe that cannot decide reports `Inconclusive` with its reason
/// (D42's third state).
#[must_use]
pub fn language_cost(
    model: &Path,
    engine: &str,
    count: &mut dyn FnMut(&str) -> Result<Counted, String>,
) -> Probed<Spend> {
    let conditions = super::conditions(&LANGUAGE_COST, model, engine);

    let mut counted: Vec<(&'static str, usize, usize)> = Vec::new();
    let mut unencodable = Vec::new();
    let mut read_by: Option<String> = None;
    for (language, sample) in SAMPLES {
        // Without the beginning-of-text marker: it is a property of the turn,
        // not of the language, and counting it would add one to every sample
        // and change every ratio. The counter is asked without it.
        match count(sample) {
            Ok(Counted { tokens, by }) => {
                counted.push((language, tokens, sample.chars().count()));
                let _named = read_by.get_or_insert(by);
            }
            Err(why) => unencodable.push((language, why)),
        }
    }

    let Some((_, english, _)) = counted
        .iter()
        .find(|(language, _, _)| *language == "English")
    else {
        // The English sample is asked first, so where nothing was counted
        // the reason is its reason, in the counter's own words (A7).
        let why = unencodable
            .iter()
            .find(|(language, _)| *language == "English")
            .map_or_else(String::new, |(_, why)| format!(": {why}"));
        return Probed::inconclusive(
            LANGUAGE_COST,
            format!(
                "the English sample could not be counted, so there is no baseline to express \
                 the others against{why}"
            ),
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
            read_by: read_by.unwrap_or_default(),
        }),
        trials,
        // No tokens were generated. Nought is the true figure and the surface
        // says why, rather than leaving a reader to wonder what a probe that
        // spent nothing did.
        tokens: 0,
        conditions,
    }
}

#[cfg(test)]
mod tests;
