//! The seeds a run draws from (B61, D19, B-290).
//!
//! **The failure this exists to prevent.** Thirty trials at one fixed seed
//! with identical inputs produce thirty identical outputs: `n=1` wearing the
//! costume of `n=30`. B61 puts it plainly — *a spread of zero reads as
//! remarkable consistency and is an artefact*. Fixing a seed does not reduce
//! variance; it conceals it, at the exact point §3.4 requires uncertainty to
//! be reported.
//!
//! **What MCF uses instead**, from D19: a declared seed *set*, with trial *i*
//! drawing seed *i*. Reproducible across machines because everyone draws the
//! same seeds; genuinely varied within a run because the seeds differ trial to
//! trial; and not exposed to an unlucky draw, because the result rests on the
//! whole set rather than on one trajectory.
//!
//! **D19 assumed a trial count, and F55 took it away.** *"The set's size is the
//! trial count and is therefore the same decision as §7.23's statistics"* was
//! written when a benchmark was expected to declare how many repeats it would
//! do. It cannot: F53 measured the same command needing seven repeats in one
//! sitting and over a hundred in another, and F55 replaced the count with a
//! stopping condition that finds out as it goes. A fixed list of thirty seeds
//! would run out on the thirty-first trial, and *what happens then* has only
//! bad answers — wrap around and repeat a trajectory, or stop measuring
//! because the list ended.
//!
//! So MCF's published set is **stated as arithmetic rather than as a list**: a
//! function from trial index to seed, unbounded, identical on every machine,
//! and short enough to write into a document. Everything D19 asked of a list it
//! gives, and it gives one more thing — the mapping is a **bijection**, so no
//! two trials can draw the same seed, which a list of literals could only
//! promise by being checked.
//!
//! **A seed is not part of identity** (D17, D18). Sampling parameters are
//! identity because they change the distribution; a seed only draws from it. So
//! the set is a *condition*, recorded beside every measurement and checked
//! before two of them are compared (A8).
//!
//! **Timing laboratories ignore seeds.** D19 is explicit: a seed changes which
//! tokens are produced and therefore possibly how many, and a timing that
//! varies because one run stopped earlier is measuring the stop. A timing trial
//! pins its generation length instead, and says so — see [`Draw`].

use core::fmt;

/// The additive constant of the published stream.
///
/// The golden-ratio constant, which is the conventional choice for this mixer
/// and is stated here so that the whole set is one screen of arithmetic.
const STRIDE: u64 = 0x9E37_79B9_7F4A_7C15;
/// The stream's two multipliers.
const FIRST: u64 = 0xBF58_476D_1CE4_E5B9;
const SECOND: u64 = 0x94D0_49BB_1331_11EB;

/// The name MCF's own published set is recorded under.
///
/// Stable for life (C5): a measurement written today names this, and a record
/// whose seed set cannot be identified is a record whose comparability nobody
/// can check.
pub const STANDARD: &str = "mcf-standard-v1";

/// Where a run's seeds come from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SeedSet {
    /// MCF's published set: unbounded, stated as arithmetic, identical on
    /// every machine.
    Standard,
    /// A set somebody declared in full.
    ///
    /// Admitted because a laboratory may need to reproduce somebody else's
    /// run, and refusing would make MCF unable to check another tool's work —
    /// which is the opposite of §II. It is finite, so it runs out, and running
    /// out is reported rather than wrapped around.
    Declared {
        /// What it is called, so a record can name it.
        name: String,
        /// The seeds, in trial order.
        seeds: Vec<u64>,
    },
}

/// Why a list of numbers is not a seed set.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NotASeedSet {
    /// Fewer than two seeds, which is one trajectory however many trials run.
    TooFew {
        /// How many were given.
        given: usize,
    },
    /// A seed appears twice.
    ///
    /// **This is B61's violation in miniature.** Two trials drawing the same
    /// seed produce the same trajectory, and the pair of them reports a spread
    /// of zero between two things that were never two things.
    Repeated {
        /// The seed that appears more than once.
        seed: u64,
        /// Where it appears again.
        at: usize,
    },
    /// The set has no name, so nothing could record which set was used.
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
    /// Declares a seed set, or says why it is not one.
    ///
    /// # Errors
    ///
    /// Where it has fewer than two seeds, repeats one, or has no name.
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

    /// The seed trial `index` draws.
    ///
    /// `None` where a declared set has run out, which is a fact to report
    /// rather than to wrap around: repeating the list would repeat a
    /// trajectory, which is the whole failure (B61). The published set never
    /// runs out.
    #[must_use]
    pub fn seed_for(&self, index: usize) -> Option<u64> {
        match self {
            Self::Standard => Some(published(u64::try_from(index).unwrap_or(u64::MAX))),
            Self::Declared { seeds, .. } => seeds.get(index).copied(),
        }
    }

    /// What the record calls this set.
    #[must_use]
    pub fn identifier(&self) -> String {
        match self {
            Self::Standard => STANDARD.to_owned(),
            Self::Declared { name, .. } => format!("declared:{name}"),
        }
    }

    /// How many trials it can supply, where that is bounded.
    ///
    /// Named `supply` rather than `len` because a seed set is not a container:
    /// the published one has no length at all, and `is_empty` is a question
    /// nothing here can be asked — a set with no seeds is not a set (see
    /// [`NotASeedSet::TooFew`]).
    ///
    /// `None` for the published set, which is unbounded — the answer F55's
    /// stopping condition needs, since how many trials a run will take is not
    /// known when it starts.
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

/// The published set's seed for a trial index.
///
/// A stride and two mix rounds — the conventional finalizer for this kind of
/// stream, written out so that the whole of MCF's seed set is six lines
/// somebody can reimplement in any language and check against.
///
/// **Every step is a bijection on `u64`**: adding a constant, multiplying by an
/// odd constant, and `x ^ (x >> k)`. Their composition is therefore a bijection,
/// so two different trial indices cannot draw the same seed. B61's violation is
/// unreachable here by arithmetic rather than by inspection.
///
/// Wrapping rather than saturating, and deliberately: this is a mixing function
/// and not a measurement, so wrap-around is the operation rather than a lost
/// number. It is the one place in this crate where that is true, which is why
/// it is said here.
#[must_use]
pub const fn published(index: u64) -> u64 {
    let mut held = index.wrapping_add(STRIDE);
    held = (held ^ (held >> 30)).wrapping_mul(FIRST);
    held = (held ^ (held >> 27)).wrapping_mul(SECOND);
    held ^ (held >> 31)
}

/// What a trial drew, and under which discipline.
///
/// A trial cannot be built without one (B-290): the two disciplines answer
/// different questions and a number that does not say which it was taken under
/// is a number nobody can place.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Draw {
    /// Trial *i* drew seed *i* from a declared set (B61, D19).
    ///
    /// The discipline for anything that measures *behaviour*: what a model
    /// produced, how often it obeyed a format, whether it repeated itself.
    Seeded {
        /// The seed this trial actually drew.
        seed: u64,
        /// Which set it came from, as the record names it.
        from: String,
    },
    /// A timing trial: the seed is held still and the generation length is
    /// pinned instead.
    ///
    /// D19, in its own words: *a seed changes which tokens are produced and
    /// therefore possibly how many, and a timing that varies because one run
    /// stopped earlier is measuring the stop, not the speed.* So a timing
    /// laboratory does the opposite of a behaviour one — it fixes the seed and
    /// fixes the length, and the variance it reports is the machine's.
    ///
    /// This is not an exemption from B61. It is the other half of the same
    /// rule, and it is a distinct variant so that a reader can never mistake a
    /// timing trial's fixed seed for a behaviour trial's mistake.
    LengthPinned {
        /// The seed held still.
        seed: u64,
        /// The generation length pinned, in tokens.
        tokens: u32,
    },
}

impl Draw {
    /// The seed, whichever discipline this was.
    #[must_use]
    pub const fn seed(&self) -> u64 {
        match *self {
            Self::Seeded { seed, .. } | Self::LengthPinned { seed, .. } => seed,
        }
    }

    /// Whether this trial's seed varies with the trial index.
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
