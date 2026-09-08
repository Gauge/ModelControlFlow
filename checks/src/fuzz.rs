use crate::property::Rng;

#[must_use]
pub fn mutate(rng: &mut Rng, input: &[u8]) -> Vec<u8> {
    let mut bytes = input.to_vec();
    if bytes.is_empty() {
        return rng.bytes(64);
    }
    let rounds = rng.index(4) + 1;
    for _ in 0..rounds {
        if bytes.is_empty() {
            break;
        }
        let at = rng.index(bytes.len());
        match rng.below(7) {
            0 => {
                if let Some(byte) = bytes.get_mut(at) {
                    *byte ^= 1 << (rng.index(8));
                }
            }
            1 => bytes.truncate(at),
            2 => {
                let delimiters = [b'"', b'\\', b'{', b'}', b'[', b']', b'\n', b0(), b':', b','];
                if let (Some(slot), Some(delimiter)) =
                    (bytes.get_mut(at), rng.pick(&delimiters).copied())
                {
                    *slot = delimiter;
                }
            }
            3 => {
                let end = (at + rng.index(16) + 1).min(bytes.len());
                let _removed: Vec<u8> = bytes.splice(at..end, std::iter::empty()).collect();
            }
            4 => {
                let end = (at + rng.index(16) + 1).min(bytes.len());
                let chunk: Vec<u8> = bytes.get(at..end).unwrap_or_default().to_vec();
                let _inserted: Vec<u8> = bytes.splice(at..at, chunk).collect();
            }
            5 => {
                let run = std::iter::repeat_n(rng.byte(), rng.index(2_000) + 1);
                let _inserted: Vec<u8> = bytes.splice(at..at, run).collect();
            }
            _ => {
                let noise = rng.bytes(24);
                let _inserted: Vec<u8> = bytes.splice(at..at, noise).collect();
            }
        }
    }
    bytes
}

const fn b0() -> u8 {
    0
}

#[must_use]
pub fn render(bytes: &[u8]) -> String {
    use std::fmt::Write as _;

    const LIMIT: usize = 400;
    let mut out = format!("{} bytes: ", bytes.len());
    for byte in bytes.iter().take(LIMIT) {
        if byte.is_ascii_graphic() || *byte == b' ' {
            out.push(char::from(*byte));
        } else {
            let _written = write!(out, "\\x{byte:02x}");
        }
    }
    if bytes.len() > LIMIT {
        out.push_str(" …(truncated in this report)");
    }
    out
}

pub const DEFAULT_CASES: u32 = 200_000;

pub const DEFAULT_SEED: u64 = 0x465A_5A00_0000_0001;

#[must_use]
pub fn cases() -> u32 {
    read_env("MCF_FUZZ_CASES").map_or(DEFAULT_CASES, |value| {
        u32::try_from(value).unwrap_or(DEFAULT_CASES)
    })
}

#[must_use]
pub fn seed() -> u64 {
    read_env("MCF_FUZZ_SEED").unwrap_or(DEFAULT_SEED)
}

#[must_use]
pub fn filesystem_cases() -> u32 {
    cases().checked_div(FILESYSTEM_DIVISOR).unwrap_or(1).max(1)
}

const FILESYSTEM_DIVISOR: u32 = 40;

fn read_env(name: &str) -> Option<u64> {
    std::env::var(name).ok()?.trim().parse::<u64>().ok()
}

#[cfg(test)]
mod tests {
    use super::{mutate, render};
    use crate::property::{Rng, Verdict, check};

    #[test]
    fn a_mutation_changes_its_input_nearly_always() {
        let original = b"{\"kind\":\"self_cost\",\"body\":{\"n\":17}}";
        let mut rng = Rng::seeded(1);
        let changed = (0..200)
            .filter(|_| mutate(&mut rng, original) != original.to_vec())
            .count();
        assert!(
            changed > 190,
            "only {changed} of 200 mutations changed the input"
        );
    }

    #[test]
    fn an_empty_input_still_produces_bytes() {
        let mut rng = Rng::seeded(2);
        let produced: usize = (0..50).map(|_| mutate(&mut rng, &[]).len()).sum();
        assert!(produced > 0, "fifty mutations of nothing produced nothing");
    }

    #[test]
    fn a_target_that_should_fail_does() {
        let verdict = check(500, |rng| {
            let damaged = mutate(rng, b"a stable input");
            if damaged == b"a stable input" {
                Ok(())
            } else {
                Err(format!("the input was damaged: {}", render(&damaged)))
            }
        });
        assert!(matches!(verdict, Verdict::Falsified { .. }), "{verdict}");
    }

    #[test]
    fn a_finding_renders_legibly() {
        let rendered = render(b"ab\x00\xff");
        assert!(rendered.starts_with("4 bytes: ab"), "{rendered}");
        assert!(rendered.contains("\\x00"), "{rendered}");
        assert!(rendered.contains("\\xff"), "{rendered}");
    }
}
