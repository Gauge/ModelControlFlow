//! The record's line format: a small, complete JSON codec.
//!
//! §3.3 requires the record be *structured and machine-readable first, human-
//! readable second*, and D20 makes the journal the record itself. That needs a
//! serialization, and §7.30 makes it a public interface the moment §XIV ships —
//! a format other machines and other versions of MCF have to read.
//!
//! **Why this is written rather than depended on.** B15 admits weight only
//! against a stated cost, and the cost here is unusually legible. The data
//! model is closed: MCF's own records, no user-defined shapes, no dynamic
//! typing, no schema anyone else supplies. A general serialization framework
//! would bring derive macros, a trait hierarchy and a compile-time cost for a
//! generality this format will never use, and it would put a third party in
//! charge of an interface §7.30 makes MCF's to keep stable for ever. What it
//! would buy is correctness, and correctness here is a testable property of
//! about three hundred lines — so it is bought with tests instead (A19).
//!
//! **What it claims.** RFC 8259 JSON, without the parts a record does not
//! need: numbers are integers, because every quantity MCF records is integral
//! by construction ([`Quantity`] requires `Ord`, which is why no floating point
//! reaches a record at all). Anything it cannot represent it refuses rather
//! than approximating — a record that silently rounded would be a record that
//! lied.
//!
//! [`Quantity`]: mcf_core::measurement::Quantity

use core::fmt;
use std::collections::BTreeMap;

/// A JSON value, as a record uses them.
///
/// No floating-point variant, and that is the point rather than an omission:
/// A6's `Quantity` is `Ord`, so every measured value MCF holds is integral, and
/// a format with no way to write a float is a format through which a rounded
/// value cannot travel.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Value {
    /// `null` — which in a record means *unknown*, never *zero* (A7).
    Null,
    /// `true` or `false`.
    Bool(bool),
    /// An integer.
    Integer(i64),
    /// A string.
    Text(String),
    /// An array.
    List(Vec<Value>),
    /// An object. Ordered by key, so that two encodings of one record are the
    /// same bytes — which is what lets a record be checksummed and compared
    /// (§3.12).
    Map(BTreeMap<String, Value>),
}

impl Value {
    /// A string value.
    #[must_use]
    pub fn text(value: impl Into<String>) -> Self {
        Self::Text(value.into())
    }

    /// An object, from pairs.
    #[must_use]
    pub fn map<K: Into<String>>(pairs: impl IntoIterator<Item = (K, Self)>) -> Self {
        Self::Map(
            pairs
                .into_iter()
                .map(|(key, value)| (key.into(), value))
                .collect(),
        )
    }

    /// The value at a key, if this is an object that has one.
    #[must_use]
    pub fn get(&self, key: &str) -> Option<&Self> {
        match self {
            Self::Map(entries) => entries.get(key),
            _ => None,
        }
    }

    /// The string, if this is one.
    #[must_use]
    pub fn as_text(&self) -> Option<&str> {
        match self {
            Self::Text(value) => Some(value),
            _ => None,
        }
    }

    /// The integer, if this is one.
    #[must_use]
    pub const fn as_integer(&self) -> Option<i64> {
        match self {
            Self::Integer(value) => Some(*value),
            _ => None,
        }
    }

    /// The elements, if this is an array.
    #[must_use]
    pub fn as_list(&self) -> Option<&[Self]> {
        match self {
            Self::List(values) => Some(values),
            _ => None,
        }
    }

    /// Writes the value as one line of JSON, with no insignificant whitespace.
    ///
    /// One line because the journal is line-delimited: a torn write is then a
    /// torn *line*, which replay can identify and report rather than being
    /// unable to find the boundary at all (B62).
    #[must_use]
    pub fn to_line(&self) -> String {
        let mut out = String::new();
        self.write(&mut out);
        out
    }

    fn write(&self, out: &mut String) {
        match self {
            Self::Null => out.push_str("null"),
            Self::Bool(true) => out.push_str("true"),
            Self::Bool(false) => out.push_str("false"),
            Self::Integer(value) => out.push_str(&value.to_string()),
            Self::Text(value) => write_string(value, out),
            Self::List(values) => {
                out.push('[');
                for (position, value) in values.iter().enumerate() {
                    if position > 0 {
                        out.push(',');
                    }
                    value.write(out);
                }
                out.push(']');
            }
            Self::Map(entries) => {
                out.push('{');
                for (position, (key, value)) in entries.iter().enumerate() {
                    if position > 0 {
                        out.push(',');
                    }
                    write_string(key, out);
                    out.push(':');
                    value.write(out);
                }
                out.push('}');
            }
        }
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.to_line())
    }
}

/// Writes a JSON string, escaping exactly what RFC 8259 requires.
///
/// Control characters below `0x20` are escaped, because a raw one inside a
/// string is invalid JSON and — more to the point here — a raw newline would
/// end the journal line early and turn one record into two unreadable ones.
fn write_string(value: &str, out: &mut String) {
    out.push('"');
    for character in value.chars() {
        match character {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{08}' => out.push_str("\\b"),
            '\u{0c}' => out.push_str("\\f"),
            control if control < '\u{20}' => {
                // The four hexadecimal digits, written out. `write!` into a
                // `String` cannot fail, and `format!` would allocate a second
                // one for four characters.
                const HEX: [u8; 16] = *b"0123456789abcdef";
                let code = u32::from(control);
                out.push_str("\\u00");
                for shift in [4_u32, 0] {
                    let nibble = (code >> shift) & 0xF;
                    let digit = HEX
                        .get(usize::try_from(nibble).unwrap_or(0))
                        .copied()
                        .unwrap_or(b'0');
                    out.push(char::from(digit));
                }
            }
            other => out.push(other),
        }
    }
    out.push('"');
}

/// What a line could not be read as, and where.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    /// The byte offset within the line.
    pub at: usize,
    /// What was expected there.
    pub expected: &'static str,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "at byte {}: expected {}", self.at, self.expected)
    }
}

impl std::error::Error for ParseError {}

/// Reads one JSON value.
///
/// # Errors
///
/// Returns a [`ParseError`] naming the offset and what was expected. A2: a line
/// that cannot be read says where it stopped, so replay can report the exact
/// extent of what it could not recover (B62) rather than only that something
/// was lost.
pub fn parse(text: &str) -> Result<Value, ParseError> {
    let bytes = text.as_bytes();
    let mut at = 0;
    let value = parse_value(bytes, &mut at)?;
    skip_space(bytes, &mut at);
    if at != bytes.len() {
        return Err(ParseError {
            at,
            expected: "end of input",
        });
    }
    Ok(value)
}

fn parse_value(bytes: &[u8], at: &mut usize) -> Result<Value, ParseError> {
    skip_space(bytes, at);
    match peek(bytes, *at) {
        Some(b'n') => literal(bytes, at, b"null", Value::Null),
        Some(b't') => literal(bytes, at, b"true", Value::Bool(true)),
        Some(b'f') => literal(bytes, at, b"false", Value::Bool(false)),
        Some(b'"') => parse_string(bytes, at).map(Value::Text),
        Some(b'[') => parse_list(bytes, at),
        Some(b'{') => parse_map(bytes, at),
        Some(b'-' | b'0'..=b'9') => parse_integer(bytes, at),
        _ => Err(ParseError {
            at: *at,
            expected: "a value",
        }),
    }
}

fn literal(
    bytes: &[u8],
    at: &mut usize,
    word: &'static [u8],
    value: Value,
) -> Result<Value, ParseError> {
    if bytes.len() < *at + word.len() || bytes.get(*at..*at + word.len()) != Some(word) {
        return Err(ParseError {
            at: *at,
            expected: "null, true or false",
        });
    }
    *at += word.len();
    Ok(value)
}

fn parse_integer(bytes: &[u8], at: &mut usize) -> Result<Value, ParseError> {
    let start = *at;
    if peek(bytes, *at) == Some(b'-') {
        *at += 1;
    }
    let digits_start = *at;
    while matches!(peek(bytes, *at), Some(b'0'..=b'9')) {
        *at += 1;
    }
    if *at == digits_start {
        return Err(ParseError {
            at: start,
            expected: "an integer",
        });
    }
    // A fractional or exponent part is refused rather than rounded. Nothing
    // MCF writes has one, so a line that does was not written by MCF, and
    // reading it as an approximation would be inventing a value (A7).
    if matches!(peek(bytes, *at), Some(b'.' | b'e' | b'E')) {
        return Err(ParseError {
            at: *at,
            expected: "an integer, and this is not one",
        });
    }
    let text = core::str::from_utf8(bytes.get(start..*at).unwrap_or_default()).map_err(|_| {
        ParseError {
            at: start,
            expected: "an integer",
        }
    })?;
    text.parse::<i64>()
        .map(Value::Integer)
        .map_err(|_| ParseError {
            at: start,
            expected: "an integer that fits",
        })
}

fn parse_string(bytes: &[u8], at: &mut usize) -> Result<String, ParseError> {
    if peek(bytes, *at) != Some(b'"') {
        return Err(ParseError {
            at: *at,
            expected: "a string",
        });
    }
    *at += 1;
    let mut out = String::new();
    loop {
        let Some(byte) = peek(bytes, *at) else {
            return Err(ParseError {
                at: *at,
                expected: "a closing quote",
            });
        };
        *at += 1;
        match byte {
            b'"' => return Ok(out),
            b'\\' => out.push(parse_escape(bytes, at)?),
            other if other < 0x20 => {
                return Err(ParseError {
                    at: *at - 1,
                    expected: "an escaped control character",
                });
            }
            _ => {
                // Multi-byte UTF-8: take the whole sequence from the original
                // text rather than pushing a byte, which would split it.
                let start = *at - 1;
                let length = utf8_length(byte);
                let Some(slice) = bytes.get(start..start + length) else {
                    return Err(ParseError {
                        at: start,
                        expected: "a complete UTF-8 sequence",
                    });
                };
                let Ok(text) = core::str::from_utf8(slice) else {
                    return Err(ParseError {
                        at: start,
                        expected: "valid UTF-8",
                    });
                };
                out.push_str(text);
                *at = start + length;
            }
        }
    }
}

const fn utf8_length(first: u8) -> usize {
    if first < 0x80 {
        1
    } else if first < 0xE0 {
        2
    } else if first < 0xF0 {
        3
    } else {
        4
    }
}

fn parse_escape(bytes: &[u8], at: &mut usize) -> Result<char, ParseError> {
    let Some(byte) = peek(bytes, *at) else {
        return Err(ParseError {
            at: *at,
            expected: "an escape",
        });
    };
    *at += 1;
    Ok(match byte {
        b'"' => '"',
        b'\\' => '\\',
        b'/' => '/',
        b'n' => '\n',
        b'r' => '\r',
        b't' => '\t',
        b'b' => '\u{08}',
        b'f' => '\u{0c}',
        b'u' => return parse_unicode_escape(bytes, at),
        _ => {
            return Err(ParseError {
                at: *at - 1,
                expected: "a known escape",
            });
        }
    })
}

fn parse_unicode_escape(bytes: &[u8], at: &mut usize) -> Result<char, ParseError> {
    let start = *at;
    let Some(slice) = bytes.get(start..start + 4) else {
        return Err(ParseError {
            at: start,
            expected: "four hexadecimal digits",
        });
    };
    let Ok(text) = core::str::from_utf8(slice) else {
        return Err(ParseError {
            at: start,
            expected: "four hexadecimal digits",
        });
    };
    let Ok(code) = u32::from_str_radix(text, 16) else {
        return Err(ParseError {
            at: start,
            expected: "four hexadecimal digits",
        });
    };
    *at = start + 4;
    // Surrogate halves are refused rather than replaced. MCF never writes one,
    // and substituting the replacement character would be a silent alteration
    // of evidence.
    char::from_u32(code).ok_or(ParseError {
        at: start,
        expected: "a character, not an unpaired surrogate",
    })
}

fn parse_list(bytes: &[u8], at: &mut usize) -> Result<Value, ParseError> {
    *at += 1;
    let mut values = Vec::new();
    skip_space(bytes, at);
    if peek(bytes, *at) == Some(b']') {
        *at += 1;
        return Ok(Value::List(values));
    }
    loop {
        values.push(parse_value(bytes, at)?);
        skip_space(bytes, at);
        match peek(bytes, *at) {
            Some(b',') => *at += 1,
            Some(b']') => {
                *at += 1;
                return Ok(Value::List(values));
            }
            _ => {
                return Err(ParseError {
                    at: *at,
                    expected: "a comma or a closing bracket",
                });
            }
        }
    }
}

fn parse_map(bytes: &[u8], at: &mut usize) -> Result<Value, ParseError> {
    *at += 1;
    let mut entries = BTreeMap::new();
    skip_space(bytes, at);
    if peek(bytes, *at) == Some(b'}') {
        *at += 1;
        return Ok(Value::Map(entries));
    }
    loop {
        skip_space(bytes, at);
        let key = parse_string(bytes, at)?;
        skip_space(bytes, at);
        if peek(bytes, *at) != Some(b':') {
            return Err(ParseError {
                at: *at,
                expected: "a colon",
            });
        }
        *at += 1;
        let value = parse_value(bytes, at)?;
        // A duplicate key is refused. RFC 8259 permits it and leaves the
        // meaning to the reader, which is exactly the kind of ambiguity a
        // record cannot carry: two readers would disagree about what the
        // record says.
        if entries.insert(key, value).is_some() {
            return Err(ParseError {
                at: *at,
                expected: "a key that has not already appeared",
            });
        }
        skip_space(bytes, at);
        match peek(bytes, *at) {
            Some(b',') => *at += 1,
            Some(b'}') => {
                *at += 1;
                return Ok(Value::Map(entries));
            }
            _ => {
                return Err(ParseError {
                    at: *at,
                    expected: "a comma or a closing brace",
                });
            }
        }
    }
}

fn skip_space(bytes: &[u8], at: &mut usize) {
    while matches!(peek(bytes, *at), Some(b' ' | b'\t' | b'\n' | b'\r')) {
        *at += 1;
    }
}

fn peek(bytes: &[u8], at: usize) -> Option<u8> {
    bytes.get(at).copied()
}

#[cfg(test)]
mod tests;
