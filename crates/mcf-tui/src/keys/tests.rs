use super::*;

#[test]
fn the_keys_the_application_acts_on_decode() {
    assert_eq!(decode(b"q"), Some((Key::Character('q'), 1)));
    assert_eq!(decode(b"\x03"), Some((Key::Interrupt, 1)));
    assert_eq!(decode(b"\r"), Some((Key::Enter, 1)));
    assert_eq!(decode(b"\t"), Some((Key::Tab, 1)));
    assert_eq!(decode(b"\x1b[A"), Some((Key::Up, 3)));
    assert_eq!(decode(b"\x1b[B"), Some((Key::Down, 3)));
    assert_eq!(decode(b"\x1b"), None);
    assert_eq!(decode(b""), None);
}

#[test]
fn an_unknown_sequence_is_consumed_whole() {
    let (key, used) = decode(b"\x1b[1;5C").expect("a modified arrow decodes to something");
    assert_eq!(key, Key::Unknown);
    assert_eq!(used, 6, "the tail would have been read as more key presses");

    let (key, used) = decode(b"\x1b[15~").expect("a function key decodes to something");
    assert_eq!(key, Key::Unknown);
    assert!(used >= 4, "consumed {used} of a five-byte sequence");
}

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

#[test]
fn a_split_sequence_waits_for_the_rest() {
    assert_eq!(decode(b"\x1b"), None, "a lone escape may still be an arrow");
    assert_eq!(decode(b"\x1b["), None, "half a sequence is not a key");
    assert_eq!(decode(b"\x1b[C"), Some((Key::Right, 3)));
}

#[test]
fn an_escape_that_stays_alone_is_the_escape_key() {
    assert_eq!(flush_incomplete(b"\x1b"), Key::Escape);
    assert_eq!(flush_incomplete(b"\x1b["), Key::Unknown);
    assert_eq!(flush_incomplete(b""), Key::Unknown);
}
