//! The sampling settings beyond temperature, top-p and top-k: the ones that keep a model
//! from repeating itself, and the floor under which a token is never drawn at all.
//!
//! Each is a default the engine is started with, for every caller that names none of its
//! own. A caller that does name one — an agent configured to send its own temperature, say —
//! overrides it for that request, because the engine reads the request before its defaults.
//! Many agent clients send a temperature and nothing else, so these are the ones that most
//! often decide how a hold behaves and are least often seen.

use mcf_core::configuration::Thousandths;
use mcf_record::json::Value;

/// One sampling setting.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Knob {
    MinP,
    PresencePenalty,
    FrequencyPenalty,
    RepeatPenalty,
    RepeatWindow,
    DryStrength,
    DryBase,
    DryAllowedRun,
    DryWindow,
}

/// Whether a setting is a whole count or a fraction written to three places.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Grain {
    Whole,
    Thousandths,
}

impl Knob {
    pub const ALL: [Self; 9] = [
        Self::MinP,
        Self::PresencePenalty,
        Self::FrequencyPenalty,
        Self::RepeatPenalty,
        Self::RepeatWindow,
        Self::DryStrength,
        Self::DryBase,
        Self::DryAllowedRun,
        Self::DryWindow,
    ];

    const fn at(self) -> usize {
        match self {
            Self::MinP => 0,
            Self::PresencePenalty => 1,
            Self::FrequencyPenalty => 2,
            Self::RepeatPenalty => 3,
            Self::RepeatWindow => 4,
            Self::DryStrength => 5,
            Self::DryBase => 6,
            Self::DryAllowedRun => 7,
            Self::DryWindow => 8,
        }
    }

    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::MinP => "Min-p",
            Self::PresencePenalty => "Presence penalty",
            Self::FrequencyPenalty => "Frequency penalty",
            Self::RepeatPenalty => "Repeat penalty",
            Self::RepeatWindow => "Repeat window",
            Self::DryStrength => "DRY strength",
            Self::DryBase => "DRY base",
            Self::DryAllowedRun => "DRY allowed run",
            Self::DryWindow => "DRY window",
        }
    }

    /// Its name in a sentence, the way the settings table names everything else.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::MinP => "min-p",
            Self::PresencePenalty => "presence penalty",
            Self::FrequencyPenalty => "frequency penalty",
            Self::RepeatPenalty => "repeat penalty",
            Self::RepeatWindow => "repeat window",
            Self::DryStrength => "DRY strength",
            Self::DryBase => "DRY base",
            Self::DryAllowedRun => "DRY allowed run",
            Self::DryWindow => "DRY window",
        }
    }

    /// The name the engine reads it by, in a request and in what MCF saves. One name for
    /// both, so a saved setting and a swept one are never two spellings of the same thing.
    #[must_use]
    pub const fn key(self) -> &'static str {
        match self {
            Self::MinP => "min_p",
            Self::PresencePenalty => "presence_penalty",
            Self::FrequencyPenalty => "frequency_penalty",
            Self::RepeatPenalty => "repeat_penalty",
            Self::RepeatWindow => "repeat_last_n",
            Self::DryStrength => "dry_multiplier",
            Self::DryBase => "dry_base",
            Self::DryAllowedRun => "dry_allowed_length",
            Self::DryWindow => "dry_penalty_last_n",
        }
    }

    #[must_use]
    pub const fn flag(self) -> &'static str {
        match self {
            Self::MinP => "--min-p",
            Self::PresencePenalty => "--presence-penalty",
            Self::FrequencyPenalty => "--frequency-penalty",
            Self::RepeatPenalty => "--repeat-penalty",
            Self::RepeatWindow => "--repeat-last-n",
            Self::DryStrength => "--dry-multiplier",
            Self::DryBase => "--dry-base",
            Self::DryAllowedRun => "--dry-allowed-length",
            Self::DryWindow => "--dry-penalty-last-n",
        }
    }

    #[must_use]
    pub const fn grain(self) -> Grain {
        match self {
            Self::RepeatWindow | Self::DryAllowedRun | Self::DryWindow => Grain::Whole,
            Self::MinP
            | Self::PresencePenalty
            | Self::FrequencyPenalty
            | Self::RepeatPenalty
            | Self::DryStrength
            | Self::DryBase => Grain::Thousandths,
        }
    }

    /// The lowest and highest it takes, in its own grain: thousandths for a fraction.
    #[must_use]
    pub const fn reach(self) -> (u32, u32) {
        match self {
            Self::MinP => (0, 1000),
            Self::PresencePenalty | Self::FrequencyPenalty => (0, 2000),
            Self::RepeatPenalty => (1000, 2000),
            Self::RepeatWindow | Self::DryWindow => (0, 131_072),
            Self::DryStrength => (0, 5000),
            Self::DryBase => (1001, 4000),
            Self::DryAllowedRun => (1, 64),
        }
    }

    /// What the engine does when nothing is set, as it says itself.
    #[must_use]
    pub const fn engine_default(self) -> &'static str {
        match self {
            Self::MinP => "0.05",
            Self::PresencePenalty | Self::FrequencyPenalty | Self::DryStrength => "0 (off)",
            Self::RepeatPenalty => "1 (off)",
            Self::RepeatWindow | Self::DryWindow => "64",
            Self::DryBase => "1.75",
            Self::DryAllowedRun => "2",
        }
    }

    #[must_use]
    pub const fn because(self) -> &'static str {
        match self {
            Self::MinP => {
                "never draw a token less likely than this share of the likeliest one. It \
                 trims the long tail a high temperature would otherwise reach into, and the \
                 engine applies one of its own when none is set — which is often why two \
                 setups of the same model behave differently"
            }
            Self::PresencePenalty => {
                "make any token that has already appeared a little less likely, once, however \
                 often it has appeared. Gentle, and the one model makers suggest for a model \
                 that loops"
            }
            Self::FrequencyPenalty => {
                "make a token less likely the more often it has appeared. It pushes a model off \
                 words the task needs as readily as off a loop, so it is used lightly"
            }
            Self::RepeatPenalty => {
                "make every token in the repeat window less likely. One is off. Code, JSON and \
                 tool calls have to repeat names, keys and brackets, so for an agent this \
                 stays at one or very near it"
            }
            Self::RepeatWindow => {
                "how many of the latest tokens the repeat penalty looks back over"
            }
            Self::DryStrength => {
                "penalise repeating a run of tokens that has already been written, harder the \
                 longer the run. Nought is off. It stops a model saying the same line again \
                 without punishing it for using a word twice — the setting for a model that \
                 loops"
            }
            Self::DryBase => {
                "how steeply the penalty grows with each token a repeated run gets longer"
            }
            Self::DryAllowedRun => {
                "how long a repeated run may get before it is penalised at all, so that short \
                 phrases a task needs are left alone"
            }
            Self::DryWindow => "how many of the latest tokens the DRY penalty looks back over",
        }
    }

    /// A value in this setting's grain, as the engine is told it.
    #[must_use]
    pub fn said(self, held: u32) -> String {
        match self.grain() {
            Grain::Whole => held.to_string(),
            Grain::Thousandths => Thousandths(held).to_string(),
        }
    }

    /// Read what somebody typed, or say why it is not a value this setting takes.
    ///
    /// # Errors
    /// When the text is not a number of the right kind, or is outside what this setting
    /// reaches.
    pub fn read(self, typed: &str) -> Result<u32, String> {
        let name = self.label().to_lowercase();
        let held = match self.grain() {
            Grain::Whole => typed
                .trim()
                .parse::<u32>()
                .map_err(|_| format!("{name} wants a whole number, not {typed:?}"))?,
            Grain::Thousandths => {
                typed
                    .trim()
                    .parse::<Thousandths>()
                    .map_err(|()| format!("{name} wants a number like 0.5, not {typed:?}"))?
                    .0
            }
        };
        let (least, most) = self.reach();
        if held < least || held > most {
            return Err(format!(
                "{name} runs from {} to {}",
                self.said(least),
                self.said(most)
            ));
        }
        Ok(held)
    }
}

/// Every sampling setting a hold is started with, each one unset until somebody sets it.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Sampling {
    held: [Option<u32>; 9],
}

impl Sampling {
    #[must_use]
    pub fn get(&self, knob: Knob) -> Option<u32> {
        self.held.get(knob.at()).copied().flatten()
    }

    pub fn set(&mut self, knob: Knob, held: Option<u32>) {
        if let Some(slot) = self.held.get_mut(knob.at()) {
            *slot = held;
        }
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.held.iter().all(Option::is_none)
    }

    /// The engine flags for everything that is set.
    #[must_use]
    pub fn arguments(&self) -> Vec<String> {
        let mut out = Vec::new();
        for knob in Knob::ALL {
            if let Some(held) = self.get(knob) {
                out.push(knob.flag().to_owned());
                out.push(knob.said(held));
            }
        }
        out
    }

    /// What is set, by the name each is saved under. Only what is set: settings saved before
    /// any of these existed are written and read back exactly as they were.
    #[must_use]
    pub fn pairs(&self) -> Vec<(&'static str, Value)> {
        Knob::ALL
            .into_iter()
            .filter_map(|knob| {
                let held = self.get(knob)?;
                Some((knob.key(), Value::Integer(i64::from(held))))
            })
            .collect()
    }

    #[must_use]
    pub fn from_value(value: &Value) -> Self {
        let mut held = Self::default();
        for knob in Knob::ALL {
            let read = value
                .get(knob.key())
                .and_then(Value::as_integer)
                .and_then(|it| u32::try_from(it).ok());
            held.set(knob, read);
        }
        held
    }

    /// One setting as the settings table shows it: its value, or what the engine does
    /// when it is left alone.
    #[must_use]
    pub fn shown(&self, knob: Knob) -> String {
        self.get(knob).map_or_else(
            || format!("the engine's own, {}", knob.engine_default()),
            |held| knob.said(held),
        )
    }
}

#[cfg(test)]
mod tests;
