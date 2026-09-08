use mcf_core::failure::Failure;

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
