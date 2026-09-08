use crate::threads::Threads;

#[must_use]
pub fn matmul_vec(matrix: &[f32], vector: &[f32], rows: usize, columns: usize) -> Vec<f32> {
    matmul_vec_across(matrix, vector, rows, columns, Threads::definition())
}

#[must_use]
pub fn matmul_vec_across(
    matrix: &[f32],
    vector: &[f32],
    rows: usize,
    columns: usize,
    threads: Threads,
) -> Vec<f32> {
    if vector.len() != columns || matrix.len() != rows.saturating_mul(columns) {
        return Vec::new();
    }
    let mut out = vec![0.0_f32; rows];
    crate::threads::each_row(&mut out, 1, columns, threads, &|row, slot| {
        if let Some(cell) = slot.first_mut() {
            *cell = row_of(matrix, vector, row, columns);
        }
    });
    out
}

fn row_of(matrix: &[f32], vector: &[f32], row: usize, columns: usize) -> f32 {
    let start = row.saturating_mul(columns);
    let slice = matrix
        .get(start..start.saturating_add(columns))
        .unwrap_or(&[]);
    let mut total = 0.0_f32;
    for (weight, value) in slice.iter().zip(vector.iter()) {
        total = weight.mul_add(*value, total);
    }
    total
}

#[must_use]
pub fn rms_norm(x: &[f32], weight: &[f32], epsilon: f32) -> Vec<f32> {
    if x.is_empty() || weight.len() != x.len() {
        return Vec::new();
    }
    let mut sum = 0.0_f32;
    for value in x {
        sum = value.mul_add(*value, sum);
    }
    let count = f32::from(u16::try_from(x.len()).unwrap_or(u16::MAX));
    let scale = (sum / count + epsilon).sqrt().recip();
    x.iter()
        .zip(weight.iter())
        .map(|(value, gain)| value * scale * gain)
        .collect()
}

pub fn softmax(values: &mut [f32]) {
    let Some(largest) = values
        .iter()
        .copied()
        .fold(None::<f32>, |best, value| match best {
            Some(best) if best >= value => Some(best),
            _ => Some(value),
        })
    else {
        return;
    };
    let mut total = 0.0_f32;
    for value in values.iter_mut() {
        *value = (*value - largest).exp();
        total += *value;
    }
    if total == 0.0 {
        return;
    }
    for value in values.iter_mut() {
        *value /= total;
    }
}

#[must_use]
pub fn layer_norm(values: &[f32], weights: &[f32], biases: &[f32], epsilon: f32) -> Vec<f32> {
    let count = f32::from(u16::try_from(values.len()).unwrap_or(1)).max(1.0);
    let mut total = 0.0_f32;
    for value in values {
        total += value;
    }
    let mean = total / count;
    let mut squares = 0.0_f32;
    for value in values {
        let spread = value - mean;
        squares = spread.mul_add(spread, squares);
    }
    let variance = squares / count;
    let scale = (variance + epsilon).sqrt().recip();
    values
        .iter()
        .enumerate()
        .map(|(index, value)| {
            (value - mean) * scale * weights.get(index).copied().unwrap_or(1.0)
                + biases.get(index).copied().unwrap_or(0.0)
        })
        .collect()
}

#[must_use]
pub fn silu(x: f32) -> f32 {
    x / (1.0 + (-x).exp())
}

#[must_use]
pub fn gelu(x: f32) -> f32 {
    const ROOT_TWO_OVER_PI: f32 = 0.797_884_6;
    const FUDGE: f32 = 0.044_715;
    0.5 * x * (1.0 + (ROOT_TWO_OVER_PI * x.mul_add(FUDGE * x * x, x)).tanh())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Activation {
    Silu,
    Gelu,
}

#[must_use]
pub fn gated(gate: &[f32], up: &[f32], activation: Activation) -> Vec<f32> {
    let apply = match activation {
        Activation::Silu => silu,
        Activation::Gelu => gelu,
    };
    gate.iter()
        .zip(up.iter())
        .map(|(gate, up)| apply(*gate) * up)
        .collect()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rotation {
    Interleaved,
    Halved,
}

pub fn rope(vector: &mut [f32], position: usize, theta: f32, rotation: Rotation) {
    let head_dimension = vector.len();
    if head_dimension < 2 {
        return;
    }
    let position = f32::from(u16::try_from(position).unwrap_or(u16::MAX));
    let pairs = head_dimension.checked_div(2).unwrap_or(0);
    for pair in 0..pairs {
        let exponent = f32::from(u16::try_from(pair).unwrap_or(0)) * 2.0
            / f32::from(u16::try_from(head_dimension).unwrap_or(2));
        let frequency = theta.powf(-exponent);
        let angle = position * frequency;
        let (sine, cosine) = angle.sin_cos();
        let (at, and) = match rotation {
            Rotation::Interleaved => (
                pair.saturating_mul(2),
                pair.saturating_mul(2).saturating_add(1),
            ),
            Rotation::Halved => (pair, pair.saturating_add(pairs)),
        };
        let (Some(first), Some(second)) = (vector.get(at).copied(), vector.get(and).copied())
        else {
            continue;
        };
        if let Some(slot) = vector.get_mut(at) {
            *slot = first.mul_add(cosine, -(second * sine));
        }
        if let Some(slot) = vector.get_mut(and) {
            *slot = first.mul_add(sine, second * cosine);
        }
    }
}

#[must_use]
pub fn add(left: &[f32], right: &[f32]) -> Vec<f32> {
    left.iter()
        .zip(right.iter())
        .map(|(left, right)| left + right)
        .collect()
}

#[must_use]
pub fn dot(left: &[f32], right: &[f32]) -> f32 {
    let mut total = 0.0_f32;
    for (left, right) in left.iter().zip(right.iter()) {
        total = left.mul_add(*right, total);
    }
    total
}

#[must_use]
pub fn argmax(values: &[f32]) -> Option<usize> {
    let mut best: Option<(usize, f32)> = None;
    for (index, value) in values.iter().enumerate() {
        match best {
            Some((_, so_far)) if so_far >= *value => {}
            _ => best = Some((index, *value)),
        }
    }
    best.map(|(index, _)| index)
}

#[cfg(test)]
mod tests;
