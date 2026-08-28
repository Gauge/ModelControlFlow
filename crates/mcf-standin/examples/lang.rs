//! How many tokens a vocabulary spends on the same sentence in different
//! languages — a demonstration for a question, not a shipped surface.

fn main() {
    let path = std::env::args().nth(1).expect("model");
    let bytes = std::fs::read(&path).expect("read");
    let file = mcf_standin::gguf::parse(&bytes).expect("parse");
    let v = mcf_standin::tokenizer::Vocabulary::read(&file).expect("vocab");
    let samples = [
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
    for (name, text) in samples {
        let ids = v.encode(text, false).expect("encode");
        let chars = text.chars().count();
        println!(
            "  {name:<9} {:>4} tokens for {:>3} characters   {:.2} tokens/char",
            ids.len(),
            chars,
            ids.len() as f64 / chars as f64
        );
    }
}
