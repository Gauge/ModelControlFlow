//! The smallest PNG that is a PNG, written here because a probe cannot ship one.
//!
//! **A probe's images have to be identical on every machine and every run**, or
//! the trials are not comparable and the conditions are not restatable (§3.4).
//! Two ways to get that: ship the bytes as an asset, or compute them. MCF ships
//! no binary assets — the window draws every panel, table and button itself
//! rather than admitting a widget toolkit (B-405) — and an image file checked
//! into the tree is a blob nobody reviews and a diff nobody can read.
//!
//! So the bytes are computed, and the whole encoder is here: a PNG is a
//! signature, three chunks, and a CRC. The compression is deflate's *stored*
//! block — no compression at all, which is legal, tiny to write, and produces
//! the same bytes every time. A probe's image is a few hundred kilobytes and
//! transient; spending a compressor on it would buy nothing.

/// The CRC-32 of a byte run, as PNG defines it.
///
/// The table is computed rather than written out: a 256-entry constant is 256
/// chances to mistype a number, and this loop is the definition.
fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFF_u32;
    for byte in bytes {
        crc ^= u32::from(*byte);
        for _ in 0..8 {
            let carry = crc & 1;
            crc >>= 1;
            if carry != 0 {
                crc ^= 0xEDB8_8320;
            }
        }
    }
    crc ^ 0xFFFF_FFFF
}

/// Adler-32, which is the checksum zlib puts after the deflate stream.
fn adler32(bytes: &[u8]) -> u32 {
    let mut low = 1_u32;
    let mut high = 0_u32;
    for byte in bytes {
        low = (low + u32::from(*byte)) % 65521;
        high = (high + low) % 65521;
    }
    (high << 16) | low
}

/// One PNG chunk: length, type, payload, CRC over type and payload.
fn chunk(kind: [u8; 4], payload: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    let length = u32::try_from(payload.len()).unwrap_or(0);
    out.extend_from_slice(&length.to_be_bytes());
    let mut checked = Vec::with_capacity(4 + payload.len());
    checked.extend_from_slice(&kind);
    checked.extend_from_slice(payload);
    out.extend_from_slice(&checked);
    out.extend_from_slice(&crc32(&checked).to_be_bytes());
    out
}

/// Wraps raw bytes as a zlib stream of stored deflate blocks.
fn stored_zlib(raw: &[u8]) -> Vec<u8> {
    // 0x78 0x01: deflate, 32 KiB window, no preset dictionary, fastest.
    let mut out = vec![0x78, 0x01];
    let mut rest = raw;
    loop {
        // A stored block carries at most 65,535 bytes and states its own
        // length twice, once complemented — which is the format's own check
        // that the length was written down right.
        let take = rest.len().min(0xFFFF);
        let (block, remainder) = rest.split_at(take);
        let last = u8::from(remainder.is_empty());
        out.push(last);
        let length = u16::try_from(block.len()).unwrap_or(0);
        out.extend_from_slice(&length.to_le_bytes());
        out.extend_from_slice(&(!length).to_le_bytes());
        out.extend_from_slice(block);
        if remainder.is_empty() {
            break;
        }
        rest = remainder;
    }
    out.extend_from_slice(&adler32(raw).to_be_bytes());
    out
}

/// A PNG of `width` by `height`, from rows of red-green-blue triples.
///
/// Each row is prefixed with filter type 0 — *none* — because a filter that
/// predicted a pixel from its neighbour would compress better and there is
/// nothing here to compress.
#[must_use]
pub fn encode(width: u32, height: u32, rows: &[Vec<u8>]) -> Vec<u8> {
    let mut raw = Vec::new();
    for row in rows {
        raw.push(0);
        raw.extend_from_slice(row);
    }

    let mut header = Vec::with_capacity(13);
    header.extend_from_slice(&width.to_be_bytes());
    header.extend_from_slice(&height.to_be_bytes());
    // Eight bits a channel, colour type 2 (truecolour), deflate, no filter, no
    // interlace.
    header.extend_from_slice(&[8, 2, 0, 0, 0]);

    let mut out = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
    out.extend_from_slice(&chunk(*b"IHDR", &header));
    out.extend_from_slice(&chunk(*b"IDAT", &stored_zlib(&raw)));
    out.extend_from_slice(&chunk(*b"IEND", &[]));
    out
}

#[cfg(test)]
mod tests;
