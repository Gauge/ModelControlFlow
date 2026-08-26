//! Turning a tensor's stored bytes into numbers to compute with.
//!
//! A GGUF tensor is a block-quantized encoding: a scale (and sometimes a
//! minimum) shared by a fixed number of values, then the values themselves at
//! four or eight bits. Dequantizing is undoing that arithmetic, and it is the
//! step where a stand-in either agrees with a vendored engine or does not —
//! which is the whole of what D31 is for.
//!
//! **Written out, not reached for.** Half-precision is not in the stable
//! standard library, so [`from_half`] is the IEEE 754 binary16 rules spelled
//! out: the exponent adjusted, the subnormals scaled, the infinities and NaN
//! carried across. The alternative is an `unsafe` intrinsic or a dependency,
//! and B15 asks what either would buy against a page of arithmetic that can be
//! checked against known values (A19).
//!
//! **What it refuses.** A scheme this crate does not implement is
//! `engine.unavailable` naming the scheme — not an approximation, and not an
//! empty tensor. D31's three states for an artifact are *runs on the vendored
//! engine*, *runs on the stand-in and is marked*, and *does not run, and MCF
//! says which component was missing*; this is where the third one is decided
//! for a quantization scheme.
//!
//! **Deliberately slow.** One value at a time, no blocking, no SIMD, no
//! attempt to be clever about memory. F8 measured what that costs and D32
//! settled that it is the right trade: this code exists to be *checkable*, and
//! the fastest way to lose that is to optimize it.

use mcf_core::failure::{Attribution, Category, Disposition, Failure, Result, Subsystem};

use crate::gguf::TensorKind;

const WHERE: Subsystem = Subsystem::new("mcf-standin::dequantize");

/// Turns a tensor's stored bytes into the values it encodes.
///
/// `elements` is how many values the caller expects, from the tensor's shape.
/// It is passed rather than derived because the last block of a quantized
/// tensor is a whole block on disk however few of its values are wanted, and a
/// decoder that returned the padding would silently lengthen every row.
///
/// # Errors
///
/// `engine.unavailable` when the scheme is one this crate does not implement,
/// naming it; `artifact.format.malformed` when the bytes are too few for the
/// count, naming how many were wanted and how many there were.
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

/// One block of one scheme.
fn decode_block(kind: TensorKind, raw: &[u8], out: &mut Vec<f32>) -> Result<()> {
    match kind {
        TensorKind::F32 => out.push(f32::from_bits(u32::from_le_bytes([
            byte(raw, 0),
            byte(raw, 1),
            byte(raw, 2),
            byte(raw, 3),
        ]))),
        TensorKind::F16 => out.push(from_half(u16::from_le_bytes([byte(raw, 0), byte(raw, 1)]))),
        TensorKind::Q8_0 => {
            // A half-precision scale, then thirty-two signed bytes.
            let scale = from_half(u16::from_le_bytes([byte(raw, 0), byte(raw, 1)]));
            for index in 0..32 {
                let value = i8::from_le_bytes([byte(raw, 2 + index)]);
                out.push(scale * f32::from(value));
            }
        }
        TensorKind::Q4_0 => {
            // A scale, then thirty-two four-bit values biased by eight. The low
            // nibble of byte *i* is value *i*; the high nibble is value *i+16*.
            // That interleaving is the format's, and getting it wrong produces
            // a tensor that is the right size and the wrong shape — which is
            // why the tests state a block by hand.
            let scale = from_half(u16::from_le_bytes([byte(raw, 0), byte(raw, 1)]));
            let packed: Vec<u8> = (0..16).map(|index| byte(raw, 2 + index)).collect();
            for value in &packed {
                out.push(scale * (f32::from(value & 0x0F) - 8.0));
            }
            for value in &packed {
                out.push(scale * (f32::from(value >> 4) - 8.0));
            }
        }
        TensorKind::Q4_1 => {
            // A scale and a minimum, then thirty-two unbiased four-bit values.
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
        TensorKind::Unknown(_) => return Err(unavailable(kind)),
    }
    Ok(())
}

/// An index divided by a power of two, written so the workspace's denial of
/// integer division does not have to be argued with at each use.
///
/// The denial is there because a truncated quotient is a silently wrong number
/// (A6). Here the quotient is an *index into a fixed layout* rather than a
/// quantity — which sub-block a position belongs to — and truncation is the
/// arithmetic the format specifies.
const fn nth(position: usize, of: usize) -> usize {
    position.wrapping_div(of)
}

/// One six-bit scale and one six-bit minimum, unpacked from the twelve bytes
/// `Q4_K` and `Q5_K` share.
///
/// Twelve bytes hold eight pairs of six-bit numbers, and not in the order
/// anybody would choose: the first four pairs are the low six bits of two runs
/// of four bytes, and the last four are built from nibbles of the second run
/// with the top two bits of the first run above them. This is the format as it
/// is written rather than as it might have been (§3.7 — a reader reads what is
/// there).
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

/// The 4-bit super-block scheme: 256 values, eight sub-blocks of 32.
///
/// Two half-precision multipliers — one for the scales, one for the minimums —
/// then a six-bit scale and a six-bit minimum per sub-block. A value is
/// `d * scale * q - dmin * minimum`, and it is the subtraction that lets four
/// bits hold a distribution which is not centred on zero. Most of a `Q4_K_M`
/// file is this.
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

/// The 5-bit super-block scheme: `Q4_K` with a fifth bit per value.
///
/// The extra bit lives in a plane of its own so that the low four stay where a
/// four-bit reader would find them, which is why the two schemes share their
/// scale packing exactly.
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

/// The 6-bit super-block scheme: 256 values, four low bits and two high.
///
/// Sixteen signed eight-bit scales against one multiplier, and the value is
/// centred by subtracting 32 — which is how six bits hold a symmetric
/// distribution with no minimum to go with the scale. `Q4_K_M` uses it for the
/// tensors it will not round further.
fn q6_k(raw: &[u8], out: &mut Vec<f32>) {
    let low_plane = raw.get(0..128).unwrap_or(&[]);
    let high_plane = raw.get(128..192).unwrap_or(&[]);
    let scales = raw.get(192..208).unwrap_or(&[]);
    let d = from_half(u16::from_le_bytes([byte(raw, 208), byte(raw, 209)]));

    for half in 0..2 {
        let low = low_plane.get(half * 64..half * 64 + 64).unwrap_or_default();
        let high = high_plane
            .get(half * 32..half * 32 + 32)
            .unwrap_or_default();
        for position in 0..32 {
            let at = |index: usize| u32::from(low.get(index).copied().unwrap_or(0));
            let bits = u32::from(high.get(position).copied().unwrap_or(0));
            let quarters = [
                ((at(position) & 0x0F) | ((bits & 3) << 4), 0_usize),
                ((at(position + 32) & 0x0F) | (((bits >> 2) & 3) << 4), 2),
                ((at(position) >> 4) | (((bits >> 4) & 3) << 4), 4),
                ((at(position + 32) >> 4) | (((bits >> 6) & 3) << 4), 6),
            ];
            for (stored, group) in quarters {
                let scale = signed(scales, half * 8 + group + nth(position, 16));
                let centred = i32::try_from(stored).unwrap_or(0) - 32;
                out.push(d * f32::from(scale) * as_float(centred));
            }
        }
    }
}

/// The 3-bit super-block scheme: two bits in one plane, the third in another.
///
/// The high plane holds the third bit **inverted**, which is not a detail a
/// reader can guess: a value whose high bit is set reads as the low two bits
/// minus four.
fn q3_k(raw: &[u8], out: &mut Vec<f32>) {
    let high_plane = raw.get(0..32).unwrap_or(&[]);
    let low_plane = raw.get(32..96).unwrap_or(&[]);
    let packed = raw.get(96..108).unwrap_or(&[]);
    let d = from_half(u16::from_le_bytes([byte(raw, 108), byte(raw, 109)]));

    // Sixteen six-bit scales: four bits in the first eight bytes, two more in
    // the last four, and centred by subtracting 32.
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

    for group in 0..16 {
        let shift = (group % 4) * 2;
        let plane = nth(group, 4);
        for position in 0..16 {
            let at = plane * 16 + position;
            let bits = (low_plane.get(at).copied().unwrap_or(0) >> shift) & 3;
            let mask = 1_u8 << (group % 8);
            let inverted = high_plane.get(at).copied().unwrap_or(0) & mask == 0;
            let centred = i32::from(bits) - if inverted { 4 } else { 0 };
            let scale = scales.get(group).copied().unwrap_or(0);
            out.push(d * as_float(scale) * as_float(centred));
        }
    }
}

/// The 2-bit super-block scheme: sixteen sub-blocks of sixteen values.
///
/// Each sub-block carries a four-bit scale and a four-bit minimum in one byte,
/// against two half-precision multipliers — the smallest scheme that still
/// holds a distribution which is not centred on zero.
fn q2_k(raw: &[u8], out: &mut Vec<f32>) {
    let scales = raw.get(0..16).unwrap_or(&[]);
    let values = raw.get(16..80).unwrap_or(&[]);
    let d = from_half(u16::from_le_bytes([byte(raw, 80), byte(raw, 81)]));
    let dmin = from_half(u16::from_le_bytes([byte(raw, 82), byte(raw, 83)]));

    for group in 0..16 {
        let packed = scales.get(group).copied().unwrap_or(0);
        let scale = d * f32::from(packed & 0x0F);
        let minimum = dmin * f32::from(packed >> 4);
        let shift = (group % 4) * 2;
        let plane = nth(group, 4);
        for position in 0..16 {
            let at = plane * 16 + position;
            let bits = (values.get(at).copied().unwrap_or(0) >> shift) & 3;
            out.push(scale * f32::from(bits) - minimum);
        }
    }
}

/// One signed eight-bit scale, read as the signed number it is.
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

/// A small integer as a float, exactly.
fn as_float(value: i32) -> f32 {
    #[allow(
        clippy::cast_precision_loss,
        reason = "every value here is a six-bit quantity or a centred scale, both exact in f32"
    )]
    {
        value as f32
    }
}

/// IEEE 754 binary16 to binary32, written out.
///
/// Exact in every case: binary32 can represent every binary16 value, including
/// the subnormals, both zeros, both infinities and every NaN payload. There is
/// no rounding here to get wrong, only bit arithmetic — which is what makes it
/// checkable against a table of known values rather than against a tolerance.
#[must_use]
pub fn from_half(half: u16) -> f32 {
    let sign = u32::from(half & 0x8000) << 16;
    let exponent = u32::from((half >> 10) & 0x1F);
    let mantissa = u32::from(half & 0x03FF);

    match exponent {
        // Zero and the subnormals. A binary16 subnormal is
        // mantissa × 2^-24, and binary32 represents all of them normally, so
        // the arithmetic is done in float rather than by shifting bits into an
        // exponent field that would have to be searched for.
        0 => {
            if mantissa == 0 {
                f32::from_bits(sign)
            } else {
                // The mantissa is ten bits, so `u16` holds it exactly and the
                // conversion is lossless — which the wider cast the workspace
                // denies would not have made obvious.
                let steps = f32::from(u16::try_from(mantissa).unwrap_or(0));
                let magnitude = steps * SUBNORMAL_STEP;
                f32::from_bits(sign | magnitude.to_bits())
            }
        }
        // Infinity and NaN keep their payload, so a NaN that arrives in a
        // tensor stays a NaN rather than becoming a large number (A7's habit,
        // at the level of a bit pattern).
        0x1F => f32::from_bits(sign | 0x7F80_0000 | (mantissa << 13)),
        // The ordinary case: rebias the exponent from 15 to 127 and place the
        // mantissa.
        _ => f32::from_bits(sign | ((exponent + 112) << 23) | (mantissa << 13)),
    }
}

/// The value of the smallest binary16 subnormal, which is 2^-24.
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
