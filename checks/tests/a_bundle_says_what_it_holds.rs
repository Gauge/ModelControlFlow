//! A bundle's header is true about what is in it (B-211, PR2, A24, A25,
//! §3.20).
//!
//! **The claim that stopped being true.** `contains_user_content` was a
//! constant `false`, on the reasoning that this module reads the journal and
//! the journal is not the content store — which was correct until a comparison
//! began recording the prompt both arms were asked as part of its method
//! (PR2). The journal is still not the content store; the *method* of a
//! measurement is text the operator wrote, and it has to travel or the bundle
//! cannot reproduce anything.
//!
//! A `false` that is sometimes wrong is worse than no field: a reader deciding
//! whether to send a file is entitled to know what leaves with it (A24), and
//! this is the field they will read.
//!
//! **What this checks.** That the header is computed from the entries rather
//! than asserted, and that every place a surface writes operator text into the
//! record is named in the list the computation walks. The second is the one
//! that rots: a field added later that holds written text and is not named
//! would make the header lie again, quietly.
//!
//! **And it did, for the half of A25 this file did not ask about** (F105). The
//! rule says *what a user typed and what a model generated*; the list named the
//! prompt and never the completion, so 4 019 entries holding what a model said
//! were carried by bundles whose header answered `contains_user_content:
//! false`. The guard was written the day a *prompt* reached the record and it
//! covered the shape of that day — the same lesson F103 drew one level out.
//! `body.text` is named now, and this file asks about both halves.
//!
//! A source check rather than a compile-fail harness, for the reason given in
//! `measurement_has_one_way_in.rs`.

// Every item in this file is test code; see the note in `taxonomy_agreement.rs`.
#![allow(clippy::panic)]

/// The header says what the bundle holds, not what the module hopes it holds.
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

/// Every path by which content ever reached the record is named.
///
/// The list is short and the check is that it is complete: a field carrying a
/// prompt or a completion that the header's computation does not know about is
/// a bundle telling somebody it is safe to send.
///
/// **Both halves of A25.** What a person typed *and* what a model generated —
/// the second is the one that was missing, and is why this is no longer named
/// for the operator's half alone (F105).
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

/// And no shipped surface writes content into an entry any more.
///
/// The paths above are *legacy shapes*: a record written before F105 holds them
/// and does not change (A1). What must not happen is a new one. The two places
/// that did are named with what they write instead, so that putting either back
/// fails here rather than three months later in somebody's export.
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

/// The surface that writes a bundle says what leaves with it, before it does.
///
/// A24 gates *publication*, and writing a file to a path the operator named is
/// not that — so producing a bundle is not gated. What the surface owes is the
/// other half: the list a person should read before they send it.
#[test]
fn the_surface_says_what_leaves_with_it() {
    let source = read("crates/mcf-cli/src/bundle.rs");
    for said in [
        "what leaves with it, if you send it",
        "the prompt both arms were asked, which is text you wrote",
        "Writing this file is not sending it",
        // The prompt is not in the record any more, so it travels beside the
        // bundle rather than in it, and the report says which file (F105).
        "the prompt is beside it in",
    ] {
        assert!(
            source.contains(said),
            "the bundle surface must say `{said}` before anything leaves (A24, §3.20)"
        );
    }
}

/// And what is *not* in it is said too.
///
/// A bundle whose conditions are missing what the machine was is a weaker
/// artifact than one whose are not, and the difference is the reader's to
/// weigh rather than something to discover later (A7).
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
