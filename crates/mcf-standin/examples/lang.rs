//! What a language costs this model's vocabulary (B-379, B-057).
//!
//! The same sentence in several languages, and how many tokens each one
//! spends. No generation, no judgement, no rater — it is the tokenizer, and it
//! answers in milliseconds.
//!
//! It is not a claim about how well a model speaks a language. It is what that
//! language *costs*: tokens are money, they are context, and they are time. On
//! the corpus, three models spend sixteen tokens each on an English sentence,
//! and on a Japanese one the first spends fifty-two where another spends twenty.
//!
//!   cargo run -p mcf-standin --example lang -- <model.gguf>

/// The same meaning, in each language, so the comparison is of vocabularies
/// rather than of sentences.
const SAMPLES: [(&str, &str); 6] = [
    (
        "English",
        "The quick brown fox jumps over the lazy dog near the riverbank at dawn.",
    ),
    (
        "German",
        "Der schnelle braune Fuchs springt über den faulen Hund am Flussufer.",
    ),
    (
        "French",
        "Le rapide renard brun saute par-dessus le chien paresseux près de la rivière.",
    ),
    (
        "Spanish",
        "El rápido zorro marrón salta sobre el perro perezoso cerca del río.",
    ),
    (
        "Japanese",
        "素早い茶色のキツネが川辺で怠け者の犬を飛び越えます。",
    ),
    (
        "Arabic",
        "الثعلب البني السريع يقفز فوق الكلب الكسول بالقرب من النهر.",
    ),
];

fn main() -> std::process::ExitCode {
    let Some(path) = std::env::args().nth(1) else {
        eprintln!("usage: lang <model.gguf>");
        return std::process::ExitCode::FAILURE;
    };
    let bytes = match std::fs::read(&path) {
        Ok(bytes) => bytes,
        Err(error) => {
            eprintln!("{path}: {error}");
            return std::process::ExitCode::FAILURE;
        }
    };
    let vocabulary = match mcf_standin::gguf::parse(&bytes)
        .and_then(|file| mcf_standin::tokenizer::Vocabulary::read(&file))
    {
        Ok(vocabulary) => vocabulary,
        Err(failure) => {
            eprintln!("{path}: {failure}");
            return std::process::ExitCode::FAILURE;
        }
    };

    for (name, text) in SAMPLES {
        let Ok(identifiers) = vocabulary.encode(text, false) else {
            println!("  {name:<9} this vocabulary cannot express it");
            continue;
        };
        let characters = text.chars().count();
        let each = if characters == 0 {
            0.0
        } else {
            f64::from(u32::try_from(identifiers.len()).unwrap_or(u32::MAX))
                / f64::from(u32::try_from(characters).unwrap_or(u32::MAX))
        };
        println!(
            "  {name:<9} {:>4} tokens for {characters:>3} characters   {each:.2} tokens/char",
            identifiers.len()
        );
    }
    std::process::ExitCode::SUCCESS
}
