use std::panic::{AssertUnwindSafe, catch_unwind};

pub const BASE_SEED: u64 = 0x4D43_4630_0000_0001;

pub const GATING_CASES: u32 = 256;

pub const FILESYSTEM_CASES: u32 = 32;

#[derive(Debug, Clone)]
pub struct Rng {
    state: u64,
}

impl Rng {
    #[must_use]
    pub const fn seeded(seed: u64) -> Self {
        Self { state: seed }
    }

    pub fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    pub fn below(&mut self, bound: u64) -> u64 {
        if bound == 0 {
            0
        } else {
            self.next_u64() % bound
        }
    }

    pub fn index(&mut self, bound: usize) -> usize {
        usize::try_from(self.below(bound as u64)).unwrap_or(0)
    }

    pub fn boolean(&mut self) -> bool {
        self.next_u64() & 1 == 1
    }

    pub fn byte(&mut self) -> u8 {
        u8::try_from(self.next_u64() & 0xFF).unwrap_or(0)
    }

    pub fn integer_between(&mut self, low: i64, high: i64) -> i64 {
        if high <= low {
            return low;
        }
        let span = u64::try_from(i128::from(high) - i128::from(low)).unwrap_or(u64::MAX);
        let offset = self.below(span.saturating_add(1));
        i64::try_from(i128::from(low) + i128::from(offset)).unwrap_or(low)
    }

    pub fn bytes(&mut self, max: usize) -> Vec<u8> {
        let length = self.index(max.saturating_add(1));
        (0..length).map(|_| self.byte()).collect()
    }

    pub fn text(&mut self, max: usize) -> String {
        let length = self.index(max.saturating_add(1));
        (0..length).map(|_| self.character()).collect()
    }

    pub fn character(&mut self) -> char {
        match self.below(8) {
            0 => '"',
            1 => '\\',
            2 => char::from_u32(self.below(0x20).try_into().unwrap_or(0)).unwrap_or('\0'),
            3 => char::from(self.byte() | 0x20).to_ascii_lowercase(),
            4 => {
                char::from_u32(0x80 + u32::try_from(self.below(0x700)).unwrap_or(0)).unwrap_or('é')
            }
            5 => char::from_u32(0x800 + u32::try_from(self.below(0xF000)).unwrap_or(0))
                .unwrap_or('☃'),
            6 => char::from_u32(0x1_0000 + u32::try_from(self.below(0xF_0000)).unwrap_or(0))
                .unwrap_or('🙂'),
            _ => char::from(b'a' + u8::try_from(self.below(26)).unwrap_or(0)),
        }
    }

    pub fn pick<'a, T>(&mut self, choices: &'a [T]) -> Option<&'a T> {
        choices.get(self.index(choices.len()))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Verdict {
    Held {
        cases: u32,
    },
    Falsified {
        seed: u64,
        case: u32,
        detail: String,
    },
}

impl Verdict {
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

pub fn check<P>(cases: u32, property: P) -> Verdict
where
    P: Fn(&mut Rng) -> Result<(), String>,
{
    check_from(BASE_SEED, cases, property)
}

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
                let mut replayed = Rng::seeded(seed);
                let drawn = replayed.below(1_000);
                assert_eq!(detail, format!("drew {drawn}"));
            }
        }
    }

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
