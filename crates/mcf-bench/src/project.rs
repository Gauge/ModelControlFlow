use mcf_core::measurement::{Basis, Estimate, PartsPerMillion};
use mcf_core::time::{Duration, Monotonic};

pub type Competing = Option<u64>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Point {
    pub bytes: u64,
    pub tokens: u32,
    pub fastest: u64,
    pub slowest: u64,
    pub competing: Competing,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NoBand {
    NoHistoryAtThatBudget { tokens: u32, instead: Vec<u32> },
    OutsideWhatWasMeasured { smallest: u64, largest: u64 },
    TooLittleHistory { points: usize },
}

impl core::fmt::Display for NoBand {
    fn fmt(&self, form: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::NoHistoryAtThatBudget { tokens, instead } => {
                if instead.is_empty() {
                    write!(
                        form,
                        "nothing has been measured on this machine, so there is nothing to \
                         project from (B34)"
                    )
                } else {
                    write!(
                        form,
                        "nothing has been measured at {tokens} tokens here; this machine has \
                         history at {} — two requests of different lengths are two different \
                         things",
                        instead
                            .iter()
                            .map(ToString::to_string)
                            .collect::<Vec<String>>()
                            .join(", ")
                    )
                }
            }
            Self::OutsideWhatWasMeasured { smallest, largest } => write!(
                form,
                "this file is outside what has been measured here ({smallest} to {largest} \
                 bytes), and a straight line beyond the data is a confident wrong number (B46)"
            ),
            Self::TooLittleHistory { points } => write!(
                form,
                "{points} measured point(s) at this budget: two are needed to read between"
            ),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Projection {
    band: Estimate<Duration<Monotonic>>,
    rested_on: Rested,
}

impl Projection {
    #[must_use]
    pub const fn band(&self) -> &Estimate<Duration<Monotonic>> {
        &self.band
    }

    #[must_use]
    pub const fn rested_on(&self) -> &Rested {
        &self.rested_on
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rested {
    pub competing: [Competing; 2],
}

impl Rested {
    #[must_use]
    pub fn busiest(&self) -> Competing {
        self.competing.iter().copied().flatten().max()
    }
}

impl core::fmt::Display for Rested {
    fn fmt(&self, form: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let cores = |held: u64| {
            format!(
                "{}.{:02} core(s)",
                held.wrapping_div(1_000),
                held.wrapping_rem(1_000).wrapping_div(10)
            )
        };
        match (self.competing[0], self.competing[1]) {
            (None, None) => form.write_str(
                "neither of the two runs it was read between recorded what the machine was \
                 doing, which is unknown and not quiet (A7)",
            ),
            (Some(one), Some(other)) => write!(
                form,
                "read between two runs with {} and {} competing",
                cores(one),
                cores(other)
            ),
            (Some(one), None) | (None, Some(one)) => write!(
                form,
                "read between one run with {} competing and one that recorded nothing about \
                 the machine",
                cores(one)
            ),
        }
    }
}

pub fn band(history: &[Point], bytes: u64, tokens: u32) -> Result<Projection, NoBand> {
    let mut at_budget: Vec<&Point> = history
        .iter()
        .filter(|point| point.tokens == tokens)
        .collect();
    if at_budget.is_empty() {
        let mut instead: Vec<u32> = history.iter().map(|point| point.tokens).collect();
        instead.sort_unstable();
        instead.dedup();
        return Err(NoBand::NoHistoryAtThatBudget { tokens, instead });
    }
    if at_budget.len() < 2 {
        return Err(NoBand::TooLittleHistory {
            points: at_budget.len(),
        });
    }
    at_budget.sort_by_key(|point| point.bytes);

    let (Some(smallest), Some(largest)) = (at_budget.first(), at_budget.last()) else {
        return Err(NoBand::TooLittleHistory { points: 0 });
    };
    if bytes < smallest.bytes || bytes > largest.bytes {
        return Err(NoBand::OutsideWhatWasMeasured {
            smallest: smallest.bytes,
            largest: largest.bytes,
        });
    }

    let mut below = **smallest;
    let mut above = **largest;
    for point in &at_budget {
        if point.bytes <= bytes && point.bytes >= below.bytes {
            below = **point;
        }
        if point.bytes >= bytes && point.bytes <= above.bytes {
            above = **point;
        }
    }

    Ok(Projection {
        rested_on: Rested {
            competing: [below.competing, above.competing],
        },
        band: Estimate::band(
            Duration::from_nanos(between(
                below.bytes,
                below.fastest,
                above.bytes,
                above.fastest,
                bytes,
            )),
            Duration::from_nanos(between(
                below.bytes,
                below.slowest,
                above.bytes,
                above.slowest,
                bytes,
            )),
            Basis::LocalHistory,
        ),
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Scored {
    pub inside: usize,
    pub outside: usize,
    pub unscorable: usize,
    pub worst: PartsPerMillion,
}

impl Default for Scored {
    fn default() -> Self {
        Self {
            inside: 0,
            outside: 0,
            unscorable: 0,
            worst: PartsPerMillion(0),
        }
    }
}

impl Scored {
    #[must_use]
    pub const fn scored(&self) -> usize {
        self.inside.saturating_add(self.outside)
    }
}

impl core::fmt::Display for Scored {
    fn fmt(&self, form: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        if self.scored() == 0 {
            return write!(
                form,
                "not scored: {} point(s), none of them with measurements on both sides to be \
                 read between",
                self.unscorable
            );
        }
        write!(
            form,
            "{} of {} measured points fall inside the band that would have been projected for \
             them; the worst miss is {}.{}%",
            self.inside,
            self.scored(),
            self.worst.0.wrapping_div(10_000),
            self.worst.0.wrapping_div(1_000).wrapping_rem(10)
        )?;
        if self.unscorable > 0 {
            write!(
                form,
                " ({} could not be scored: nothing measured on one side of them)",
                self.unscorable
            )?;
        }
        Ok(())
    }
}

#[must_use]
pub fn score(history: &[Point]) -> Scored {
    let mut held = Scored::default();
    for (at, point) in history.iter().enumerate() {
        let others: Vec<Point> = history
            .iter()
            .enumerate()
            .filter(|(other, _)| *other != at)
            .map(|(_, one)| *one)
            .collect();
        match band(&others, point.bytes, point.tokens) {
            Err(_) => held.unscorable = held.unscorable.saturating_add(1),
            Ok(projection) => {
                let said = projection.band();
                let (low, high) = (said.low().as_nanos(), said.high().as_nanos());
                if point.slowest >= low && point.fastest <= high {
                    held.inside = held.inside.saturating_add(1);
                } else {
                    held.outside = held.outside.saturating_add(1);
                    let missed = if point.fastest > high {
                        point.fastest.saturating_sub(high)
                    } else {
                        low.saturating_sub(point.slowest)
                    };
                    let against = if point.fastest > high { high } else { low }.max(1);
                    let by = u128::from(missed)
                        .saturating_mul(1_000_000)
                        .wrapping_div(u128::from(against));
                    let by = PartsPerMillion(u64::try_from(by).unwrap_or(u64::MAX));
                    if by > held.worst {
                        held.worst = by;
                    }
                }
            }
        }
    }
    held
}

fn between(x0: u64, y0: u64, x1: u64, y1: u64, at: u64) -> u64 {
    if x1 <= x0 {
        return y0;
    }
    let span = u128::from(x1.saturating_sub(x0));
    let along = u128::from(at.saturating_sub(x0));
    let rise = i128::from(y1) - i128::from(y0);
    let moved = rise
        .saturating_mul(i128::try_from(along).unwrap_or(0))
        .checked_div(i128::try_from(span).unwrap_or(1))
        .unwrap_or(0);
    u64::try_from(i128::from(y0).saturating_add(moved)).unwrap_or(y0)
}

#[cfg(test)]
mod tests;
