//! Whether a model's own vocabulary can say a piece of text and read it back
//! (B-370, A1).
//!
//! **What this can show and what it cannot.** A tokenizer that loses a byte is
//! a tokenizer that changes the question the model was asked, and that is
//! checkable against nothing but the text itself: encode, decode, compare. What
//! it cannot show is that the *cut* was the right one — two different
//! pre-tokenizers both round-trip perfectly and produce different identifiers,
//! which is exactly the defect F23 found by reading somebody else's source
//! rather than by running anything. That one waits for B-368's oracle.
//!
//! So this is a floor rather than a proof: every vocabulary must pass it, and
//! passing it says only that nothing was dropped.
//!
//!   cargo run -p mcf-standin --example roundtrip -- <model.gguf>

/// Text chosen for the ways a tokenizer goes wrong: digits in runs, which the
/// four expressions cut four different ways; punctuation against letters, which
/// is where a lead character either joins or does not; characters outside ASCII
/// that are several bytes and therefore several byte-fallback tokens (F19);
/// runs of spaces and newlines, which every expression treats specially; and
/// the empty string.
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
        // `SentencePiece` puts a space in front of every text it encodes, and
        // no decoder can tell that space from one the text really had. So the
        // convention is allowed for exactly the vocabularies that declare it,
        // and named in the output rather than quietly tolerated (A5).
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
