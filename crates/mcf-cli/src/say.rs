//! How MCF says a failure to a person (A2, A6's habit, C1).
//!
//! **One renderer, because four were three too many.** Every command that can
//! refuse had grown its own way of showing a failure: the same three lines,
//! assembled slightly differently, so that the same refusal read differently
//! depending on which command produced it. A6 asks that a number never be shown
//! two ways; the same is true of a refusal, and for a better reason — an
//! operator learns to read one shape.
//!
//! **`Display` is one line by design, and one line is not enough here.** C1
//! makes the rendering a *view* of a failure rather than the failure: the record
//! keeps the structure, and a surface builds what it needs from the fields. What
//! a person needs is the context — which path, which repository, what to do —
//! and that is exactly what `Display` leaves out. So this is where it is put
//! back, once.
//!
//! **A cause is not a footnote.** A failure that was caused by another carries
//! it, and the chain is printed in order, because *the disk was full* explains
//! *the model was not acquired* and a reader given only the second is left
//! guessing at the first (A1).

use mcf_core::failure::Failure;

/// A refusal, with everything a person needs to act on it.
///
/// `what` is what MCF was doing, in the operator's terms — *nothing was
/// acquired*, *the daemon did not start*. The failure supplies the rest.
#[must_use]
pub(crate) fn refusal(what: &str, failure: &Failure) -> String {
    let mut lines = vec![format!("mcf: {what}"), format!("  {failure}")];
    for entry in failure.context() {
        lines.push(format!("    {}: {}", entry.key, entry.value));
    }
    for cause in failure.chain().skip(1) {
        lines.push(format!("  caused by: {cause}"));
        for entry in cause.context() {
            lines.push(format!("    {}: {}", entry.key, entry.value));
        }
    }
    lines.join("\n")
}

/// The same, for a place that has already said what it was doing.
///
/// The failure and its context, indented to sit under whatever came before it.
#[must_use]
pub(crate) fn beneath(failure: &Failure) -> String {
    let mut lines = vec![format!("  {failure}")];
    for entry in failure.context() {
        lines.push(format!("    {}: {}", entry.key, entry.value));
    }
    for cause in failure.chain().skip(1) {
        lines.push(format!("  caused by: {cause}"));
    }
    lines.join("\n")
}

/// Why the daemon refused, read from the body it refused with.
///
/// Five readers each looked for a key the encoding never writes and printed
/// a false statement of silence over a body that said exactly why (A2).
#[must_use]
pub(crate) fn refused_because(body: &mcf_record::json::Value) -> String {
    mcf_record::decode::failure_said(body).unwrap_or_else(|| {
        format!(
            "MCF refused with something that is not a failure: {}",
            body.to_line()
        )
    })
}

#[cfg(test)]
mod tests;
