//! Everything the window says in passing, in one place.
//!
//! There used to be eleven of these, each its own `Option<String>` on the desk, each drawn
//! somewhere different, each choosing its own colour, and none of them ever expiring — a
//! line saying seventy gigabytes had been freed sat on the page until something unrelated
//! happened to replace it. Work under way was reported five more ways again.
//!
//! One notice now, with a tone. The tone picks the ink and the tone picks the clock, so how
//! long a thing stays is a property of what kind of thing it is rather than of whoever
//! wrote it.

use std::time::{Duration, Instant};

use mcf_core::time::Timestamp;

/// How long a notice that wants no answer stays. Long enough to read a sentence and look
/// up; short enough that the strip is about now rather than about earlier.
pub const SETTLES_AFTER: Duration = Duration::from_secs(8);

/// How many are kept to look back through. Past this the oldest go; the record keeps what
/// matters for longer, and `mcf failures` reads it back.
pub const KEPT: usize = 50;

/// What kind of thing is being said.
///
/// The order is the order of precedence: where several notices are live at once, the strip
/// shows the one that comes first here, because a refusal outranks a finished download and
/// work under way outranks both.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Tone {
    /// Something is happening. Lives as long as the work does, and is replaced by whatever
    /// the work came to.
    Working,
    /// Something needs answering. Stays until it is dismissed.
    Refused,
    /// Something is true and worth knowing while it stays true — a hold open to the
    /// network with no key. Goes when whoever raised it says the condition has gone.
    Warning,
    /// Something finished and wants nothing. Fades.
    Done,
}

impl Tone {
    /// Whether a notice of this tone goes on its own, and after how long.
    #[must_use]
    pub const fn fades_after(self) -> Option<Duration> {
        match self {
            Self::Done => Some(SETTLES_AFTER),
            // Work is cleared by whoever started it, when it stops being work. A refusal
            // and a warning are answered, not waited out.
            Self::Working | Self::Refused | Self::Warning => None,
        }
    }

    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Working => "working",
            Self::Refused => "refused",
            Self::Warning => "warning",
            Self::Done => "done",
        }
    }
}

/// One thing the window has to say.
#[derive(Debug, Clone)]
pub struct Notice {
    pub tone: Tone,
    /// One line, naming the thing it is about.
    pub what: String,
    /// The refusal's own context, or the figures. Quiet, and optional.
    pub detail: Option<String>,
    /// What this is about — a model's path, a transfer's number, an engine's name.
    ///
    /// A second notice about the same thing replaces the first rather than stacking, which
    /// is what keeps a download reporting itself once a second from filling the list.
    pub about: String,
    /// How far along, where that is known. Unknown is not nought: a share MCF was not
    /// given is not drawn.
    pub share: Option<f32>,
    /// When it was said, for the list.
    pub at: Timestamp,
    /// When it was said, for the clock. Monotonic, so a clock that steps does not expire
    /// everything at once.
    pub since: Instant,
}

impl Notice {
    #[must_use]
    pub fn new(tone: Tone, about: impl Into<String>, what: impl Into<String>) -> Self {
        Self {
            tone,
            what: what.into(),
            detail: None,
            about: about.into(),
            share: None,
            at: Timestamp::now(),
            since: Instant::now(),
        }
    }

    #[must_use]
    pub fn saying(mut self, detail: impl Into<String>) -> Self {
        let detail = detail.into();
        self.detail = (!detail.trim().is_empty()).then_some(detail);
        self
    }

    #[must_use]
    pub fn so_far(mut self, share: Option<f32>) -> Self {
        self.share = share.map(|share| share.clamp(0.0, 1.0));
        self
    }

    /// Whether its time is up.
    #[must_use]
    pub fn spent(&self, now: Instant) -> bool {
        self.tone
            .fades_after()
            .is_some_and(|after| now.saturating_duration_since(self.since) >= after)
    }

    /// The wall clock where this machine is, as hours and minutes.
    ///
    /// Local, not UTC: a list of times somebody reads against their own clock is a list in
    /// their own clock. Where the zone could not be read the time is UTC and says so.
    #[must_use]
    pub fn clock_said(&self) -> String {
        let (shifted, marked) = match self.at.offset() {
            mcf_core::attested::Attested::Known(offset) => (
                Timestamp::from_utc_nanos(
                    self.at
                        .utc_nanos()
                        .saturating_add(i128::from(offset.seconds_east()) * 1_000_000_000),
                    mcf_core::attested::Attested::Unknown,
                ),
                "",
            ),
            mcf_core::attested::Attested::Unknown => (self.at, "Z"),
        };
        let civil = shifted.civil_utc();
        format!("{:02}:{:02}{marked}", civil.hour, civil.minute)
    }
}

/// Everything the window has to say, newest last.
#[derive(Debug, Default)]
pub struct Notices {
    held: Vec<Notice>,
}

impl Notices {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Say something. Anything already said about the same subject gives way to it, so a
    /// transfer reporting itself once a second is one line rather than a thousand.
    pub fn say(&mut self, notice: Notice) {
        if let Some(at) = self.held.iter().position(|held| held.about == notice.about) {
            let _replaced = self.held.remove(at);
        }
        self.held.push(notice);
        while self.held.len() > KEPT {
            let _oldest = self.held.remove(0);
        }
    }

    pub fn working(&mut self, about: impl Into<String>, what: impl Into<String>) {
        self.say(Notice::new(Tone::Working, about, what));
    }

    pub fn done(&mut self, about: impl Into<String>, what: impl Into<String>) {
        self.say(Notice::new(Tone::Done, about, what));
    }

    pub fn refused(&mut self, about: impl Into<String>, what: impl Into<String>) {
        self.say(Notice::new(Tone::Refused, about, what));
    }

    pub fn warning(&mut self, about: impl Into<String>, what: impl Into<String>) {
        self.say(Notice::new(Tone::Warning, about, what));
    }

    /// Take back whatever was said about a subject — the work stopped, the condition went.
    pub fn forget(&mut self, about: &str) {
        self.held.retain(|held| held.about != about);
    }

    /// Whether anything is being said about a subject.
    #[must_use]
    pub fn about(&self, about: &str) -> Option<&Notice> {
        self.held.iter().find(|held| held.about == about)
    }

    /// Drop whatever has waited long enough. Called every frame.
    pub fn expire(&mut self, now: Instant) {
        self.held.retain(|held| !held.spent(now));
    }

    /// Put one away. What is dismissed is gone from the list too: it was answered.
    pub fn dismiss(&mut self, about: &str) {
        self.forget(about);
    }

    /// Put away everything that is not still happening.
    pub fn dismiss_the_settled(&mut self) {
        self.held.retain(|held| held.tone == Tone::Working);
    }

    /// What the strip shows: work first, then a refusal, then a warning, then what
    /// finished — and the newest of whichever that is.
    #[must_use]
    pub fn foremost(&self) -> Option<&Notice> {
        self.held
            .iter()
            .min_by(|one, two| one.tone.cmp(&two.tone).then(two.since.cmp(&one.since)))
    }

    /// Everything, newest first, for the list.
    #[must_use]
    pub fn recent(&self) -> Vec<&Notice> {
        let mut held: Vec<&Notice> = self.held.iter().collect();
        held.reverse();
        held
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.held.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.held.is_empty()
    }

    /// How many are refusals, which the strip says outright because they want answering.
    #[must_use]
    pub fn refusals(&self) -> usize {
        self.held
            .iter()
            .filter(|held| held.tone == Tone::Refused)
            .count()
    }
}

#[cfg(test)]
mod tests;
