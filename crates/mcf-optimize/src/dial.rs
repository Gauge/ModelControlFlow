#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Dial {
    #[default]
    ThinkingBudget,
    Temperature,
    TopP,
    TopK,
    MicroBatch,
    DraftDepth,
}

impl Dial {
    pub const ALL: [Self; 6] = [
        Self::ThinkingBudget,
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
            Self::Temperature | Self::TopP | Self::TopK => "",
        }
    }

    #[must_use]
    pub fn suggested(self) -> Vec<Step> {
        match self {
            Self::ThinkingBudget => [0, 512, 1024, 2048, 4096, 8192]
                .into_iter()
                .map(Step::Whole)
                .collect(),
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
            Self::Temperature | Self::TopP | Self::TopK => None,
        }
    }

    #[must_use]
    pub const fn field(self) -> Option<&'static str> {
        match self {
            Self::Temperature => Some("temperature"),
            Self::TopP => Some("top_p"),
            Self::TopK => Some("top_k"),
            Self::ThinkingBudget | Self::MicroBatch | Self::DraftDepth => None,
        }
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
