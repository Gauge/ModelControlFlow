//! The standard seed set is shown to be representative, not assumed to be
//! (B-291, D19, §6.16, §7.13).
//!
//! **§6.16, turned on MCF's own instrument.** *The instrument does not get to
//! grade itself.* D19 applies that to the seed set: the published set is the
//! first *n* draws of a stated stream, and *periodically, a larger random set
//! is run and its distribution compared with the fixed set's.* If the prefix
//! behaves differently from the body, the prefix is unrepresentative and is
//! replaced — with the replacement recorded as a break in comparability
//! (§7.13).
//!
//! **The larger draw is the same stream, far from its start.** D19 says *a
//! larger random set*, and a set drawn from somewhere else would be answering
//! a question about that somewhere else. The published stream is a bijection
//! over `u64`, so its first thirty-two values and three hundred and twenty of
//! its values a million indices later are two draws from one space — and
//! whether the first is like the rest is exactly what *is the prefix
//! representative* means. It is also reproducible, which a genuinely random
//! draw would not be (§3.12).
//!
//! **It samples with nucleus, and it has to.** MCF's shipped generation is
//! greedy, and **greedy ignores the seed entirely** — every seed produces the
//! same tokens, so a validation run against it would compare a constant with a
//! constant and clear the set for a reason that is not about the set. The tier
//! therefore names a stochastic sampler explicitly. When MCF ships a
//! stochastic default (D18, and a sweep that has not happened), this is the
//! line that must change to match it.
//!
//! **Scheduled rather than gating**, because it needs a model on the disk and
//! runs hundreds of generations: `scripts/ci.sh --with-seed-set`. The model is
//! named by `MCF_SEED_SET_MODEL`, which the script finds and sets.
//!
//! This is not a measurement of a model, and nothing here is timed: the
//! quantity is a count of distinct tokens, which is a behaviour statistic and
//! is what a stand-in may legitimately produce (B65, D31).

// Every item in this file is test code; see the note in `taxonomy_agreement.rs`.
#![allow(clippy::panic, clippy::expect_used)]

use std::collections::BTreeSet;

use mcf_bench::seeds::{Representative, representative};
use mcf_core::measurement::PartsPerMillion;
use mcf_core::trial::published;
use mcf_standin::sample::Settings;
use mcf_standin::session::{Generated, Request};

/// How many seeds the published set is being validated over.
///
/// A chosen number, stated in one line: it is the size a behaviour laboratory
/// would plausibly use for one arm, and the tier's cost is linear in it.
const STANDARD: usize = 32;

/// How much larger the comparison draw is.
///
/// Ten times, which is what makes it *a larger set* rather than a second one
/// the same size — a set of equal size could differ from the standard one by
/// luck as easily as the standard one differs from the stream.
const LARGER: usize = 10;

/// Where in the stream the larger draw is taken from.
///
/// Far enough from the start that it shares no value with the standard set,
/// which the stream's bijectivity then guarantees for every index.
const FAR: u64 = 1_000_000;

/// How large a difference between the two distributions would matter.
///
/// Ten percent. Chosen, and stated: a prefix within a tenth of the stream is
/// representative for any purpose MCF has, and one a tenth off is not.
const RESOLVING: PartsPerMillion = PartsPerMillion(100_000);

/// The sampler the seed set is validated against.
///
/// Nucleus rather than greedy, for the reason in this file's header: greedy
/// ignores the seed, so validating against it would clear the set without
/// looking at it. The values are ordinary ones and are conditions of the
/// result, not choices about quality.
const SAMPLER: Settings = Settings::Nucleus {
    temperature: 1.0,
    top_k: 0,
    top_p: 0.95,
    min_p: 0.0,
};

/// How many tokens each trial generates.
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

    // One prompt, held still: what varies between trials must be the seed and
    // nothing else (A8).
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
        // The mark travels with the value (A5): this reads through it rather
        // than unwrapping, and what it takes out is a count and never a speed.
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

    // The two draws share no seed, which the stream's bijectivity gives and
    // which is asserted here rather than assumed: a larger draw that overlapped
    // the standard set would be comparing part of the set with itself.
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
