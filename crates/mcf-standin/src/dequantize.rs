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
        TensorKind::Unknown(_) => return Err(unavailable(kind)),
    }
    Ok(())
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
