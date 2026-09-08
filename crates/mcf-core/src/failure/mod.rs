mod axes;
mod category;

pub use axes::{Attribution, Disposition};
pub use category::{Branch, Category, Domain};

use core::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Subsystem(&'static str);

impl Subsystem {
    #[must_use]
    pub const fn new(path: &'static str) -> Self {
        Self(path)
    }

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContextEntry {
    pub key: &'static str,
    pub value: String,
}

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

    #[must_use]
    pub fn with_context(mut self, key: &'static str, value: impl Into<String>) -> Self {
        self.context.push(ContextEntry {
            key,
            value: value.into(),
        });
        self
    }

    #[must_use]
    pub fn caused_by(mut self, cause: Self) -> Self {
        self.cause = Some(Box::new(cause));
        self
    }

    #[must_use]
    pub const fn category(&self) -> Category {
        self.category
    }

    #[must_use]
    pub const fn attribution(&self) -> Attribution {
        self.attribution
    }

    #[must_use]
    pub const fn disposition(&self) -> Disposition {
        self.disposition
    }

    #[must_use]
    pub const fn subsystem(&self) -> Subsystem {
        self.subsystem
    }

    #[must_use]
    pub fn detail(&self) -> &str {
        &self.detail
    }

    #[must_use]
    pub fn context(&self) -> &[ContextEntry] {
        &self.context
    }

    #[must_use]
    pub fn context_value(&self, key: &str) -> Option<&str> {
        self.context
            .iter()
            .find(|entry| entry.key == key)
            .map(|entry| entry.value.as_str())
    }

    #[must_use]
    pub fn cause(&self) -> Option<&Self> {
        self.cause.as_deref()
    }

    pub fn chain(&self) -> impl Iterator<Item = &Self> {
        core::iter::successors(Some(self), |failure| failure.cause())
    }

    #[must_use]
    pub fn is_unclassified(&self) -> bool {
        self.category == Category::InternalUnclassified
    }
}

impl fmt::Display for Failure {
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

pub type Result<T> = core::result::Result<T, Failure>;

#[cfg(test)]
mod tests;

impl Failure {
    #[must_use]
    pub const fn branch(&self) -> Option<Branch> {
        self.attribution.branch()
    }
}
