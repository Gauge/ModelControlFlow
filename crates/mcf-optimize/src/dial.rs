/// The settings a sweep can move, in the order they are offered. How a prompt is fed
/// through first, then how much the model thinks about it, then how it draws its tokens,
/// then the draft head — which is the order somebody dialling a model in goes through them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Dial {
    #[default]
    MicroBatch,
    Batch,
    CacheWidth,
    Experts,
    FlashAttention,
    ThreadsForAPrompt,
    ThinkingLevel,
    ThinkingBudget,
    Temperature,
    TopP,
    TopK,
    DraftDepth,
}

impl Dial {
    pub const ALL: [Self; 12] = [
        Self::MicroBatch,
        Self::Batch,
        Self::CacheWidth,
        Self::Experts,
        Self::FlashAttention,
        Self::ThreadsForAPrompt,
        Self::ThinkingLevel,
        Self::ThinkingBudget,
        Self::Temperature,
        Self::TopP,
        Self::TopK,
        Self::DraftDepth,
    ];

    /// What a level named this means: not a word for the template to read, but the whole
    /// thinking section cut off before it starts. The engine enforces it by watching for the
    /// tag the template opens a thinking section with, so it holds whatever the template
    /// makes of the words around it.
    pub const OFF: &'static str = "off";

    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::ThinkingBudget => "Thinking budget",
            Self::ThinkingLevel => "Thinking level",
            Self::Temperature => "Temperature",
            Self::TopP => "Top-p",
            Self::TopK => "Top-k",
            Self::MicroBatch => "Micro-batch",
            Self::Batch => "Prompt batch",
            Self::CacheWidth => "Cache width",
            Self::Experts => "Experts",
            Self::FlashAttention => "Flash attention",
            Self::ThreadsForAPrompt => "Threads for reading a prompt",
            Self::DraftDepth => "Draft depth",
        }
    }

    #[must_use]
    pub const fn reloads_the_engine(self) -> bool {
        matches!(
            self,
            Self::ThinkingBudget
                | Self::MicroBatch
                | Self::Batch
                | Self::DraftDepth
                | Self::CacheWidth
                | Self::Experts
                | Self::FlashAttention
                | Self::ThreadsForAPrompt
        )
    }

    #[must_use]
    pub const fn unit(self) -> &'static str {
        match self {
            Self::ThinkingBudget => "tokens",
            Self::MicroBatch => "tokens per pass",
            Self::Batch => "tokens a batch",
            Self::ThreadsForAPrompt => "threads",
            Self::DraftDepth => "drafted tokens",
            Self::CacheWidth
            | Self::Experts
            | Self::FlashAttention
            | Self::ThinkingLevel
            | Self::Temperature
            | Self::TopP
            | Self::TopK => "",
        }
    }

    #[must_use]
    pub const fn scale(self) -> Scale {
        match self {
            Self::ThinkingBudget
            | Self::ThinkingLevel
            | Self::MicroBatch
            | Self::Batch
            | Self::DraftDepth
            | Self::CacheWidth
            | Self::Experts
            | Self::FlashAttention
            | Self::ThreadsForAPrompt
            | Self::TopK => Scale::Whole,
            Self::Temperature | Self::TopP => Scale::Thousandths,
        }
    }

    #[must_use]
    pub const fn span(self) -> Span {
        match self {
            Self::ThinkingBudget => Span::new(0, crate::trial::TOKENS_THOUGHT, 256),
            Self::ThinkingLevel => Span::new(0, 7, 1),
            Self::Temperature => Span::new(0, 1000, 25),
            Self::TopP => Span::new(500, 1000, 10),
            Self::TopK => Span::new(0, 200, 5),
            Self::MicroBatch | Self::Batch => Span::new(64, 32_768, 256),
            Self::Experts | Self::FlashAttention => Span::new(0, 1, 1),
            Self::ThreadsForAPrompt => Span::new(1, 256, 1),
            Self::DraftDepth | Self::CacheWidth => Span::new(0, 8, 1),
        }
    }

    /// Where an automatic search starts climbing. The bottom of the span for most
    /// settings, so that the value which turns the setting off is the first thing tried;
    /// a micro-batch starts higher, because a pass smaller than this is slower than it is
    /// worth holding the model again to measure.
    #[must_use]
    pub const fn climbs_from(self) -> u32 {
        match self {
            Self::MicroBatch | Self::Batch => 256,
            Self::ThreadsForAPrompt => 4,
            Self::ThinkingBudget
            | Self::ThinkingLevel
            | Self::Temperature
            | Self::TopP
            | Self::TopK
            | Self::DraftDepth
            | Self::CacheWidth
            | Self::Experts
            | Self::FlashAttention => self.span().floor,
        }
    }

    /// How a climb gets from one rung to the next.
    ///
    /// A setting whose useful values are spread over orders of magnitude is walked by
    /// doubling: a micro-batch runs from two hundred and fifty six to thirty-two thousand,
    /// and stepping through that evenly would be a hundred and twenty rungs.
    ///
    /// One whose values live on a short even scale is walked in equal steps. Doubling
    /// through a temperature spends its first rungs at nought point nought two five,
    /// nought point nought five and nought point one — three trials inside a tenth of the
    /// scale, where nothing an answer does can be told apart — and then steps from nought
    /// point four straight over nought point six to nought point eight.
    #[must_use]
    pub const fn climbs_by(self) -> Climb {
        match self {
            Self::MicroBatch | Self::Batch | Self::ThinkingBudget | Self::DraftDepth => {
                Climb::Doubling
            }
            Self::Temperature => Climb::Evenly(200),
            Self::TopP => Climb::Evenly(100),
            Self::TopK => Climb::Evenly(40),
            Self::ThinkingLevel | Self::CacheWidth | Self::Experts | Self::FlashAttention => {
                Climb::Evenly(1)
            }
            Self::ThreadsForAPrompt => Climb::Doubling,
        }
    }

    /// The rung above this one.
    #[must_use]
    pub fn climbs_to(self, from: u32) -> u32 {
        let span = self.span();
        match self.climbs_by() {
            // Nothing doubled is still nothing, so the first rung above it is the finest
            // step the setting takes.
            Climb::Doubling if from == 0 => span.clamped(span.finest),
            Climb::Doubling => span.clamped(from.saturating_mul(2)),
            Climb::Evenly(by) => span.clamped(from.saturating_add(by)),
        }
    }

    #[must_use]
    pub const fn step_of(self, value: u32) -> Step {
        match self.scale() {
            Scale::Whole => Step::Whole(value),
            Scale::Thousandths => Step::Thousandths(value),
        }
    }

    #[must_use]
    pub fn suggested(self) -> Vec<Step> {
        match self {
            Self::ThinkingBudget => [0, 512, 1024, 2048, 4096, 8192]
                .into_iter()
                .map(Step::Whole)
                .collect(),
            Self::ThinkingLevel => (0..3).map(Step::Whole).collect(),
            Self::Temperature => [0, 200, 400, 600, 800, 1000]
                .into_iter()
                .map(Step::Thousandths)
                .collect(),
            Self::TopP => [800, 900, 950, 1000]
                .into_iter()
                .map(Step::Thousandths)
                .collect(),
            Self::TopK => [0, 20, 40, 100].into_iter().map(Step::Whole).collect(),
            Self::MicroBatch => [256, 512, 1024, 2048]
                .into_iter()
                .map(Step::Whole)
                .collect(),
            Self::Batch => [512, 1024, 2048, 4096]
                .into_iter()
                .map(Step::Whole)
                .collect(),
            Self::DraftDepth => [0, 2, 3, 5].into_iter().map(Step::Whole).collect(),
            // Every width the engine takes, because which one costs what is the question
            // and a shortlist would be MCF answering it in advance.
            Self::CacheWidth => (0..9).map(Step::Whole).collect(),
            Self::Experts | Self::FlashAttention => (0..2).map(Step::Whole).collect(),
            Self::ThreadsForAPrompt => [2, 4, 8, 16, 32].into_iter().map(Step::Whole).collect(),
        }
    }

    #[must_use]
    pub const fn flag(self) -> Option<&'static str> {
        match self {
            Self::ThinkingBudget => Some("--reasoning-budget"),
            Self::MicroBatch => Some("--ubatch-size"),
            Self::Batch => Some("--batch-size"),
            Self::CacheWidth => Some("--cache-type-k/v"),
            Self::Experts => Some("--cpu-moe"),
            Self::FlashAttention => Some("--flash-attn"),
            Self::ThreadsForAPrompt => Some("--threads-batch"),
            Self::DraftDepth => Some("--spec-draft-n-max"),
            Self::ThinkingLevel | Self::Temperature | Self::TopP | Self::TopK => None,
        }
    }

    #[must_use]
    pub const fn field(self) -> Option<&'static str> {
        match self {
            Self::Temperature => Some("temperature"),
            Self::TopP => Some("top_p"),
            Self::TopK => Some("top_k"),
            Self::ThinkingLevel => Some("reasoning_effort"),
            Self::ThinkingBudget
            | Self::MicroBatch
            | Self::Batch
            | Self::DraftDepth
            | Self::CacheWidth
            | Self::Experts
            | Self::FlashAttention
            | Self::ThreadsForAPrompt => None,
        }
    }

    /// A setting that cannot change what a model answers, only how fast it answers it. A
    /// micro-batch is how many prompt tokens go through the device in one pass, and a
    /// prompt batch is how many are handed over at a time for it to work through; the
    /// tokens that come back are the same tokens whatever either is, so there is nothing
    /// for a marked answer to say about them.
    ///
    /// A draft head is not on this list, though its guesses are checked against the model.
    /// The check preserves the distribution rather than the draw, so what comes back is a
    /// legitimate answer and not necessarily the same one — which is a thing worth marking.
    #[must_use]
    pub const fn cannot_change_an_answer(self) -> bool {
        matches!(
            self,
            Self::MicroBatch
                | Self::Batch
                | Self::Experts
                | Self::FlashAttention
                | Self::ThreadsForAPrompt
        )
    }

    /// What a sweep of this setting is ranked by before anybody says otherwise. Marking the
    /// answers is the useful default nearly everywhere: a setting is worth moving because of
    /// what it does to the answers, and the rate comes off the same run for free. The
    /// exception is the one setting that cannot touch an answer at all.
    #[must_use]
    pub const fn ranked_by(self) -> crate::reading::Measure {
        if self.cannot_change_an_answer() {
            return crate::reading::Measure::Speed;
        }
        crate::reading::Measure::Correctness
    }

    /// A speed trial times one of two pieces of work, and they are not the same work. The
    /// micro-batch is how many prompt tokens the device takes in one pass, so it shows in
    /// how fast a prompt is read and not at all in how fast an answer is written: an answer
    /// is written one token at a time whatever the micro-batch is. A draft head is the
    /// other way round. Timing the wrong one reads the same number back at every value.
    #[must_use]
    pub const fn times_reading_the_prompt(self) -> bool {
        matches!(
            self,
            Self::MicroBatch | Self::Batch | Self::FlashAttention | Self::ThreadsForAPrompt
        )
    }

    /// Whether this setting's values are places in a list rather than points on a scale.
    ///
    /// A cache width is not a bigger or smaller version of the width beside it — f16 and
    /// bf16 are the same size and different arithmetic — and where the experts sit is two
    /// places, not a range. Nothing like that is climbed: a search doubles or steps
    /// towards better, which needs the values to be ordered, and these are only listed.
    /// They are swept across every value instead, which is what a list of nine or two is
    /// short enough to allow.
    #[must_use]
    pub fn values_are_a_list(self) -> bool {
        self.is_named_by_the_model() || !self.own_words().is_empty()
    }

    #[must_use]
    pub const fn is_named_by_the_model(self) -> bool {
        matches!(self, Self::ThinkingLevel)
    }

    #[must_use]
    pub fn said(self, step: Step) -> String {
        step.said()
    }

    #[must_use]
    pub fn said_among(self, step: Step, named: &[String]) -> String {
        if let Some(word) = self.own_words().get(
            step.whole()
                .and_then(|at| usize::try_from(at).ok())
                .unwrap_or(usize::MAX),
        ) {
            return (*word).to_owned();
        }
        if !self.is_named_by_the_model() {
            return step.said();
        }
        step.whole()
            .and_then(|at| named.get(usize::try_from(at).unwrap_or(usize::MAX)))
            .map_or_else(|| step.said(), Clone::clone)
    }

    /// The words this setting's values go by, where they are words and MCF's own rather
    /// than the model's.
    ///
    /// A cache width, where the experts sit and whether flash attention is on are not
    /// numbers, and showing a person "2" where the engine will read "bf16" would be
    /// showing them the index of the answer instead of the answer. The order is the order
    /// of the values, so a step is a place in this list.
    #[must_use]
    pub const fn own_words(self) -> &'static [&'static str] {
        match self {
            Self::CacheWidth => &[
                "f32", "f16", "bf16", "q8_0", "q5_1", "q5_0", "q4_1", "q4_0", "iq4_nl",
            ],
            Self::Experts => &["with the model", "on the processor"],
            Self::FlashAttention => &["off", "on"],
            _ => &[],
        }
    }

    #[must_use]
    pub fn read_among(self, typed: &str, named: &[String]) -> Option<Step> {
        if let Some(at) = self
            .own_words()
            .iter()
            .position(|word| word.eq_ignore_ascii_case(typed.trim()))
            .and_then(|at| u32::try_from(at).ok())
        {
            return Some(Step::Whole(at));
        }
        if !self.is_named_by_the_model() {
            return None;
        }
        let wanted = typed.trim().to_ascii_lowercase();
        named
            .iter()
            .position(|held| held.to_ascii_lowercase() == wanted)
            .and_then(|at| u32::try_from(at).ok())
            .map(Step::Whole)
    }
}

/// How the rungs of a climb are spaced.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Climb {
    Doubling,
    Evenly(u32),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scale {
    Whole,
    Thousandths,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    pub floor: u32,
    pub ceiling: u32,
    pub finest: u32,
}

impl Span {
    #[must_use]
    pub const fn new(floor: u32, ceiling: u32, finest: u32) -> Self {
        Self {
            floor,
            ceiling,
            finest,
        }
    }

    #[must_use]
    pub const fn holds(self, value: u32) -> bool {
        value >= self.floor && value <= self.ceiling
    }

    #[must_use]
    pub const fn clamped(self, value: u32) -> u32 {
        if value < self.floor {
            self.floor
        } else if value > self.ceiling {
            self.ceiling
        } else {
            value
        }
    }

    /// The nearest value on the grain, counted from nothing rather than from the floor, and
    /// then brought inside the span. Counting from nothing is what keeps a coarse grain on
    /// round numbers: a micro-batch counted from a floor of 64 lands on 320 and 576, and a
    /// setting nobody would type by hand is a poor thing for a search to report back.
    #[must_use]
    pub fn rounded(self, value: u32) -> u32 {
        let finest = if self.finest == 0 { 1 } else { self.finest };
        let half = finest.checked_div(2).unwrap_or(0);
        let steps = value.saturating_add(half).checked_div(finest).unwrap_or(0);
        self.clamped(steps.saturating_mul(finest))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    Whole(u32),
    Thousandths(u32),
}

impl Step {
    #[must_use]
    pub fn said(self) -> String {
        match self {
            Self::Whole(held) => held.to_string(),
            Self::Thousandths(held) => {
                let whole = held.checked_div(1000).unwrap_or(0);
                let part = held.checked_rem(1000).unwrap_or(0);
                if part == 0 {
                    whole.to_string()
                } else {
                    format!("{whole}.{part:03}")
                        .trim_end_matches('0')
                        .to_owned()
                }
            }
        }
    }

    #[must_use]
    pub const fn whole(self) -> Option<u32> {
        match self {
            Self::Whole(held) => Some(held),
            Self::Thousandths(_) => None,
        }
    }

    #[must_use]
    pub const fn thousandths(self) -> Option<mcf_core::configuration::Thousandths> {
        match self {
            Self::Thousandths(held) => Some(mcf_core::configuration::Thousandths(held)),
            Self::Whole(_) => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sweep {
    pub dial: Dial,
    pub steps: Vec<Step>,
    pub sets: Vec<usize>,
    pub repeats: u8,
}

impl Default for Sweep {
    fn default() -> Self {
        Self {
            dial: Dial::default(),
            steps: Dial::default().suggested(),
            sets: (1..=8).collect(),
            repeats: 1,
        }
    }
}

impl Sweep {
    #[must_use]
    pub fn trials(&self) -> usize {
        self.steps
            .len()
            .saturating_mul(self.sets.len())
            .saturating_mul(usize::from(self.repeats.max(1)))
    }

    #[must_use]
    pub fn tasks(&self) -> usize {
        self.trials().saturating_mul(8)
    }

    #[must_use]
    pub fn on(dial: Dial) -> Self {
        Self {
            dial,
            steps: dial.suggested(),
            ..Self::default()
        }
    }

    #[must_use]
    pub fn said(&self) -> String {
        self.said_among(&[])
    }

    /// What this sweep would run, in the words the values go by. A setting whose values are
    /// named by the model is swept over the places in that list rather than over numbers,
    /// and a summary that prints the places is a summary of something nobody set.
    #[must_use]
    pub fn said_among(&self, named: &[String]) -> String {
        if self.steps.is_empty() || self.sets.is_empty() {
            return "nothing to run: choose at least one value and one set".to_owned();
        }
        let steps = self
            .steps
            .iter()
            .map(|step| self.dial.said_among(*step, named))
            .collect::<Vec<_>>()
            .join(", ");
        format!(
            "{} over {steps} — {} trials, {} tasks",
            self.dial.label(),
            self.trials(),
            self.tasks()
        )
    }
}

#[cfg(test)]
mod tests;
