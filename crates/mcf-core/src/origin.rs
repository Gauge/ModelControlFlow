use core::fmt;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LocallyMeasured<T> {
    value: T,
}

impl<T> LocallyMeasured<T> {
    #[must_use]
    pub const fn new(value: T) -> Self {
        Self { value }
    }

    #[must_use]
    pub const fn value(&self) -> &T {
        &self.value
    }

    pub fn map<U>(self, f: impl FnOnce(T) -> U) -> LocallyMeasured<U> {
        LocallyMeasured {
            value: f(self.value),
        }
    }
}

impl<T: fmt::Display> fmt::Display for LocallyMeasured<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} (measured here)", self.value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FromCorpus<T> {
    value: T,
    reports: usize,
}

impl<T> FromCorpus<T> {
    #[must_use]
    pub const fn new(value: T, reports: usize) -> Self {
        Self { value, reports }
    }

    #[must_use]
    pub const fn advises(&self) -> &T {
        &self.value
    }

    #[must_use]
    pub const fn reports(&self) -> usize {
        self.reports
    }

    pub fn map<U>(self, f: impl FnOnce(T) -> U) -> FromCorpus<U> {
        FromCorpus {
            value: f(self.value),
            reports: self.reports,
        }
    }
}

impl<T: fmt::Display> fmt::Display for FromCorpus<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.reports {
            0 => write!(f, "{} (from the corpus, resting on nothing)", self.value),
            1 => write!(f, "{} (from the corpus, 1 report)", self.value),
            n => write!(f, "{} (from the corpus, {n} reports)", self.value),
        }
    }
}

#[cfg(test)]
mod tests;
