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

use crate::codebook;
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
        TensorKind::IQ4_NL => iq4_nl(raw, out),
        TensorKind::IQ4_XS => iq4_xs(raw, out),
        TensorKind::IQ3_S => iq3_s(raw, out),
        TensorKind::Unknown(_) => return Err(unavailable(kind)),
    }
    Ok(())
}

/// How many values a K-scheme super-block holds.
const SUPER: usize = 256;

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
/// tensors it will not round further, including the projection to logits.
///
/// **The four values a byte-pair produces are 32 apart, not adjacent.** Getting
/// that wrong is the defect [findings.md](../../../doc/findings.md) F19 records
/// and the reason the first real model produced a plausible-looking tensor of
/// entirely misplaced numbers: every value was decoded correctly and written
/// somewhere else.
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
            // Four values, each in its own quarter of the 128 this half covers,
            // and each with its own scale two apart in the scale array.
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

/// The 3-bit super-block scheme: two bits in one plane, the third in another.
///
/// The high plane holds the third bit **inverted**: a value whose bit is clear
/// reads as the low two bits minus four. Nobody would guess that, which is why
/// it is read from the format rather than derived.
///
/// The walk is the same shape as `Q2_K`'s: two halves of 128, each covering
/// four shifts of the same 32 bytes, sixteen values at a time.
fn q3_k(raw: &[u8], out: &mut Vec<f32>) {
    let high_plane = raw.get(0..32).unwrap_or_default();
    let low_plane = raw.get(32..96).unwrap_or_default();
    let packed = raw.get(96..108).unwrap_or_default();
    let d = from_half(u16::from_le_bytes([byte(raw, 108), byte(raw, 109)]));

    // Sixteen six-bit scales, four bits in the first eight bytes and two more
    // in the last four, centred by subtracting 32.
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
                    let inverted = high_plane.get(half * 32 + at).copied().unwrap_or(0) & mask == 0;
                    let centred = i32::from(bits) - if inverted { 4 } else { 0 };
                    out.push(d * as_float(scale) * as_float(centred));
                }
            }
            mask = mask.rotate_left(1);
        }
    }
}

/// The 2-bit super-block scheme: sixteen sub-blocks of sixteen values.
///
/// Each sub-block carries a four-bit scale and a four-bit minimum in one byte,
/// against two half-precision multipliers — the smallest scheme that still
/// holds a distribution which is not centred on zero.
///
/// The walk is `Q3_K`'s without the third bit: two halves of 128, four shifts
/// of the same 32 bytes, two runs of sixteen per shift.
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

/// The 4-bit non-linear scheme: 32 values, indices into a fixed table.
///
/// *Non-linear* means the codes are not multiples of a scale — they name
/// entries in [`codebook::IQ4_VALUES`], chosen so that four bits land where a
/// weight distribution actually is rather than where an even spacing would put
/// them. The scale multiplies what the table says.
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

/// The same table over a super-block of 256.
///
/// Eight sub-blocks of 32, each with a six-bit scale assembled from four bits
/// in one plane and two in another, centred by subtracting 32. This is most of
/// what an *Unsloth Dynamic* quantization is made of — 117 of the reference
/// model's 866 tensors.
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

/// The 3-bit scheme that indexes a grid of four values at a time.
///
/// Each code names one of five hundred and twelve four-value groups; a ninth
/// bit for each code lives in a separate plane, the signs live in a third, and
/// the scales are four bits doubled and offset by one. Nothing here is
/// derivable — the grid is the arithmetic (see [`codebook`]).
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
                    // The ninth bit for this code, shifted out of the byte the
                    // group shares. The two codes of a quarter take adjacent
                    // bits, which is why the shift counts down by two.
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

/// One entry of the four-bit non-linear table.
fn value_of(code: u8) -> i8 {
    codebook::IQ4_VALUES
        .get(usize::from(code & 0x0F))
        .copied()
        .unwrap_or(0)
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
