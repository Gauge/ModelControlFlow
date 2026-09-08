use crate::ops;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Settings {
    Greedy,
    Nucleus {
        temperature: f32,
        top_k: usize,
        top_p: f32,
        min_p: f32,
    },
}

impl Settings {
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

#[derive(Debug, Clone)]
pub struct Rng {
    state: u64,
}

impl Rng {
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

    fn unit(&mut self) -> f32 {
        let bits = self.next_u64() >> 40;
        f32::from(u16::try_from(bits >> 8).unwrap_or(0)) / 65_536.0
    }
}

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
                return ops::argmax(logits);
            }
            nucleus(logits, temperature, (top_k, top_p, min_p), rng)
        }
    }
}

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

    let mut order: Vec<usize> = (0..probabilities.len()).collect();
    order.sort_by(|left, right| {
        let left_probability = probabilities.get(*left).copied().unwrap_or(0.0);
        let right_probability = probabilities.get(*right).copied().unwrap_or(0.0);
        right_probability
            .partial_cmp(&left_probability)
            .unwrap_or(core::cmp::Ordering::Equal)
            .then(left.cmp(right))
    });

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
