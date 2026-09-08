use super::*;

#[test]
fn an_event_is_read_at_the_offsets_sdl_documents() {
    let mut event = [0_u8; EVENT_BYTES];
    event[..4].copy_from_slice(&EVENT_KEY_DOWN.to_ne_bytes());
    event[KEY_OFFSET..KEY_OFFSET + 4].copy_from_slice(&KEY_RIGHT.to_ne_bytes());
    assert_eq!(event_type(&event), EVENT_KEY_DOWN);
    assert_eq!(event_key(&event), KEY_RIGHT);
}

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

#[test]
fn the_keys_are_the_codes_sdl_sends() {
    for key in [KEY_LEFT, KEY_RIGHT, KEY_UP, KEY_DOWN] {
        assert_eq!(key & KEY_MASK, KEY_MASK, "{key:#x} is missing the mask");
    }
    for key in [KEY_RETURN, KEY_ESCAPE] {
        assert_eq!(key & KEY_MASK, 0, "{key:#x} should be a plain character");
    }
    let all = [
        KEY_LEFT, KEY_RIGHT, KEY_UP, KEY_DOWN, KEY_RETURN, KEY_ESCAPE,
    ];
    for (index, key) in all.iter().enumerate() {
        for other in all.iter().skip(index + 1) {
            assert_ne!(key, other);
        }
    }
}

#[test]
#[cfg(not(have_sdl))]
fn a_missing_library_refuses_with_what_to_do() {
    let refused = Window::open("MCF", 100, 100).expect_err("there is no library");
    assert!(refused.contains("provision"), "{refused}");
}

#[test]
fn the_modifiers_are_read_where_sdl_puts_them() {
    let mut event = [0_u8; EVENT_BYTES];
    event[..4].copy_from_slice(&EVENT_KEY_DOWN.to_ne_bytes());
    event[KEY_OFFSET..KEY_OFFSET + 4].copy_from_slice(&u32::from(b'v').to_ne_bytes());

    assert!(
        !event_has_ctrl(&event),
        "an unmodified v is a letter, not a paste"
    );

    for held in [0x0040_u16, 0x0080_u16] {
        event[MOD_OFFSET..MOD_OFFSET + 2].copy_from_slice(&held.to_ne_bytes());
        assert!(event_has_ctrl(&event), "{held:#06x} is a Ctrl key");
    }

    event[MOD_OFFSET..MOD_OFFSET + 2].copy_from_slice(&0x0001_u16.to_ne_bytes());
    assert!(!event_has_ctrl(&event), "shift is not ctrl");

    assert_eq!(event_key(&event), u32::from(b'v'));
}

#[test]
fn the_modifier_read_is_bounded() {
    let empty = [0_u8; EVENT_BYTES];
    assert_eq!(event_mod(&empty), 0);
    assert!(!event_has_ctrl(&empty));
}
