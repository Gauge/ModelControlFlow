//! A pin, and what it means when a checkout comes back different (B-367, D39,
//! §3.12).
//!
//! A provisioned component is pinned to a commit so that its environment can be
//! restated on another machine. The pin is only worth anything if the checkout
//! that was *asked for* is the checkout that was *got* — and a remote can move,
//! a tag can be repointed, a hash can be mistyped. This is the judgement, kept
//! here rather than at the surface that happens to run `git`, so that it is
//! one judgement everywhere and one the laboratory can produce (A13, D26).

use crate::failure::{Attribution, Category, Disposition, Failure, Result, Subsystem};

const WHERE: Subsystem = Subsystem::new("mcf-core::provenance::pin");

/// Whether what was checked out is what was pinned.
///
/// # Errors
///
/// `exchange.reproduce.divergent` when it is not: what was reproduced differs
/// from the record that was meant to reproduce it, which is the same failure a
/// bundle has when a machine cannot restate its conditions — and it names both
/// hashes, because the difference is the whole of the finding.
pub fn checked_out(pinned: &str, got: &str) -> Result<()> {
    if pinned == got.trim() {
        return Ok(());
    }
    Err(Failure::new(
        Category::ExchangeReproduceDivergent,
        Attribution::Hub,
        Disposition::Aborted,
        WHERE,
        "the checkout did not land on the pinned commit",
    )
    .with_context("pinned", pinned.to_owned())
    .with_context("got", got.trim().to_owned()))
}

#[cfg(test)]
mod tests {
    use super::checked_out;
    use crate::failure::Category;

    #[test]
    fn the_same_commit_is_the_same_commit() {
        assert!(checked_out("abc123", "abc123\n").is_ok());
    }

    #[test]
    fn a_different_commit_is_named_on_both_sides() {
        let failure = checked_out("abc123", "def456").expect_err("diverged");
        assert_eq!(failure.category(), Category::ExchangeReproduceDivergent);
        let context: Vec<String> = failure
            .context()
            .iter()
            .map(|entry| format!("{}={}", entry.key, entry.value))
            .collect();
        assert!(context.contains(&"pinned=abc123".to_owned()), "{context:?}");
        assert!(context.contains(&"got=def456".to_owned()), "{context:?}");
    }
}
