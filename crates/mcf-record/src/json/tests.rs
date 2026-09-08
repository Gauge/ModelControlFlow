use super::{Value, parse};
use std::collections::BTreeMap;

#[test]
fn scalars_encode_as_the_specification_says() {
    assert_eq!(Value::Null.to_line(), "null");
    assert_eq!(Value::Bool(true).to_line(), "true");
    assert_eq!(Value::Bool(false).to_line(), "false");
    assert_eq!(Value::Integer(0).to_line(), "0");
    assert_eq!(Value::Integer(-42).to_line(), "-42");
    assert_eq!(Value::Integer(i64::MAX).to_line(), "9223372036854775807");
    assert_eq!(Value::Integer(i64::MIN).to_line(), "-9223372036854775808");
    assert_eq!(Value::text("plain").to_line(), "\"plain\"");
}

#[test]
fn strings_escape_exactly_what_the_specification_requires() {
    let cases = [
        ("a\"b", "\"a\\\"b\""),
        ("a\\b", "\"a\\\\b\""),
        ("a\nb", "\"a\\nb\""),
        ("a\rb", "\"a\\rb\""),
        ("a\tb", "\"a\\tb\""),
        ("a\u{08}b", "\"a\\bb\""),
        ("a\u{0c}b", "\"a\\fb\""),
        ("a\u{01}b", "\"a\\u0001b\""),
        ("a\u{1f}b", "\"a\\u001fb\""),
    ];
    for (input, expected) in cases {
        assert_eq!(Value::text(input).to_line(), expected, "{input:?}");
    }
}

#[test]
fn no_encoding_contains_a_raw_newline() {
    let awkward = Value::map([
        ("detail", Value::text("line one\nline two")),
        ("path", Value::text("/tmp/a\rb")),
    ]);
    let line = awkward.to_line();
    assert!(!line.contains('\n'), "{line}");
    assert!(!line.contains('\r'), "{line}");
    assert_eq!(parse(&line), Ok(awkward));
}

#[test]
fn objects_encode_in_key_order() {
    let built_one_way = Value::map([("zulu", Value::Integer(1)), ("alpha", Value::Integer(2))]);
    let built_another = Value::map([("alpha", Value::Integer(2)), ("zulu", Value::Integer(1))]);
    assert_eq!(built_one_way.to_line(), built_another.to_line());
    assert_eq!(built_one_way.to_line(), r#"{"alpha":2,"zulu":1}"#);
}

#[test]
fn nested_structures_round_trip() {
    let record = Value::map([
        ("id", Value::text("obs_1")),
        ("known", Value::Bool(true)),
        ("unknown", Value::Null),
        (
            "machine",
            Value::map([
                ("cores", Value::Integer(16)),
                ("model", Value::text("a processor")),
            ]),
        ),
        (
            "samples",
            Value::List(vec![
                Value::Integer(1),
                Value::Integer(2),
                Value::Integer(3),
            ]),
        ),
    ]);
    assert_eq!(parse(&record.to_line()), Ok(record));
}

#[test]
fn empty_containers_round_trip() {
    for value in [Value::List(Vec::new()), Value::Map(BTreeMap::new())] {
        assert_eq!(parse(&value.to_line()), Ok(value));
    }
}

#[test]
fn text_outside_ascii_round_trips() {
    for input in ["café", "日本語", "🙂", "a\u{0301}"] {
        let value = Value::text(input);
        assert_eq!(parse(&value.to_line()), Ok(value), "{input}");
    }
}

#[test]
fn escapes_this_encoder_does_not_write_are_still_read() {
    assert_eq!(parse(r#""a\/b""#), Ok(Value::text("a/b")));
    assert_eq!(parse(r#""A""#), Ok(Value::text("A")));
    assert_eq!(parse(r#""é""#), Ok(Value::text("é")));
}

#[test]
fn incidental_whitespace_is_accepted_and_not_produced() {
    assert_eq!(
        parse(" { \"a\" : [ 1 , 2 ] } "),
        Ok(Value::map([(
            "a",
            Value::List(vec![Value::Integer(1), Value::Integer(2)])
        )]))
    );
    assert!(
        !Value::map([("a", Value::Integer(1))])
            .to_line()
            .contains(' ')
    );
}

#[test]
fn what_cannot_be_read_says_where_it_stopped() {
    for (input, at) in [
        ("", 0_usize),
        ("{", 1),
        ("{\"a\":", 5),
        ("[1,", 3),
        ("\"unterminated", 13),
        ("tru", 0),
    ] {
        let error = parse(input).expect_err("this is not a value");
        assert_eq!(error.at, at, "{input:?} → {error}");
        assert!(!error.expected.is_empty());
    }
}

#[test]
fn a_number_this_format_does_not_carry_is_kept_as_written() {
    for written in ["1.5", "1e3", "1E+3", "-2.25e-06", "0.0"] {
        let value = parse(written).expect("a number this format does not carry");
        assert_eq!(value, Value::ForeignNumber(written.to_owned()), "{written}");
        assert_eq!(value.as_integer(), None, "{written} read as a quantity");
        assert_eq!(
            value.to_line(),
            written,
            "{written} was not kept as written"
        );
    }
    assert_eq!(
        parse("[1.0]").expect("a list of them"),
        Value::List(vec![Value::ForeignNumber("1.0".to_owned())])
    );
}

#[test]
fn something_that_only_looks_like_a_number_is_refused() {
    for written in ["1.", "1e", "1e+", "1.2.3", ".5", "1ee3"] {
        assert!(parse(written).is_err(), "{written} was read as a number");
    }
}

#[test]
fn an_integer_that_does_not_fit_is_refused() {
    assert!(parse("9223372036854775808").is_err());
    assert!(parse("-9223372036854775809").is_err());
}

#[test]
fn a_duplicate_key_is_refused() {
    let error = parse(r#"{"a":1,"a":2}"#).expect_err("a duplicate key is not readable");
    assert!(error.expected.contains("not already appeared"), "{error}");
}

#[test]
fn a_raw_control_character_in_a_string_is_refused() {
    assert!(parse("\"a\nb\"").is_err());
    assert!(parse("\"a\u{01}b\"").is_err());
}

#[test]
fn an_unpaired_surrogate_is_refused() {
    let error = parse(r#""\ud800""#).expect_err("an unpaired surrogate is not a character");
    assert!(error.expected.contains("surrogate"), "{error}");
}

#[test]
fn trailing_content_is_refused() {
    assert!(parse("1 2").is_err());
    assert!(parse("{} {}").is_err());
}

#[test]
fn accessors_do_not_coerce() {
    let record = Value::map([("n", Value::Integer(7)), ("s", Value::text("7"))]);
    assert_eq!(record.get("n").and_then(Value::as_integer), Some(7));
    assert_eq!(record.get("n").and_then(Value::as_text), None);
    assert_eq!(record.get("s").and_then(Value::as_integer), None);
    assert_eq!(record.get("s").and_then(Value::as_text), Some("7"));
    assert_eq!(record.get("absent"), None);
    assert_eq!(Value::Null.get("anything"), None);
}
