use std::path::Path;

use mcf_core::probe::{Method, Outcome, Probed};

use super::Counted;

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cost {
    pub language: &'static str,
    pub tokens: usize,
    pub characters: usize,
    pub against_english_ppm: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Spend {
    pub costs: Vec<Cost>,
    pub dearest: &'static str,
    pub cheapest: &'static str,
    pub unencodable: Vec<(&'static str, String)>,
    pub read_by: String,
}

pub const LANGUAGE_COST: Method = Method {
    name: "language-cost",
    asks: "the same sentence in six languages of the tokenizer that generates for this model, \
           and counts the identifiers each one spends — no generation, no rater, and no \
           judgement",
    decides: "how much of a context budget, a token budget and a turn each language costs on \
              this file — and nothing whatever about how well the model speaks it, which is a \
              graded task and belongs to a laboratory (D42, §XIII)",
};

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
        tokens: 0,
        conditions,
    }
}

#[cfg(test)]
mod tests;
