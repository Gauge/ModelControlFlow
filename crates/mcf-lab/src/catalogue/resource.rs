//! Scenarios in which the machine itself is what runs out.
//!
//! B-372's case: the stand-in engine dequantizes every tensor to `f32` on
//! load, and a model whose dequantized weight exceeds free memory must be
//! refused from its directory — the alternative is the kernel killing the
//! process mid-load, an exit with no account (A2).
//!
//! **What is simulated is the observation** (D26): a directory whose
//! arithmetic exceeds a stated amount of memory. The judgement under test is
//! the shipped one — `Model::fits_dequantized` — against a fixture small
//! enough to write and a budget small enough to exceed, because the claim is
//! about the comparison and not about any particular machine's size.

use mcf_core::failure::Category;

use crate::scenario::{Outcome, Scenario};
use crate::world::World;

/// A model that cannot fit dequantized is refused by arithmetic.
pub(super) const MODEL_LARGER_THAN_MEMORY: Scenario = Scenario {
    id: "resource/model-larger-than-memory",
    produces: Category::ResourceMemoryExhausted,
    summary: "a model whose dequantized weight exceeds free memory is refused from its \
              directory, both numbers named, before any tensor is read",
    run: model_larger_than_memory,
};

fn model_larger_than_memory(_world: &World) -> Outcome {
    // A one-tensor llama-shaped directory: 1024 F32 elements, 4096 bytes
    // dequantized. The budget it is judged against is smaller, which is the
    // whole of the condition — the same comparison the surface makes with the
    // machine's own number (§3.15: the observation at the edge, the arithmetic
    // in the library).
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
