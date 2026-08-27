//! What the pieces of the embedding path have to get right on values small
//! enough to check by hand.

use crate::ops;

/// Layer normalization subtracts the mean, which RMS normalization does not —
/// the difference that makes them non-interchangeable.
#[test]
fn layer_norm_subtracts_the_mean() {
    let normalized = ops::layer_norm(&[1.0, 3.0], &[1.0, 1.0], &[0.0, 0.0], 0.0);
    // Mean 2, spread ±1, variance 1: the result is exactly ±1.
    assert!((normalized[0] - -1.0).abs() < 1e-6, "{normalized:?}");
    assert!((normalized[1] - 1.0).abs() < 1e-6, "{normalized:?}");
    // A constant vector normalizes to its bias alone.
    let flat = ops::layer_norm(&[5.0, 5.0], &[1.0, 1.0], &[0.25, -0.25], 1e-12);
    assert!(
        flat[0].abs() - 0.25 < 1e-4 && flat[1].abs() - 0.25 < 1e-4,
        "{flat:?}"
    );
}

/// The weight scales and the bias shifts, per lane.
#[test]
fn layer_norm_applies_weight_and_bias_per_lane() {
    let normalized = ops::layer_norm(&[1.0, 3.0], &[2.0, 0.5], &[10.0, -10.0], 0.0);
    assert!((normalized[0] - 8.0).abs() < 1e-5, "{normalized:?}");
    assert!((normalized[1] - -9.5).abs() < 1e-5, "{normalized:?}");
}
