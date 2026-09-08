use mcf_record::json::Value;

use super::{Found, Reading, Site, as_integer};

pub const NAME: &str = "nearest";

pub const TRIPLES: &[(&str, &str, &str, &str)] = &[
    (
        "ferry",
        "The ferry leaves the harbour every morning at eight.",
        "Each morning at eight the boat departs from the port.",
        "The recipe calls for two eggs and a cup of flour.",
    ),
    (
        "library",
        "The library closes early on Fridays.",
        "On Fridays the library shuts earlier than usual.",
        "The engine overheated on the motorway.",
    ),
    (
        "garden",
        "She planted tomatoes along the sunny wall.",
        "Tomatoes were put in beside the wall that gets the sun.",
        "The invoice is due in thirty days.",
    ),
    (
        "bridge",
        "The old bridge was closed for repairs last spring.",
        "Last spring they shut the old bridge to repair it.",
        "He learned three chords on the guitar.",
    ),
    (
        "train",
        "The train to the coast was delayed by an hour.",
        "The coastal train ran sixty minutes late.",
        "The museum added a wing for modern art.",
    ),
    (
        "bread",
        "Fresh bread is baked here before dawn.",
        "They bake the bread here early, before the sun is up.",
        "The chess club meets on Tuesday evenings.",
    ),
    (
        "storm",
        "A storm knocked out the power across the valley.",
        "The valley lost electricity when the storm hit.",
        "The dictionary lists four meanings for the word.",
    ),
    (
        "cat",
        "The cat sleeps on the warm windowsill all afternoon.",
        "All afternoon the cat dozes on the sunny sill.",
        "The committee approved the budget for next year.",
    ),
];

#[must_use]
pub fn measure(site: &Site<'_>) -> Found {
    let texts: Vec<[&str; 3]> = TRIPLES.iter().map(|(_, a, b, c)| [*a, *b, *c]).collect();
    let Some(similarities) = crate::probes::embedding::similarities(site.model, &texts) else {
        return Found::could_not_tell(
            "this file did not produce an embedding, which is what a file that is not an \
             embedding model does; the embedding probe says the same",
        );
    };
    let mut rows = Vec::new();
    let mut lines = vec![format!(
        "  {} triple(s) — a sentence, a paraphrase, an unrelated sentence — embedded by MCF's \
         own engine in this process; the paraphrase should sit closer",
        TRIPLES.len()
    )];
    let mut ordered_all = 0_usize;
    for ((name, _, _, _), (near, far)) in TRIPLES.iter().zip(similarities) {
        let ordered = near > far;
        ordered_all = ordered_all.saturating_add(usize::from(ordered));
        let dims = [("triple", Value::text(*name))];
        rows.push(Reading::new(
            &dims,
            "paraphrase_similarity",
            near,
            "millionths",
        ));
        rows.push(Reading::new(
            &dims,
            "unrelated_similarity",
            far,
            "millionths",
        ));
        rows.push(Reading::new(
            &dims,
            "margin",
            near.saturating_sub(far),
            "millionths",
        ));
        rows.push(Reading::new(&dims, "ordered", i64::from(ordered), "bool"));
        lines.push(format!(
            "  {name:<8} paraphrase {near:>8}   unrelated {far:>8}   {}",
            if ordered { "ordered" } else { "REVERSED" }
        ));
    }
    lines.push(format!(
        "  the paraphrase nearer in {ordered_all} of {} triple(s)",
        TRIPLES.len()
    ));
    Found {
        lines,
        fields: vec![
            ("triples", Value::Integer(as_integer(TRIPLES.len()))),
            ("ordered", Value::Integer(as_integer(ordered_all))),
        ],
        rows,
    }
}
