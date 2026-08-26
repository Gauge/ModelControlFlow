//! What a model's own vocabulary makes of a piece of text (B-365, A19).
//!
//! A diagnostic rather than a surface: when a model produces nonsense, the
//! first question is whether it was asked the right question, and that means
//! seeing the tokens rather than inferring them from the answer.
//!
//!   cargo run -p mcf-standin --example tokenize -- <model.gguf> "some text"

fn main() -> std::process::ExitCode {
    let mut arguments = std::env::args().skip(1);
    let (Some(path), Some(text)) = (arguments.next(), arguments.next()) else {
        eprintln!("usage: tokenize <model.gguf> \"some text\"");
        return std::process::ExitCode::FAILURE;
    };

    let bytes = match std::fs::read(&path) {
        Ok(bytes) => bytes,
        Err(error) => {
            eprintln!("{path}: {error}");
            return std::process::ExitCode::FAILURE;
        }
    };
    let file = match mcf_standin::gguf::parse(&bytes) {
        Ok(file) => file,
        Err(failure) => {
            eprintln!("{path}: {failure}");
            return std::process::ExitCode::FAILURE;
        }
    };
    let vocabulary = match mcf_standin::tokenizer::Vocabulary::read(&file) {
        Ok(vocabulary) => vocabulary,
        Err(failure) => {
            eprintln!("{path}: {failure}");
            return std::process::ExitCode::FAILURE;
        }
    };

    println!(
        "tokenizer model: {:?}",
        file.get("tokenizer.ggml.model")
            .and_then(mcf_standin::gguf::Value::as_text)
    );
    for key in [
        "tokenizer.ggml.tokens",
        "tokenizer.ggml.scores",
        "tokenizer.ggml.token_type",
        "tokenizer.ggml.merges",
    ] {
        let what = match file.get(key) {
            Some(mcf_standin::gguf::Value::List(items)) => format!("{} items", items.len()),
            Some(other) => format!("{other:?}"),
            None => "absent".to_owned(),
        };
        println!("  {key}: {what}");
    }

    match vocabulary.encode(&text, true) {
        Ok(tokens) => {
            println!("\n{} token(s) for {text:?}:", tokens.len());
            for token in &tokens {
                println!("  {token:>6}  {:?}", vocabulary.decode(&[*token]));
            }
        }
        Err(failure) => eprintln!("\nthe vocabulary refused it: {failure}"),
    }
    std::process::ExitCode::SUCCESS
}
