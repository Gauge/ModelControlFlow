use crate::attested::Attested;

use super::{Timestamp, UtcOffset};

const ZONE_FILE: &str = "/etc/localtime";

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Zone {
    transitions: Vec<i64>,
    offsets: Vec<i32>,
    initial: i32,
}

impl Zone {
    #[must_use]
    pub fn parse(bytes: &[u8]) -> Option<Self> {
        let first = Block::parse(bytes, 4)?;
        if first.version >= b'2' {
            let rest = bytes.get(first.length..)?;
            let second = Block::parse(rest, 8)?;
            return second.into_zone();
        }
        first.into_zone()
    }

    #[must_use]
    pub fn offset_at(&self, moment: Timestamp) -> Option<UtcOffset> {
        const NANOS_PER_SECOND: i128 = 1_000_000_000;
        let seconds = i64::try_from(moment.utc_nanos().div_euclid(NANOS_PER_SECOND)).ok()?;

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

    #[must_use]
    pub const fn safe_horizon() -> i64 {
        SAFE_HORIZON
    }
}

const SAFE_HORIZON: i64 = 0;

#[derive(Debug)]
struct Block {
    version: u8,
    length: usize,
    transitions: Vec<i64>,
    indices: Vec<u8>,
    offsets: Vec<i32>,
}

impl Block {
    fn parse(bytes: &[u8], time_size: usize) -> Option<Self> {
        if bytes.get(..4)? != b"TZif" {
            return None;
        }
        let version = *bytes.get(4)?;
        if !matches!(version, 0 | b'1' | b'2' | b'3' | b'4') {
            return None;
        }

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
            at += 6;
        }

        at += charcnt;
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
