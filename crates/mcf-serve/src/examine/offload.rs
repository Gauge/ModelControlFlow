//! The offload curve: tokens a second at nought, a quarter, half, three
//! quarters and all of the layers on the card (B-492, D52).
//!
//! **Why it is a curve and not a number.** MCF resolves a model to the
//! card whole or not at all, because a partial offload needs a
//! measurement nobody had taken. This is that measurement: one pinned
//! generation and one first-token timing at each of five layer counts,
//! so a person whose model does not fit whole can read what each count
//! buys. Nothing here chooses; the settings stay where somebody put them
//! (D43).

use mcf_record::json::Value;

use super::{Found, Site, as_integer, as_ms, filler, per_second, timed, whole};
use crate::generation::Draw;
use crate::served::{Prompt, Startup};

/// The measurement's name, as the run and the record know it.
pub const NAME: &str = "offload-curve";

/// How deep the prompt is, in identifiers.
const DEPTH: usize = 256;

/// How many tokens the longer of the pair produces.
const PRODUCE: usize = 32;

/// Runs it.
#[must_use]
pub fn measure(site: &Site<'_>) -> Found {
    if !site.has_card {
        return Found::could_not_tell(
            "no provisioned engine here computes on a card, and the curve is a card's",
        );
    }
    let Some(layers) = layers_of(site.model) else {
        return Found::could_not_tell("the file's header does not say how many layers it has");
    };
    let counts: Vec<u32> = [0_u32, 1, 2, 3, 4]
        .iter()
        .map(|quarter| {
            #[expect(clippy::integer_division, reason = "a quarter of the layers, floored")]
            let some = layers * quarter / 4;
            // All of them means the output layer too, which the engine
            // counts as one past the blocks.
            if *quarter == 4 { layers + 1 } else { some }
        })
        .collect();
    let mut lines = vec![format!(
        "  {layers} layers in the file; a prompt of {DEPTH} identifiers, then {PRODUCE} tokens \
         pinned, at each count"
    )];
    let mut rows = Vec::new();
    let mut measured = 0_usize;
    for count in counts {
        if site.asker_gone() {
            break;
        }
        match at_layers(site, count) {
            Ok((per_token, first)) => {
                measured = measured.saturating_add(1);
                lines.push(format!(
                    "  {count:>4} on the card   {:>8} ms a token   {:>8} prompt tokens a second",
                    as_ms(per_token),
                    per_second(DEPTH as u64, first)
                ));
                rows.push(Value::map([
                    ("layers", Value::Integer(i64::from(count))),
                    ("ns_per_token", whole(per_token)),
                    ("first_token_ns", whole(first)),
                    ("measured", Value::Bool(true)),
                ]));
            }
            Err(why) => {
                lines.push(format!("  {count:>4} on the card   not measured: {why}"));
                rows.push(Value::map([
                    ("layers", Value::Integer(i64::from(count))),
                    ("measured", Value::Bool(false)),
                    ("why", Value::text(why)),
                ]));
            }
        }
    }
    if measured == 0 {
        return Found::could_not_tell("no layer count could be measured");
    }
    Found {
        lines,
        fields: vec![
            ("layers_total", Value::Integer(i64::from(layers))),
            ("depth", Value::Integer(as_integer(DEPTH))),
            ("produce", Value::Integer(as_integer(PRODUCE))),
            ("measured", Value::Integer(as_integer(measured))),
            ("curve", Value::List(rows)),
        ],
    }
}

/// One layer count: the cost a token and the time to a first token.
fn at_layers(site: &Site<'_>, count: u32) -> Result<(u64, u64), String> {
    let engine = site.server(&Startup {
        gpu_layers: count,
        context: 1024,
        projector: None,
        ..site.startup()
    })?;
    let prompt = filler(DEPTH);
    let said = |failure: mcf_core::Failure| failure.detail().to_owned();
    // A first request after a start pays for what the engine sets up
    // lazily; it is made and not read.
    let _warm = engine
        .complete(
            Prompt::Identifiers(&prompt),
            1,
            Draw::greedy(0),
            true,
            site.timed(),
        )
        .map_err(said)?;
    let (one, first) = timed(|| {
        engine.complete(
            Prompt::Identifiers(&prompt),
            1,
            Draw::greedy(0),
            true,
            site.timed(),
        )
    });
    let one = one.map_err(said)?;
    let (many, long) = timed(|| {
        engine.complete(
            Prompt::Identifiers(&prompt),
            PRODUCE,
            Draw::greedy(0),
            true,
            site.timed(),
        )
    });
    let many = many.map_err(said)?;
    if one.predicted != 1 || many.predicted != PRODUCE {
        return Err(format!(
            "the pair did not produce what it pinned ({} and {} tokens)",
            one.predicted, many.predicted
        ));
    }
    if long <= first {
        return Err("the longer run finished no later than the shorter".to_owned());
    }
    #[expect(
        clippy::integer_division,
        reason = "a difference in nanoseconds over the tokens between the pair"
    )]
    let per_token = (long - first) / (PRODUCE as u64 - 1);
    Ok((per_token, first))
}

/// How many blocks the file declares.
pub(crate) fn layers_of(model: &std::path::Path) -> Option<u32> {
    let bytes = crate::probes::run::read_prefix(model)?;
    let file = mcf_standin::gguf::parse(&bytes).ok()?;
    let architecture = file.architecture()?;
    file.get(&format!("{architecture}.block_count"))
        .and_then(mcf_standin::gguf::Value::as_integer)
        .and_then(|held| u32::try_from(held).ok())
}
