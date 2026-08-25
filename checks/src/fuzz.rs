//! Mutating known-good inputs, for the tier that examines untrusted bytes.
//!
//! D10 asks for *fuzz tests wherever untrusted bytes enter* (§3.7). At M0 that
//! is three places, and they are all parsers: the record's own codec, the
//! journal replay that has to read a file a crash was writing, and the zone
//! file the platform publishes. None of them is a place MCF chose the bytes.
//!
//! **Mutation from a corpus rather than random bytes.** A uniform random
//! generator reaches a parser's error path on its first byte and stays there:
//! it never produces a nearly-valid journal, which is the input that finds the
//! interesting defect. So the tier starts from inputs MCF itself wrote and
//! damages them — the same shape a crash, a truncated download or a partially
//! written file produce.
//!
//! **A find is reproducible from the report.** The seed base is stated in the
//! output, the case is derived from it, and [`render`] prints the offending
//! bytes so the input can be pasted into a test. B-143's discipline is that an
//! escaped defect becomes a permanent fixture; this is the half of it the fuzz
//! tier owes.
//!
//! **What the tier asserts is deliberately weak, and that is the point.** A
//! parser is allowed to refuse anything. What it is not allowed to do is panic,
//! hang, or accept something it then describes incorrectly — so the invariants
//! below are *no panic*, *the refusal is classified*, and *what it says about
//! what it read is true*. Anything stronger would be a claim about which
//! damaged inputs are valid, and nobody has made one.

use crate::property::Rng;

/// One damaged copy of an input.
///
/// The mutations are the ones a real medium produces — a byte flipped, a run
/// truncated, a chunk repeated or deleted — plus the two a parser is most often
/// caught by: an inserted delimiter and a very long run of one byte.
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
            // A bit flipped in place: the medium's own failure.
            0 => {
                if let Some(byte) = bytes.get_mut(at) {
                    *byte ^= 1 << (rng.index(8));
                }
            }
            // Truncated: what a crash mid-write leaves.
            1 => bytes.truncate(at),
            // A byte replaced by one of the delimiters the formats care about.
            2 => {
                let delimiters = [b'"', b'\\', b'{', b'}', b'[', b']', b'\n', b0(), b':', b','];
                if let (Some(slot), Some(delimiter)) =
                    (bytes.get_mut(at), rng.pick(&delimiters).copied())
                {
                    *slot = delimiter;
                }
            }
            // A chunk deleted.
            3 => {
                let end = (at + rng.index(16) + 1).min(bytes.len());
                let _removed: Vec<u8> = bytes.splice(at..end, std::iter::empty()).collect();
            }
            // A chunk repeated, which is how a resumed transfer goes wrong.
            4 => {
                let end = (at + rng.index(16) + 1).min(bytes.len());
                let chunk: Vec<u8> = bytes.get(at..end).unwrap_or_default().to_vec();
                let _inserted: Vec<u8> = bytes.splice(at..at, chunk).collect();
            }
            // A long run of one byte: the input that finds a quadratic loop or
            // an unbounded allocation.
            5 => {
                let run = std::iter::repeat_n(rng.byte(), rng.index(2_000) + 1);
                let _inserted: Vec<u8> = bytes.splice(at..at, run).collect();
            }
            // Arbitrary bytes spliced in.
            _ => {
                let noise = rng.bytes(24);
                let _inserted: Vec<u8> = bytes.splice(at..at, noise).collect();
            }
        }
    }
    bytes
}

/// A nul byte, named rather than written as an escape inside an array of
/// characters that are otherwise all punctuation.
const fn b0() -> u8 {
    0
}

/// Bytes as a report can print them: printable ASCII as itself, everything else
/// as an escape, and a length so a truncation is visible.
#[must_use]
pub fn render(bytes: &[u8]) -> String {
    use std::fmt::Write as _;

    const LIMIT: usize = 400;
    let mut out = format!("{} bytes: ", bytes.len());
    for byte in bytes.iter().take(LIMIT) {
        if byte.is_ascii_graphic() || *byte == b' ' {
            out.push(char::from(*byte));
        } else {
            // The write cannot fail: the target is a String.
            let _written = write!(out, "\\x{byte:02x}");
        }
    }
    if bytes.len() > LIMIT {
        out.push_str(" …(truncated in this report)");
    }
    out
}

/// How many cases each fuzz target examines by default.
///
/// The tier is scheduled rather than gating, so the number is one that examines
/// a great deal rather than one that fits in a gate. Two hundred thousand takes
/// about four seconds here, measured; the two targets that write files divide
/// it, because a case that touches the disk costs three orders of magnitude
/// more than one that only parses. `MCF_FUZZ_CASES` moves it for a longer
/// campaign.
pub const DEFAULT_CASES: u32 = 200_000;

/// The seed base the tier explores from unless told otherwise.
///
/// Fixed by default, so that a scheduled run is reproducible and two machines
/// examine the same inputs. `MCF_FUZZ_SEED` moves it, which is how a longer
/// campaign explores somewhere new — and the tier prints whichever base it
/// used, because a find nobody can reproduce is a find nobody can fix.
pub const DEFAULT_SEED: u64 = 0x465A_5A00_0000_0001;

/// The number of cases this run should examine.
#[must_use]
pub fn cases() -> u32 {
    read_env("MCF_FUZZ_CASES").map_or(DEFAULT_CASES, |value| {
        u32::try_from(value).unwrap_or(DEFAULT_CASES)
    })
}

/// The seed base this run should explore from.
#[must_use]
pub fn seed() -> u64 {
    read_env("MCF_FUZZ_SEED").unwrap_or(DEFAULT_SEED)
}

/// The number of cases a target that writes files should examine.
///
/// A fortieth of [`cases`], and never fewer than one. A case that opens a
/// journal, appends to it with a durability barrier per entry and reads it back
/// costs three orders of magnitude more than one that parses a line, and a tier
/// that spent its whole budget on four hundred disk cases would examine the
/// codec far less than it looks like it does.
#[must_use]
pub fn filesystem_cases() -> u32 {
    // `checked_div` rather than `/`: the workspace denies integer division
    // because a silently truncated quotient is a wrong number (A6's habit),
    // and the divisor being a non-zero constant is not something the lint can
    // see.
    cases().checked_div(FILESYSTEM_DIVISOR).unwrap_or(1).max(1)
}

/// How much cheaper a parse-only case is than one that touches the disk.
const FILESYSTEM_DIVISOR: u32 = 40;

fn read_env(name: &str) -> Option<u64> {
    std::env::var(name).ok()?.trim().parse::<u64>().ok()
}

#[cfg(test)]
mod tests {
    use super::{mutate, render};
    use crate::property::{Rng, Verdict, check};

    /// The mutator damages what it is given. A mutator that returned its input
    /// would make every fuzz target a very slow way of parsing valid data, and
    /// the campaign would report the same green either way — the vacuous
    /// success `scripts/check-lints-bite.sh` states the lesson for.
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

    /// An empty input is not a case the mutator can damage, so it produces
    /// bytes rather than returning nothing — otherwise a target whose corpus
    /// was empty would examine two hundred thousand empty inputs and pass.
    #[test]
    fn an_empty_input_still_produces_bytes() {
        let mut rng = Rng::seeded(2);
        let produced: usize = (0..50).map(|_| mutate(&mut rng, &[]).len()).sum();
        assert!(produced > 0, "fifty mutations of nothing produced nothing");
    }

    /// The control: a property that is false of damaged input is falsified, so
    /// the tier's green means the invariants held rather than that nothing was
    /// examined.
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

    /// A report is legible: printable bytes as themselves, the rest escaped,
    /// and the length stated so a truncation is visible.
    #[test]
    fn a_finding_renders_legibly() {
        let rendered = render(b"ab\x00\xff");
        assert!(rendered.starts_with("4 bytes: ab"), "{rendered}");
        assert!(rendered.contains("\\x00"), "{rendered}");
        assert!(rendered.contains("\\xff"), "{rendered}");
    }
}
