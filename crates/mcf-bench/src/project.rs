//! What this machine would probably do, from what it has already done
//! (B-214, PR3, A20, B34, B46).
//!
//! **PR3's question, before a byte is fetched:** *how fast would this be here?*
//! MCF cannot measure a model it does not have, and refusing to say anything
//! makes choosing between twenty published quantizations cost tens of gigabytes
//! a guess. A20 admits the middle answer and then draws the line absolutely:
//! *an estimate can never be mistaken for a measurement, never be promoted into
//! one, and never be compared with one. It can only be replaced by one.* That
//! wall is [`Estimate`]'s and is a compiler check; what this module owes is the
//! other three conditions.
//!
//! **Band-shaped** (B46). A duration predicted from a rate is a range, and
//! rendering it as one number is *the smallest possible version of a confident
//! wrong number*. The band here is not invented: it is the slowest and fastest
//! trials actually seen at the bracketing sizes, carried through the same
//! interpolation.
//!
//! **From local history only** (B34). The corpus advises and never decides,
//! and there is no corpus yet; every point here is a comparison this machine
//! took and wrote down.
//!
//! **Absent where there is no history** — and, more strictly than the register
//! asks, absent where there is no history *around* the thing being projected.
//! Interpolating between two measured sizes is reading between points MCF has;
//! projecting past the largest or below the smallest is extrapolation, and a
//! straight line beyond the data is exactly the confident wrong number B46
//! names. F67 measured the relationship this rests on — latency monotone in
//! file size, because every trial loads the model — and F67 also measured where
//! it stops being straight: a line through the extremes predicted the largest
//! point eleven percent low.
//!
//! **What it is projecting.** Not throughput in the abstract: *the duration of
//! one request of a stated token budget, on this machine, through whatever
//! engine the history was taken through.* A projection for a budget this
//! machine has no history at is absent, because two requests of different
//! lengths are two different things and averaging over them would be inventing
//! a rate nobody measured.
//!
//! [`Estimate`]: mcf_core::measurement::Estimate

use mcf_core::measurement::{Basis, Estimate, PartsPerMillion};
use mcf_core::time::{Duration, Monotonic};

/// How busy the machine was while a point was measured.
///
/// Thousandths of a processor, the larger of the readings taken either side of
/// the run — the larger because a band planned from history should inherit the
/// worse of the two conditions rather than the flattering one.
///
/// `None` where the entry recorded nothing about the machine, which is not
/// *zero* and must never render as it: A7 keeps unknown unknown, and the
/// entries written before `B-217` existed are exactly that case.
pub type Competing = Option<u64>;

/// One thing this machine has measured.
///
/// The bounds are the fastest and slowest trials of that arm, not a summary of
/// them: B56 keeps the trials and derives nothing that discards them, and a
/// band built from a mean would be a band around a number MCF does not compute.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Point {
    /// How large the model file is.
    pub bytes: u64,
    /// How many tokens the request was pinned to.
    pub tokens: u32,
    /// The fastest trial of that arm.
    pub fastest: u64,
    /// The slowest.
    pub slowest: u64,
    /// What else the machine was doing while it was measured (B-385, §3.4).
    ///
    /// **Why a point carries this.** F74: an unrelated test suite held
    /// twenty-six cores of this machine, and a generation that takes 400 ms
    /// quiet took eighteen seconds. Those measurements are true and stay in
    /// the record (A1), and a band read between them is not wrong — but a band
    /// that does not say what it rested on has dropped the conditions, which
    /// is what §3.4 and A6 exist to prevent.
    ///
    /// Carried rather than filtered: filtering needs a threshold, and the
    /// threshold is DEC-007's to set.
    pub competing: Competing,
}

/// Why there is no projection.
///
/// Each is a state to report rather than a number to invent (A7). *MCF has
/// never measured anything like this here* is a useful answer; a band with
/// nothing under it is not.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NoBand {
    /// Nothing has been measured at this token budget.
    NoHistoryAtThatBudget {
        /// The budget asked about.
        tokens: u32,
        /// The budgets this machine does have history at, sorted.
        instead: Vec<u32>,
    },
    /// There is history, and it is all on one side of the thing asked about.
    ///
    /// Projecting past it is extrapolation, and a straight line beyond the data
    /// is the confident wrong number B46 names.
    OutsideWhatWasMeasured {
        /// The smallest file measured at this budget.
        smallest: u64,
        /// The largest.
        largest: u64,
    },
    /// Fewer than two points, which cannot bracket anything.
    TooLittleHistory {
        /// How many points there are at this budget.
        points: usize,
    },
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

/// A band, and the conditions of the two measurements it was read between.
///
/// The two travel together because separating them is the defect B-385 names:
/// a caller holding only the band has no way to say what it rested on, and
/// every surface that renders it drops the conditions silently.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Projection {
    band: Estimate<Duration<Monotonic>>,
    rested_on: Rested,
}

impl Projection {
    /// The band.
    #[must_use]
    pub const fn band(&self) -> &Estimate<Duration<Monotonic>> {
        &self.band
    }

    /// What it was read between.
    #[must_use]
    pub const fn rested_on(&self) -> &Rested {
        &self.rested_on
    }
}

/// The conditions of the two measurements a band was read between (B-385).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rested {
    /// How busy the machine was for each of the two, in thousandths of a
    /// processor, in size order.
    pub competing: [Competing; 2],
}

impl Rested {
    /// The busier of the two, where either is known.
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

/// What a request of `tokens` on a file of `bytes` would probably take here.
///
/// # Errors
///
/// Every reason there is no band, by name.
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

    // The two measured points this file sits between. Reading between points
    // MCF has, rather than past them.
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
            // B34: the corpus advises and never decides, and there is no
            // corpus. Every point behind this is a comparison this machine
            // took.
            Basis::LocalHistory,
        ),
    })
}

/// How well the projection has done against what it was later measured to be
/// (B-215, §6.16, §3.4).
///
/// **§6.16 turned on the projection.** *The instrument does not get to grade
/// itself*, and a projection that nobody scores is a claim MCF makes for ever
/// without ever finding out whether it was any good.
///
/// **Scored by leaving each point out.** For every measurement in the history,
/// the band that *would have been* projected for it from the others is
/// computed and compared with what it actually was. That needs no stored
/// predictions and no new record: it is recomputed from the history each time
/// it is asked, so it tracks as the history grows — which is what B-215 means
/// by *over time*. A stored score would be a score about a record that has
/// since changed.
///
/// **The points at the ends are not scored**, and that is not a gap: with them
/// left out there is nothing to read between, so there is no projection to
/// score. Counting them as misses would be scoring the refusal to extrapolate,
/// which is the thing the model gets right.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Scored {
    /// How many points the band would have contained.
    pub inside: usize,
    /// How many it would have missed.
    pub outside: usize,
    /// How many could not be scored, having nothing to be read between.
    pub unscorable: usize,
    /// The worst miss, in parts per million against the nearer edge of the
    /// band it missed.
    pub worst: PartsPerMillion,
}

impl Default for Scored {
    /// Nothing scored yet — not *scored and perfect*, which is what a zero
    /// worst-miss with a zero count would read as. `scored()` is what tells
    /// them apart, and `Display` checks it first.
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
    /// How many points were scored at all.
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

/// Scores the projection against every measurement this machine has.
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
                // The measured value is the point itself, and *inside* means
                // the bands overlap: a point whose own fastest-to-slowest range
                // meets the projected one was not missed.
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

/// The value at `at`, read between two measured points.
///
/// Integer arithmetic in `u128`, because this crate holds no floating-point
/// number and an interpolation is not a reason to introduce one (A6). Where the
/// two points share a size there is nothing to read between and the nearer
/// value is the answer.
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
