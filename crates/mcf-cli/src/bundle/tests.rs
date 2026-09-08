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

#[test]
fn a_claim_rests_on_the_artifacts_it_names() {
    assert_eq!(
        rests_on(&a_claim()),
        vec!["/models/a.gguf".to_owned(), "/models/b.gguf".to_owned()]
    );
}

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

#[test]
fn only_a_claim_can_be_bundled() {
    let response = super::run("machine_profile_1970-01-01T00-00-00Z_deadbeef_0000", None);
    assert!(!response.served, "{}", response.text);
}

#[test]
fn the_prompt_beside_a_bundle_says_which_of_three_things_is_true() {
    let root = std::env::temp_dir().join(format!("mcf-bundle-prompt-{}", std::process::id()));
    let _cleared = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("a scratch directory is creatable");
    let journal = root.join("record.jsonl");
    let bundle = root.join("claim.mcf-bundle");

    let said = prompt_beside(&journal, &bundle, "comparison_1");
    assert!(said.contains("NOT here"), "{said}");

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

    let filed = mcf_record::content::ContentStore::beside(&journal).join("comparison_1");
    std::fs::remove_file(&filed).expect("the filed prompt is there");
    std::fs::create_dir(&filed).expect("a directory takes its place");
    let said = prompt_beside(&journal, &bundle, "comparison_1");
    assert!(said.contains("would not be read"), "{said}");

    let _removed = std::fs::remove_dir_all(&root);
}
