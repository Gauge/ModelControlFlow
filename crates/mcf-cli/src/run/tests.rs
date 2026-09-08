use super::{TOKENS, examined, resolve, run_where};

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
        None,
        mcf_serve::declared::Started::default(),
    )
}

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

#[test]
fn a_file_that_is_not_a_model_is_refused_legibly() {
    let scratch = std::env::temp_dir().join(format!("mcf-run-not-a-model-{}", std::process::id()));
    std::fs::create_dir_all(&scratch).expect("a scratch directory");
    let file = scratch.join("model.gguf");
    std::fs::write(&file, b"ONNX and not much else").expect("a file");

    let response = without_a_daemon(file.to_str().unwrap_or_default(), "hello", None, 0, None);
    assert!(!response.served);
    assert!(response.text.contains("did not run"), "{}", response.text);
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

#[test]
fn the_budget_is_in_tokens_and_is_stated() {
    assert_eq!(TOKENS, 32);
}

#[test]
fn a_model_too_large_to_dequantize_is_refused_from_the_header() {
    let mut out = b"GGUF".to_vec();
    out.extend_from_slice(&3_u32.to_le_bytes());
    out.extend_from_slice(&1_u64.to_le_bytes());
    out.extend_from_slice(&1_u64.to_le_bytes());

    let key = b"general.architecture";
    out.extend_from_slice(&(key.len() as u64).to_le_bytes());
    out.extend_from_slice(key);
    out.extend_from_slice(&8_u32.to_le_bytes());
    let value = b"llama";
    out.extend_from_slice(&(value.len() as u64).to_le_bytes());
    out.extend_from_slice(value);

    let name = b"token_embd.weight";
    out.extend_from_slice(&(name.len() as u64).to_le_bytes());
    out.extend_from_slice(name);
    out.extend_from_slice(&1_u32.to_le_bytes());
    out.extend_from_slice(&(1_u64 << 61).to_le_bytes());
    out.extend_from_slice(&0_u32.to_le_bytes());
    out.extend_from_slice(&0_u64.to_le_bytes());

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
