use super::*;

#[test]
fn what_is_written_is_a_png() {
    let rows = vec![vec![1, 2, 3, 4, 5, 6], vec![7, 8, 9, 10, 11, 12]];
    let out = encode(2, 2, &rows);
    assert_eq!(
        out.get(..8),
        Some([0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A].as_slice()),
        "a PNG starts with the signature that says it is one"
    );
    let has = |kind: &[u8]| out.windows(kind.len()).any(|held| held == kind);
    assert!(has(b"IHDR"), "the header chunk is there");
    assert!(has(b"IDAT"), "the data chunk is there");
    assert!(has(b"IEND"), "the end chunk is there");
    let ihdr = out
        .windows(4)
        .position(|held| held == b"IHDR")
        .expect("the header chunk is there");
    assert_eq!(
        out.get(ihdr + 4..ihdr + 8),
        Some(2_u32.to_be_bytes().as_slice())
    );
    assert_eq!(
        out.get(ihdr + 8..ihdr + 12),
        Some(2_u32.to_be_bytes().as_slice())
    );
}

#[test]
fn the_same_picture_is_the_same_bytes() {
    let rows = vec![vec![9, 9, 9], vec![1, 1, 1]];
    assert_eq!(encode(1, 2, &rows), encode(1, 2, &rows));
}

#[test]
fn the_checksums_are_the_ones_the_format_defines() {
    assert_eq!(crc32(b"123456789"), 0xCBF4_3926);
    assert_eq!(adler32(b"Wikipedia"), 0x11E6_0398);
    assert_eq!(adler32(b""), 1);
}

#[test]
fn a_picture_bigger_than_one_block_is_still_one_stream() {
    let rows: Vec<Vec<u8>> = (0..40).map(|_| vec![7_u8; 3 * 700]).collect();
    let out = encode(700, 40, &rows);
    assert!(
        out.len() > 0xFFFF,
        "this picture is meant to need more than one stored block"
    );
    assert!(out.windows(4).any(|held| held == b"IEND"));
}
