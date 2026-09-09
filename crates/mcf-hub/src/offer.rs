use crate::client::Hub;
use crate::fitment::{self, Requirement, Shape, Verdict};
use crate::source::Listing;
use mcf_core::measurement::Bytes;

pub const PLANNING_CONTEXT: u64 = 4096;

#[derive(Debug, Clone)]
pub struct Plan {
    pub available: Bytes,
    pub context: u64,
    pub verdicts: Vec<(String, Verdict)>,
}

pub fn plan_for(hub: &Hub, listing: &Listing, available: Bytes) -> Result<Plan, String> {
    let shape = shape_from_configuration(hub, listing)?;
    plan_with(listing, shape, available)
}

pub fn shape_from_configuration(hub: &Hub, listing: &Listing) -> Result<Shape, String> {
    let configuration = match hub.configuration(listing) {
        Ok(Some(configuration)) => configuration,
        Ok(None) => {
            return Err(
                "this repository publishes no configuration, and a plan needs one".to_owned(),
            );
        }
        Err(failure) => {
            return Err(format!("its configuration could not be read — {failure}"));
        }
    };
    Shape::from_configuration(
        &configuration,
        mcf_core::configuration::CacheType::default(),
    )
    .ok_or_else(|| {
        "its configuration does not say how many blocks, key/value heads and head dimensions \
         the model has, and MCF will not guess at a shape (A7)"
            .to_owned()
    })
}

pub fn plan_with(listing: &Listing, shape: Shape, available: Bytes) -> Result<Plan, String> {
    let requirements: Vec<Requirement> = listing
        .variants()
        .into_iter()
        .map(|variant| Requirement {
            name: variant.first,
            weights: Bytes(variant.bytes),
            shape,
        })
        .collect();
    if requirements.is_empty() {
        return Err("this repository publishes nothing in a format MCF reads".to_owned());
    }

    let verdicts = fitment::plan(&requirements, PLANNING_CONTEXT, available)
        .map_err(|failure| format!("the arithmetic would not add up — {failure}"))?;
    Ok(Plan {
        available,
        context: PLANNING_CONTEXT,
        verdicts,
    })
}
