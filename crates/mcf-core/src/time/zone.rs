//! Reading the machine's offset from UTC.
//!
//! D9: *records are timestamped in UTC, stored with the local offset alongside
//! rather than baked in, so a record is both comparable across machines and
//! legible about where it was taken.* B-352 is the offset.
//!
//! **The offset is never folded into the moment**, which is what
//! [`Timestamp`](super::Timestamp) already enforces. This module supplies the
//! number that travels beside it, and nothing here can change the moment.
//!
//! **Why the zone file is parsed rather than a platform call made.** The
//! obvious route is the C library's `localtime_r`, whose `tm_gmtoff` is not
//! POSIX but a widely-implemented extension — which means declaring another
//! platform's `struct tm` layout by hand, in `unsafe`, for a field that is not
//! standardized. The zone file is a published, stable, fixed format that MCF can
//! read in safe Rust and test against a value the machine itself can be asked
//! for. B15 admits weight against a stated cost, and this is the cheaper side.
//!
//! **What it claims.** RFC 8536's `TZif`, versions 1 through 4, far enough to
//! answer *what is the offset now* — which is the only question a timestamp
//! asks. It does not evaluate the POSIX rule in a version 2+ footer, so a
//! moment beyond the file's last recorded transition is [`Attested::Unknown`]
//! rather than extrapolated (A7). Zone files carry transitions decades ahead,
//! so that is a boundary rather than a common case, and it is stated rather
//! than guessed past.

use crate::attested::Attested;

use super::{Timestamp, UtcOffset};

/// Where the platform publishes the machine's zone.
const ZONE_FILE: &str = "/etc/localtime";

/// Reads the offset in force at a moment.
///
/// `Unknown` where the platform publishes no zone file, where the file cannot
/// be read or understood, or where the moment lies beyond the transitions the
/// file records. Each of those is a different reason and none of them is a
/// reason to return zero: `+00:00` is a real offset that most machines do not
/// have, and A7 forbids the plausible substitute.
#[must_use]
pub fn offset_at(moment: Timestamp) -> Attested<UtcOffset> {
    let Ok(bytes) = std::fs::read(ZONE_FILE) else {
        return Attested::Unknown;
    };
    match Zone::parse(&bytes).and_then(|zone| zone.offset_at(moment)) {
        Some(offset) => Attested::Known(offset),
        None => Attested::Unknown,
    }
}

/// The transitions a zone file records.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Zone {
    /// When each transition happens, in seconds since the epoch, ascending.
    transitions: Vec<i64>,
    /// The offset in force from each transition, in seconds east.
    offsets: Vec<i32>,
    /// The offset in force before the first transition.
    initial: i32,
}

impl Zone {
    /// Reads a zone file.
    ///
    /// # Errors
    ///
    /// `None` for anything this reader does not claim: a file that is not
    /// `TZif`, a version it does not know, or a body that ends early. A7's habit
    /// applied to a parser — what it cannot read, it does not approximate.
    #[must_use]
    pub fn parse(bytes: &[u8]) -> Option<Self> {
        let first = Block::parse(bytes, 4)?;
        // Version 2 and later repeat the header and follow it with a block
        // using 8-byte transition times, which is the one to use: the 4-byte
        // block cannot represent a moment past 2038.
        if first.version >= b'2' {
            let rest = bytes.get(first.length..)?;
            let second = Block::parse(rest, 8)?;
            return second.into_zone();
        }
        first.into_zone()
    }

    /// The offset in force at a moment, if the file records one.
    #[must_use]
    pub fn offset_at(&self, moment: Timestamp) -> Option<UtcOffset> {
        const NANOS_PER_SECOND: i128 = 1_000_000_000;
        let seconds = i64::try_from(moment.utc_nanos().div_euclid(NANOS_PER_SECOND)).ok()?;

        // Beyond the last transition the file says nothing, and the POSIX rule
        // in the footer is not evaluated (see the module note). Unknown rather
        // than extrapolated.
        let last = self.transitions.last().copied();
        if last.is_some_and(|last| seconds > last.saturating_add(SAFE_HORIZON)) {
            return None;
        }

        let at = match self.transitions.partition_point(|when| *when <= seconds) {
            0 => self.initial,
            index => *self.offsets.get(index - 1)?,
        };
        UtcOffset::from_seconds_east(at)
    }

    /// How far past its last recorded transition a zone file is still believed.
    ///
    /// Zero: the file's last transition is the last thing it says, and a moment
    /// after it is a moment the file does not describe. The constant exists so
    /// that the boundary is named rather than implied by an off-by-one.
    #[must_use]
    pub const fn safe_horizon() -> i64 {
        SAFE_HORIZON
    }
}

const SAFE_HORIZON: i64 = 0;

/// One data block of a zone file, and the header that describes it.
#[derive(Debug)]
struct Block {
    version: u8,
    length: usize,
    transitions: Vec<i64>,
    indices: Vec<u8>,
    offsets: Vec<i32>,
}

impl Block {
    /// Reads a header and its block. `time_size` is 4 for a version 1 block and
    /// 8 for the block that follows a version 2 or later header.
    fn parse(bytes: &[u8], time_size: usize) -> Option<Self> {
        if bytes.get(..4)? != b"TZif" {
            return None;
        }
        let version = *bytes.get(4)?;
        if !matches!(version, 0 | b'1' | b'2' | b'3' | b'4') {
            return None;
        }

        // Six counts, big-endian, after fifteen reserved bytes.
        // A negative count is a malformed file, not a large one: `try_from`
        // refuses it rather than wrapping it into an enormous length (A7's
        // habit — what cannot be read is not approximated).
        let count = |index: usize| -> Option<usize> {
            be32(bytes, 20 + index * 4).and_then(|n| usize::try_from(n).ok())
        };
        let isutcnt = count(0)?;
        let isstdcnt = count(1)?;
        let leapcnt = count(2)?;
        let timecnt = count(3)?;
        let typecnt = count(4)?;
        let charcnt = count(5)?;
        if typecnt == 0 {
            return None;
        }

        let mut at = 44;
        let mut transitions = Vec::with_capacity(timecnt);
        for _ in 0..timecnt {
            transitions.push(if time_size == 8 {
                be64(bytes, at)?
            } else {
                i64::from(be32(bytes, at)?)
            });
            at += time_size;
        }

        let indices = bytes.get(at..at + timecnt)?.to_vec();
        at += timecnt;

        let mut offsets = Vec::with_capacity(typecnt);
        for _ in 0..typecnt {
            offsets.push(be32(bytes, at)?);
            // Each record is a 4-byte offset, an is-DST byte and a
            // designation index.
            at += 6;
        }

        at += charcnt;
        // A leap-second record is a time and a correction.
        at += leapcnt * (time_size + 4);
        at += isstdcnt;
        at += isutcnt;
        if at > bytes.len() {
            return None;
        }

        Some(Self {
            version,
            length: at,
            transitions,
            indices,
            offsets,
        })
    }

    fn into_zone(self) -> Option<Zone> {
        // The offset before the first transition. RFC 8536 says to use the
        // first record that is not daylight saving; this reader uses the first,
        // which agrees for every zone that has transitions and is the only
        // available answer for one that does not.
        let initial = *self.offsets.first()?;
        let offsets = self
            .indices
            .iter()
            .map(|index| self.offsets.get(usize::from(*index)).copied())
            .collect::<Option<Vec<i32>>>()?;
        Some(Zone {
            transitions: self.transitions,
            offsets,
            initial,
        })
    }
}

fn be32(bytes: &[u8], at: usize) -> Option<i32> {
    let slice = bytes.get(at..at + 4)?;
    Some(i32::from_be_bytes([
        *slice.first()?,
        *slice.get(1)?,
        *slice.get(2)?,
        *slice.get(3)?,
    ]))
}

fn be64(bytes: &[u8], at: usize) -> Option<i64> {
    let slice = bytes.get(at..at + 8)?;
    let mut value = [0_u8; 8];
    value.copy_from_slice(slice);
    Some(i64::from_be_bytes(value))
}

#[cfg(test)]
mod tests;
