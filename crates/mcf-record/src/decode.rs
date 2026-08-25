//! Reading back what the record holds.
//!
//! B-007's condition is that *the §3.4 floor is captured from a live machine
//! and round-trips through the record store losslessly*, and a round trip needs
//! both directions. [`encode`] writes; this reads.
//!
//! **What losslessly means here, and what it does not.** A condition MCF wrote
//! comes back as what MCF wrote; a condition MCF could not read comes back as
//! [`Attested::Unknown`] and not as the word *unknown*. Those two are the whole
//! of the property: a floor whose unknowns came back as strings would compare
//! equal to a floor that had read something, which is exactly A7's substitution
//! arriving through the back door of a decoder.
//!
//! **A line this version cannot understand is not decoded into a guess.** Every
//! function here returns `None` on a shape it does not recognize, and the caller
//! decides whether that is `record.schema.unknown` or a corrupt line — which is
//! the same discipline `EntryKind::parse` follows, for §7.30's reason.
//!
//! [`encode`]: crate::encode

use mcf_core::attested::Attested;
use mcf_core::measurement::{ConditionValue, Conditions, Floor};

use crate::json::Value;

/// Reads a condition floor back.
///
/// Returns `None` when a question the floor asks is missing from the record
/// entirely — which is a *different* thing from a question that was asked and
/// not answered. The first means this line was not written by a version that
/// asks the same questions; the second is `null`, and comes back as
/// [`Attested::Unknown`].
#[must_use]
pub fn floor(value: &Value) -> Option<Floor> {
    Some(Floor {
        hardware_state: condition(value, "hardware_state")?,
        thermal_state: condition(value, "thermal_state")?,
        driver_versions: condition(value, "driver_versions")?,
        runtime_versions: condition(value, "runtime_versions")?,
        quantization: condition(value, "quantization")?,
        context_length: condition(value, "context_length")?,
        batch_shape: condition(value, "batch_shape")?,
        mcf_configuration: condition(value, "mcf_configuration")?,
        realized_placement: condition(value, "realized_placement")?,
        instrumentation: condition(value, "instrumentation")?,
    })
}

/// Reads a condition set back, instrument included.
///
/// The build identity is *not* reconstructed from the record: [`Conditions`]
/// binds a floor to the instrument that read it, and the instrument a decoder
/// could offer is the one running now, not the one that wrote the line. The
/// caller supplies it, which forces the question of whose instrument this is to
/// be answered at the call site rather than assumed by a parser (§3.4).
#[must_use]
pub fn conditions(
    value: &Value,
    read_by: mcf_core::build_identity::BuildIdentity,
) -> Option<Conditions> {
    Some(Conditions::new(read_by, floor(value)?))
}

/// One condition: present and readable, present and null, or absent.
fn condition(value: &Value, question: &str) -> Option<Attested<ConditionValue>> {
    match value.get(question)? {
        Value::Null => Some(Attested::Unknown),
        Value::Text(text) => Some(Attested::Known(ConditionValue::text(text.clone()))),
        Value::Integer(number) => Some(Attested::Known(ConditionValue::integer(*number))),
        // A condition written as a boolean, a list or an object is a shape this
        // version does not ask for. It is not decoded into text, because a
        // decoder that coerced would make a record say something nobody wrote.
        Value::Bool(_) | Value::List(_) | Value::Map(_) => None,
    }
}
