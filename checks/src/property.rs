//! Generated-input properties: the tier that states an invariant rather than
//! an example.
//!
//! D10 names *unit and property tests for logic* as the first of the tiers, and
//! B-191 builds them. A unit test states that one input produces one output; a
//! property states something that holds for every input, and then tries to
//! falsify it. The difference matters most where MCF's own claims are
//! universal: A6 says the reported spread is five values that *were actually
//! observed*, B62 says a damaged journal reports what was lost *and how much*,
//! and neither is a statement about a particular input.
//!
//! **Generation is deterministic, and that is a decision rather than a
//! shortcut.** §3.12 makes reproducibility a precedence rule (P3), and a suite
//! that generates fresh inputs on every run gates a change against a different
//! question each time — a failure nobody can reproduce is a failure nobody can
//! fix, and a green run says only that today's inputs passed. So the seeds are
//! a fixed set, stated in [`BASE_SEED`], and a case that falsifies a property
//! names the seed that produced it. Exploration belongs to the fuzz tier
//! (`checks/tests/fuzz.rs`), which is scheduled rather than gating and says
//! outright which seed base it explored from.
//!
//! **A falsified property is reported, not thrown.** [`check`] returns a
//! [`Verdict`], the same shape `mcf_lab::run` uses for the same reason: the
//! harness reporting *its own* result as an error would be the manager dying
//! with the managed (A3, one level up). A property that panics — an overflow, a
//! slice out of range inside MCF's own code — is a finding too, and is caught
//! and reported with its seed rather than tearing down the run.

use std::panic::{AssertUnwindSafe, catch_unwind};

/// The seed every property tier run starts from.
///
/// Fixed, and changed only deliberately. Two runs of the gating tier on two
/// machines examine the same inputs, so a failure one of them reports is a
/// failure the other can reproduce from the seed alone (P3).
pub const BASE_SEED: u64 = 0x4D43_4630_0000_0001;

/// How many cases a gating-tier property examines.
///
/// Small enough that ten properties cost less than a second — B38 requires the
/// gating tier stay fast, because a gate people skip does not gate — and large
/// enough that a generator covering a handful of shapes reaches each of them
/// many times. The scheduled fuzz tier is where a large number belongs.
pub const GATING_CASES: u32 = 256;

/// How many cases a property that builds a real journal examines.
///
/// Fewer, and stated as its own number rather than derived from
/// [`GATING_CASES`]: each case writes files and calls `sync_data` per append,
/// so the cost is three orders of magnitude above a case that only computes.
/// A property that touches the disk is still worth having — B62's loss
/// reporting is a claim about a file — but it is not worth having at the same
/// count.
pub const FILESYSTEM_CASES: u32 = 32;

/// A deterministic source of generated values.
///
/// `splitmix64`: a stated, reproducible algorithm rather than the platform's,
/// so a seed means the same sequence on every machine and in every version of
/// the standard library. §3.12's requirement applied to the suite itself.
#[derive(Debug, Clone)]
pub struct Rng {
    state: u64,
}

impl Rng {
    /// A generator at a stated seed.
    #[must_use]
    pub const fn seeded(seed: u64) -> Self {
        Self { state: seed }
    }

    /// The next 64 bits.
    pub fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// A value below `bound`, or zero when `bound` is zero.
    ///
    /// Zero rather than a panic: A7's habit applied to a generator — the
    /// degenerate case has an answer, and it is not an aborted run.
    pub fn below(&mut self, bound: u64) -> u64 {
        if bound == 0 {
            0
        } else {
            self.next_u64() % bound
        }
    }

    /// An index below `bound`, or zero when `bound` is zero.
    pub fn index(&mut self, bound: usize) -> usize {
        usize::try_from(self.below(bound as u64)).unwrap_or(0)
    }

    /// A coin.
    pub fn boolean(&mut self) -> bool {
        self.next_u64() & 1 == 1
    }

    /// A byte.
    pub fn byte(&mut self) -> u8 {
        u8::try_from(self.next_u64() & 0xFF).unwrap_or(0)
    }

    /// An integer in `low..=high`, inclusive at both ends.
    ///
    /// Returns `low` when the range is inverted, which is the only total
    /// answer available and is stated rather than left to an overflow.
    pub fn integer_between(&mut self, low: i64, high: i64) -> i64 {
        if high <= low {
            return low;
        }
        let span = u64::try_from(i128::from(high) - i128::from(low)).unwrap_or(u64::MAX);
        let offset = self.below(span.saturating_add(1));
        i64::try_from(i128::from(low) + i128::from(offset)).unwrap_or(low)
    }

    /// Up to `max` arbitrary bytes.
    pub fn bytes(&mut self, max: usize) -> Vec<u8> {
        let length = self.index(max.saturating_add(1));
        (0..length).map(|_| self.byte()).collect()
    }

    /// Up to `max` characters, drawn to hit the places a codec breaks.
    ///
    /// Not uniform over `char`, and deliberately so: the interesting inputs for
    /// a JSON codec are the ones the format has opinions about — quote,
    /// backslash, the C0 controls it must escape numerically, the boundaries of
    /// the multi-byte encodings, and the astral plane that has to travel as a
    /// surrogate pair. A uniform sampler would spend nearly every case in the
    /// range that has never been a problem.
    pub fn text(&mut self, max: usize) -> String {
        let length = self.index(max.saturating_add(1));
        (0..length).map(|_| self.character()).collect()
    }

    /// One character from the classes a codec has to get right.
    pub fn character(&mut self) -> char {
        match self.below(8) {
            0 => '"',
            1 => '\\',
            2 => char::from_u32(self.below(0x20).try_into().unwrap_or(0)).unwrap_or('\0'),
            3 => char::from(self.byte() | 0x20).to_ascii_lowercase(),
            4 => {
                char::from_u32(0x80 + u32::try_from(self.below(0x700)).unwrap_or(0)).unwrap_or('é')
            }
            // `char::from_u32` already refuses the surrogate range, which is
            // the point of drawing from it: a generator that could not produce
            // the gap would never ask the codec about it.
            5 => char::from_u32(0x800 + u32::try_from(self.below(0xF000)).unwrap_or(0))
                .unwrap_or('☃'),
            6 => char::from_u32(0x1_0000 + u32::try_from(self.below(0xF_0000)).unwrap_or(0))
                .unwrap_or('🙂'),
            _ => char::from(b'a' + u8::try_from(self.below(26)).unwrap_or(0)),
        }
    }

    /// One of the choices, or `None` when there are none.
    pub fn pick<'a, T>(&mut self, choices: &'a [T]) -> Option<&'a T> {
        choices.get(self.index(choices.len()))
    }
}

/// What examining a property established.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Verdict {
    /// Every case held, and this is how many were examined.
    Held {
        /// The number of cases examined.
        cases: u32,
    },
    /// A case falsified it, and this is the seed that reproduces it.
    Falsified {
        /// The seed of the failing case.
        seed: u64,
        /// Which case it was, counting from zero.
        case: u32,
        /// What the property said about it.
        detail: String,
    },
}

impl Verdict {
    /// Whether every case held.
    #[must_use]
    pub const fn held(&self) -> bool {
        matches!(*self, Self::Held { .. })
    }
}

impl std::fmt::Display for Verdict {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Held { cases } => write!(f, "held over {cases} cases"),
            Self::Falsified { seed, case, detail } => write!(
                f,
                "falsified at case {case} (seed {seed:#018x}): {detail}\n  \
                 reproduce with Rng::seeded({seed:#018x})"
            ),
        }
    }
}

/// Examines a property over a stated number of generated cases.
///
/// The property returns `Err` with what it observed, rather than asserting, so
/// that the seed and the observation arrive together — a bare assertion failure
/// inside a generated case names neither. A property that panics is caught and
/// reported the same way: a panic inside MCF's own code is exactly the kind of
/// finding this tier exists to produce, and losing its seed would make it
/// unreproducible.
pub fn check<P>(cases: u32, property: P) -> Verdict
where
    P: Fn(&mut Rng) -> Result<(), String>,
{
    check_from(BASE_SEED, cases, property)
}

/// Examines a property from a stated seed base.
///
/// The fuzz tier uses this to explore from somewhere other than [`BASE_SEED`],
/// and says in its output which base it used, so that anything it finds is
/// reproducible from the report alone.
pub fn check_from<P>(base: u64, cases: u32, property: P) -> Verdict
where
    P: Fn(&mut Rng) -> Result<(), String>,
{
    for case in 0..cases {
        let seed = base.wrapping_add(u64::from(case).wrapping_mul(0x9E37_79B9_7F4A_7C15));
        let mut rng = Rng::seeded(seed);
        let outcome = catch_unwind(AssertUnwindSafe(|| property(&mut rng)));
        match outcome {
            Ok(Ok(())) => {}
            Ok(Err(detail)) => return Verdict::Falsified { seed, case, detail },
            Err(payload) => {
                return Verdict::Falsified {
                    seed,
                    case,
                    detail: format!("the property panicked: {}", describe(&payload)),
                };
            }
        }
    }
    Verdict::Held { cases }
}

/// What a caught panic said, where it said anything this can read.
fn describe(payload: &Box<dyn std::any::Any + Send>) -> String {
    payload.downcast_ref::<&str>().map_or_else(
        || {
            payload
                .downcast_ref::<String>()
                .cloned()
                .unwrap_or_else(|| "a payload of unknown type".to_owned())
        },
        |text| (*text).to_owned(),
    )
}

#[cfg(test)]
mod tests {
    use super::{BASE_SEED, Rng, Verdict, check, check_from};

    /// A seed is the whole state: two generators at one seed produce one
    /// sequence. Without this the seed a verdict prints would not be a way back
    /// to the failing case, and P3's reproducibility would be decorative.
    #[test]
    fn a_seed_reproduces_its_sequence() {
        let mut one = Rng::seeded(BASE_SEED);
        let mut other = Rng::seeded(BASE_SEED);
        let first: Vec<u64> = (0..64).map(|_| one.next_u64()).collect();
        let second: Vec<u64> = (0..64).map(|_| other.next_u64()).collect();
        assert_eq!(first, second);
        assert!(
            first.windows(2).any(|pair| pair.first() != pair.last()),
            "a generator that returns one value forever would pass every property"
        );
    }

    /// The harness's own negative control, and the reason it exists: a runner
    /// that cannot report a false property would report ten true ones and mean
    /// nothing (the lesson `scripts/check-lints-bite.sh` states for lints).
    #[test]
    fn a_false_property_is_falsified_and_names_its_seed() {
        let verdict = check(64, |rng| {
            let drawn = rng.below(1_000);
            if drawn == 0 {
                Ok(())
            } else {
                Err(format!("drew {drawn}"))
            }
        });
        match verdict {
            Verdict::Held { .. } => panic!("a property that fails almost always was not falsified"),
            Verdict::Falsified { seed, detail, .. } => {
                // The seed is the way back: replaying it alone reproduces the
                // same case, which is what makes a generated failure fixable.
                let mut replayed = Rng::seeded(seed);
                let drawn = replayed.below(1_000);
                assert_eq!(detail, format!("drew {drawn}"));
            }
        }
    }

    /// A property that panics is a finding about MCF, not a torn-down run
    /// (A3's shape, one level up).
    #[test]
    fn a_panicking_property_is_reported_rather_than_propagated() {
        let verdict = check(4, |_| panic!("an overflow deep in the code under test"));
        match verdict {
            Verdict::Held { .. } => panic!("a panicking property was reported as holding"),
            Verdict::Falsified { detail, .. } => {
                assert!(detail.contains("panicked"), "{detail}");
                assert!(detail.contains("an overflow"), "{detail}");
            }
        }
    }

    /// A true property holds over every case it was given, and says how many.
    #[test]
    fn a_true_property_holds_and_counts_its_cases() {
        let verdict = check_from(7, 100, |rng| {
            let value = rng.integer_between(-10, 10);
            if (-10..=10).contains(&value) {
                Ok(())
            } else {
                Err(format!("{value} is outside the range it was drawn from"))
            }
        });
        assert_eq!(verdict, Verdict::Held { cases: 100 });
    }

    /// Two bases explore different cases. A `check_from` that ignored its base
    /// would make the fuzz tier's stated seed base a fiction.
    #[test]
    fn the_seed_base_decides_what_is_examined() {
        let seen = |base: u64| -> Vec<u64> {
            let drawn = std::cell::RefCell::new(Vec::new());
            let verdict = check_from(base, 8, |rng| {
                drawn.borrow_mut().push(rng.next_u64());
                Ok(())
            });
            assert!(verdict.held());
            drawn.into_inner()
        };
        assert_ne!(seen(1), seen(2));
    }
}
