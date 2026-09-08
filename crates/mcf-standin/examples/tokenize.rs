fn main() -> std::process::ExitCode {
    let mut arguments = std::env::args().skip(1);
    let (Some(path), Some(text)) = (arguments.next(), arguments.next()) else {
        eprintln!("usage: tokenize <model.gguf> \"some text\" [--ids]");
        return std::process::ExitCode::FAILURE;
    };
    let only_ids = arguments.any(|argument| argument == "--ids");

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

    if only_ids {
        return match vocabulary.encode(&text, true) {
            Ok(tokens) => {
                println!(
                    "{}",
                    tokens
                        .iter()
                        .map(usize::to_string)
                        .collect::<Vec<_>>()
                        .join(" ")
                );
                std::process::ExitCode::SUCCESS
            }
            Err(failure) => {
                eprintln!("{failure}");
                std::process::ExitCode::FAILURE
            }
        };
    }

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
