use super::*;

#[test]
fn the_keys_the_application_acts_on_decode() {
    assert_eq!(decode(b"q"), Some((Key::Character('q'), 1)));
    assert_eq!(decode(b"\x03"), Some((Key::Interrupt, 1)));
    assert_eq!(decode(b"\r"), Some((Key::Enter, 1)));
    assert_eq!(decode(b"\x1b[A"), Some((Key::Up, 3)));
    assert_eq!(decode(b"\x1b[B"), Some((Key::Down, 3)));
    // A lone escape is incomplete until nothing more arrives; see
    // `a_split_sequence_waits_for_the_rest`.
    assert_eq!(decode(b"\x1b"), None);
    assert_eq!(decode(b""), None);
}

/// A sequence MCF does not act on is consumed whole.
///
/// If only its head were consumed, its tail would be read as separate key
/// presses — a function key would scroll the list and nobody would know why.
#[test]
fn an_unknown_sequence_is_consumed_whole() {
    let (key, used) = decode(b"\x1b[1;5C").expect("a modified arrow decodes to something");
    assert_eq!(key, Key::Unknown);
    assert_eq!(used, 6, "the tail would have been read as more key presses");

    let (key, used) = decode(b"\x1b[15~").expect("a function key decodes to something");
    assert_eq!(key, Key::Unknown);
    assert!(used >= 4, "consumed {used} of a five-byte sequence");
}

/// Several keys arriving in one read are decoded one at a time.
#[test]
fn a_burst_decodes_in_order() {
    let mut pending: Vec<u8> = b"\x1b[Bq".to_vec();
    let mut seen = Vec::new();
    while let Some((key, used)) = decode(&pending) {
        pending.drain(..used);
        seen.push(key);
    }
    assert_eq!(seen, vec![Key::Down, Key::Character('q')]);
}

/// A sequence that arrives split is not decided on half of it.
///
/// The monitor's read gives up after a second and can return `\x1b` with `[C`
/// still in flight. Answering `Escape` there turned every arrow key into three
/// keys, and the menu did not move.
#[test]
fn a_split_sequence_waits_for_the_rest() {
    assert_eq!(decode(b"\x1b"), None, "a lone escape may still be an arrow");
    assert_eq!(decode(b"\x1b["), None, "half a sequence is not a key");
    // And once the rest arrives it is the arrow it always was.
    assert_eq!(decode(b"\x1b[C"), Some((Key::Right, 3)));
}

/// When nothing more arrives, a lone escape really was the escape key.
#[test]
fn an_escape_that_stays_alone_is_the_escape_key() {
    assert_eq!(flush_incomplete(b"\x1b"), Key::Escape);
    assert_eq!(flush_incomplete(b"\x1b["), Key::Unknown);
    assert_eq!(flush_incomplete(b""), Key::Unknown);
}
