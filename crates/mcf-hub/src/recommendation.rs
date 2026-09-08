use mcf_core::attested::Attested;
use mcf_core::configuration::{Sampling, Thousandths};
use mcf_core::failure::{Category, Result};

use crate::client::Hub;
use crate::source::Listing;
use mcf_record::json::Value;

pub const WHERE: &str = "generation_config.json";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Recommendation {
    Declared {
        sampling: Sampling,
        fields: Vec<String>,
    },
    NothingStated,
    NoneDeclared,
}

impl Recommendation {
    #[must_use]
    pub const fn sampling(&self) -> Option<&Sampling> {
        match self {
            Self::Declared { sampling, .. } => Some(sampling),
            Self::NothingStated | Self::NoneDeclared => None,
        }
    }

    #[must_use]
    pub fn describe(&self) -> String {
        match self {
            Self::Declared { sampling, fields } => format!(
                "{sampling} — declared in {WHERE} ({}), unverified here",
                fields.join(", ")
            ),
            Self::NothingStated => format!(
                "{WHERE} is published and states no sampler parameter MCF reads — the publisher \
                 looked and said nothing"
            ),
            Self::NoneDeclared => format!(
                "no {WHERE} in this repository, so it recommends nothing MCF can adopt; a \
                 conversion repository usually publishes none, and the recommendation is in the \
                 base repository it came from (F63)"
            ),
        }
    }
}

pub fn read(hub: &Hub, listing: &Listing) -> Result<Recommendation> {
    let Some(body) = hub.metadata_file(listing, WHERE)? else {
        return Ok(Recommendation::NoneDeclared);
    };
    Ok(from_json(&body))
}

#[must_use]
pub fn from_json(body: &Value) -> Recommendation {
    let mut fields = Vec::new();
    let mut fraction = |name: &str| -> Attested<Thousandths> {
        match body.get(name).and_then(thousandths) {
            Some(held) => {
                fields.push(name.to_owned());
                Attested::Known(held)
            }
            None => Attested::Unknown,
        }
    };
    let temperature = fraction("temperature");
    let top_p = fraction("top_p");
    let min_p = fraction("min_p");
    let repetition_penalty = fraction("repetition_penalty");

    let mut whole = |name: &str| -> Attested<u32> {
        match body.get(name).and_then(Value::as_integer) {
            Some(held) => match u32::try_from(held) {
                Ok(held) => {
                    fields.push(name.to_owned());
                    Attested::Known(held)
                }
                Err(_) => Attested::Unknown,
            },
            None => Attested::Unknown,
        }
    };
    let top_k = whole("top_k");
    let max_output_tokens = whole("max_new_tokens");

    if fields.is_empty() {
        return Recommendation::NothingStated;
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
        fields,
    }
}

fn thousandths(value: &Value) -> Option<Thousandths> {
    match value {
        Value::Integer(held) => u32::try_from(*held)
            .ok()?
            .checked_mul(1_000)
            .map(Thousandths),
        Value::Text(held) => held.parse().ok(),
        _ => None,
    }
}

#[allow(dead_code, reason = "read by the doc comment on `read`")]
const ABSENCE: Category = Category::HubRefNotFound;

#[cfg(test)]
mod tests;
