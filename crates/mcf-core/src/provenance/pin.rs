use crate::failure::{Attribution, Category, Disposition, Failure, Result, Subsystem};

const WHERE: Subsystem = Subsystem::new("mcf-core::provenance::pin");

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
