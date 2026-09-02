//! Choosing the next token from the logits.
//!
//! **Sampling is identity, not a knob** (D18). Two runs that sampled
//! differently are two configurations rather than two readings of one, so
//! everything here is explicit: the settings are a value a caller states, the
//! seed is a value a caller states, and nothing reads a clock or a global.
//!
//! **The seed is a condition** (D19). A generator seeded the same way produces
//! the same tokens, on any machine and in any order, which is what makes a
//! disagreement between this and a vendored engine attributable to the engines
//! rather than to chance (§3.12, A19).
//!
//! **The generator is written out.** `splitmix64`: three lines of arithmetic
//! with a stated algorithm, so that "the same seed" means the same thing
//! forever rather than the same thing until a dependency changes its mind.

use crate::ops;

/// How the next token is chosen.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Settings {
    /// The largest logit, always. Deterministic without a seed.
    Greedy,
    /// The distribution, softened by a temperature and narrowed to the smallest
    /// set of tokens whose probability reaches `top_p`.
    ///
    /// A temperature of zero is greedy — the limit rather than a special case —
    /// and `top_p` of one is the whole distribution.
    Nucleus {
        /// How much the distribution is flattened. Above one is flatter, below
        /// one is sharper.
        temperature: f32,
        /// The probability mass the candidate set must reach.
        top_p: f32,
    },
}

impl Settings {
    /// The whole distribution at a temperature stated in thousandths, so a
    /// caller that keeps its temperature exact never holds a float; nought
    /// is greedy.
    #[must_use]
    #[allow(
        clippy::cast_precision_loss,
        reason = "thousandths fit in a float exactly enough for a sampler"
    )]
    pub fn at_thousandths(temperature: u32) -> Self {
        if temperature == 0 {
            Self::Greedy
        } else {
            Self::Nucleus {
                temperature: temperature as f32 / 1000.0,
                top_p: 1.0,
            }
        }
    }
}

/// A deterministic generator, from a stated seed.
///
/// `splitmix64`, the same algorithm the suite's property tier uses, for the
/// same reason: a stated algorithm means a seed identifies a sequence rather
/// than identifying whatever the platform's generator does this year.
#[derive(Debug, Clone)]
pub struct Rng {
    state: u64,
}

impl Rng {
    /// A generator at a stated seed.
    #[must_use]
    pub const fn seeded(seed: u64) -> Self {
        Self { state: seed }
    }

    fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// A value in `[0, 1)`.
    ///
    /// The top 24 bits, which is exactly the precision an `f32` has: taking
    /// more would be arithmetic that looks more careful and rounds to the same
    /// numbers.
    fn unit(&mut self) -> f32 {
        let bits = self.next_u64() >> 40;
        f32::from(u16::try_from(bits >> 8).unwrap_or(0)) / 65_536.0
    }
}

/// Chooses the next token.
///
/// Returns `None` only when there are no logits at all, which is a model with
/// an empty vocabulary and not a case a caller has to handle twice.
#[must_use]
pub fn next(logits: &[f32], settings: Settings, rng: &mut Rng) -> Option<usize> {
    match settings {
        Settings::Greedy => ops::argmax(logits),
        Settings::Nucleus { temperature, top_p } => {
            if temperature <= 0.0 {
                // The limit of the distribution as the temperature falls, and
                // stated as such rather than refused: a caller sweeping a
                // temperature down to zero should get greedy, not an error.
                return ops::argmax(logits);
            }
            nucleus(logits, temperature, top_p, rng)
        }
    }
}

/// The nucleus itself: soften, sort, take the smallest set that reaches the
/// mass, and draw from it.
fn nucleus(logits: &[f32], temperature: f32, top_p: f32, rng: &mut Rng) -> Option<usize> {
    if logits.is_empty() {
        return None;
    }
    let mut probabilities: Vec<f32> = logits.iter().map(|logit| logit / temperature).collect();
    ops::softmax(&mut probabilities);

    // Sorted by probability, descending, with ties broken toward the lower
    // token so the candidate set is a function of the distribution alone.
    let mut order: Vec<usize> = (0..probabilities.len()).collect();
    order.sort_by(|left, right| {
        let left_probability = probabilities.get(*left).copied().unwrap_or(0.0);
        let right_probability = probabilities.get(*right).copied().unwrap_or(0.0);
        right_probability
            .partial_cmp(&left_probability)
            .unwrap_or(core::cmp::Ordering::Equal)
            .then(left.cmp(right))
    });

    // The smallest prefix whose mass reaches `top_p`. At least one token
    // always, because a threshold below the largest probability would otherwise
    // select nothing and there is always a next token to choose.
    let mut kept = Vec::new();
    let mut mass = 0.0_f32;
    for index in order {
        let probability = probabilities.get(index).copied().unwrap_or(0.0);
        kept.push((index, probability));
        mass += probability;
        if mass >= top_p {
            break;
        }
    }

    // Draw within the kept mass, so that narrowing the set does not change the
    // relative odds of what remains.
    let draw = rng.unit() * mass;
    let mut running = 0.0_f32;
    for (index, probability) in &kept {
        running += probability;
        if draw <= running {
            return Some(*index);
        }
    }
    kept.last().map(|(index, _)| *index)
}

#[cfg(test)]
mod tests;
