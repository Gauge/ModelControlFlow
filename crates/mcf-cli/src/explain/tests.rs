//! What an explanation shows, and what it refuses to claim.

use super::run;

/// The three columns are all there, and each says what kind of thing it is:
/// declared, read, or chosen. A table whose provenance a reader has to guess is
/// the thing this surface exists to replace (A21, §3.15).
#[test]
fn an_explanation_separates_declared_read_and_chosen() {
    let scratch = std::env::temp_dir().join(format!("mcf-explain-{}", std::process::id()));
    std::fs::create_dir_all(&scratch).expect("a scratch directory");
    let model = scratch.join("model.gguf");
    std::fs::write(&model, mcf_lab::fixture::a_model_that_runs()).expect("a model file");

    let response = run(model.to_str().unwrap_or_default());
    assert!(response.served, "{}", response.text);
    let said = response.text;

    assert!(said.contains("WHAT THE FILE DECLARES"), "{said}");
    assert!(said.contains("WHAT MCF READ FROM THE BYTES"), "{said}");
    assert!(said.contains("WHAT MCF WOULD CHOOSE"), "{said}");
    assert!(said.contains("WHAT MCF CANNOT TELL YOU"), "{said}");

    // The declarations are the fixture's own.
    assert!(said.contains("llama"), "{said}");
    assert!(said.contains("4 tokens"), "{said}");
    // What MCF read is of the bytes in front of it.
    assert!(said.contains("sha256"), "{said}");
    assert!(said.contains("11 tensors"), "{said}");
    assert!(said.contains("F32"), "{said}");
    // And the choices name where they are written down, so a reader can go and
    // disagree with them.
    assert!(said.contains("greedy"), "{said}");
    assert!(said.contains("crates/mcf-cli/src/run.rs"), "{said}");

    let _cleared = std::fs::remove_dir_all(&scratch);
}

/// The unanswered questions are named with their reasons, because a defaults
/// screen that listed only what MCF chose would imply it had a basis for
/// choosing (§6.5, C7).
#[test]
fn what_mcf_cannot_say_is_said_with_the_reason() {
    let scratch = std::env::temp_dir().join(format!("mcf-explain-why-{}", std::process::id()));
    std::fs::create_dir_all(&scratch).expect("a scratch directory");
    let model = scratch.join("model.gguf");
    std::fs::write(&model, mcf_lab::fixture::a_model_that_runs()).expect("a model file");

    let said = run(model.to_str().unwrap_or_default()).text;
    assert!(said.contains("Which quantization should I run?"), "{said}");
    assert!(said.contains("DEC-002"), "{said}");
    assert!(said.contains("How fast is it on this machine?"), "{said}");
    assert!(said.contains("B65"), "{said}");

    let _cleared = std::fs::remove_dir_all(&scratch);
}

/// A file that is not a model is refused rather than explained, and the refusal
/// says what MCF reads.
#[test]
fn something_that_is_not_a_model_is_refused() {
    let scratch = std::env::temp_dir().join(format!("mcf-explain-not-{}", std::process::id()));
    std::fs::create_dir_all(&scratch).expect("a scratch directory");
    let model = scratch.join("model.gguf");
    std::fs::write(&model, b"not a model at all").expect("a file");

    let response = run(model.to_str().unwrap_or_default());
    assert!(!response.served);
    assert!(response.text.contains("GGUF"), "{}", response.text);

    let _cleared = std::fs::remove_dir_all(&scratch);
}

/// A model that is not there is said, not explained.
#[test]
fn a_model_that_is_not_there_is_said() {
    let response = run("owner/model:absent.gguf");
    assert!(!response.served);
    assert!(
        response.text.contains("there is no model"),
        "{}",
        response.text
    );
}
