//! Each operation against a value computed by hand.
//!
//! A19: an independently known answer, not a second implementation of the same
//! idea. Every case below is small enough to work out on paper, which is the
//! only way a test of arithmetic says anything the arithmetic did not.

// The comparisons here are of exact dyadic values, or of values against a
// stated tolerance where a transcendental function is involved. `float_cmp`
// is right in general and wrong for the first kind; the second kind states its
// tolerance explicitly.
#![allow(clippy::float_cmp)]

use super::{add, argmax, dot, matmul_vec, rms_norm, rope, silu, softmax, swiglu};

/// How close two floats must be where an exponential or a sine is involved.
///
/// Stated once, and generously: these tests are about whether the *formula* is
/// right, and a tolerance tight enough to catch a last-bit difference would be
/// a test of the standard library's transcendental functions instead.
const CLOSE: f32 = 1e-6;

fn assert_close(produced: f32, expected: f32, what: &str) {
    assert!(
        (produced - expected).abs() < CLOSE,
        "{what}: produced {produced}, expected {expected}"
    );
}

/// A 2×3 matrix by a 3-vector, worked out by hand.
#[test]
fn a_matrix_times_a_vector_is_the_dot_product_of_each_row() {
    let matrix = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0];
    let vector = [1.0, 0.5, -1.0];
    // Row 0: 1·1 + 2·0.5 + 3·(-1) = -1. Row 1: 4 + 2.5 - 6 = 0.5.
    assert_eq!(matmul_vec(&matrix, &vector, 2, 3), vec![-1.0, 0.5]);
}

/// Shapes that disagree with the data produce nothing rather than a rectangle
/// assembled from whatever followed in memory.
#[test]
fn a_shape_that_does_not_match_produces_nothing() {
    let matrix = [1.0, 2.0, 3.0, 4.0];
    assert!(matmul_vec(&matrix, &[1.0, 1.0], 3, 2).is_empty());
    assert!(matmul_vec(&matrix, &[1.0, 1.0, 1.0], 2, 2).is_empty());
}

/// RMS normalization, with the epsilon inside the root, on a vector whose
/// mean square is exactly one.
#[test]
fn rms_norm_divides_by_the_root_mean_square() {
    // Mean of squares is (1 + 1 + 1 + 1)/4 = 1, so with no epsilon the scale is
    // exactly 1 and the output is the weights.
    let x = [1.0, -1.0, 1.0, -1.0];
    let weight = [2.0, 3.0, 4.0, 5.0];
    let out = rms_norm(&x, &weight, 0.0);
    assert_eq!(out, vec![2.0, -3.0, 4.0, -5.0]);

    // And a vector of twos has mean square 4, so the scale is 1/2.
    let out = rms_norm(&[2.0, 2.0], &[1.0, 1.0], 0.0);
    assert_eq!(out, vec![1.0, 1.0]);
}

/// The epsilon is inside the square root. A version that added it afterwards
/// would differ by a little at every layer and by a lot after thirty.
#[test]
fn the_epsilon_is_inside_the_root() {
    let out = rms_norm(&[0.0, 0.0], &[1.0, 1.0], 4.0);
    // sqrt(0 + 4) = 2, so the scale is 1/2 — and 0/2 is 0 either way, so use a
    // vector that is not zero.
    assert_eq!(out, vec![0.0, 0.0]);

    let out = rms_norm(&[3.0, 3.0], &[1.0, 1.0], 7.0);
    // mean square 9, plus 7 is 16, root 4, so each 3 becomes 0.75.
    assert_eq!(out, vec![0.75, 0.75]);
}

/// Softmax sums to one and is invariant to a constant shift, which is the
/// property that lets the maximum be subtracted for safety.
#[test]
fn softmax_sums_to_one_and_ignores_a_shift() {
    let mut values = [1.0, 2.0, 3.0];
    softmax(&mut values);
    assert_close(values.iter().sum::<f32>(), 1.0, "the total");
    assert!(values[2] > values[1] && values[1] > values[0]);

    let mut shifted = [101.0, 102.0, 103.0];
    softmax(&mut shifted);
    for (index, (one, other)) in values.iter().zip(shifted.iter()).enumerate() {
        assert_close(
            *one,
            *other,
            &format!("element {index} after a shift of 100"),
        );
    }
}

/// An enormous logit does not become an infinity that flattens the row.
#[test]
fn softmax_survives_a_logit_that_would_overflow() {
    let mut values = [800.0, 0.0, -800.0];
    softmax(&mut values);
    assert_close(values.iter().sum::<f32>(), 1.0, "the total");
    assert_close(values[0], 1.0, "the largest takes essentially all of it");
    assert!(values.iter().all(|value| value.is_finite()));
}

/// Two equal logits share the weight equally, which a shift-free implementation
/// gets right and an overflowing one does not.
#[test]
fn equal_logits_share_equally() {
    let mut values = [5.0, 5.0];
    softmax(&mut values);
    assert_close(values[0], 0.5, "half");
    assert_close(values[1], 0.5, "half");
}

/// `SiLU` at values whose answers are known.
#[test]
fn silu_is_x_times_the_logistic() {
    assert_eq!(silu(0.0), 0.0);
    // silu(1) = 1/(1+e^-1) = 0.7310586
    assert_close(silu(1.0), 0.731_058_6, "silu(1)");
    // Large negative values go to zero, large positive to themselves.
    assert_close(silu(-20.0), 0.0, "silu(-20)");
    assert_close(silu(20.0), 20.0, "silu(20)");
}

#[test]
fn swiglu_gates_one_vector_by_the_other() {
    let out = swiglu(&[0.0, 1.0], &[3.0, 2.0]);
    assert_eq!(out.first().copied(), Some(0.0));
    assert_close(
        out.get(1).copied().unwrap_or(0.0),
        0.731_058_6 * 2.0,
        "gated",
    );
}

/// `RoPE` at position zero is the identity, which is the first thing a wrong
/// convention breaks.
#[test]
fn rope_at_position_zero_changes_nothing() {
    let mut vector = [1.0, 2.0, 3.0, 4.0];
    rope(&mut vector, 0, 10_000.0);
    assert_eq!(vector, [1.0, 2.0, 3.0, 4.0]);
}

/// The first pair rotates by exactly the position in radians, because its
/// frequency is theta^0 = 1. A quarter turn takes (1, 0) to (0, 1).
#[test]
fn the_first_pair_rotates_by_the_position() {
    let mut vector = [1.0, 0.0];
    // A rotation of π/2 radians: position 1 with theta such that the frequency
    // is π/2 is awkward, so rotate by 1 radian and check against cos and sin.
    rope(&mut vector, 1, 10_000.0);
    assert_close(vector[0], 1.0_f32.cos(), "the real part");
    assert_close(vector[1], 1.0_f32.sin(), "the imaginary part");
}

/// Pairs are *adjacent*, not split across the halves of the vector. A model run
/// under the other convention produces fluent nonsense that gets worse with
/// distance, so the convention is asserted rather than assumed.
#[test]
fn rope_pairs_adjacent_dimensions() {
    let mut vector = [1.0, 0.0, 1.0, 0.0];
    rope(&mut vector, 1, 10_000.0);
    // Pair 0 has frequency 1 and pair 1 has frequency theta^-1, so the two
    // pairs rotate by different angles — and the second element of each pair
    // becomes the sine of its own angle.
    assert_close(vector[1], 1.0_f32.sin(), "pair 0's sine");
    let second_frequency = 10_000.0_f32.powf(-0.5);
    assert_close(vector[3], second_frequency.sin(), "pair 1's sine");
    assert!(
        (vector[1] - vector[3]).abs() > 0.1,
        "both pairs rotated by the same angle, so the frequencies are not being applied"
    );
}

/// Rotation preserves length, which is the invariant that catches a sign error
/// in either term.
#[test]
fn rotation_preserves_length() {
    for position in [1, 7, 64, 1000] {
        let mut vector = [0.6, 0.8, -1.5, 2.0];
        let before: f32 = vector.iter().map(|value| value * value).sum();
        rope(&mut vector, position, 10_000.0);
        let after: f32 = vector.iter().map(|value| value * value).sum();
        assert!(
            (before - after).abs() < 1e-4,
            "position {position}: length went from {before} to {after}"
        );
    }
}

#[test]
fn addition_and_dot_product_are_what_they_say() {
    assert_eq!(add(&[1.0, 2.0], &[0.5, -2.0]), vec![1.5, 0.0]);
    assert_eq!(dot(&[1.0, 2.0, 3.0], &[4.0, 5.0, 6.0]), 32.0);
}

/// Greedy decoding is a function of the logits alone, so a tie goes to the
/// lowest index rather than to whichever the iterator saw last.
#[test]
fn argmax_breaks_ties_toward_the_lowest_index() {
    assert_eq!(argmax(&[1.0, 3.0, 2.0]), Some(1));
    assert_eq!(argmax(&[5.0, 5.0, 1.0]), Some(0));
    assert_eq!(argmax(&[]), None);
    assert_eq!(argmax(&[-3.0, -1.0, -2.0]), Some(1));
}
