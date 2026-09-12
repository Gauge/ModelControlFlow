use core::fmt;
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Value {
    Null,
    Bool(bool),
    Integer(i64),
    ForeignNumber(String),
    Text(String),
    List(Vec<Value>),
    Map(BTreeMap<String, Value>),
}

impl Value {
    #[must_use]
    pub fn text(value: impl Into<String>) -> Self {
        Self::Text(value.into())
    }

    #[must_use]
    pub fn exact_thousandths(held: mcf_core::configuration::Thousandths) -> Self {
        Self::ForeignNumber(held.to_string())
    }

    #[must_use]
    pub fn map<K: Into<String>>(pairs: impl IntoIterator<Item = (K, Self)>) -> Self {
        Self::Map(
            pairs
                .into_iter()
                .map(|(key, value)| (key.into(), value))
                .collect(),
        )
    }

    #[must_use]
    pub fn get(&self, key: &str) -> Option<&Self> {
        match self {
            Self::Map(entries) => entries.get(key),
            _ => None,
        }
    }

    #[must_use]
    pub fn as_text(&self) -> Option<&str> {
        match self {
            Self::Text(value) => Some(value),
            _ => None,
        }
    }

    #[must_use]
    pub const fn as_integer(&self) -> Option<i64> {
        match self {
            Self::Integer(value) => Some(*value),
            _ => None,
        }
    }

    #[must_use]
    pub const fn as_bool(&self) -> Option<bool> {
        match self {
            Self::Bool(value) => Some(*value),
            _ => None,
        }
    }

    #[must_use]
    pub fn as_list(&self) -> Option<&[Self]> {
        match self {
            Self::List(values) => Some(values),
            _ => None,
        }
    }

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
            Self::ForeignNumber(written) => out.push_str(written),
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    pub at: usize,
    pub expected: &'static str,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "at byte {}: expected {}", self.at, self.expected)
    }
}

impl std::error::Error for ParseError {}

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
    if matches!(peek(bytes, *at), Some(b'.' | b'e' | b'E')) {
        return foreign_number(bytes, at, start);
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

fn foreign_number(bytes: &[u8], at: &mut usize, start: usize) -> Result<Value, ParseError> {
    if peek(bytes, *at) == Some(b'.') {
        *at += 1;
        let fraction_start = *at;
        while matches!(peek(bytes, *at), Some(b'0'..=b'9')) {
            *at += 1;
        }
        if *at == fraction_start {
            return Err(ParseError {
                at: *at,
                expected: "a digit after the decimal point",
            });
        }
    }
    if matches!(peek(bytes, *at), Some(b'e' | b'E')) {
        *at += 1;
        if matches!(peek(bytes, *at), Some(b'+' | b'-')) {
            *at += 1;
        }
        let exponent_start = *at;
        while matches!(peek(bytes, *at), Some(b'0'..=b'9')) {
            *at += 1;
        }
        if *at == exponent_start {
            return Err(ParseError {
                at: *at,
                expected: "a digit in the exponent",
            });
        }
    }
    let text = core::str::from_utf8(bytes.get(start..*at).unwrap_or_default()).map_err(|_| {
        ParseError {
            at: start,
            expected: "a number",
        }
    })?;
    Ok(Value::ForeignNumber(text.to_owned()))
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
