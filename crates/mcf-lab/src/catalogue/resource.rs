use mcf_core::failure::Category;

use crate::scenario::{Outcome, Scenario};
use crate::world::World;

pub(super) const MODEL_LARGER_THAN_MEMORY: Scenario = Scenario {
    id: "resource/model-larger-than-memory",
    produces: Category::ResourceMemoryExhausted,
    summary: "a model whose dequantized weight exceeds free memory is refused from its \
              directory, both numbers named, before any tensor is read",
    run: model_larger_than_memory,
};

fn model_larger_than_memory(_world: &World) -> Outcome {
    let mut header = b"GGUF".to_vec();
    header.extend_from_slice(&3_u32.to_le_bytes());
    header.extend_from_slice(&1_u64.to_le_bytes());
    header.extend_from_slice(&1_u64.to_le_bytes());
    let key = b"general.architecture";
    header.extend_from_slice(&(key.len() as u64).to_le_bytes());
    header.extend_from_slice(key);
    header.extend_from_slice(&8_u32.to_le_bytes());
    let value = b"llama";
    header.extend_from_slice(&(value.len() as u64).to_le_bytes());
    header.extend_from_slice(value);
    let name = b"token_embd.weight";
    header.extend_from_slice(&(name.len() as u64).to_le_bytes());
    header.extend_from_slice(name);
    header.extend_from_slice(&1_u32.to_le_bytes());
    header.extend_from_slice(&1024_u64.to_le_bytes());
    header.extend_from_slice(&0_u32.to_le_bytes());
    header.extend_from_slice(&0_u64.to_le_bytes());

    let file = match mcf_standin::gguf::parse(&header) {
        Ok(file) => file,
        Err(failure) => {
            return Outcome::Unexpected(format!("the fixture header did not parse: {failure}"));
        }
    };

    match file.fits_dequantized(1024) {
        Err(failure) => Outcome::Produced(failure),
        Ok(()) => Outcome::Unexpected(
            "4096 bytes were judged to fit in 1024, and the ceiling is not a ceiling".to_owned(),
        ),
    }
}
