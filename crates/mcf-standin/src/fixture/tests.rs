use super::{TOKENS, a_model_that_runs};
use crate::gguf;
use crate::llama::load;
use crate::sample::Settings;
use crate::session::{Request, Stopped, generate};
use crate::tokenizer::Vocabulary;

#[test]
fn the_fixture_reads_loads_and_answers() {
    let bytes = a_model_that_runs();
    let file = gguf::parse(&bytes).expect("it is a model file");
    let vocabulary = Vocabulary::read(&file).expect("it has a vocabulary");
    assert_eq!(vocabulary.len(), TOKENS.len());
    let model = load(&file, &bytes).expect("it loads");

    let prompt = vocabulary.encode("yes", true).expect("it segments");
    let marked = generate(
        &model,
        "the fixture's own test",
        &Request {
            prompt,
            limit: 3,
            settings: Settings::Greedy,
            seed: 0,
            stop: Vec::new(),
        },
    )
    .expect("it runs");

    let produced = marked.value().observed();
    assert_eq!(produced.stopped, Stopped::AtLimit);
    assert_eq!(
        vocabulary.decode(&produced.tokens),
        " yes yes yes",
        "the one-hot table means greedy decoding repeats what it is given"
    );
    assert!(
        !marked.degradation().to_string().is_empty(),
        "a stand-in's answer arrived without its mark (A5, B65)"
    );
}
