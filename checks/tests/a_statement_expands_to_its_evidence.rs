//! Every generalized statement resolves to what it rests on (B55, B-252,
//! §3.28, §3.15).
//!
//! B55: *generalization happens in the rendering only — the record keeps the
//! precise measurement, and any generalized statement expands on demand into
//! the measurements, conditions and spread behind it.* Its violation is *"41.2
//! vs 38.4 tok/s" offered as a recommendation*, and the other half of the same
//! failure is a generalization with no way back to the numbers.
//!
//! Three things hold it, and each is one edit from not holding it:
//!
//! 1. `mcf log` prints one generalized line an event, and `mcf show` expands
//!    any of them **without leaving the interface** (A22);
//! 2. the expansion is of what the record holds, unfolded rather than
//!    summarized — a nested value printed as one line of JSON would be a
//!    machine's answer given to a person;
//! 3. `null` survives the expansion, because a question asked and unanswered
//!    is not a question nobody asked (A7).
//!
//! A source check rather than a compile-fail harness, for the reason given in
//! `measurement_has_one_way_in.rs`.

// Every item in this file is test code; see the note in `taxonomy_agreement.rs`.
#![allow(clippy::panic)]

/// There is a way from a statement to its evidence, and it is a command.
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

/// The expansion is of what the record holds, and unfolds it.
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

/// An unanswered condition survives the expansion.
///
/// A7: what MCF did not read comes back unknown. An expansion that dropped the
/// nulls would turn *eleven questions, nine unanswered* into *two conditions*,
/// which is the strongest claim in the file made by omission.
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

/// And the record kept full precision to expand *from*.
///
/// B55's second half: the record keeps the precise measurement. A comparison
/// keeps every pair's two raw durations, so the expansion has something to
/// show — a record of verdicts would leave `mcf show` with nothing under the
/// summary but the summary again.
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

/// The source with its documentation comments removed, so that a sentence
/// quoting a forbidden shape is not read as the shape itself.
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
