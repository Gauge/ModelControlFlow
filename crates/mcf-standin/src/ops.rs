//! The ordinary operations of a transformer, written to be read.
//!
//! Every function here is the definition. No blocking, no fusion, no
//! accelerator path, and no attempt to be clever about memory — F8 measured
//! what that costs against a specialist's kernel and D32 settled that it is the
//! right trade for this code, whose job is to be *checkable*.
//!
//! **The one exception is threads, and it changes no arithmetic.**
//! [`matmul_vec_across`] partitions a product's *rows*; every row is still the
//! same sum in the same order, computed by the same function (`row_of`), so
//! the answer is the same bytes however many threads run it (B-366,
//! [`crate::threads`]). That is a division of labour rather than an
//! optimization of the definition, which is why it is allowed here.
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

use crate::threads::Threads;

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
    matmul_vec_across(matrix, vector, rows, columns, Threads::definition())
}

/// The same product, with the rows partitioned across `threads` (B-366).
///
/// **The answer is the same bytes at one thread and at many, and that is
/// structural.** Every output element is one row's dot product, computed by
/// `row_of` — the single function the serial path and every partition both
/// call — so a thread count decides *who* computes a row and can never reach
/// the order the row is summed in. Floating-point addition is not associative,
/// which is what makes that distinction the whole of the property (§3.12, D19).
///
/// `tests/threads_do_not_change_the_answer.rs` asserts the consequence over
/// generated inputs, and `checks/tests/a_reduction_is_never_split.rs` holds the
/// shape that produces it.
///
/// Returns an empty vector on a shape disagreement, exactly as [`matmul_vec`].
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

/// One row's dot product with the vector, summed low index to high.
///
/// The order of this sum is the model's answer. It is written once and called
/// from both the serial and the partitioned path so that no future edit can
/// give the two different arithmetic (B-366).
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

/// Classic layer normalization: mean subtracted, variance divided, then a
/// learned scale and shift.
///
/// The other normalization in this crate, beside [`rms_norm`] — and they are
/// not interchangeable. The bert family subtracts the mean and carries a bias;
/// the llama line does neither. An engine that used one where a model was
/// trained with the other would be wrong everywhere by an amount that never
/// looks like an error (F24's register of quiet differences).
#[must_use]
pub fn layer_norm(values: &[f32], weights: &[f32], biases: &[f32], epsilon: f32) -> Vec<f32> {
    let count = f32::from(u16::try_from(values.len()).unwrap_or(1)).max(1.0);
    // Accumulated longhand, the way `rms_norm` and `softmax` are: these are
    // the model's own arithmetic over one vector, not a summary of trials —
    // what B56 forbids is a statistic that discards samples, and the shipped
    // suite checks for that by its usual spellings.
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

/// The `SiLU` (swish) activation: `x · sigmoid(x)`.
#[must_use]
pub fn silu(x: f32) -> f32 {
    x / (1.0 + (-x).exp())
}

/// The `GELU` activation, in the `tanh` approximation every one of these models
/// was trained with.
///
/// The exact form uses the Gaussian error function; the approximation below is
/// what the reference implementations compute, and computing the *exact* one
/// here would make MCF disagree with them by a small amount everywhere — which
/// is the kind of difference that is invisible in the output and fatal to a
/// comparison against an oracle (A19).
#[must_use]
pub fn gelu(x: f32) -> f32 {
    const ROOT_TWO_OVER_PI: f32 = 0.797_884_6;
    const FUDGE: f32 = 0.044_715;
    0.5 * x * (1.0 + (ROOT_TWO_OVER_PI * x.mul_add(FUDGE * x * x, x)).tanh())
}

/// Which activation a gated feed-forward block uses.
///
/// Not observable from the file: the tensors of a `SiLU`-gated block and a
/// `GELU`-gated one are the same tensors of the same shapes. It is a property
/// of the architecture, so it lives in the one table that holds those (B28,
/// DEC-053).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Activation {
    /// `x · sigmoid(x)`, what the llama family was trained with.
    Silu,
    /// The Gaussian error linear unit, what the gemma family was trained with.
    Gelu,
}

/// The gated feed-forward activation: `activation(gate) · up`, elementwise.
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

/// Which two components of a head a rotary embedding turns together.
///
/// **This is not a detail, and no file states it.** GGUF carries the base and
/// the head width but never says which pairing the model was trained with, so
/// it is a property of the architecture and MCF keeps a table of it. A model
/// run under the wrong one produces fluent nonsense that gets worse with
/// distance — which is exactly the failure a second implementation exists to
/// catch (A19), and exactly what MCF produced for Qwen3 before this existed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rotation {
    /// Component `2i` with component `2i+1`: what llama was trained with, and
    /// what llama.cpp calls `NORM`.
    Interleaved,
    /// Component `i` with component `i + head_dim/2`: what the GPT-NeoX line
    /// and everything descended from it was trained with, Qwen included, and
    /// what llama.cpp calls `NEOX`.
    Halved,
}

/// Rotary position embedding, applied in place to one head's vector.
///
/// Pair `i` is rotated by `position · theta^(-2i/head_dim)` under either
/// convention; the convention decides only *which two components* are the pair.
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
