#![allow(clippy::panic, clippy::expect_used)]

use std::path::{Path, PathBuf};

use mcf_record::json::Value;

const THE_READER: &str = "crates/mcf-record/src/json.rs";

const THE_ENGINE_REQUEST: [&str; 2] = [
    "crates/mcf-serve/src/served.rs",
    "crates/mcf-optimize/src/trial.rs",
];

#[test]
fn nothing_but_the_reader_constructs_a_number_the_record_cannot_carry() {
    let root = mcf_checks::workspace::root();
    let mut found = Vec::new();
    for file in sources(&root) {
        let relative = file
            .strip_prefix(&root)
            .unwrap_or(&file)
            .display()
            .to_string();
        if relative == THE_READER || relative.starts_with("crates/mcf-record/src/json/") {
            continue;
        }
        let text = std::fs::read_to_string(&file).unwrap_or_default();
        for (number, line) in text.lines().enumerate() {
            let code = line.split("//").next().unwrap_or_default();
            if code.contains("ForeignNumber(") && !code.contains("Value::ForeignNumber(_)") {
                found.push(format!("{relative}:{}", number.saturating_add(1)));
            }
            if code.contains("exact_thousandths(") && !THE_ENGINE_REQUEST.contains(&relative.as_str()) {
                found.push(format!("{relative}:{}", number.saturating_add(1)));
            }
        }
    }
    assert_eq!(
        found,
        Vec::<String>::new(),
        "something outside {THE_READER} constructs a number the record cannot carry. \
         A6 makes every recorded quantity `Ord`, and a value nobody can order is a value \
         that must not reach the record"
    );
}

#[test]
fn a_number_the_record_cannot_carry_is_not_a_quantity() {
    let value = mcf_record::json::parse("1e-06").expect("a number this format does not carry");
    assert_eq!(value.as_integer(), None);
    assert_eq!(value.as_text(), None);
    assert_eq!(value.as_list(), None);
    assert_eq!(value.to_line(), "1e-06", "it is not kept as written");
}

#[test]
fn a_record_mcf_wrote_holds_no_number_it_cannot_carry() {
    let machine = mcf_core::hardware::Machine::read();
    let entries = [
        mcf_record::encode::machine(&machine),
        mcf_record::encode::failure(&mcf_core::failure::Failure::new(
            mcf_core::failure::Category::ConfigInvalid,
            mcf_core::failure::Attribution::User,
            mcf_core::failure::Disposition::Refused,
            mcf_core::failure::Subsystem::new("a check"),
            "something went wrong",
        )),
    ];
    for entry in entries {
        let line = entry.to_line();
        let read = mcf_record::json::parse(&line).expect("MCF's own encoding reads back");
        assert!(
            integral(&read),
            "an encoder produced a number the record cannot carry: {line}"
        );
    }
}

fn integral(value: &Value) -> bool {
    match value {
        Value::ForeignNumber(_) => false,
        Value::List(values) => values.iter().all(integral),
        Value::Map(pairs) => pairs.values().all(integral),
        Value::Null | Value::Bool(_) | Value::Integer(_) | Value::Text(_) => true,
    }
}

fn sources(root: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    walk(&root.join("crates"), &mut found);
    found
}

fn walk(directory: &Path, into: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(directory) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            walk(&path, into);
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            into.push(path);
        }
    }
}
