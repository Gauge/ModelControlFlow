use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

/// Whether a transfer has been asked to stop.
///
/// A transfer of a model file runs for minutes or hours, and the operator is entitled to
/// stop one part way through — to pause it, or to give it up. Stopping is asked for here
/// and read where the bytes are written, so a transfer stops where it stands rather than
/// when the file it is on happens to finish.
///
/// What has already arrived is left on disk. The partial file is what a resumed transfer
/// continues from, so stopping costs nothing that was already paid for; a transfer given
/// up for good is swept by whoever gave it up, not here.
#[derive(Debug, Clone, Default)]
pub struct Stopping(Option<Arc<AtomicBool>>);

impl Stopping {
    /// A transfer nobody can stop: what a one-shot command asks for, where the only way
    /// to stop it is to stop the command.
    #[must_use]
    pub const fn never() -> Self {
        Self(None)
    }

    #[must_use]
    pub const fn on(asked: Arc<AtomicBool>) -> Self {
        Self(Some(asked))
    }

    #[must_use]
    pub fn asked(&self) -> bool {
        self.0
            .as_ref()
            .is_some_and(|asked| asked.load(Ordering::Relaxed))
    }
}

#[cfg(test)]
mod tests;
