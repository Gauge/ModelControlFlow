//! What `run` says, and what it refuses.
//!
//! The engine itself is exercised end to end in `mcf-standin`'s own tests
//! against a model built for the purpose; what is here is the surface's part —
//! finding the model, keeping the mark, and refusing legibly.

use super::{TOKENS, examined, resolve, run_where};

/// Every test here asks MCF to answer for itself.
///
/// Not because the daemon is uninteresting, but because *which* of them
/// answers changes the refusal, and a test that took whichever was running
/// reported on the machine rather than on the code (F46, B-378). A test about
/// a daemon says so by passing one.
fn without_a_daemon(
    model: &str,
    prompt: &str,
    limit: Option<usize>,
    seed: u64,
    engine: Option<&str>,
) -> crate::Response {
    run_where(
        None,
        model,
        prompt,
        limit,
        seed,
        engine,
        &mcf_serve::turn::Turn::default(),
    )
}

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
        Ok(Some(file.clone()))
    );
    assert_eq!(resolve("nothing/at/all.gguf"), Ok(None));

    let _cleared = std::fs::remove_dir_all(&scratch);
}

/// A model that is not there is said, and the message points at the command
/// that would list one.
#[test]
fn a_model_that_is_not_there_is_said() {
    let response = without_a_daemon("owner/model:absent.gguf", "hello", None, 0, None);
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

    let response = without_a_daemon(file.to_str().unwrap_or_default(), "hello", None, 0, None);
    assert!(!response.served);
    assert!(response.text.contains("did not run"), "{}", response.text);
    // The refusal says what MCF's reader handles and what to do about a model
    // it will not take. It no longer cites a rule at somebody who has never
    // read one.
    assert!(
        response.text.contains("provision"),
        "the refusal must say what to do: {}",
        response.text
    );
    assert!(
        !response.text.contains("B-320"),
        "a citation reached a person: {}",
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

/// A model too large to dequantize is refused from its directory alone, with
/// both numbers named (B-372).
///
/// The fixture is a *header*: a llama-family directory declaring one tensor of
/// 2^61 elements, which dequantized is 2^63 bytes and larger than any machine
/// that exists. No tensor data is behind it and none is needed — the point of
/// the check is that the answer comes from arithmetic on the directory, not
/// from an attempt to read what cannot fit.
#[test]
fn a_model_too_large_to_dequantize_is_refused_from_the_header() {
    let mut out = b"GGUF".to_vec();
    out.extend_from_slice(&3_u32.to_le_bytes());
    out.extend_from_slice(&1_u64.to_le_bytes()); // one tensor
    out.extend_from_slice(&1_u64.to_le_bytes()); // one metadata pair

    // general.architecture = "llama": a family MCF covers, so the refusal
    // under test is the memory ceiling and not the architecture check.
    let key = b"general.architecture";
    out.extend_from_slice(&(key.len() as u64).to_le_bytes());
    out.extend_from_slice(key);
    out.extend_from_slice(&8_u32.to_le_bytes());
    let value = b"llama";
    out.extend_from_slice(&(value.len() as u64).to_le_bytes());
    out.extend_from_slice(value);

    // One F32 tensor of 2^61 elements.
    let name = b"token_embd.weight";
    out.extend_from_slice(&(name.len() as u64).to_le_bytes());
    out.extend_from_slice(name);
    out.extend_from_slice(&1_u32.to_le_bytes()); // one dimension
    out.extend_from_slice(&(1_u64 << 61).to_le_bytes());
    out.extend_from_slice(&0_u32.to_le_bytes()); // F32
    out.extend_from_slice(&0_u64.to_le_bytes()); // offset

    let scratch = std::env::temp_dir().join(format!("mcf-ceiling-{}", std::process::id()));
    std::fs::create_dir_all(&scratch).expect("a scratch directory");
    let path = scratch.join("too-large.gguf");
    std::fs::write(&path, &out).expect("written");

    let failure = examined(&path).expect_err("2^63 bytes fits nowhere");
    let _cleared = std::fs::remove_dir_all(&scratch);
    assert_eq!(
        failure.category(),
        mcf_core::failure::Category::ResourceMemoryExhausted,
        "the refusal is the memory ceiling: {failure}"
    );
    let contexts: Vec<String> = failure
        .context()
        .iter()
        .map(|entry| format!("{}={}", entry.key, entry.value))
        .collect();
    assert!(
        contexts
            .iter()
            .any(|entry| entry.starts_with("dequantized=")),
        "the refusal names what the model needs: {contexts:?}"
    );
    assert!(
        contexts.iter().any(|entry| entry.starts_with("available=")),
        "the refusal names what the machine has: {contexts:?}"
    );
}
