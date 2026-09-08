#![allow(clippy::panic)]

#[test]
fn the_header_is_computed_rather_than_asserted() {
    let source = code_only(&read("crates/mcf-record/src/export.rs"));
    assert!(
        source.contains(r#"("contains_user_content", Value::Bool(carries_written_text))"#),
        "the header must be computed from what is carried (A24, A25)"
    );
    assert!(
        !source.contains(r#"("contains_user_content", Value::Bool(false))"#),
        "a constant `false` is a claim about entries nobody looked at"
    );
    assert!(
        source.contains("fn holds_written_text(entry: &Value) -> bool"),
        "and there must be a place that looks"
    );
}

#[test]
fn every_place_content_reaches_the_record_is_named() {
    let named = code_only(&read("crates/mcf-record/src/export.rs"));
    for path in [
        r#"&["body", "method", "prompt"]"#,
        r#"&["body", "prompt"]"#,
        r#"&["body", "text"]"#,
    ] {
        assert!(
            named.contains(path),
            "the header's computation does not name {path}, so a record that holds content \
             there is exported as one that holds none (A24, A25, F105)"
        );
    }
}

#[test]
fn nothing_writes_content_into_the_record_any_more() {
    for (path, gone, instead) in [
        (
            "crates/mcf-bench/src/record.rs",
            r#"("prompt", Value::text(held.prompt.clone()))"#,
            "prompt_digest",
        ),
        (
            "crates/mcf-serve/src/generation.rs",
            r#"("text", Value::text("#,
            "text_bytes",
        ),
    ] {
        let source = code_only(&read(path));
        assert!(
            !source.contains(gone),
            "{path} writes content into a record entry again: `{gone}` (A25, F105)"
        );
        assert!(
            source.contains(instead),
            "{path} no longer records `{instead}`, so the record has stopped saying how much \
             was said — the measurement about content that A6 wants and A25 permits"
        );
    }
}

#[test]
fn the_surface_says_what_leaves_with_it() {
    let source = read("crates/mcf-cli/src/bundle.rs");
    for said in [
        "what leaves with it, if you send it",
        "the prompt both arms were asked, which is text you wrote",
        "Writing this file is not sending it",
        "the prompt is beside it in",
    ] {
        assert!(
            source.contains(said),
            "the bundle surface must say `{said}` before anything leaves (A24, §3.20)"
        );
    }
}

#[test]
fn what_is_missing_is_said() {
    let source = read("crates/mcf-cli/src/bundle.rs");
    assert!(
        source.contains("NOT in it"),
        "a bundle must say what it could not carry"
    );
    assert!(
        source.contains("no machine profile was recorded"),
        "and name it, rather than leaving a gap in a list"
    );
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
