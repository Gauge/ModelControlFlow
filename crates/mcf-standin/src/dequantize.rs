use mcf_core::failure::{Attribution, Category, Disposition, Failure, Result, Subsystem};

use crate::codebook;
use crate::gguf::TensorKind;

const WHERE: Subsystem = Subsystem::new("mcf-standin::dequantize");

pub fn tensor(kind: TensorKind, bytes: &[u8], elements: usize) -> Result<Vec<f32>> {
    let block = usize::try_from(kind.block_size()).unwrap_or(0);
    if block == 0 {
        return Err(unavailable(kind));
    }
    let blocks = elements.div_ceil(block);
    let needed = blocks
        .checked_mul(usize::try_from(kind.bytes_per_block()).unwrap_or(0))
        .ok_or_else(|| short(kind, usize::MAX, bytes.len()))?;
    if bytes.len() < needed {
        return Err(short(kind, needed, bytes.len()));
    }

    let mut out = Vec::with_capacity(elements);
    for index in 0..blocks {
        let start = index.saturating_mul(usize::try_from(kind.bytes_per_block()).unwrap_or(0));
        let end = start.saturating_add(usize::try_from(kind.bytes_per_block()).unwrap_or(0));
        let raw = bytes
            .get(start..end)
            .ok_or_else(|| short(kind, end, bytes.len()))?;
        decode_block(kind, raw, &mut out)?;
    }
    out.truncate(elements);
    Ok(out)
}

fn decode_block(kind: TensorKind, raw: &[u8], out: &mut Vec<f32>) -> Result<()> {
    match kind {
        TensorKind::F32 => out.push(f32::from_bits(u32::from_le_bytes([
            byte(raw, 0),
            byte(raw, 1),
            byte(raw, 2),
            byte(raw, 3),
        ]))),
        TensorKind::F16 => out.push(from_half(u16::from_le_bytes([byte(raw, 0), byte(raw, 1)]))),
        TensorKind::BF16 => out.push(f32::from_bits(
            u32::from(u16::from_le_bytes([byte(raw, 0), byte(raw, 1)])) << 16,
        )),
        TensorKind::Q8_0 => {
            let scale = from_half(u16::from_le_bytes([byte(raw, 0), byte(raw, 1)]));
            for index in 0..32 {
                let value = i8::from_le_bytes([byte(raw, 2 + index)]);
                out.push(scale * f32::from(value));
            }
        }
        TensorKind::Q4_0 => {
            let scale = from_half(u16::from_le_bytes([byte(raw, 0), byte(raw, 1)]));
            let packed: Vec<u8> = (0..16).map(|index| byte(raw, 2 + index)).collect();
            for value in &packed {
                out.push(scale * (f32::from(value & 0x0F) - 8.0));
            }
            for value in &packed {
                out.push(scale * (f32::from(value >> 4) - 8.0));
            }
        }
        TensorKind::Q5_0 => q5(raw, out, false),
        TensorKind::Q5_1 => q5(raw, out, true),
        TensorKind::Q4_1 => {
            let scale = from_half(u16::from_le_bytes([byte(raw, 0), byte(raw, 1)]));
            let minimum = from_half(u16::from_le_bytes([byte(raw, 2), byte(raw, 3)]));
            let packed: Vec<u8> = (0..16).map(|index| byte(raw, 4 + index)).collect();
            for value in &packed {
                out.push(scale.mul_add(f32::from(value & 0x0F), minimum));
            }
            for value in &packed {
                out.push(scale.mul_add(f32::from(value >> 4), minimum));
            }
        }
        TensorKind::Q2_K => q2_k(raw, out),
        TensorKind::Q3_K => q3_k(raw, out),
        TensorKind::Q4_K => q4_k(raw, out),
        TensorKind::Q5_K => q5_k(raw, out),
        TensorKind::Q6_K => q6_k(raw, out),
        TensorKind::IQ4_NL => iq4_nl(raw, out),
        TensorKind::IQ4_XS => iq4_xs(raw, out),
        TensorKind::IQ3_S => iq3_s(raw, out),
        TensorKind::MXFP4 => mxfp4(raw, out),
        TensorKind::Unknown(_) => return Err(unavailable(kind)),
    }
    Ok(())
}

const SUPER: usize = 256;

const fn nth(position: usize, of: usize) -> usize {
    position.wrapping_div(of)
}

fn scale_and_minimum(packed: &[u8], which: usize) -> (u8, u8) {
    let at = |index: usize| packed.get(index).copied().unwrap_or(0);
    if which < 4 {
        (at(which) & 63, at(which + 4) & 63)
    } else {
        let low = at(which + 4);
        (
            (low & 0x0F) | ((at(which - 4) >> 6) << 4),
            (low >> 4) | ((at(which) >> 6) << 4),
        )
    }
}

fn q4_k(raw: &[u8], out: &mut Vec<f32>) {
    let d = from_half(u16::from_le_bytes([byte(raw, 0), byte(raw, 1)]));
    let dmin = from_half(u16::from_le_bytes([byte(raw, 2), byte(raw, 3)]));
    let scales = raw.get(4..16).unwrap_or(&[]);
    let values = raw.get(16..144).unwrap_or(&[]);

    for pair in 0..4 {
        let (low_scale, low_minimum) = scale_and_minimum(scales, pair * 2);
        let (high_scale, high_minimum) = scale_and_minimum(scales, pair * 2 + 1);
        let (low_scale, low_minimum) = (d * f32::from(low_scale), dmin * f32::from(low_minimum));
        let (high_scale, high_minimum) =
            (d * f32::from(high_scale), dmin * f32::from(high_minimum));
        let chunk = values.get(pair * 32..pair * 32 + 32).unwrap_or_default();
        for value in chunk {
            out.push(low_scale * f32::from(value & 0x0F) - low_minimum);
        }
        for value in chunk {
            out.push(high_scale * f32::from(value >> 4) - high_minimum);
        }
    }
}

fn q5_k(raw: &[u8], out: &mut Vec<f32>) {
    let d = from_half(u16::from_le_bytes([byte(raw, 0), byte(raw, 1)]));
    let dmin = from_half(u16::from_le_bytes([byte(raw, 2), byte(raw, 3)]));
    let scales = raw.get(4..16).unwrap_or(&[]);
    let high = raw.get(16..48).unwrap_or(&[]);
    let values = raw.get(48..176).unwrap_or(&[]);

    let mut low_bit = 1_u8;
    for pair in 0..4 {
        let (low_scale, low_minimum) = scale_and_minimum(scales, pair * 2);
        let (high_scale, high_minimum) = scale_and_minimum(scales, pair * 2 + 1);
        let (low_scale, low_minimum) = (d * f32::from(low_scale), dmin * f32::from(low_minimum));
        let (high_scale, high_minimum) =
            (d * f32::from(high_scale), dmin * f32::from(high_minimum));
        let chunk = values.get(pair * 32..pair * 32 + 32).unwrap_or_default();
        let high_bit = low_bit << 1;
        for (position, value) in chunk.iter().enumerate() {
            let plane = high.get(position).copied().unwrap_or(0);
            let fifth = u8::from(plane & low_bit != 0) << 4;
            out.push(low_scale * f32::from((value & 0x0F) | fifth) - low_minimum);
        }
        for (position, value) in chunk.iter().enumerate() {
            let plane = high.get(position).copied().unwrap_or(0);
            let fifth = u8::from(plane & high_bit != 0) << 4;
            out.push(high_scale * f32::from((value >> 4) | fifth) - high_minimum);
        }
        low_bit <<= 2;
    }
}

fn q6_k(raw: &[u8], out: &mut Vec<f32>) {
    let low_plane = raw.get(0..128).unwrap_or_default();
    let high_plane = raw.get(128..192).unwrap_or_default();
    let scales = raw.get(192..208).unwrap_or_default();
    let d = from_half(u16::from_le_bytes([byte(raw, 208), byte(raw, 209)]));

    let mut block = [0.0_f32; SUPER];
    for half in 0..2_usize {
        let low = low_plane.get(half * 64..half * 64 + 64).unwrap_or_default();
        let high = high_plane
            .get(half * 32..half * 32 + 32)
            .unwrap_or_default();
        for position in 0..32_usize {
            let at = |index: usize| u32::from(low.get(index).copied().unwrap_or(0));
            let bits = u32::from(high.get(position).copied().unwrap_or(0));
            let which = nth(position, 16);
            let quarters = [
                ((at(position) & 0x0F) | ((bits & 3) << 4), 0_usize, 0_usize),
                ((at(position + 32) & 0x0F) | (((bits >> 2) & 3) << 4), 32, 2),
                ((at(position) >> 4) | (((bits >> 4) & 3) << 4), 64, 4),
                ((at(position + 32) >> 4) | (((bits >> 6) & 3) << 4), 96, 6),
            ];
            for (stored, offset, scale_at) in quarters {
                let scale = signed(scales, half * 8 + which + scale_at);
                let centred = i32::try_from(stored).unwrap_or(0) - 32;
                if let Some(slot) = block.get_mut(half * 128 + position + offset) {
                    *slot = d * f32::from(scale) * as_float(centred);
                }
            }
        }
    }
    out.extend_from_slice(&block);
}

fn q3_k(raw: &[u8], out: &mut Vec<f32>) {
    let high_plane = raw.get(0..32).unwrap_or_default();
    let low_plane = raw.get(32..96).unwrap_or_default();
    let packed = raw.get(96..108).unwrap_or_default();
    let d = from_half(u16::from_le_bytes([byte(raw, 108), byte(raw, 109)]));

    let mut scales = [0_i32; 16];
    for (index, slot) in scales.iter_mut().enumerate() {
        let at = |i: usize| u32::from(packed.get(i).copied().unwrap_or(0));
        let low = if index < 8 {
            at(index) & 0x0F
        } else {
            at(index - 8) >> 4
        };
        let high = (at(8 + (index % 4)) >> (2 * nth(index, 4))) & 3;
        *slot = i32::try_from(low | (high << 4)).unwrap_or(0) - 32;
    }

    let mut scale_at = 0_usize;
    let mut mask = 1_u8;
    for half in 0..2_usize {
        let low = low_plane.get(half * 32..half * 32 + 32).unwrap_or_default();
        for shift in [0_u8, 2, 4, 6] {
            for run in 0..2_usize {
                let scale = scales.get(scale_at).copied().unwrap_or(0);
                scale_at = scale_at.saturating_add(1);
                for position in 0..16_usize {
                    let at = run * 16 + position;
                    let bits = (low.get(at).copied().unwrap_or(0) >> shift) & 3;
                    let inverted = high_plane.get(at).copied().unwrap_or(0) & mask == 0;
                    let centred = i32::from(bits) - if inverted { 4 } else { 0 };
                    out.push(d * as_float(scale) * as_float(centred));
                }
            }
            mask = mask.rotate_left(1);
        }
    }
}

fn q5(block: &[u8], out: &mut Vec<f32>, with_minimum: bool) {
    let d = from_half(u16::from_le_bytes([byte(block, 0), byte(block, 1)]));
    let (m, at) = if with_minimum {
        (
            from_half(u16::from_le_bytes([byte(block, 2), byte(block, 3)])),
            4,
        )
    } else {
        (0.0, 2)
    };
    let high = u32::from_le_bytes([
        byte(block, at),
        byte(block, at + 1),
        byte(block, at + 2),
        byte(block, at + 3),
    ]);
    let nibbles = block.get(at + 4..at + 20).unwrap_or_default();
    for (j, packed) in nibbles.iter().enumerate() {
        let fifth = u8::try_from((high >> j) & 1).unwrap_or(0) << 4;
        let value = i32::from((packed & 0x0F) | fifth);
        out.push(five(value, d, m, with_minimum));
    }
    for (j, packed) in nibbles.iter().enumerate() {
        let fifth = u8::try_from((high >> (j + 16)) & 1).unwrap_or(0) << 4;
        let value = i32::from((packed >> 4) | fifth);
        out.push(five(value, d, m, with_minimum));
    }
}

fn five(value: i32, d: f32, m: f32, with_minimum: bool) -> f32 {
    if with_minimum {
        d.mul_add(as_float(value), m)
    } else {
        d * as_float(value - 16)
    }
}

fn q2_k(raw: &[u8], out: &mut Vec<f32>) {
    let scales = raw.get(0..16).unwrap_or_default();
    let values = raw.get(16..80).unwrap_or_default();
    let d = from_half(u16::from_le_bytes([byte(raw, 80), byte(raw, 81)]));
    let dmin = from_half(u16::from_le_bytes([byte(raw, 82), byte(raw, 83)]));

    let mut scale_at = 0_usize;
    for half in 0..2_usize {
        let low = values.get(half * 32..half * 32 + 32).unwrap_or_default();
        for shift in [0_u8, 2, 4, 6] {
            for run in 0..2_usize {
                let packed = scales.get(scale_at).copied().unwrap_or(0);
                scale_at = scale_at.saturating_add(1);
                let scale = d * f32::from(packed & 0x0F);
                let minimum = dmin * f32::from(packed >> 4);
                for position in 0..16_usize {
                    let at = run * 16 + position;
                    let bits = (low.get(at).copied().unwrap_or(0) >> shift) & 3;
                    out.push(scale * f32::from(bits) - minimum);
                }
            }
        }
    }
}

fn signed(scales: &[u8], at: usize) -> i8 {
    #[allow(
        clippy::cast_possible_wrap,
        reason = "the scales are signed eight-bit numbers, and reading them unsigned is what \
                  would be wrong"
    )]
    {
        scales.get(at).copied().unwrap_or(0) as i8
    }
}

fn as_float(value: i32) -> f32 {
    #[allow(
        clippy::cast_precision_loss,
        reason = "every value here is a six-bit quantity or a centred scale, both exact in f32"
    )]
    {
        value as f32
    }
}

fn iq4_nl(raw: &[u8], out: &mut Vec<f32>) {
    let d = from_half(u16::from_le_bytes([byte(raw, 0), byte(raw, 1)]));
    let codes = raw.get(2..18).unwrap_or_default();
    for code in codes {
        out.push(d * f32::from(value_of(code & 0x0F)));
    }
    for code in codes {
        out.push(d * f32::from(value_of(code >> 4)));
    }
}

fn iq4_xs(raw: &[u8], out: &mut Vec<f32>) {
    let d = from_half(u16::from_le_bytes([byte(raw, 0), byte(raw, 1)]));
    let scales_high = u16::from_le_bytes([byte(raw, 2), byte(raw, 3)]);
    let scales_low = raw.get(4..8).unwrap_or_default();
    let codes = raw.get(8..136).unwrap_or_default();

    for sub in 0..8_usize {
        let low = u16::from(scales_low.get(nth(sub, 2)).copied().unwrap_or(0));
        let nibble = (low >> (4 * (sub % 2))) & 0x0F;
        let high = (scales_high >> (2 * sub)) & 3;
        let scale = i32::from(nibble | (high << 4)) - 32;
        let step = d * as_float(scale);
        let chunk = codes.get(sub * 16..sub * 16 + 16).unwrap_or_default();
        for code in chunk {
            out.push(step * f32::from(value_of(code & 0x0F)));
        }
        for code in chunk {
            out.push(step * f32::from(value_of(code >> 4)));
        }
    }
}

fn mxfp4(raw: &[u8], out: &mut Vec<f32>) {
    let scale = power_of_two(byte(raw, 0));
    let codes = raw.get(1..17).unwrap_or_default();
    for code in codes {
        out.push(scale * e2m1(code & 0x0F));
    }
    for code in codes {
        out.push(scale * e2m1(code >> 4));
    }
}

fn power_of_two(exponent: u8) -> f32 {
    match exponent {
        0 => f32::MIN_POSITIVE / 2.0,
        255 => f32::NAN,
        held => f32::from_bits(u32::from(held) << 23),
    }
}

fn e2m1(code: u8) -> f32 {
    const MAGNITUDES: [f32; 8] = [0.0, 0.5, 1.0, 1.5, 2.0, 3.0, 4.0, 6.0];
    let magnitude = MAGNITUDES
        .get(usize::from(code & 0x07))
        .copied()
        .unwrap_or(0.0);
    if code & 0x08 == 0 {
        magnitude
    } else {
        -magnitude
    }
}

fn iq3_s(raw: &[u8], out: &mut Vec<f32>) {
    let d = from_half(u16::from_le_bytes([byte(raw, 0), byte(raw, 1)]));
    let codes = raw.get(2..66).unwrap_or_default();
    let ninth = raw.get(66..74).unwrap_or_default();
    let signs = raw.get(74..106).unwrap_or_default();
    let scales = raw.get(106..110).unwrap_or_default();

    for pair in 0..4_usize {
        let packed = scales.get(pair).copied().unwrap_or(0);
        let steps = [
            d * (1.0 + 2.0 * f32::from(packed & 0x0F)),
            d * (1.0 + 2.0 * f32::from(packed >> 4)),
        ];
        for (half, step) in steps.into_iter().enumerate() {
            let group = pair * 2 + half;
            let high = u32::from(ninth.get(group).copied().unwrap_or(0));
            for quarter in 0..4_usize {
                let at = group * 8 + quarter * 2;
                let sign = signs.get(group * 4 + quarter).copied().unwrap_or(0);
                for which in 0..2_usize {
                    let code = u32::from(codes.get(at + which).copied().unwrap_or(0));
                    let shift = 8
                        - 2 * u32::try_from(quarter).unwrap_or(0)
                        - u32::try_from(which).unwrap_or(0);
                    let index = usize::try_from(code | ((high << shift) & 256)).unwrap_or(0);
                    let entry = codebook::IQ3S_GRID.get(index).copied().unwrap_or([0; 4]);
                    for (position, magnitude) in entry.into_iter().enumerate() {
                        let bit = 1_u8 << (which * 4 + position);
                        let negative = sign & bit != 0;
                        let value = step * f32::from(magnitude);
                        out.push(if negative { -value } else { value });
                    }
                }
            }
        }
    }
}

fn value_of(code: u8) -> i8 {
    codebook::IQ4_VALUES
        .get(usize::from(code & 0x0F))
        .copied()
        .unwrap_or(0)
}

#[must_use]
pub fn from_half(half: u16) -> f32 {
    let sign = u32::from(half & 0x8000) << 16;
    let exponent = u32::from((half >> 10) & 0x1F);
    let mantissa = u32::from(half & 0x03FF);

    match exponent {
        0 => {
            if mantissa == 0 {
                f32::from_bits(sign)
            } else {
                let steps = f32::from(u16::try_from(mantissa).unwrap_or(0));
                let magnitude = steps * SUBNORMAL_STEP;
                f32::from_bits(sign | magnitude.to_bits())
            }
        }
        0x1F => f32::from_bits(sign | 0x7F80_0000 | (mantissa << 13)),
        _ => f32::from_bits(sign | ((exponent + 112) << 23) | (mantissa << 13)),
    }
}

const SUBNORMAL_STEP: f32 = 1.0 / 16_777_216.0;

fn byte(raw: &[u8], at: usize) -> u8 {
    raw.get(at).copied().unwrap_or(0)
}

fn unavailable(kind: TensorKind) -> Failure {
    Failure::new(
        Category::EngineUnavailable,
        Attribution::Mcf,
        Disposition::Refused,
        WHERE,
        "the stand-in engine does not implement this quantization scheme",
    )
    .with_context("scheme", kind.to_string())
}

fn short(kind: TensorKind, wanted: usize, had: usize) -> Failure {
    Failure::new(
        Category::ArtifactFormatMalformed,
        Attribution::Artifact,
        Disposition::Refused,
        WHERE,
        "a tensor's bytes end before the values it is supposed to hold",
    )
    .with_context("scheme", kind.to_string())
    .with_context("wanted", wanted.to_string())
    .with_context("had", had.to_string())
}

#[cfg(test)]
mod tests;
