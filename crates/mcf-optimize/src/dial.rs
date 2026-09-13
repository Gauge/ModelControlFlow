#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Dial {
    #[default]
    ThinkingBudget,
    ThinkingLevel,
    Temperature,
    TopP,
    TopK,
    MicroBatch,
    DraftDepth,
}

impl Dial {
    pub const ALL: [Self; 7] = [
        Self::ThinkingBudget,
        Self::ThinkingLevel,
        Self::Temperature,
        Self::TopP,
        Self::TopK,
        Self::MicroBatch,
        Self::DraftDepth,
    ];

    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::ThinkingBudget => "Thinking budget",
            Self::ThinkingLevel => "Thinking level",
            Self::Temperature => "Temperature",
            Self::TopP => "Top-p",
            Self::TopK => "Top-k",
            Self::MicroBatch => "Micro-batch",
            Self::DraftDepth => "Draft depth",
        }
    }

    #[must_use]
    pub const fn reloads_the_engine(self) -> bool {
        matches!(
            self,
            Self::ThinkingBudget | Self::MicroBatch | Self::DraftDepth
        )
    }

    #[must_use]
    pub const fn unit(self) -> &'static str {
        match self {
            Self::ThinkingBudget => "tokens",
            Self::MicroBatch => "tokens per pass",
            Self::DraftDepth => "drafted tokens",
            Self::ThinkingLevel | Self::Temperature | Self::TopP | Self::TopK => "",
        }
    }

    #[must_use]
    pub const fn scale(self) -> Scale {
        match self {
            Self::ThinkingBudget
            | Self::ThinkingLevel
            | Self::MicroBatch
            | Self::DraftDepth
            | Self::TopK => Scale::Whole,
            Self::Temperature | Self::TopP => Scale::Thousandths,
        }
    }

    #[must_use]
    pub const fn span(self) -> Span {
        match self {
            Self::ThinkingBudget => Span::new(0, 32_768, 256),
            Self::ThinkingLevel => Span::new(0, 7, 1),
            Self::Temperature => Span::new(0, 1000, 25),
            Self::TopP => Span::new(500, 1000, 10),
            Self::TopK => Span::new(0, 200, 5),
            Self::MicroBatch => Span::new(64, 8192, 16),
            Self::DraftDepth => Span::new(0, 8, 1),
        }
    }

    #[must_use]
    pub fn coarse(self) -> Vec<Step> {
        let held: &[u32] = match self {
            Self::ThinkingBudget => &[0, 4096, 8192, 16_384, 32_768],
            Self::ThinkingLevel => &[0, 1, 2],
            Self::Temperature => &[0, 250, 500, 750, 1000],
            Self::TopP => &[500, 625, 750, 875, 1000],
            Self::TopK => &[0, 50, 100, 150, 200],
            Self::MicroBatch => &[64, 512, 1024, 2048, 4096],
            Self::DraftDepth => &[0, 2, 4, 6, 8],
        };
        held.iter().map(|held| self.step_of(*held)).collect()
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
            Self::DraftDepth => [0, 2, 3, 5].into_iter().map(Step::Whole).collect(),
        }
    }

    #[must_use]
    pub const fn flag(self) -> Option<&'static str> {
        match self {
            Self::ThinkingBudget => Some("--reasoning-budget"),
            Self::MicroBatch => Some("--ubatch-size"),
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
            Self::ThinkingBudget | Self::MicroBatch | Self::DraftDepth => None,
        }
    }

    /// A setting that cannot change what a model answers, only how fast it answers it.
    /// A micro-batch changes how the prompt is fed through, and a draft head's guesses are
    /// checked against the model itself, so both leave the tokens identical.
    #[must_use]
    pub const fn only_changes_speed(self) -> bool {
        matches!(self, Self::MicroBatch | Self::DraftDepth)
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
        if !self.is_named_by_the_model() {
            return step.said();
        }
        step.whole()
            .and_then(|at| named.get(usize::try_from(at).unwrap_or(usize::MAX)))
            .map_or_else(|| step.said(), Clone::clone)
    }

    #[must_use]
    pub fn read_among(self, typed: &str, named: &[String]) -> Option<Step> {
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

    #[must_use]
    pub fn rounded(self, value: u32) -> u32 {
        let finest = if self.finest == 0 { 1 } else { self.finest };
        let from = self.floor;
        let over = value.saturating_sub(from);
        let half = finest.checked_div(2).unwrap_or(0);
        let steps = over.saturating_add(half).checked_div(finest).unwrap_or(0);
        self.clamped(from.saturating_add(steps.saturating_mul(finest)))
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
        if self.steps.is_empty() || self.sets.is_empty() {
            return "nothing to run: choose at least one value and one set".to_owned();
        }
        let steps = self
            .steps
            .iter()
            .map(|step| step.said())
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
