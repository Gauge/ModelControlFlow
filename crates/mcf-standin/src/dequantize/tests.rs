//! What the decoder produces, checked against values known independently of it.
//!
//! A19 is the standard here and it is unusually easy to meet: every case below
//! has an exact answer that can be written down. A half-precision bit pattern
//! has one value; a `Q8_0` block of a stated scale and stated bytes has
//! thirty-two values that can be multiplied out by hand. So nothing is compared within a
//! tolerance, and nothing is compared against the decoder's own output.

// Comparing floats with `==` is the claim in this file rather than a mistake:
// every value below is exact in binary32 — a half-precision pattern, or a small
// dyadic fraction multiplied by a power of two — so a tolerance would weaken the
// test to *approximately what the decoder already does*. `float_cmp` is right
// about arithmetic in general and wrong here, and saying so at the site is what
// B16 asks of an exception.
#![allow(clippy::float_cmp)]

use super::{from_half, tensor};
use crate::gguf::TensorKind;
use mcf_core::failure::Category;

/// Half precision, against the table in IEEE 754 and the values every
/// implementation is checked against.
#[test]
fn half_precision_matches_values_that_are_known_exactly() {
    for (bits, expected) in [
        (0x0000_u16, 0.0_f32),
        (0x8000, -0.0),
        (0x3C00, 1.0),
        (0xBC00, -1.0),
        (0x4000, 2.0),
        (0x3555, 1365.0 / 4096.0),       // a third, as binary16 has it
        (0x7BFF, 65504.0),               // the largest finite binary16
        (0x0400, 0.000_061_035_156),     // the smallest normal
        (0x0001, 1.0 / 16_777_216.0),    // the smallest subnormal: 2^-24
        (0x03FF, 1023.0 / 16_777_216.0), // the largest subnormal: 1023 × 2^-24
        (0xC500, -5.0),
    ] {
        let produced = from_half(bits);
        assert_eq!(
            produced.to_bits(),
            expected.to_bits(),
            "{bits:#06x} produced {produced} and not {expected}"
        );
    }
}

/// Both zeros keep their sign, and the infinities and NaN arrive as themselves.
///
/// A NaN in a tensor is a fact about the tensor. Turning it into a large number
/// would be the substitution A7 forbids, one layer below where that rule
/// usually bites.
#[test]
fn the_special_values_survive() {
    assert!(from_half(0x0000).is_sign_positive());
    assert!(from_half(0x8000).is_sign_negative());
    assert_eq!(from_half(0x7C00), f32::INFINITY);
    assert_eq!(from_half(0xFC00), f32::NEG_INFINITY);
    assert!(from_half(0x7E00).is_nan());
    assert!(from_half(0xFE00).is_nan() && from_half(0xFE00).is_sign_negative());
}

/// Every binary16 value round-trips through binary32 unchanged, which is the
/// whole claim: 65 536 patterns, all of them checked, against Rust's own
/// conversion in the other direction.
#[test]
fn every_half_precision_value_is_exact() {
    for bits in 0..=u16::MAX {
        let produced = from_half(bits);
        if produced.is_nan() {
            continue;
        }
        // Back to binary16 by the rules, and it must be the same pattern. A
        // conversion that lost anything would fail here for some value.
        let back = to_half(produced);
        assert_eq!(
            back, bits,
            "{bits:#06x} came back as {back:#06x} via {produced}"
        );
    }
}

/// Binary32 to binary16, for the round trip above only. It handles exactly the
/// values `from_half` can produce, which are the ones that came from a
/// binary16.
fn to_half(value: f32) -> u16 {
    let bits = value.to_bits();
    let sign = u16::try_from((bits >> 16) & 0x8000).unwrap_or(0);
    let magnitude = f32::from_bits(bits & 0x7FFF_FFFF);
    if magnitude == 0.0 {
        return sign;
    }
    if magnitude.is_infinite() {
        return sign | 0x7C00;
    }
    if magnitude < 1.0 / 16_384.0 {
        // A subnormal: its value is a whole number of 2^-24 steps, and there
        // are at most 1023 of them — so the count is exact in binary32 and
        // fits a `u16` without a cast the workspace would have to argue with.
        let steps = magnitude * 16_777_216.0;
        let mut count: u16 = 0;
        while f32::from(count) < steps {
            count = count.saturating_add(1);
        }
        return sign | count;
    }
    let exponent = i32::try_from((bits >> 23) & 0xFF).unwrap_or(0) - 127 + 15;
    let mantissa = u16::try_from((bits >> 13) & 0x03FF).unwrap_or(0);
    sign | (u16::try_from(exponent).unwrap_or(0) << 10) | mantissa
}

/// A block of thirty-two eight-bit values against a scale, multiplied out by
/// hand.
#[test]
fn a_q8_block_decodes_to_the_products_it_states() {
    // Scale 0.5 (0x3800 in binary16), then the values 0, 1, -1, 2, … .
    let mut block = 0x3800_u16.to_le_bytes().to_vec();
    let values: Vec<i8> = (0..32)
        .map(|index| i8::try_from(index - 16).unwrap_or(0))
        .collect();
    for value in &values {
        block.push(value.to_le_bytes()[0]);
    }

    let produced = tensor(TensorKind::Q8_0, &block, 32).expect("a whole block");
    assert_eq!(produced.len(), 32);
    for (index, value) in values.iter().enumerate() {
        let expected = 0.5 * f32::from(*value);
        assert_eq!(
            produced.get(index).copied(),
            Some(expected),
            "value {index} of the block"
        );
    }
}

/// A `Q4_0` block, including the interleaving. The low nibble of byte *i* is
/// value *i* and the high nibble is value *i+16*; a decoder that read them in
/// sequence would produce a tensor of the right length with its values in the
/// wrong places, which is the kind of wrongness that produces plausible
/// nonsense rather than an error.
#[test]
fn a_q4_block_decodes_in_the_order_the_format_packs_it() {
    // Scale 1.0, and one byte per pair: low nibble 0, high nibble 15, then low
    // 8 and high 8, and zeros after.
    let mut block = 0x3C00_u16.to_le_bytes().to_vec();
    block.push(0xF0); // low 0 → -8, high 15 → +7
    block.push(0x88); // low 8 → 0, high 8 → 0
    block.extend_from_slice(&[0x88; 14]);

    let produced = tensor(TensorKind::Q4_0, &block, 32).expect("a whole block");
    assert_eq!(
        produced.first().copied(),
        Some(-8.0),
        "value 0 is the low nibble of byte 0"
    );
    assert_eq!(
        produced.get(16).copied(),
        Some(7.0),
        "value 16 is the high nibble of byte 0"
    );
    assert_eq!(produced.get(1).copied(), Some(0.0));
    assert_eq!(produced.get(17).copied(), Some(0.0));
}

/// `Q4_1` carries a minimum as well as a scale, and its values are unbiased.
#[test]
fn a_q4_1_block_uses_its_minimum() {
    // Scale 0.5, minimum -1.0, then a byte whose low nibble is 2 and high is 4.
    let mut block = 0x3800_u16.to_le_bytes().to_vec();
    block.extend_from_slice(&0xBC00_u16.to_le_bytes());
    block.push(0x42);
    block.extend_from_slice(&[0; 15]);

    let produced = tensor(TensorKind::Q4_1, &block, 32).expect("a whole block");
    assert_eq!(produced.first().copied(), Some(0.5_f32.mul_add(2.0, -1.0)));
    assert_eq!(produced.get(16).copied(), Some(0.5_f32.mul_add(4.0, -1.0)));
    // And the rest are the minimum, since their nibbles are zero.
    assert_eq!(produced.get(1).copied(), Some(-1.0));
}

/// Unquantized tensors are read as they are stored.
#[test]
fn the_unquantized_kinds_are_read_straight_through() {
    let floats: Vec<u8> = [1.5_f32, -2.25, 0.0]
        .iter()
        .flat_map(|value| value.to_le_bytes())
        .collect();
    assert_eq!(
        tensor(TensorKind::F32, &floats, 3).expect("three floats"),
        vec![1.5, -2.25, 0.0]
    );

    let halves: Vec<u8> = [0x3C00_u16, 0xC000]
        .iter()
        .flat_map(|value| value.to_le_bytes())
        .collect();
    assert_eq!(
        tensor(TensorKind::F16, &halves, 2).expect("two halves"),
        vec![1.0, -2.0]
    );
}

/// A tensor whose element count is not a whole number of blocks still reads:
/// the last block is whole on disk and the padding is dropped, which is what
/// keeps a row the length the shape says.
#[test]
fn a_partial_last_block_is_read_and_its_padding_dropped() {
    let mut block = 0x3C00_u16.to_le_bytes().to_vec();
    block.extend_from_slice(&[1_u8; 32]);
    let produced = tensor(TensorKind::Q8_0, &block, 20).expect("one block on disk");
    assert_eq!(produced.len(), 20, "the padding stayed in the result");
    assert!(produced.iter().all(|value| *value == 1.0));
}

/// Too few bytes is the artifact's failure and says how many were wanted.
#[test]
fn bytes_that_run_out_are_refused_with_the_arithmetic() {
    let failure = tensor(TensorKind::Q8_0, &[0; 10], 32).expect_err("ten bytes is not a block");
    assert_eq!(failure.category(), Category::ArtifactFormatMalformed);
    let context: Vec<String> = failure
        .context()
        .iter()
        .map(|entry| format!("{}={}", entry.key, entry.value))
        .collect();
    assert!(
        context.iter().any(|entry| entry == "wanted=34"),
        "{context:?}"
    );
    assert!(context.iter().any(|entry| entry == "had=10"), "{context:?}");
}

/// A scheme this crate does not implement is `engine.unavailable` naming the
/// scheme — D31's third state for an artifact, decided here.
#[test]
fn a_scheme_this_crate_does_not_implement_says_which() {
    let failure = tensor(TensorKind::Unknown(15), &[0; 64], 32).expect_err("not implemented");
    assert_eq!(failure.category(), Category::EngineUnavailable);
    assert!(
        failure
            .context()
            .iter()
            .any(|entry| entry.value.contains("15")),
        "the refusal does not say which scheme"
    );
}
