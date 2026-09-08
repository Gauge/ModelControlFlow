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

fn adler32(bytes: &[u8]) -> u32 {
    let mut low = 1_u32;
    let mut high = 0_u32;
    for byte in bytes {
        low = (low + u32::from(*byte)) % 65521;
        high = (high + low) % 65521;
    }
    (high << 16) | low
}

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

fn stored_zlib(raw: &[u8]) -> Vec<u8> {
    let mut out = vec![0x78, 0x01];
    let mut rest = raw;
    loop {
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
    header.extend_from_slice(&[8, 2, 0, 0, 0]);

    let mut out = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
    out.extend_from_slice(&chunk(*b"IHDR", &header));
    out.extend_from_slice(&chunk(*b"IDAT", &stored_zlib(&raw)));
    out.extend_from_slice(&chunk(*b"IEND", &[]));
    out
}

#[cfg(test)]
mod tests;
