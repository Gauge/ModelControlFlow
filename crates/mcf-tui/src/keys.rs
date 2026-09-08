#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Key {
    Up,
    Down,
    Left,
    Right,
    Enter,
    Tab,
    Escape,
    Interrupt,
    Character(char),
    Unknown,
}

#[must_use]
pub fn decode(bytes: &[u8]) -> Option<(Key, usize)> {
    let first = *bytes.first()?;
    match first {
        0x03 => Some((Key::Interrupt, 1)),
        b'\r' | b'\n' => Some((Key::Enter, 1)),
        b'\t' => Some((Key::Tab, 1)),
        0x1b => {
            if bytes.len() == 1 || (bytes.len() == 2 && bytes.get(1) == Some(&b'[')) {
                return None;
            }
            if bytes.get(1) == Some(&b'[') {
                return match bytes.get(2) {
                    Some(b'A') => Some((Key::Up, 3)),
                    Some(b'B') => Some((Key::Down, 3)),
                    Some(b'C') => Some((Key::Right, 3)),
                    Some(b'D') => Some((Key::Left, 3)),
                    Some(_) => {
                        let end = bytes
                            .iter()
                            .skip(2)
                            .position(u8::is_ascii_alphabetic)
                            .map_or(bytes.len(), |at| at + 3);
                        Some((Key::Unknown, end))
                    }
                    None => None,
                };
            }
            Some((Key::Unknown, 2))
        }
        0x20..=0x7e => Some((Key::Character(first as char), 1)),
        _ => Some((Key::Unknown, 1)),
    }
}

#[must_use]
pub fn flush_incomplete(bytes: &[u8]) -> Key {
    if bytes == [0x1b] {
        Key::Escape
    } else {
        Key::Unknown
    }
}

#[cfg(test)]
mod tests;
