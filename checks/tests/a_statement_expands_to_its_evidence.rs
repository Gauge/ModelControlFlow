#![allow(clippy::panic)]

#[test]
fn a_statement_can_be_expanded_from_the_interface() {
    let source = code_only(&read("crates/mcf-cli/src/main.rs"));
    assert!(
        source.contains("Request::Show { id } => show::run(id),"),
        "there must be a command that expands one recorded entry (B55, B-252, A22)"
    );
    let usage = read("crates/mcf-cli/src/main.rs");
    assert!(
        usage.contains("mcf show <entry-id>"),
        "and it must be advertised, or it is a surface nobody can find (§3.9)"
    );
}

#[test]
fn the_expansion_unfolds_rather_than_summarizes() {
    let source = code_only(&read("crates/mcf-cli/src/show.rs"));
    assert!(
        source.contains("fn unfolded(value: &Value, depth: usize) -> Vec<String>"),
        "a nested value must be unfolded, not printed as one line of JSON"
    );
    for forbidden in ["fn abbreviate", "fn summarise", ".truncate(", "…\")"] {
        assert!(
            !source.contains(forbidden),
            "`{forbidden}` would summarize the evidence, which is the thing being expanded (B55)"
        );
    }
}

#[test]
fn an_unanswered_condition_survives_the_expansion() {
    let source = code_only(&read("crates/mcf-cli/src/show.rs"));
    assert!(
        source.contains("Value::Null => \"— not answered\""),
        "`null` must be printed rather than skipped (A7, §3.4)"
    );
    assert!(
        !source.contains("Value::Null => String::new()"),
        "a blank where a condition was asked and unanswered is an omission, not a rendering"
    );
}

#[test]
fn the_record_kept_what_there_is_to_expand() {
    let source = code_only(&read("crates/mcf-bench/src/record.rs"));
    for kept in ["left_ns", "right_ns", "left_position", "drew", "conditions"] {
        assert!(
            source.contains(kept),
            "the record must keep `{kept}`, or there is nothing under the summary (B55, B56)"
        );
    }
}

fn code_only(source: &str) -> String {
    source
        .lines()
        .filter(|line| {
            let trimmed = line.trim_start();
            !trimmed.starts_with("//!") && !trimmed.starts_with("///") && !trimmed.starts_with("//")
        })
        .collect::<Vec<&str>>()
        .join("\n")
}

fn read(relative: &str) -> String {
    let path = mcf_checks::workspace::root().join(relative);
    std::fs::read_to_string(&path).unwrap_or_else(|error| {
        panic!("{} is readable: {error}", path.display());
    })
}
