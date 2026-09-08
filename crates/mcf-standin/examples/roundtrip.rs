const AWKWARD: &[&str] = &[
    "The capital of France is Paris.",
    "In 2024 there were 365 days, and 1234567 seconds is not 2 weeks.",
    "it's (parenthesized), \"quoted\" and hyphen-joined!",
    "  leading and trailing   ",
    "line one\nline two\r\n\r\nline four",
    "caf\u{e9} na\u{ef}ve \u{4f60}\u{597d} \u{1f600}\u{1f600}",
    "tabs\tand\tmore\ttabs",
    "",
];

fn main() -> std::process::ExitCode {
    let Some(path) = std::env::args().nth(1) else {
        eprintln!("usage: roundtrip <model.gguf>");
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

    let mut lost = 0;
    for text in AWKWARD {
        let identifiers = match vocabulary.encode(text, false) {
            Ok(identifiers) => identifiers,
            Err(failure) => {
                println!("  REFUSED {text:?}: {failure}");
                lost += 1;
                continue;
            }
        };
        let back = vocabulary.decode(&identifiers);
        let expected = if vocabulary.adds_a_space_prefix() {
            format!(" {text}")
        } else {
            (*text).to_owned()
        };
        if back == expected {
            println!(
                "  {} token(s)  {text:?}{}",
                identifiers.len(),
                if vocabulary.adds_a_space_prefix() && !text.is_empty() {
                    "  (+ the space SentencePiece adds)"
                } else {
                    ""
                }
            );
        } else {
            println!(
                "  LOST  {text:?}\n        wanted {expected:?}\n        came back as {back:?}"
            );
            lost += 1;
        }
    }

    if lost == 0 {
        println!("every string came back as itself");
        std::process::ExitCode::SUCCESS
    } else {
        eprintln!("{lost} string(s) did not come back");
        std::process::ExitCode::FAILURE
    }
}
