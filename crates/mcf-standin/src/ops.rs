//! The ordinary operations of a transformer, written to be read.
//!
//! Every function here is the definition. No blocking, no fusion, no threading,
//! no accelerator path, and no attempt to be clever about memory — F8 measured
//! what that costs against a specialist's kernel and D32 settled that it is the
//! right trade for this code, whose job is to be *checkable*.
//!
//! **What "checkable" means in practice.** Each operation has a closed form a
//! test can compute by hand, and the tests do exactly that rather than
//! comparing against a second implementation of the same idea (A19). Where an
//! operation has a convention that could be got wrong silently — the order `RoPE`
//! pairs its dimensions, whether a norm divides before or after scaling — the
//! convention is stated here and asserted there, because a transposed
//! convention produces output that is the right shape and quietly wrong.
//!
//! **Shapes are arguments, not inferred.** A slice is a slice; a matrix is a
//! slice plus the two numbers that say how to read it. Passing them explicitly
//! is what lets a caller be wrong in a way that fails immediately instead of
//! producing a rectangle of nonsense.
//!
//! **Nothing here allocates in a loop it did not have to.** That is legibility
//! rather than optimization: a function that returns a `Vec` is easier to read
//! than one that writes into an out-parameter, and this crate is allowed to be
//! slow but not to be confusing.

/// A row-major matrix multiplied by a vector: `out[row] = Σ matrix[row][col] ·
/// vector[col]`.
///
/// GGUF stores a weight matrix with its fastest-varying dimension first, which
/// for a projection is the *input* dimension — so `matrix` is `rows × columns`
/// laid out row by row, and this is the ordinary dot product of each row with
/// the vector. Getting that transposed is the classic way to produce a model
/// that runs and speaks nonsense.
///
/// Returns an empty vector when the shapes disagree with the data, rather than
/// reading past the end: a caller that got the shape wrong gets nothing, not a
/// rectangle assembled from whatever followed.
#[must_use]
pub fn matmul_vec(matrix: &[f32], vector: &[f32], rows: usize, columns: usize) -> Vec<f32> {
    if vector.len() != columns || matrix.len() != rows.saturating_mul(columns) {
        return Vec::new();
    }
    let mut out = Vec::with_capacity(rows);
    for row in 0..rows {
        let start = row.saturating_mul(columns);
        let slice = matrix.get(start..start + columns).unwrap_or(&[]);
        let mut total = 0.0_f32;
        for (weight, value) in slice.iter().zip(vector.iter()) {
            total = weight.mul_add(*value, total);
        }
        out.push(total);
    }
    out
}

/// Root-mean-square normalization, as the llama family defines it:
/// `out[i] = x[i] / sqrt(mean(x²) + eps) · weight[i]`.
///
/// The epsilon is inside the square root, which is the convention the models
/// this reads were trained under. Outside it, the result differs by a little at
/// every layer and by a lot after thirty of them — the sort of divergence that
/// makes a stand-in disagree with the engine it exists to check.
#[must_use]
pub fn rms_norm(x: &[f32], weight: &[f32], epsilon: f32) -> Vec<f32> {
    if x.is_empty() || weight.len() != x.len() {
        return Vec::new();
    }
    let mut sum = 0.0_f32;
    for value in x {
        sum = value.mul_add(*value, sum);
    }
    // The length is bounded by a model's embedding width, so this conversion is
    // exact for anything that exists.
    let count = f32::from(u16::try_from(x.len()).unwrap_or(u16::MAX));
    let scale = (sum / count + epsilon).sqrt().recip();
    x.iter()
        .zip(weight.iter())
        .map(|(value, gain)| value * scale * gain)
        .collect()
}

/// Softmax in place, shifted by the maximum so that no exponential overflows.
///
/// The shift is not an optimization: `exp(800)` is infinity, and a single
/// infinity in an attention row turns every other weight into a zero and the
/// row into a one-hot. Subtracting the maximum is exactly neutral
/// mathematically and is what keeps that from happening.
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

/// The `SiLU` (swish) activation: `x · sigmoid(x)`.
#[must_use]
pub fn silu(x: f32) -> f32 {
    x / (1.0 + (-x).exp())
}

/// The gated feed-forward activation the llama family uses:
/// `silu(gate) · up`, elementwise.
#[must_use]
pub fn swiglu(gate: &[f32], up: &[f32]) -> Vec<f32> {
    gate.iter()
        .zip(up.iter())
        .map(|(gate, up)| silu(*gate) * up)
        .collect()
}

/// Rotary position embedding, applied in place to one head's vector.
///
/// The convention here is the one GGUF's llama models are written for: the
/// vector is treated as `head_dim / 2` **adjacent pairs**, and pair `i` is
/// rotated by `position · theta^(-2i/head_dim)`. The other convention in the
/// wild splits the vector in half and pairs `i` with `i + head_dim/2`; a model
/// run under the wrong one produces fluent nonsense that gets worse with
/// distance, which is precisely the failure a second implementation exists to
/// catch (A19).
pub fn rope(vector: &mut [f32], position: usize, theta: f32) {
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
        let at = pair.saturating_mul(2);
        let (Some(first), Some(second)) = (
            vector.get(at).copied(),
            vector.get(at.saturating_add(1)).copied(),
        ) else {
            continue;
        };
        if let Some(slot) = vector.get_mut(at) {
            *slot = first.mul_add(cosine, -(second * sine));
        }
        if let Some(slot) = vector.get_mut(at.saturating_add(1)) {
            *slot = first.mul_add(sine, second * cosine);
        }
    }
}

/// Elementwise addition, which is what a residual connection is.
#[must_use]
pub fn add(left: &[f32], right: &[f32]) -> Vec<f32> {
    left.iter()
        .zip(right.iter())
        .map(|(left, right)| left + right)
        .collect()
}

/// The dot product of two vectors, which attention is made of.
#[must_use]
pub fn dot(left: &[f32], right: &[f32]) -> f32 {
    let mut total = 0.0_f32;
    for (left, right) in left.iter().zip(right.iter()) {
        total = left.mul_add(*right, total);
    }
    total
}

/// Where the largest value is, and the first of them when several tie.
///
/// Ties are broken toward the lowest index deliberately: greedy decoding must
/// be a function of the logits alone, and "whichever the iterator saw last"
/// would make it a function of the iteration order as well.
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
