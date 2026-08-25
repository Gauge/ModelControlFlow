//! SHA-256 against the published vectors.
//!
//! A19 in its strongest available form: every expectation here is a value
//! published by NIST, not one this implementation produced. An implementation
//! checked against itself is an implementation nobody has checked.

use super::{Sha256, sha256};

/// The three vectors from NIST's SHA-256 examples, and the empty string, which
/// exercises the padding path with no message at all.
#[test]
fn the_published_vectors_hold() {
    let cases: [(&str, &str); 3] = [
        (
            "",
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
        ),
        (
            "abc",
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
        ),
        (
            "abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq",
            "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1",
        ),
    ];
    for (message, expected) in cases {
        assert_eq!(sha256(message.as_bytes()).hex(), expected, "{message:?}");
    }
}

/// NIST's long message: one million `a`. It is the vector that catches a length
/// counter that overflows or a block loop that drifts, which the short ones
/// cannot.
#[test]
fn the_long_vector_holds() {
    let mut hasher = Sha256::new();
    for _ in 0..1_000 {
        hasher.update(&[b'a'; 1_000]);
    }
    assert_eq!(
        hasher.finish().hex(),
        "cdc76e5c9914fb9281a1c7e284d73e67f1809a48a497200e046d39ccc7112cd0"
    );
}

/// The same bytes fed in different-sized pieces give the same digest. This is
/// the property §7.49 depends on: a gigabyte file is read a block at a time,
/// and a buffering error would show as a digest that depends on the read size.
#[test]
fn the_digest_does_not_depend_on_how_the_bytes_arrived() {
    let message: Vec<u8> = (0..1_000_u32).map(|i| (i % 251) as u8).collect();
    let whole = sha256(&message);
    for chunk in [1_usize, 7, 63, 64, 65, 127, 128, 999] {
        let mut hasher = Sha256::new();
        for piece in message.chunks(chunk) {
            hasher.update(piece);
        }
        assert_eq!(hasher.finish(), whole, "chunked by {chunk}");
    }
}

/// A single changed bit changes the digest. Stated because it is the whole
/// point: §7.49's silent disk corruption is a handful of flipped bits.
#[test]
fn one_flipped_bit_changes_the_digest() {
    let clean = [0_u8; 4096];
    let mut corrupted = clean;
    if let Some(byte) = corrupted.get_mut(2048) {
        *byte ^= 0x01;
    }
    assert_ne!(sha256(&clean), sha256(&corrupted));
}

/// The hexadecimal form is lower-case and sixty-four characters, which is what
/// `Checksum` requires and what the hubs publish.
#[test]
fn the_hexadecimal_form_is_what_the_rest_of_mcf_expects() {
    let hex = sha256(b"abc").hex();
    assert_eq!(hex.len(), 64);
    assert!(
        hex.chars()
            .all(|c| c.is_ascii_digit() || c.is_ascii_lowercase())
    );
    assert_eq!(
        crate::provenance::Checksum::sha256(&hex).map(|c| c.hex().to_owned()),
        Some(hex)
    );
}

/// A digest is thirty-two bytes, and the hexadecimal is those bytes.
#[test]
fn the_bytes_and_the_hexadecimal_agree() {
    let digest = sha256(b"abc");
    assert_eq!(digest.bytes().len(), 32);
    assert_eq!(digest.bytes().first(), Some(&0xba));
    assert!(digest.hex().starts_with("ba78"));
}
