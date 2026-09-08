use core::fmt;

const STRIDE: u64 = 0x9E37_79B9_7F4A_7C15;
const FIRST: u64 = 0xBF58_476D_1CE4_E5B9;
const SECOND: u64 = 0x94D0_49BB_1331_11EB;

pub const STANDARD: &str = "mcf-standard-v1";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SeedSet {
    Standard,
    Declared { name: String, seeds: Vec<u64> },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NotASeedSet {
    TooFew { given: usize },
    Repeated { seed: u64, at: usize },
    Unnamed,
}

impl fmt::Display for NotASeedSet {
    fn fmt(&self, form: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooFew { given } => write!(
                form,
                "{given} seed(s): a set of one is one trajectory however many trials draw from it \
                 (B61)"
            ),
            Self::Repeated { seed, at } => write!(
                form,
                "seed {seed} appears again at trial {at}: two trials drawing it produce the same \
                 trajectory, and report a spread of zero between two things that were never two \
                 things (B61)"
            ),
            Self::Unnamed => form.write_str(
                "a seed set with no name cannot be recorded, and a measurement whose seed set \
                 cannot be identified cannot be compared with another (D19, A8)",
            ),
        }
    }
}

impl SeedSet {
    pub fn declared(name: impl Into<String>, seeds: Vec<u64>) -> Result<Self, NotASeedSet> {
        let name = name.into();
        if name.trim().is_empty() {
            return Err(NotASeedSet::Unnamed);
        }
        if seeds.len() < 2 {
            return Err(NotASeedSet::TooFew { given: seeds.len() });
        }
        for (at, seed) in seeds.iter().enumerate() {
            if seeds.iter().take(at).any(|earlier| earlier == seed) {
                return Err(NotASeedSet::Repeated { seed: *seed, at });
            }
        }
        Ok(Self::Declared { name, seeds })
    }

    #[must_use]
    pub fn seed_for(&self, index: usize) -> Option<u64> {
        match self {
            Self::Standard => Some(published(u64::try_from(index).unwrap_or(u64::MAX))),
            Self::Declared { seeds, .. } => seeds.get(index).copied(),
        }
    }

    #[must_use]
    pub fn identifier(&self) -> String {
        match self {
            Self::Standard => STANDARD.to_owned(),
            Self::Declared { name, .. } => format!("declared:{name}"),
        }
    }

    #[must_use]
    pub fn supply(&self) -> Option<usize> {
        match self {
            Self::Standard => None,
            Self::Declared { seeds, .. } => Some(seeds.len()),
        }
    }
}

impl fmt::Display for SeedSet {
    fn fmt(&self, form: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Standard => write!(form, "{STANDARD} (unbounded)"),
            Self::Declared { name, seeds } => {
                write!(form, "declared:{name} ({} seeds)", seeds.len())
            }
        }
    }
}

#[must_use]
pub const fn published(index: u64) -> u64 {
    let mut held = index.wrapping_add(STRIDE);
    held = (held ^ (held >> 30)).wrapping_mul(FIRST);
    held = (held ^ (held >> 27)).wrapping_mul(SECOND);
    held ^ (held >> 31)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Draw {
    Seeded { seed: u64, from: String },
    LengthPinned { seed: u64, tokens: u32 },
}

impl Draw {
    #[must_use]
    pub const fn seed(&self) -> u64 {
        match *self {
            Self::Seeded { seed, .. } | Self::LengthPinned { seed, .. } => seed,
        }
    }

    #[must_use]
    pub const fn is_seeded(&self) -> bool {
        matches!(*self, Self::Seeded { .. })
    }
}

impl fmt::Display for Draw {
    fn fmt(&self, form: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Seeded { seed, from } => write!(form, "seed {seed} from {from}"),
            Self::LengthPinned { seed, tokens } => {
                write!(form, "seed {seed} held still, {tokens} token(s) pinned")
            }
        }
    }
}

#[cfg(test)]
mod tests;
