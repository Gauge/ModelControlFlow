use mcf_core::attested::Attested;
use mcf_core::configuration::{Sampling, Thousandths};

use crate::gguf::{Model, Value};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Recommendation {
    Declared {
        sampling: Sampling,
        keys: Vec<String>,
    },
    NoneDeclared,
}

impl Recommendation {
    #[must_use]
    pub const fn sampling(&self) -> Option<&Sampling> {
        match self {
            Self::Declared { sampling, .. } => Some(sampling),
            Self::NoneDeclared => None,
        }
    }
}

#[must_use]
pub fn read(file: &Model) -> Recommendation {
    let architecture = file.get("general.architecture").and_then(Value::as_text);
    let mut keys = Vec::new();
    let mut fraction = |engine_name: &str, spec_name: &str| -> Attested<Thousandths> {
        match stated(file, architecture, engine_name, spec_name) {
            Some((key, value)) => thousandths(value).map_or(Attested::Unknown, |held| {
                keys.push(key);
                Attested::Known(held)
            }),
            None => Attested::Unknown,
        }
    };
    let temperature = fraction("temp", "temperature");
    let top_p = fraction("top_p", "top_p");
    let min_p = fraction("min_p", "min_p");
    let repetition_penalty = fraction("penalty_repeat", "repetition_penalty");

    let mut whole = |engine_name: &str, spec_name: &str| -> Attested<u32> {
        match stated(file, architecture, engine_name, spec_name) {
            Some((key, value)) => count(value).map_or(Attested::Unknown, |held| {
                keys.push(key);
                Attested::Known(held)
            }),
            None => Attested::Unknown,
        }
    };
    let top_k = whole("top_k", "top_k");
    let max_output_tokens = whole("max_output_tokens", "max_output_tokens");

    if keys.is_empty() {
        return Recommendation::NoneDeclared;
    }
    Recommendation::Declared {
        sampling: Sampling {
            temperature,
            top_p,
            top_k,
            min_p,
            repetition_penalty,
            max_output_tokens,
        },
        keys,
    }
}

fn stated<'file>(
    file: &'file Model,
    architecture: Option<&str>,
    engine_name: &str,
    spec_name: &str,
) -> Option<(String, &'file Value)> {
    let engine_key = format!("general.sampling.{engine_name}");
    let spec_key = architecture.map(|held| format!("{held}.{spec_name}"));
    [Some(engine_key), spec_key]
        .into_iter()
        .flatten()
        .find_map(|key| file.get(&key).map(|value| (key, value)))
}

fn thousandths(value: &Value) -> Option<Thousandths> {
    let held = match value {
        Value::Float(held) => {
            if !held.is_finite() || *held < 0.0 || *held > 1_000.0 {
                return None;
            }
            #[expect(
                clippy::cast_possible_truncation,
                reason = "the value is finite and within [0, 1000], asserted immediately above"
            )]
            let scaled = (held * 1_000.0).round() as i64;
            u32::try_from(scaled).ok()?
        }
        Value::Integer(held) => u32::try_from(*held).ok()?.checked_mul(1_000)?,
        _ => return None,
    };
    Some(Thousandths(held))
}

fn count(value: &Value) -> Option<u32> {
    match value {
        Value::Integer(held) => u32::try_from(*held).ok(),
        _ => None,
    }
}

#[cfg(test)]
mod tests;
