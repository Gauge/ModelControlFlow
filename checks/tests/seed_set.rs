#![allow(clippy::panic, clippy::expect_used)]

use std::collections::BTreeSet;

use mcf_bench::seeds::{Representative, representative};
use mcf_core::measurement::PartsPerMillion;
use mcf_core::trial::published;
use mcf_standin::sample::Settings;
use mcf_standin::session::{Generated, Request};

const STANDARD: usize = 32;

const LARGER: usize = 10;

const FAR: u64 = 1_000_000;

const RESOLVING: PartsPerMillion = PartsPerMillion(100_000);

const SAMPLER: Settings = Settings::Nucleus {
    temperature: 1.0,
    top_k: 0,
    top_p: 0.95,
    min_p: 0.0,
};

const TOKENS: usize = 48;

#[test]
#[ignore = "scheduled: needs a model on the disk and runs hundreds of generations"]
fn the_standard_seed_set_is_representative_of_the_stream() {
    let Ok(path) = std::env::var("MCF_SEED_SET_MODEL") else {
        panic!(
            "MCF_SEED_SET_MODEL names the model this tier runs; scripts/check-seed-set.sh finds \
             one and sets it. A tier that could not run must say so rather than pass (B38)"
        );
    };
    let bytes = std::fs::read(&path).unwrap_or_else(|error| panic!("{path} is readable: {error}"));
    let file = mcf_standin::gguf::parse(&bytes).expect("the model file parses");
    let vocabulary = mcf_standin::tokenizer::Vocabulary::read(&file).expect("a vocabulary");
    let model = mcf_standin::llama::load(&file, &bytes).expect("the model loads");
    let build = mcf_core::build_identity::BuildIdentity::current()
        .version
        .to_owned();

    let prompt = vocabulary
        .encode("Once upon a time", true)
        .expect("the prompt encodes");

    let distinct = |seed: u64| -> u64 {
        let produced = mcf_standin::session::generate(
            &model,
            &build,
            &Request {
                prompt: prompt.clone(),
                limit: TOKENS,
                settings: SAMPLER,
                seed,
                stop: Vec::new(),
            },
        )
        .expect("a generation");
        let held: &Generated = produced.value().observed();
        let unique: BTreeSet<usize> = held.tokens.iter().copied().collect();
        u64::try_from(unique.len()).unwrap_or(0)
    };

    let standard: Vec<u64> = (0..STANDARD)
        .map(|at| distinct(published(u64::try_from(at).unwrap_or(0))))
        .collect();
    let larger: Vec<u64> = (0..STANDARD.saturating_mul(LARGER))
        .map(|at| {
            distinct(published(
                FAR.saturating_add(u64::try_from(at).unwrap_or(0)),
            ))
        })
        .collect();

    let taken: BTreeSet<u64> = (0..STANDARD)
        .map(|at| published(u64::try_from(at).unwrap_or(0)))
        .collect();
    let against: BTreeSet<u64> = (0..STANDARD.saturating_mul(LARGER))
        .map(|at| published(FAR.saturating_add(u64::try_from(at).unwrap_or(0))))
        .collect();
    assert!(
        taken.intersection(&against).next().is_none(),
        "the larger draw must not include the set it is validating"
    );
    assert_eq!(against.len(), STANDARD.saturating_mul(LARGER));

    let held = representative(&standard, &larger, RESOLVING);
    println!("  sampler: {SAMPLER:?}, {TOKENS} tokens, model {path}");
    println!("  {held}");
    assert!(
        held.clears_the_set(),
        "the published seed set is not shown representative of the stream it is a prefix of. \
         D19's answer is to replace it and record the replacement as a break in comparability \
         (§7.13): {held}"
    );
    assert!(
        matches!(held, Representative::Yes { .. }),
        "and *not decided* is not clearance (§6.16)"
    );
}
