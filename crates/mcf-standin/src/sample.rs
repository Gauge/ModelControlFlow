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
    /// The distribution, softened by a temperature and narrowed three ways:
    /// to the `top_k` likeliest tokens, to those at least `min_p` of the
    /// likeliest's probability, and to the smallest set whose mass reaches
    /// `top_p` — in that order, each on what the one before left.
    ///
    /// A temperature of zero is greedy — the limit rather than a special case.
    /// `top_k` of nought, `min_p` of nought and `top_p` of one each leave the
    /// distribution whole, which is how *off* is stated (B-440).
    ///
    /// **Where this differs from the provisioned engine**: that engine
    /// truncates the raw distribution and applies the temperature after; this
    /// applies the temperature first. `top_k` is unaffected, and `top_p` and
    /// `min_p` select the same set only at a temperature of one.
    Nucleus {
        /// How much the distribution is flattened. Above one is flatter, below
        /// one is sharper.
        temperature: f32,
        /// How many of the likeliest tokens are kept; nought keeps every one.
        top_k: usize,
        /// The probability mass the candidate set must reach.
        top_p: f32,
        /// The least probability kept, as a fraction of the likeliest token's.
        min_p: f32,
    },
}

impl Settings {
    /// The distribution at a temperature stated in thousandths, cut by a
    /// `top_k` count and `top_p` and `min_p` stated in thousandths, so a
    /// caller that keeps its numbers exact never holds a float; nought
    /// temperature is greedy, and the cut is then moot. `top_k` nought,
    /// `top_p` 1000 and `min_p` nought are each *off*.
    #[must_use]
    #[allow(
        clippy::cast_precision_loss,
        reason = "thousandths fit in a float exactly enough for a sampler"
    )]
    pub fn at_thousandths(temperature: u32, top_k: usize, top_p: u32, min_p: u32) -> Self {
        if temperature == 0 {
            Self::Greedy
        } else {
            Self::Nucleus {
                temperature: temperature as f32 / 1000.0,
                top_k,
                top_p: top_p as f32 / 1000.0,
                min_p: min_p as f32 / 1000.0,
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
        Settings::Nucleus {
            temperature,
            top_k,
            top_p,
            min_p,
        } => {
            if temperature <= 0.0 {
                // The limit of the distribution as the temperature falls, and
                // stated as such rather than refused: a caller sweeping a
                // temperature down to zero should get greedy, not an error.
                return ops::argmax(logits);
            }
            nucleus(logits, temperature, (top_k, top_p, min_p), rng)
        }
    }
}

/// The nucleus itself: soften, sort, cut to the `top_k` likeliest, drop what
/// is under `min_p` of the likeliest, take the smallest set that reaches the
/// mass, and draw from it.
fn nucleus(
    logits: &[f32],
    temperature: f32,
    (top_k, top_p, min_p): (usize, f32, f32),
    rng: &mut Rng,
) -> Option<usize> {
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

    // The `top_k` likeliest, then those within `min_p` of the likeliest, then
    // the smallest prefix whose mass reaches `top_p`. At least one token
    // always, because a threshold below the largest probability would otherwise
    // select nothing and there is always a next token to choose.
    let likeliest = order
        .first()
        .and_then(|index| probabilities.get(*index))
        .copied()
        .unwrap_or(0.0);
    let floor = likeliest * min_p;
    let mut kept = Vec::new();
    let mut mass = 0.0_f32;
    for (rank, index) in order.into_iter().enumerate() {
        let probability = probabilities.get(index).copied().unwrap_or(0.0);
        let cut_by_k = top_k > 0 && rank >= top_k;
        let cut_by_p = rank > 0 && probability < floor;
        if cut_by_k || cut_by_p {
            break;
        }
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
