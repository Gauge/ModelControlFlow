//! What a bundle has to carry, and what it has to say it carries.

use mcf_record::json::Value;

use super::{destination, keeps, prompt_beside, rests_on};

fn entry(kind: &str, id: &str, body: Value) -> Value {
    Value::map([
        ("id", Value::text(id)),
        ("kind", Value::text(kind)),
        ("body", body),
    ])
}

fn a_claim() -> mcf_record::journal::Entry {
    mcf_record::journal::Entry::new(
        mcf_record::journal::EntryKind::Comparison,
        mcf_core::time::Timestamp::from_utc_nanos(0, mcf_core::attested::Attested::Unknown),
        Value::map([
            ("left", Value::map([("arm", Value::text("/models/a.gguf"))])),
            (
                "right",
                Value::map([("arm", Value::text("/models/b.gguf"))]),
            ),
        ]),
    )
}

/// A claim rests on the artifacts it names, and the record has no foreign key
/// but the path — so that is the join, and it is the claim's own words.
#[test]
fn a_claim_rests_on_the_artifacts_it_names() {
    assert_eq!(
        rests_on(&a_claim()),
        vec!["/models/a.gguf".to_owned(), "/models/b.gguf".to_owned()]
    );
}

/// The bundle carries the claim, the provenance of what it measured, the
/// engine and what the machine was — and nothing else.
#[test]
fn it_carries_the_claim_and_what_it_rests_on() {
    let arms = rests_on(&a_claim());
    let acquired = |path: &str| {
        entry(
            "artifact_acquired",
            "artifact_acquired_x",
            Value::map([("path", Value::text(path))]),
        )
    };

    assert!(keeps(
        &entry("comparison", "the-claim", Value::map::<String>([])),
        "the-claim",
        &arms,
        Some("m")
    ));
    assert!(keeps(
        &acquired("/models/a.gguf"),
        "the-claim",
        &arms,
        Some("m")
    ));
    assert!(keeps(
        &acquired("/models/b.gguf"),
        "the-claim",
        &arms,
        Some("m")
    ));
    assert!(keeps(
        &entry("component_provisioned", "p", Value::map::<String>([])),
        "the-claim",
        &arms,
        Some("m")
    ));
    assert!(keeps(
        &entry("machine_profile", "m", Value::map::<String>([])),
        "the-claim",
        &arms,
        Some("m")
    ));
}

/// **A bundle is one claim, not the record.** Another comparison, another
/// machine's model, a generation, a daemon's life — none of it is what this
/// claim rests on, and carrying it would make a bundle an export with a
/// smaller name (PR2, §XIV).
#[test]
fn it_carries_nothing_else() {
    let arms = rests_on(&a_claim());
    for (kind, body) in [
        ("comparison", Value::map::<String>([])),
        (
            "artifact_acquired",
            Value::map([("path", Value::text("/models/somebody-elses.gguf"))]),
        ),
        ("generated", Value::map::<String>([])),
        ("daemon_started", Value::map::<String>([])),
        ("failure", Value::map::<String>([])),
        ("fitment_planned", Value::map::<String>([])),
        // The machine on another day is not the machine this claim was taken
        // on, and carrying it would describe a machine nobody measured.
        ("machine_profile", Value::map::<String>([])),
    ] {
        assert!(
            !keeps(
                &entry(kind, "another-entry", body),
                "the-claim",
                &arms,
                Some("m")
            ),
            "{kind} is not what this claim rests on"
        );
    }
}

/// A named directory takes the bundle under the claim's own name; a named file
/// is taken as written. Neither is invented: a bundle written somewhere the
/// operator did not name is a file they will not find (§3.9).
#[test]
fn where_it_goes_is_where_the_operator_said() {
    let scratch = std::env::temp_dir();
    let into = destination(Some(&scratch.display().to_string()), "the-claim");
    assert_eq!(into, scratch.join("the-claim.mcf-bundle"));

    assert_eq!(
        destination(Some("/tmp/named-by-hand.bundle"), "the-claim"),
        std::path::PathBuf::from("/tmp/named-by-hand.bundle")
    );
    assert_eq!(
        destination(None, "the-claim"),
        std::path::PathBuf::from("the-claim.mcf-bundle")
    );
}

/// A bundle can only be made of a claim.
#[test]
fn only_a_claim_can_be_bundled() {
    let response = super::run("machine_profile_1970-01-01T00-00-00Z_deadbeef_0000", None);
    // Whether that entry exists depends on the machine; either way the command
    // must not serve a bundle of something that is not a claim.
    assert!(!response.served, "{}", response.text);
}

/// The prompt travels beside the bundle, and each of the three things that can
/// be true of it is said rather than left out (A25, A7, F105).
///
/// The record does not hold the prompt any more — it holds its length and its
/// digest — so a bundle assembled from record lines cannot carry it, and
/// `mcf_record::export` must stay unable to reach content or A25's guarantee
/// becomes a filter again. B-211 still needs the input, so it is disclosed
/// here, deliberately, into a file of its own.
#[test]
fn the_prompt_beside_a_bundle_says_which_of_three_things_is_true() {
    let root = std::env::temp_dir().join(format!("mcf-bundle-prompt-{}", std::process::id()));
    let _cleared = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("a scratch directory is creatable");
    let journal = root.join("record.jsonl");
    let bundle = root.join("claim.mcf-bundle");

    // Nothing filed: the claim predates content being kept, and the report
    // says so rather than leaving the line out.
    let said = prompt_beside(&journal, &bundle, "comparison_1");
    assert!(said.contains("NOT here"), "{said}");

    // Filed: it is written beside the bundle, in its own file.
    let store = mcf_record::content::ContentStore::open(
        &mcf_record::content::ContentStore::beside(&journal),
    )
    .expect("the content store opens");
    store
        .keep(
            "comparison_1",
            &mcf_record::content::Content::new("Once upon a time"),
        )
        .expect("content is filed");
    let said = prompt_beside(&journal, &bundle, "comparison_1");
    assert!(said.contains("16 byte(s)"), "{said}");
    let beside = bundle.with_extension("mcf-bundle.prompt");
    assert_eq!(
        std::fs::read_to_string(&beside).expect("the prompt is beside it"),
        "Once upon a time"
    );

    // Filed and unreadable: not the same answer as never filed, which is the
    // distinction `record.content.unreadable` exists for.
    let filed = mcf_record::content::ContentStore::beside(&journal).join("comparison_1");
    std::fs::remove_file(&filed).expect("the filed prompt is there");
    std::fs::create_dir(&filed).expect("a directory takes its place");
    let said = prompt_beside(&journal, &bundle, "comparison_1");
    assert!(said.contains("would not be read"), "{said}");

    let _removed = std::fs::remove_dir_all(&root);
}
