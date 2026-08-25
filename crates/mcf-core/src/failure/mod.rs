//! The failure type every fallible boundary in MCF returns.
//!
//! §3.1 is this project's central rule and A2 is its enforceable form: every
//! failure is caught, classified against the taxonomy, attributed to a
//! subsystem, and persisted with enough context to reconstruct it without a
//! rerun. B16 says to prefer the machine-checked form of a rule, so the three
//! axes are constructor arguments rather than fields somebody might leave
//! empty — [`Failure`] has no constructor that omits one, and the workspace
//! denies the constructs (`unwrap`, `expect`, `panic!`) that would let a
//! caller avoid producing one.
//!
//! What this module does **not** do is persist. B21 makes the record
//! sufficient exactly when the laboratory can rebuild the failure from it, and
//! that is B-004's store and B-009's laboratory. What is here is the shape the
//! record will hold, built so that a failure cannot reach the store missing an
//! axis.
//!
//! ```
//! use mcf_core::failure::{Attribution, Category, Disposition, Failure, Subsystem};
//!
//! let failure = Failure::new(
//!     Category::AccelDriverQueryFailed,
//!     Attribution::Machine,
//!     Disposition::Degraded,
//!     Subsystem::new("mcf-core::hardware"),
//!     "the driver is present and did not answer",
//! )
//! .with_context("attempts", "3");
//!
//! assert_eq!(failure.category().code(), "accel.driver.query_failed");
//! assert_eq!(failure.context_value("attempts"), Some("3"));
//! ```

mod axes;
mod category;

pub use axes::{Attribution, Disposition};
pub use category::{Category, Domain};

use core::fmt;

/// The part of MCF that observed the failure.
///
/// A `&'static str` so that it names a place in this codebase rather than
/// something assembled at runtime: A2 asks for attribution to a subsystem, and
/// a subsystem that varies per call site is a message, not an attribution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Subsystem(&'static str);

impl Subsystem {
    /// Names a subsystem. By convention the module path, as
    /// `mcf-core::hardware::nvml`.
    #[must_use]
    pub const fn new(path: &'static str) -> Self {
        Self(path)
    }

    /// The name.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        self.0
    }
}

impl fmt::Display for Subsystem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.0)
    }
}

/// One piece of the context a failure carries.
///
/// The key is static and the value is not: the *question* a context entry
/// answers is decided when the code is written, and only its answer is
/// discovered at runtime. That asymmetry is what makes a set of failures
/// queryable later (§3.3, structured first).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContextEntry {
    /// What this entry answers.
    pub key: &'static str,
    /// What the answer was.
    pub value: String,
}

/// A classified, attributed failure.
///
/// Construction requires all three axes, which is A2 expressed as a signature.
/// There is deliberately no `Default`, no `From<&str>` and no constructor that
/// takes a message alone: each of those is a way to produce a failure nobody
/// classified, and an unclassified failure is the silent one §3.1 forbids
/// wearing a type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Failure {
    category: Category,
    attribution: Attribution,
    disposition: Disposition,
    subsystem: Subsystem,
    detail: String,
    context: Vec<ContextEntry>,
    cause: Option<Box<Failure>>,
}

impl Failure {
    /// Classifies a failure.
    ///
    /// `detail` is one line saying what happened, in the present tense and
    /// without a remedy: what to do about it is the caller's judgment, and a
    /// remedy baked into a record is a guess that outlives the situation that
    /// produced it.
    #[must_use]
    pub fn new(
        category: Category,
        attribution: Attribution,
        disposition: Disposition,
        subsystem: Subsystem,
        detail: impl Into<String>,
    ) -> Self {
        Self {
            category,
            attribution,
            disposition,
            subsystem,
            detail: detail.into(),
            context: Vec::new(),
            cause: None,
        }
    }

    /// Adds a piece of context.
    ///
    /// B21 measures the record by whether the laboratory can rebuild the
    /// failure from it. When it cannot, the fix is more of this at the failure
    /// site — never a return to ambient observation, which is the trade §6.15
    /// already made.
    #[must_use]
    pub fn with_context(mut self, key: &'static str, value: impl Into<String>) -> Self {
        self.context.push(ContextEntry {
            key,
            value: value.into(),
        });
        self
    }

    /// Records what this failure arose from.
    ///
    /// A1: never lose information. A failure that wraps another keeps it,
    /// rather than replacing it with a summary of it.
    #[must_use]
    pub fn caused_by(mut self, cause: Self) -> Self {
        self.cause = Some(Box::new(cause));
        self
    }

    /// What failed.
    #[must_use]
    pub const fn category(&self) -> Category {
        self.category
    }

    /// Whose failure it is.
    #[must_use]
    pub const fn attribution(&self) -> Attribution {
        self.attribution
    }

    /// What MCF did about it.
    #[must_use]
    pub const fn disposition(&self) -> Disposition {
        self.disposition
    }

    /// Where it was observed.
    #[must_use]
    pub const fn subsystem(&self) -> Subsystem {
        self.subsystem
    }

    /// The one-line statement of what happened.
    #[must_use]
    pub fn detail(&self) -> &str {
        &self.detail
    }

    /// Everything the failure carries, in the order it was added.
    #[must_use]
    pub fn context(&self) -> &[ContextEntry] {
        &self.context
    }

    /// The value of a context key, if it was recorded.
    #[must_use]
    pub fn context_value(&self, key: &str) -> Option<&str> {
        self.context
            .iter()
            .find(|entry| entry.key == key)
            .map(|entry| entry.value.as_str())
    }

    /// What this failure arose from, if anything.
    #[must_use]
    pub fn cause(&self) -> Option<&Self> {
        self.cause.as_deref()
    }

    /// This failure and every failure beneath it, outermost first.
    pub fn chain(&self) -> impl Iterator<Item = &Self> {
        core::iter::successors(Some(self), |failure| failure.cause())
    }

    /// Whether MCF could not classify this failure at all.
    ///
    /// The taxonomy makes `internal.unclassified` a tracked defect metric
    /// rather than a bucket: every occurrence is a missing category, and adding
    /// the category is the fix. This predicate is what lets the count be
    /// reported against its target of zero.
    #[must_use]
    pub fn is_unclassified(&self) -> bool {
        self.category == Category::InternalUnclassified
    }
}

impl fmt::Display for Failure {
    /// One line, all three axes.
    ///
    /// C1: this is a *rendering* of the failure, never the failure itself. The
    /// record keeps the structure; a surface that needs the full block the M0
    /// mockup draws builds it from the fields rather than parsing this.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} [{}, {}] at {}: {}",
            self.category, self.attribution, self.disposition, self.subsystem, self.detail
        )
    }
}

impl std::error::Error for Failure {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.cause
            .as_deref()
            .map(|cause| cause as &(dyn std::error::Error + 'static))
    }
}

/// What a fallible MCF boundary returns.
///
/// Named so that the alias, and not `core::result::Result`, is what appears in
/// signatures: A2's requirement is that the error side be classified, and an
/// alias that hard-codes [`Failure`] is how that becomes the path of least
/// resistance rather than a review comment.
pub type Result<T> = core::result::Result<T, Failure>;

#[cfg(test)]
mod tests;
