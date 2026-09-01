use super::*;

/// The two fields MCF reads out of an event are read from the right places.
///
/// An `SDL_Event` is a union and MCF describes two offsets of it rather than
/// the whole shape. If either moved, every key would be the wrong key — so the
/// offsets are pinned here against a buffer laid out by hand.
#[test]
fn an_event_is_read_at_the_offsets_sdl_documents() {
    let mut event = [0_u8; EVENT_BYTES];
    event[..4].copy_from_slice(&EVENT_KEY_DOWN.to_ne_bytes());
    event[KEY_OFFSET..KEY_OFFSET + 4].copy_from_slice(&KEY_RIGHT.to_ne_bytes());
    assert_eq!(event_type(&event), EVENT_KEY_DOWN);
    assert_eq!(event_key(&event), KEY_RIGHT);
}

/// The keycode is where SDL documents it, and reading past the buffer gives
/// nothing rather than a panic.
///
/// An `SDL_Event` is a union and MCF describes two offsets into it. If either
/// moved, every key would be the wrong key.
#[test]
fn the_event_buffer_holds_a_whole_event() {
    let empty = [0_u8; EVENT_BYTES];
    assert_eq!(event_key(&empty), 0, "an empty event has no key");
    let mut filled = [0_u8; EVENT_BYTES];
    filled[KEY_OFFSET..KEY_OFFSET + 4].copy_from_slice(&KEY_UP.to_ne_bytes());
    assert_eq!(
        event_key(&filled),
        KEY_UP,
        "the keycode is not where SDL documents it"
    );
}

/// Special keys carry the mask; printable ones are themselves.
#[test]
fn the_keys_are_the_codes_sdl_sends() {
    for key in [KEY_LEFT, KEY_RIGHT, KEY_UP, KEY_DOWN] {
        assert_eq!(key & KEY_MASK, KEY_MASK, "{key:#x} is missing the mask");
    }
    for key in [KEY_RETURN, KEY_ESCAPE] {
        assert_eq!(key & KEY_MASK, 0, "{key:#x} should be a plain character");
    }
    // And they are distinct, or two arrows would do one thing.
    let all = [
        KEY_LEFT, KEY_RIGHT, KEY_UP, KEY_DOWN, KEY_RETURN, KEY_ESCAPE,
    ];
    for (index, key) in all.iter().enumerate() {
        for other in all.iter().skip(index + 1) {
            assert_ne!(key, other);
        }
    }
}

/// Without a provisioned library the window refuses, and says what to run.
///
/// A crate that would not build without it is a crate that breaks the build for
/// everybody who has not provisioned it; a window that fails with no reason is
/// worse than one that does not open.
#[test]
#[cfg(not(have_sdl))]
fn a_missing_library_refuses_with_what_to_do() {
    let refused = Window::open("MCF", 100, 100).expect_err("there is no library");
    assert!(refused.contains("provision"), "{refused}");
}

/// The modifier bits are read from where SDL puts them, and Ctrl is Ctrl.
///
/// The keycode alone cannot tell `v` from Ctrl+V, so a paste is a keycode and
/// a modifier read together. `mod` is a `Uint16` and sits after the keycode —
/// read with `offsetof` against the provisioned header, like every constant
/// here, because a modifier read four bytes wide or two bytes late would make
/// every paste either silent or unconditional.
#[test]
fn the_modifiers_are_read_where_sdl_puts_them() {
    let mut event = [0_u8; EVENT_BYTES];
    event[..4].copy_from_slice(&EVENT_KEY_DOWN.to_ne_bytes());
    event[KEY_OFFSET..KEY_OFFSET + 4].copy_from_slice(&u32::from(b'v').to_ne_bytes());

    assert!(
        !event_has_ctrl(&event),
        "an unmodified v is a letter, not a paste"
    );

    // The left Ctrl key alone, then the right one alone: either is Ctrl.
    for held in [0x0040_u16, 0x0080_u16] {
        event[MOD_OFFSET..MOD_OFFSET + 2].copy_from_slice(&held.to_ne_bytes());
        assert!(event_has_ctrl(&event), "{held:#06x} is a Ctrl key");
    }

    // Shift is not Ctrl, and must not be mistaken for it.
    event[MOD_OFFSET..MOD_OFFSET + 2].copy_from_slice(&0x0001_u16.to_ne_bytes());
    assert!(!event_has_ctrl(&event), "shift is not ctrl");

    // The modifier sits past the keycode: writing one leaves the other alone.
    assert_eq!(event_key(&event), u32::from(b'v'));
}

/// Reading modifiers past the end of the buffer gives nothing, not a panic.
#[test]
fn the_modifier_read_is_bounded() {
    let empty = [0_u8; EVENT_BYTES];
    assert_eq!(event_mod(&empty), 0);
    assert!(!event_has_ctrl(&empty));
}
