//! What the sampler chooses, from distributions small enough to state.

#![allow(clippy::float_cmp)]

use super::{Rng, Settings, next};

/// Greedy is the argmax, and needs no generator to say so.
#[test]
fn greedy_takes_the_largest_logit() {
    let mut rng = Rng::seeded(1);
    assert_eq!(next(&[0.1, 9.0, 0.2], Settings::Greedy, &mut rng), Some(1));
    // And the generator was not consulted: the same call again is the same
    // answer, which it would be anyway, but the state has not moved either.
    let before = rng.clone();
    let _again = next(&[0.1, 9.0, 0.2], Settings::Greedy, &mut rng);
    assert_eq!(format!("{before:?}"), format!("{rng:?}"));
}

/// A temperature of zero is greedy — the limit, stated rather than refused, so
/// that a caller sweeping a temperature down gets an answer at the bottom.
#[test]
fn a_temperature_of_zero_is_greedy() {
    let mut rng = Rng::seeded(7);
    let settings = Settings::Nucleus {
        temperature: 0.0,
        top_p: 1.0,
    };
    assert_eq!(next(&[1.0, 2.0, 3.0], settings, &mut rng), Some(2));
}

/// A seed identifies a sequence. Two generators seeded the same way choose the
/// same tokens, which is what makes a disagreement with another engine
/// attributable to the engines (D19, §3.12).
#[test]
fn the_same_seed_chooses_the_same_tokens() {
    let logits = [1.0, 1.2, 0.9, 1.1, 0.7];
    let settings = Settings::Nucleus {
        temperature: 1.0,
        top_p: 1.0,
    };
    let mut one = Rng::seeded(42);
    let mut other = Rng::seeded(42);
    let first: Vec<Option<usize>> = (0..32).map(|_| next(&logits, settings, &mut one)).collect();
    let second: Vec<Option<usize>> = (0..32)
        .map(|_| next(&logits, settings, &mut other))
        .collect();
    assert_eq!(first, second);

    // And a different seed is a different sequence, or the seed would be
    // decorative.
    let mut third = Rng::seeded(43);
    let other: Vec<Option<usize>> = (0..32)
        .map(|_| next(&logits, settings, &mut third))
        .collect();
    assert_ne!(first, other);
}

/// Nucleus sampling never chooses outside the candidate set. With one token
/// holding almost all the mass, a small `top_p` must always return it.
#[test]
fn the_nucleus_bounds_what_can_be_chosen() {
    // Softmax of these is dominated by index 3.
    let logits = [0.0, 0.0, 0.0, 12.0, 0.0];
    let settings = Settings::Nucleus {
        temperature: 1.0,
        top_p: 0.5,
    };
    let mut rng = Rng::seeded(9);
    for _ in 0..200 {
        assert_eq!(next(&logits, settings, &mut rng), Some(3));
    }
}

/// A uniform distribution with the whole mass admitted reaches every token, and
/// in proportion. Deterministic given the seed, so this is an assertion rather
/// than a hope.
#[test]
fn a_uniform_distribution_is_sampled_across() {
    let logits = [0.0, 0.0, 0.0, 0.0];
    let settings = Settings::Nucleus {
        temperature: 1.0,
        top_p: 1.0,
    };
    let mut rng = Rng::seeded(5);
    let mut counts = [0_u32; 4];
    for _ in 0..4_000 {
        if let Some(index) = next(&logits, settings, &mut rng)
            && let Some(slot) = counts.get_mut(index)
        {
            *slot += 1;
        }
    }
    for (index, count) in counts.iter().enumerate() {
        assert!(
            *count > 800 && *count < 1_200,
            "token {index} was chosen {count} times of 4000, which is not a quarter"
        );
    }
}

/// A sharper temperature concentrates the choice, which is the whole of what
/// the setting does.
#[test]
fn a_lower_temperature_concentrates_the_choice() {
    let logits = [0.0, 1.0];
    let mut counts = [0_u32; 2];
    let mut rng = Rng::seeded(11);
    for _ in 0..2_000 {
        let settings = Settings::Nucleus {
            temperature: 0.25,
            top_p: 1.0,
        };
        if let Some(index) = next(&logits, settings, &mut rng)
            && let Some(slot) = counts.get_mut(index)
        {
            *slot += 1;
        }
    }
    // At a temperature of a quarter the logits are 0 and 4, so the second token
    // takes about 98 % of the mass.
    assert!(
        counts.get(1).copied().unwrap_or(0) > 1_900,
        "the sharpened distribution chose the larger logit {} times of 2000",
        counts.get(1).copied().unwrap_or(0)
    );
}

/// An empty vocabulary has no next token, and says so rather than choosing
/// zero.
#[test]
fn nothing_to_choose_from_is_nothing() {
    let mut rng = Rng::seeded(1);
    assert_eq!(next(&[], Settings::Greedy, &mut rng), None);
    assert_eq!(
        next(
            &[],
            Settings::Nucleus {
                temperature: 1.0,
                top_p: 1.0
            },
            &mut rng
        ),
        None
    );
}
