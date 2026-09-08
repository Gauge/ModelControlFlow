use crate::ops;

#[test]
fn layer_norm_subtracts_the_mean() {
    let normalized = ops::layer_norm(&[1.0, 3.0], &[1.0, 1.0], &[0.0, 0.0], 0.0);
    assert!((normalized[0] - -1.0).abs() < 1e-6, "{normalized:?}");
    assert!((normalized[1] - 1.0).abs() < 1e-6, "{normalized:?}");
    let flat = ops::layer_norm(&[5.0, 5.0], &[1.0, 1.0], &[0.25, -0.25], 1e-12);
    assert!(
        flat[0].abs() - 0.25 < 1e-4 && flat[1].abs() - 0.25 < 1e-4,
        "{flat:?}"
    );
}

#[test]
fn layer_norm_applies_weight_and_bias_per_lane() {
    let normalized = ops::layer_norm(&[1.0, 3.0], &[2.0, 0.5], &[10.0, -10.0], 0.0);
    assert!((normalized[0] - 8.0).abs() < 1e-5, "{normalized:?}");
    assert!((normalized[1] - -9.5).abs() < 1e-5, "{normalized:?}");
}
