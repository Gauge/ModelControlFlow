//! Multilingual exact answers: the same arithmetic and reading tasks put
//! in six languages, the same exact match (B-528, D55, B-057).
//!
//! Every task has a whole-number answer, so the same parser reads every
//! language: the first whole number in the reply. What the rows show is
//! whether a model that gets a sum right in English gets it right in
//! Portuguese, task by task and language by language; nothing here
//! judges the prose the number came in.

use mcf_record::json::Value;

use super::{Found, Reading, Site, as_integer, framed_ids, timed};
use crate::generation::Draw;
use crate::served::{Prompt, Startup};

/// The measurement's name.
pub const NAME: &str = "multilingual";

/// How many tokens an answer may take.
const BUDGET: usize = 80;

/// The languages, by their own names' first letters, in one order.
pub const LANGUAGES: [&str; 6] = ["en", "fr", "de", "es", "it", "pt"];

/// One task: its answer and its wording in each language, in
/// `LANGUAGES` order.
#[derive(Debug)]
pub struct Task {
    /// Its name.
    pub name: &'static str,
    /// The answer, a whole number.
    pub answer: i64,
    /// The task in each language.
    pub asks: [&'static str; 6],
}

/// The tasks.
pub const TASKS: &[Task] = &[
    Task {
        name: "seventeen-times-twenty-three",
        answer: 391,
        asks: [
            "What is 17 times 23? Answer with the number only.",
            "Combien font 17 fois 23 ? Réponds uniquement par le nombre.",
            "Was ist 17 mal 23? Antworte nur mit der Zahl.",
            "¿Cuánto es 17 por 23? Responde solo con el número.",
            "Quanto fa 17 per 23? Rispondi solo con il numero.",
            "Quanto é 17 vezes 23? Responda apenas com o número.",
        ],
    },
    Task {
        name: "a-thousand-minus-357",
        answer: 643,
        asks: [
            "What is 1000 minus 357? Answer with the number only.",
            "Combien font 1000 moins 357 ? Réponds uniquement par le nombre.",
            "Was ist 1000 minus 357? Antworte nur mit der Zahl.",
            "¿Cuánto es 1000 menos 357? Responde solo con el número.",
            "Quanto fa 1000 meno 357? Rispondi solo con il numero.",
            "Quanto é 1000 menos 357? Responda apenas com o número.",
        ],
    },
    Task {
        name: "144-divided-by-12",
        answer: 12,
        asks: [
            "What is 144 divided by 12? Answer with the number only.",
            "Combien font 144 divisé par 12 ? Réponds uniquement par le nombre.",
            "Was ist 144 geteilt durch 12? Antworte nur mit der Zahl.",
            "¿Cuánto es 144 dividido entre 12? Responde solo con el número.",
            "Quanto fa 144 diviso 12? Rispondi solo con il numero.",
            "Quanto é 144 dividido por 12? Responda apenas com o número.",
        ],
    },
    Task {
        name: "the-platform-number",
        answer: 7,
        asks: [
            "The train leaves at 14:35 from platform 7. Which platform does it leave from? Answer with the number only.",
            "Le train part à 14 h 35 de la voie 7. De quelle voie part-il ? Réponds uniquement par le nombre.",
            "Der Zug fährt um 14:35 von Gleis 7 ab. Von welchem Gleis fährt er ab? Antworte nur mit der Zahl.",
            "El tren sale a las 14:35 del andén 7. ¿De qué andén sale? Responde solo con el número.",
            "Il treno parte alle 14:35 dal binario 7. Da quale binario parte? Rispondi solo con il numero.",
            "O comboio parte às 14:35 da plataforma 7. De que plataforma parte? Responda apenas com o número.",
        ],
    },
    Task {
        name: "the-younger-brother",
        answer: 29,
        asks: [
            "Anna is 34 years old and her brother is 5 years younger. How old is her brother? Answer with the number only.",
            "Anna a 34 ans et son frère a 5 ans de moins. Quel âge a son frère ? Réponds uniquement par le nombre.",
            "Anna ist 34 Jahre alt und ihr Bruder ist 5 Jahre jünger. Wie alt ist ihr Bruder? Antworte nur mit der Zahl.",
            "Anna tiene 34 años y su hermano es 5 años menor. ¿Cuántos años tiene su hermano? Responde solo con el número.",
            "Anna ha 34 anni e suo fratello ha 5 anni di meno. Quanti anni ha suo fratello? Rispondi solo con il numero.",
            "A Anna tem 34 anos e o irmão dela é 5 anos mais novo. Quantos anos tem o irmão dela? Responda apenas com o número.",
        ],
    },
    Task {
        name: "apples-left",
        answer: 18,
        asks: [
            "A box holds 24 apples. Six are taken out. How many apples are left in the box? Answer with the number only.",
            "Une caisse contient 24 pommes. On en retire six. Combien de pommes reste-t-il dans la caisse ? Réponds uniquement par le nombre.",
            "In einer Kiste sind 24 Äpfel. Sechs werden herausgenommen. Wie viele Äpfel bleiben in der Kiste? Antworte nur mit der Zahl.",
            "Una caja contiene 24 manzanas. Se sacan seis. ¿Cuántas manzanas quedan en la caja? Responde solo con el número.",
            "Una cassetta contiene 24 mele. Ne vengono tolte sei. Quante mele restano nella cassetta? Rispondi solo con il numero.",
            "Uma caixa tem 24 maçãs. Seis são retiradas. Quantas maçãs ficam na caixa? Responda apenas com o número.",
        ],
    },
];

/// Runs it.
#[must_use]
#[allow(
    clippy::too_many_lines,
    reason = "one measurement read straight through: each task in each language a row, the count a language beside them"
)]
pub fn measure(site: &Site<'_>) -> Found {
    let engine = match site.server(&Startup {
        projector: None,
        ..site.startup()
    }) {
        Ok(engine) => engine,
        Err(why) => return Found::could_not_tell(&why),
    };
    let mut rows = Vec::new();
    let mut lines = vec![format!(
        "  {} task(s) in {} language(s), greedy; the reply's one number, or the one after its \
         last =, or its last, is the answer",
        TASKS.len(),
        LANGUAGES.len()
    )];
    let mut right_by_language = [0_usize; 6];
    let mut asked = 0_usize;
    for task in TASKS {
        let mut said = Vec::with_capacity(LANGUAGES.len());
        for (at, (language, asks)) in LANGUAGES.iter().zip(task.asks.iter()).enumerate() {
            if site.asker_gone() {
                return Found::could_not_tell(crate::served::CLIENT_LEFT);
            }
            let ids = match framed_ids(&engine, asks) {
                Ok(ids) => ids,
                Err(why) => return Found::could_not_tell(&why),
            };
            let (done, ns) = timed(|| {
                engine.complete(
                    Prompt::Identifiers(&ids),
                    BUDGET,
                    Draw::greedy(0),
                    false,
                    site.timed(),
                )
            });
            let completed = match done {
                Ok(completed) => completed,
                Err(failure) => return Found::could_not_tell(failure.detail()),
            };
            let numbers = super::paraphrase::numbers_in(&completed.text);
            let answer = super::paraphrase::answer_in(&completed.text);
            let right = answer == Some(task.answer);
            asked = asked.saturating_add(1);
            if let Some(count) = right_by_language.get_mut(at) {
                *count = count.saturating_add(usize::from(right));
            }
            let dims = [
                ("task", Value::text(task.name)),
                ("language", Value::text(*language)),
            ];
            rows.push(Reading::new(
                &dims,
                "read",
                i64::from(answer.is_some()),
                "bool",
            ));
            if let Some(answer) = answer {
                rows.push(Reading::new(&dims, "answer", answer, "count"));
            }
            rows.push(Reading::new(&dims, "right", i64::from(right), "bool"));
            rows.push(Reading::new(
                &dims,
                "numbers",
                as_integer(numbers.len()),
                "count",
            ));
            rows.push(Reading::new(
                &dims,
                "stated",
                i64::from(numbers.contains(&task.answer)),
                "bool",
            ));
            rows.push(Reading::new(
                &dims,
                "tokens",
                as_integer(completed.predicted),
                "tokens",
            ));
            rows.push(Reading::new(
                &dims,
                "ns",
                i64::try_from(ns).unwrap_or(i64::MAX),
                "ns",
            ));
            said.push(format!(
                "{language} {}",
                answer.map_or_else(|| "?".to_owned(), |n| n.to_string())
            ));
        }
        lines.push(format!("  {:<30} {}", task.name, said.join("   ")));
    }
    for (language, count) in LANGUAGES.iter().zip(right_by_language) {
        rows.push(Reading::new(
            &[("language", Value::text(*language))],
            "right",
            as_integer(count),
            "count",
        ));
    }
    let right_all: usize = right_by_language.iter().sum();
    lines.push(format!(
        "  {right_all} of {asked} right; by language {}",
        LANGUAGES
            .iter()
            .zip(right_by_language)
            .map(|(language, count)| format!("{language} {count}/{}", TASKS.len()))
            .collect::<Vec<String>>()
            .join(", ")
    ));
    let mut fields = vec![
        ("tasks", Value::Integer(as_integer(TASKS.len()))),
        ("asked", Value::Integer(as_integer(asked))),
        ("right", Value::Integer(as_integer(right_all))),
    ];
    for (language, count) in LANGUAGES.iter().zip(right_by_language) {
        fields.push((*language, Value::Integer(as_integer(count))));
    }
    Found {
        lines,
        fields,
        rows,
    }
}

#[cfg(test)]
mod tests {
    use super::{LANGUAGES, TASKS};

    #[test]
    fn every_task_is_put_in_every_language_and_carries_its_figures() {
        for task in TASKS {
            assert_eq!(task.asks.len(), LANGUAGES.len());
            for asks in task.asks {
                let digits: Vec<i64> = task.asks[0]
                    .split(|c: char| !c.is_ascii_digit())
                    .filter(|word| !word.is_empty())
                    .filter_map(|word| word.parse().ok())
                    .collect();
                for figure in &digits {
                    assert!(asks.contains(&figure.to_string()), "{}: {asks}", task.name);
                }
            }
        }
    }
}
