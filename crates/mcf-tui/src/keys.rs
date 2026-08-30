//! What the operator pressed, from the bytes the terminal sent.
//!
//! A terminal reports a key press as one byte, or as an escape sequence of
//! several, and the sequences differ between terminals in their tail rather
//! than their head. So this decodes what it recognises and reports
//! [`Key::Unknown`] for the rest — a key MCF does not act on is not an error,
//! and guessing at one would make an arrow key do something arbitrary.

/// A key, as the application thinks about it.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Key {
    /// Cursor up.
    Up,
    /// Cursor down.
    Down,
    /// Cursor left.
    Left,
    /// Cursor right.
    Right,
    /// Return or enter.
    Enter,
    /// The escape key alone, not the start of a sequence.
    Escape,
    /// Ctrl-C. Delivered as a key rather than a signal, because the terminal
    /// has to be given back before the process ends.
    Interrupt,
    /// A printable character.
    Character(char),
    /// Something the terminal sent that MCF does not act on. Not an error: a
    /// key with no meaning here is a key with no meaning here (A7).
    Unknown,
}

/// Decodes one key from a buffer, returning it and how many bytes it used.
///
/// `None` where the buffer holds the beginning of a sequence but not the end,
/// so the caller reads more rather than deciding on half of one.
#[must_use]
pub fn decode(bytes: &[u8]) -> Option<(Key, usize)> {
    let first = *bytes.first()?;
    match first {
        0x03 => Some((Key::Interrupt, 1)),
        b'\r' | b'\n' => Some((Key::Enter, 1)),
        0x1b => {
            if bytes.len() == 1 {
                // Escape alone, or the start of a sequence whose rest has not
                // arrived. The terminal sends the whole sequence in one write,
                // so a lone escape after a complete read is a lone escape.
                return Some((Key::Escape, 1));
            }
            if bytes.get(1) == Some(&b'[') {
                return match bytes.get(2) {
                    Some(b'A') => Some((Key::Up, 3)),
                    Some(b'B') => Some((Key::Down, 3)),
                    Some(b'C') => Some((Key::Right, 3)),
                    Some(b'D') => Some((Key::Left, 3)),
                    // A longer sequence — a function key, a modifier, a mouse
                    // report. Consumed to its terminating letter so that its
                    // tail is not read as separate key presses.
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
        // Printable ASCII. Anything above is the start of a UTF-8 sequence,
        // which no key MCF acts on produces, so it is consumed as one byte and
        // reported unknown rather than decoded.
        0x20..=0x7e => Some((Key::Character(first as char), 1)),
        _ => Some((Key::Unknown, 1)),
    }
}

#[cfg(test)]
mod tests;
