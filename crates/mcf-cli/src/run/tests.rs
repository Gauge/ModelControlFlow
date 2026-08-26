//! What `run` says, and what it refuses.
//!
//! The engine itself is exercised end to end in `mcf-standin`'s own tests
//! against a model built for the purpose; what is here is the surface's part —
//! finding the model, keeping the mark, and refusing legibly.

use super::{TOKENS, resolve, run};

/// A path is a model, and so is something under the store — both spellings,
/// because both are things somebody will type.
#[test]
fn a_model_is_found_by_path_or_by_name() {
    let scratch = std::env::temp_dir().join(format!("mcf-run-{}", std::process::id()));
    std::fs::create_dir_all(&scratch).expect("a scratch directory");
    let file = scratch.join("model.gguf");
    std::fs::write(&file, b"GGUF").expect("a file");

    assert_eq!(
        resolve(file.to_str().unwrap_or_default()),
        Some(file.clone())
    );
    assert_eq!(resolve("nothing/at/all.gguf"), None);

    let _cleared = std::fs::remove_dir_all(&scratch);
}

/// A model that is not there is said, and the message points at the command
/// that would list one.
#[test]
fn a_model_that_is_not_there_is_said() {
    let response = run("owner/model:absent.gguf", "hello", None, 0);
    assert!(!response.served);
    assert!(
        response.text.contains("there is no model"),
        "{}",
        response.text
    );
    assert!(response.text.contains("mcf list"), "{}", response.text);
}

/// A file that is not a model is refused with what MCF saw, and the refusal
/// says why MCF's reader is strict — there is no vendored engine to fall back
/// to (D31, B-320).
#[test]
fn a_file_that_is_not_a_model_is_refused_legibly() {
    let scratch = std::env::temp_dir().join(format!("mcf-run-not-a-model-{}", std::process::id()));
    std::fs::create_dir_all(&scratch).expect("a scratch directory");
    let file = scratch.join("model.gguf");
    std::fs::write(&file, b"ONNX and not much else").expect("a file");

    let response = run(file.to_str().unwrap_or_default(), "hello", None, 0);
    assert!(!response.served);
    assert!(response.text.contains("did not run"), "{}", response.text);
    assert!(
        response.text.contains("no vendored engine"),
        "{}",
        response.text
    );

    let _cleared = std::fs::remove_dir_all(&scratch);
}

/// The token budget is a stated number rather than a wall clock, because a
/// stand-in is slow by design and a limit in time would measure the machine
/// (B49).
#[test]
fn the_budget_is_in_tokens_and_is_stated() {
    assert_eq!(TOKENS, 32);
}
