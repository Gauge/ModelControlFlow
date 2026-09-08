#![allow(clippy::float_cmp)]

use super::{
    Activation, Rotation, add, argmax, dot, gated, gelu, matmul_vec, rms_norm, rope, silu, softmax,
};

const CLOSE: f32 = 1e-6;

fn assert_close(produced: f32, expected: f32, what: &str) {
    assert!(
        (produced - expected).abs() < CLOSE,
        "{what}: produced {produced}, expected {expected}"
    );
}

#[test]
fn a_matrix_times_a_vector_is_the_dot_product_of_each_row() {
    let matrix = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0];
    let vector = [1.0, 0.5, -1.0];
    assert_eq!(matmul_vec(&matrix, &vector, 2, 3), vec![-1.0, 0.5]);
}

#[test]
fn a_shape_that_does_not_match_produces_nothing() {
    let matrix = [1.0, 2.0, 3.0, 4.0];
    assert!(matmul_vec(&matrix, &[1.0, 1.0], 3, 2).is_empty());
    assert!(matmul_vec(&matrix, &[1.0, 1.0, 1.0], 2, 2).is_empty());
}

#[test]
fn rms_norm_divides_by_the_root_mean_square() {
    let x = [1.0, -1.0, 1.0, -1.0];
    let weight = [2.0, 3.0, 4.0, 5.0];
    let out = rms_norm(&x, &weight, 0.0);
    assert_eq!(out, vec![2.0, -3.0, 4.0, -5.0]);

    let out = rms_norm(&[2.0, 2.0], &[1.0, 1.0], 0.0);
    assert_eq!(out, vec![1.0, 1.0]);
}

#[test]
fn the_epsilon_is_inside_the_root() {
    let out = rms_norm(&[0.0, 0.0], &[1.0, 1.0], 4.0);
    assert_eq!(out, vec![0.0, 0.0]);

    let out = rms_norm(&[3.0, 3.0], &[1.0, 1.0], 7.0);
    assert_eq!(out, vec![0.75, 0.75]);
}

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

#[test]
fn softmax_survives_a_logit_that_would_overflow() {
    let mut values = [800.0, 0.0, -800.0];
    softmax(&mut values);
    assert_close(values.iter().sum::<f32>(), 1.0, "the total");
    assert_close(values[0], 1.0, "the largest takes essentially all of it");
    assert!(values.iter().all(|value| value.is_finite()));
}

#[test]
fn equal_logits_share_equally() {
    let mut values = [5.0, 5.0];
    softmax(&mut values);
    assert_close(values[0], 0.5, "half");
    assert_close(values[1], 0.5, "half");
}

#[test]
fn silu_is_x_times_the_logistic() {
    assert_eq!(silu(0.0), 0.0);
    assert_close(silu(1.0), 0.731_058_6, "silu(1)");
    assert_close(silu(-20.0), 0.0, "silu(-20)");
    assert_close(silu(20.0), 20.0, "silu(20)");
}

#[test]
fn swiglu_gates_one_vector_by_the_other() {
    let out = gated(&[0.0, 1.0], &[3.0, 2.0], Activation::Silu);
    assert_eq!(out.first().copied(), Some(0.0));
    assert_close(
        out.get(1).copied().unwrap_or(0.0),
        0.731_058_6 * 2.0,
        "gated",
    );
}

#[test]
fn rope_at_position_zero_changes_nothing() {
    let mut vector = [1.0, 2.0, 3.0, 4.0];
    rope(&mut vector, 0, 10_000.0, Rotation::Interleaved);
    assert_eq!(vector, [1.0, 2.0, 3.0, 4.0]);
}

#[test]
fn the_first_pair_rotates_by_the_position() {
    let mut vector = [1.0, 0.0];
    rope(&mut vector, 1, 10_000.0, Rotation::Interleaved);
    assert_close(vector[0], 1.0_f32.cos(), "the real part");
    assert_close(vector[1], 1.0_f32.sin(), "the imaginary part");
}

#[test]
fn rope_pairs_adjacent_dimensions() {
    let mut vector = [1.0, 0.0, 1.0, 0.0];
    rope(&mut vector, 1, 10_000.0, Rotation::Interleaved);
    assert_close(vector[1], 1.0_f32.sin(), "pair 0's sine");
    let second_frequency = 10_000.0_f32.powf(-0.5);
    assert_close(vector[3], second_frequency.sin(), "pair 1's sine");
    assert!(
        (vector[1] - vector[3]).abs() > 0.1,
        "both pairs rotated by the same angle, so the frequencies are not being applied"
    );
}

#[test]
fn rotation_preserves_length() {
    for position in [1, 7, 64, 1000] {
        let mut vector = [0.6, 0.8, -1.5, 2.0];
        let before: f32 = vector.iter().map(|value| value * value).sum();
        rope(&mut vector, position, 10_000.0, Rotation::Interleaved);
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

#[test]
fn argmax_breaks_ties_toward_the_lowest_index() {
    assert_eq!(argmax(&[1.0, 3.0, 2.0]), Some(1));
    assert_eq!(argmax(&[5.0, 5.0, 1.0]), Some(0));
    assert_eq!(argmax(&[]), None);
    assert_eq!(argmax(&[-3.0, -1.0, -2.0]), Some(1));
}

#[test]
fn the_two_rotations_are_not_the_same_rotation() {
    let mut interleaved = vec![0.0_f32, 0.0, 1.0, 0.0];
    let mut halved = interleaved.clone();
    rope(&mut interleaved, 1, 10_000.0, Rotation::Interleaved);
    rope(&mut halved, 1, 10_000.0, Rotation::Halved);
    assert_ne!(
        interleaved, halved,
        "the two conventions produced the same vector, which would make the distinction unreal"
    );
    assert_eq!(interleaved.first().copied(), Some(0.0));
    let moved = halved.first().copied().unwrap_or(0.0);
    assert!(
        (moved - -1.0_f32.sin()).abs() < 1e-6,
        "component 0 should have picked up -sin(1), got {moved}"
    );
}

#[test]
fn a_rotation_keeps_the_length_of_the_pair() {
    for rotation in [Rotation::Interleaved, Rotation::Halved] {
        let before = vec![0.3_f32, -1.2, 0.7, 2.0, -0.5, 1.1, 0.9, -0.2];
        let mut after = before.clone();
        rope(&mut after, 7, 10_000.0, rotation);
        let length = |vector: &[f32]| vector.iter().map(|value| value * value).sum::<f32>();
        assert!(
            (length(&before) - length(&after)).abs() < 1e-4,
            "{rotation:?} changed the length of the vector"
        );
    }
}

#[test]
fn the_two_activations_are_not_the_same_function() {
    assert!((gelu(0.0) - 0.0).abs() < 1e-7);
    assert!((silu(0.0) - 0.0).abs() < 1e-7);
    assert!(
        (gelu(1.0) - silu(1.0)).abs() > 0.05,
        "gelu(1) = {}, silu(1) = {} — too close to tell apart",
        gelu(1.0),
        silu(1.0)
    );
    assert!(
        (gelu(1.0) - 0.841_192).abs() < 1e-4,
        "gelu(1) = {}",
        gelu(1.0)
    );
    assert!(
        (gelu(-1.0) - -0.158_808).abs() < 1e-4,
        "gelu(-1) = {}",
        gelu(-1.0)
    );
    assert!((gelu(10.0) - 10.0).abs() < 1e-3);
    assert!(gelu(-10.0).abs() < 1e-3);
}

#[test]
fn the_gate_decides_which_activation_is_applied() {
    let silu_out = gated(&[1.0, -1.0], &[2.0, 2.0], Activation::Silu);
    let gelu_out = gated(&[1.0, -1.0], &[2.0, 2.0], Activation::Gelu);
    assert_ne!(silu_out, gelu_out, "the two gates produced the same vector");
    let doubled = gated(&[1.0, -1.0], &[4.0, 4.0], Activation::Gelu);
    for (one, two) in gelu_out.iter().zip(doubled.iter()) {
        assert!((one * 2.0 - two).abs() < 1e-5);
    }
}
