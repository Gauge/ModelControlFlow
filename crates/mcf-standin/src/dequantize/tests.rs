#![allow(clippy::float_cmp)]

use super::{from_half, tensor};
use crate::gguf::TensorKind;
use mcf_core::failure::Category;

#[test]
fn half_precision_matches_values_that_are_known_exactly() {
    for (bits, expected) in [
        (0x0000_u16, 0.0_f32),
        (0x8000, -0.0),
        (0x3C00, 1.0),
        (0xBC00, -1.0),
        (0x4000, 2.0),
        (0x3555, 1365.0 / 4096.0),
        (0x7BFF, 65504.0),
        (0x0400, 0.000_061_035_156),
        (0x0001, 1.0 / 16_777_216.0),
        (0x03FF, 1023.0 / 16_777_216.0),
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

#[test]
fn the_special_values_survive() {
    assert!(from_half(0x0000).is_sign_positive());
    assert!(from_half(0x8000).is_sign_negative());
    assert_eq!(from_half(0x7C00), f32::INFINITY);
    assert_eq!(from_half(0xFC00), f32::NEG_INFINITY);
    assert!(from_half(0x7E00).is_nan());
    assert!(from_half(0xFE00).is_nan() && from_half(0xFE00).is_sign_negative());
}

#[test]
fn every_half_precision_value_is_exact() {
    for bits in 0..=u16::MAX {
        let produced = from_half(bits);
        if produced.is_nan() {
            continue;
        }
        let back = to_half(produced);
        assert_eq!(
            back, bits,
            "{bits:#06x} came back as {back:#06x} via {produced}"
        );
    }
}

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

#[test]
fn a_q8_block_decodes_to_the_products_it_states() {
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

#[test]
fn a_q4_block_decodes_in_the_order_the_format_packs_it() {
    let mut block = 0x3C00_u16.to_le_bytes().to_vec();
    block.push(0xF0);
    block.push(0x88);
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

#[test]
fn a_q4_1_block_uses_its_minimum() {
    let mut block = 0x3800_u16.to_le_bytes().to_vec();
    block.extend_from_slice(&0xBC00_u16.to_le_bytes());
    block.push(0x42);
    block.extend_from_slice(&[0; 15]);

    let produced = tensor(TensorKind::Q4_1, &block, 32).expect("a whole block");
    assert_eq!(produced.first().copied(), Some(0.5_f32.mul_add(2.0, -1.0)));
    assert_eq!(produced.get(16).copied(), Some(0.5_f32.mul_add(4.0, -1.0)));
    assert_eq!(produced.get(1).copied(), Some(-1.0));
}

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

#[test]
fn a_partial_last_block_is_read_and_its_padding_dropped() {
    let mut block = 0x3C00_u16.to_le_bytes().to_vec();
    block.extend_from_slice(&[1_u8; 32]);
    let produced = tensor(TensorKind::Q8_0, &block, 20).expect("one block on disk");
    assert_eq!(produced.len(), 20, "the padding stayed in the result");
    assert!(produced.iter().all(|value| *value == 1.0));
}

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

#[test]
fn a_q4_k_super_block_decodes_to_the_arithmetic_its_format_states() {
    let mut raw = vec![0x00, 0x40, 0x00, 0x44];
    let mut packed = [0_u8; 12];
    for sub in 0..4_usize {
        packed[sub] = u8::try_from(sub + 1).unwrap_or(0);
        packed[sub + 4] = u8::try_from(sub).unwrap_or(0);
    }
    for sub in 4..8_usize {
        let scale = u8::try_from(sub + 1).unwrap_or(0);
        let minimum = u8::try_from(sub).unwrap_or(0);
        packed[sub + 4] = (scale & 0x0F) | ((minimum & 0x0F) << 4);
        packed[sub - 4] |= (scale >> 4) << 6;
        packed[sub] |= (minimum >> 4) << 6;
    }
    raw.extend_from_slice(&packed);
    for index in 0..128_usize {
        let low = u8::try_from(index % 16).unwrap_or(0);
        raw.push(low | ((15 - low) << 4));
    }

    let decoded = tensor(TensorKind::Q4_K, &raw, 256).expect("a Q4_K super-block decodes");
    assert_eq!(decoded.len(), 256);

    for sub in 0..8_usize {
        let scale = 2.0 * (small(sub) + 1.0);
        let minimum = 4.0 * small(sub);
        for position in 0..32_usize {
            let index = sub.wrapping_div(2) * 32 + position;
            let nibble = if sub % 2 == 0 {
                small(index % 16)
            } else {
                small(15 - (index % 16))
            };
            let want = scale * nibble - minimum;
            let got = decoded[sub * 32 + position];
            assert!(
                (got - want).abs() < 1e-3,
                "sub-block {sub} value {position}: wanted {want}, got {got}"
            );
        }
    }
}

fn small(value: usize) -> f32 {
    f32::from(u8::try_from(value).unwrap_or(0))
}

#[test]
fn a_q6_k_super_block_puts_each_value_where_the_format_says() {
    let mut raw = vec![0_u8; 210];
    for half in 0..2_usize {
        for l in 0..32_usize {
            raw[half * 64 + l] =
                u8::try_from(l % 16).unwrap_or(0) | (u8::try_from((l + 1) % 16).unwrap_or(0) << 4);
            raw[half * 64 + l + 32] = u8::try_from((l + 2) % 16).unwrap_or(0)
                | (u8::try_from((l + 3) % 16).unwrap_or(0) << 4);
        }
        for slot in 0..8_usize {
            raw[192 + half * 8 + slot] = u8::try_from(slot + 1).unwrap_or(0);
        }
    }
    raw[208] = 0x00;
    raw[209] = 0x3C;

    let decoded = tensor(TensorKind::Q6_K, &raw, 256).expect("a Q6_K super-block decodes");
    for half in 0..2_usize {
        for l in 0..32_usize {
            let which = l.wrapping_div(16);
            for (offset, nibble, scale_at) in [
                (0_usize, l % 16, 0_usize),
                (32, (l + 2) % 16, 2),
                (64, (l + 1) % 16, 4),
                (96, (l + 3) % 16, 6),
            ] {
                let scale = small(which + scale_at + 1);
                let want = scale * (small(nibble) - 32.0);
                let got = decoded[half * 128 + l + offset];
                assert!(
                    (got - want).abs() < 1e-3,
                    "half {half}, position {l}, offset {offset}: wanted {want}, got {got}"
                );
            }
        }
    }
}

#[test]
fn a_q2_k_super_block_walks_its_sub_blocks_in_the_formats_order() {
    let mut raw = vec![0_u8; 84];
    for (index, slot) in raw.iter_mut().take(16).enumerate() {
        *slot = u8::try_from(index % 15 + 1).unwrap_or(0);
    }
    for slot in raw.iter_mut().skip(16).take(64) {
        *slot = 0b1110_0100;
    }
    raw[80] = 0x00;
    raw[81] = 0x3C;
    raw[82] = 0x00;
    raw[83] = 0x00;

    let decoded = tensor(TensorKind::Q2_K, &raw, 256).expect("a Q2_K super-block decodes");
    let mut at = 0_usize;
    for half in 0..2_usize {
        for shift in 0..4_usize {
            for run in 0..2_usize {
                let scale = small((half * 8 + shift * 2 + run) % 15 + 1);
                for _ in 0..16_usize {
                    let want = scale * small(shift);
                    assert!(
                        (decoded[at] - want).abs() < 1e-3,
                        "value {at}: wanted {want}, got {}",
                        decoded[at]
                    );
                    at = at.saturating_add(1);
                }
            }
        }
    }
}

#[test]
fn every_k_scheme_fills_its_super_block_exactly() {
    for kind in [
        TensorKind::Q2_K,
        TensorKind::Q3_K,
        TensorKind::Q4_K,
        TensorKind::Q5_K,
        TensorKind::Q6_K,
    ] {
        let bytes = vec![0_u8; usize::try_from(kind.bytes_per_block()).unwrap_or(0)];
        let decoded = tensor(kind, &bytes, 256).unwrap_or_else(|failure| {
            panic!("{kind} did not decode a zero block: {failure}");
        });
        assert_eq!(decoded.len(), 256, "{kind} produced the wrong count");
        assert_eq!(
            usize::try_from(kind.block_size()).unwrap_or(0),
            256,
            "{kind} states a super-block that is not 256"
        );
    }
}

#[test]
fn a_scheme_that_is_not_implemented_is_named() {
    let refused = tensor(TensorKind::Unknown(30), &[0; 64], 32)
        .expect_err("a scheme with no decoder is refused");
    assert!(refused.to_string().contains("engine"), "{refused}");
}

#[test]
fn a_non_linear_block_reads_its_codes_as_table_entries() {
    let mut raw = vec![0x00, 0x3C];
    for index in 0..16_u8 {
        raw.push(index | ((15 - index) << 4));
    }
    let decoded = tensor(TensorKind::IQ4_NL, &raw, 32).expect("an IQ4_NL block decodes");
    assert_eq!(decoded.len(), 32);

    let table = crate::codebook::IQ4_VALUES;
    assert_eq!(table[0], -127, "the table's first entry is not what it was");
    assert_eq!(table[15], 113, "the table's last entry is not what it was");
    for index in 0..16_usize {
        assert!(
            (decoded[index] - f32::from(table[index])).abs() < 1e-3,
            "low nibble {index}: wanted {}, got {}",
            table[index],
            decoded[index]
        );
        assert!(
            (decoded[16 + index] - f32::from(table[15 - index])).abs() < 1e-3,
            "high nibble {index}: wanted {}, got {}",
            table[15 - index],
            decoded[16 + index]
        );
    }
}

#[test]
fn a_non_linear_super_block_assembles_its_scale_from_two_planes() {
    let mut raw = vec![0x00, 0x3C];
    let mut low = [0_u8; 4];
    let mut high = 0_u16;
    for sub in 0..8_usize {
        let scale = u16::try_from(sub + 32).unwrap_or(0);
        let nibble = u8::try_from(scale & 0x0F).unwrap_or(0);
        low[sub.wrapping_div(2)] |= nibble << (4 * (sub % 2));
        high |= ((scale >> 4) & 3) << (2 * sub);
    }
    raw.extend_from_slice(&high.to_le_bytes());
    raw.extend_from_slice(&low);
    raw.extend(std::iter::repeat_n(0x88_u8, 128));

    let decoded = tensor(TensorKind::IQ4_XS, &raw, 256).expect("an IQ4_XS super-block decodes");
    assert_eq!(decoded.len(), 256);
    assert_eq!(
        crate::codebook::IQ4_VALUES[8],
        1,
        "the table moved under this test"
    );

    for sub in 0..8_usize {
        let want = small(sub);
        for position in 0..32_usize {
            let got = decoded[sub * 32 + position];
            assert!(
                (got - want).abs() < 1e-3,
                "sub-block {sub} value {position}: wanted {want}, got {got}"
            );
        }
    }
}

#[test]
fn a_microscaling_block_scales_a_three_bit_float_by_a_bare_exponent() {
    let mut raw = vec![129_u8];
    raw.extend((0..16_u8).map(|code| (code & 0x07) | ((code & 0x07) | 0x08) << 4));
    let decoded = tensor(TensorKind::MXFP4, &raw, 32).expect("an MXFP4 block decodes");
    assert_eq!(decoded.len(), 32);
    let magnitudes = [0.0, 0.5, 1.0, 1.5, 2.0, 3.0, 4.0, 6.0];
    for (position, held) in decoded.iter().enumerate() {
        let magnitude = magnitudes[position % 8] * 4.0;
        let want = if position < 16 { magnitude } else { -magnitude };
        assert!(
            (held - want).abs() < 1e-6,
            "position {position}: wanted {want}, got {held}"
        );
    }
    let small = tensor(
        TensorKind::MXFP4,
        &[0, 0x22, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
        32,
    )
    .expect("decodes");
    assert!(
        small[0] > 0.0 && small[0] < f32::MIN_POSITIVE,
        "{}",
        small[0]
    );
    let reserved = tensor(
        TensorKind::MXFP4,
        &[255, 0x22, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
        32,
    )
    .expect("decodes");
    assert!(reserved[0].is_nan(), "{}", reserved[0]);
}

#[test]
fn a_grid_scheme_takes_its_magnitudes_from_the_grid_and_its_signs_from_the_plane() {
    let mut raw = vec![0x00, 0x3C];
    raw.extend(std::iter::repeat_n(0_u8, 64));
    raw.extend(std::iter::repeat_n(0_u8, 8));
    let mut signs = vec![0_u8; 32];
    signs[0] = 0xFF;
    raw.extend_from_slice(&signs);
    raw.extend(std::iter::repeat_n(0_u8, 4));

    let decoded = tensor(TensorKind::IQ3_S, &raw, 256).expect("an IQ3_S super-block decodes");
    assert_eq!(decoded.len(), 256);

    let entry = crate::codebook::IQ3S_GRID[0];
    for (position, magnitude) in entry.into_iter().enumerate() {
        let want = -f32::from(magnitude);
        assert!(
            (decoded[position] - want).abs() < 1e-3,
            "the first quarter is not negated: wanted {want}, got {}",
            decoded[position]
        );
    }
    let later = decoded[64];
    assert!(
        later >= 0.0,
        "a value outside the negated span came back negative: {later}"
    );
}

#[test]
fn the_transcribed_tables_are_the_size_and_shape_they_were() {
    assert_eq!(crate::codebook::IQ3S_GRID.len(), 512);
    assert_eq!(crate::codebook::IQ4_VALUES.len(), 16);
    assert_eq!(crate::codebook::IQ3S_GRID[0], [1, 1, 1, 1]);
    assert_eq!(
        crate::codebook::IQ3S_GRID[511],
        [1, 1, 15, 15],
        "the grid's last entry is not what was transcribed"
    );
    for (index, entry) in crate::codebook::IQ3S_GRID.iter().enumerate() {
        for byte in entry {
            assert_eq!(byte % 2, 1, "grid entry {index} holds an even magnitude");
        }
    }
}

#[test]
fn q5_0_places_the_fifth_bit_where_the_reference_does() {
    let mut block = vec![0_u8; 22];
    block[0..2].copy_from_slice(&to_half(1.0).to_le_bytes());
    block[2..6].copy_from_slice(&0x0001_0001_u32.to_le_bytes());
    let mut out = Vec::new();
    super::q5(&block, &mut out, false);
    assert_eq!(out.len(), 32);
    for (index, value) in out.iter().enumerate() {
        let expected = if index == 0 || index == 16 {
            0.0
        } else {
            -16.0
        };
        assert!(
            (value - expected).abs() < 1e-6,
            "value {index} was {value}, expected {expected}"
        );
    }
}

#[test]
fn q5_1_scales_and_shifts_without_centring() {
    let mut block = vec![0_u8; 24];
    block[0..2].copy_from_slice(&to_half(2.0).to_le_bytes());
    block[2..4].copy_from_slice(&to_half(1.0).to_le_bytes());
    block[8] = 0x53;
    let mut out = Vec::new();
    super::q5(&block, &mut out, true);
    assert!((out[0] - 7.0).abs() < 1e-6, "{}", out[0]);
    assert!((out[16] - 11.0).abs() < 1e-6, "{}", out[16]);
    assert!(
        (out[1] - 1.0).abs() < 1e-6,
        "an empty nibble is the minimum alone"
    );
}

#[test]
fn q3_k_reads_one_plane_for_both_halves() {
    let mut block = vec![0_u8; 110];
    for byte in block.iter_mut().take(32) {
        *byte = 0xFF;
    }
    for byte in block.iter_mut().skip(96).take(8) {
        *byte = 0x11;
    }
    for byte in block.iter_mut().skip(104).take(4) {
        *byte = 0xAA;
    }
    block[108..110].copy_from_slice(&to_half(1.0).to_le_bytes());

    let mut out = Vec::new();
    super::q3_k(&block, &mut out);
    assert_eq!(out.len(), 256);
    for (index, value) in out.iter().enumerate() {
        assert!(
            value.abs() < 1e-6,
            "value {index} decoded to {value}; the second half reads the plane off its end"
        );
    }
}
