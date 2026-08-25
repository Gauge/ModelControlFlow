//! A moment on the calendar, in UTC, with the local offset alongside.
//!
//! D9: records are timestamped in UTC, stored with the local offset alongside
//! rather than baked in, so a record is both comparable across machines and
//! legible about where it was taken.
//!
//! **A timestamp has no subtraction.** That is B37's whole point: the wall
//! clock steps, drifts and is adjusted underneath a running process, so
//! `end - start` on two of these would report an NTP correction as latency.
//! Intervals come from [`Instant`], which has no calendar meaning.
//!
//! [`Instant`]: super::Instant

use core::fmt;

use crate::attested::Attested;

/// The local offset from UTC, in seconds east.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct UtcOffset(i32);

impl UtcOffset {
    /// UTC itself.
    pub const UTC: Self = Self(0);

    /// An offset, if it is one.
    ///
    /// Returns `None` beyond ±26 hours, which is wider than any real zone and
    /// narrow enough to catch a value that is not an offset at all.
    #[must_use]
    pub const fn from_seconds_east(seconds: i32) -> Option<Self> {
        const LIMIT: i32 = 26 * 3600;
        if seconds < -LIMIT || seconds > LIMIT {
            None
        } else {
            Some(Self(seconds))
        }
    }

    /// The offset in seconds east of UTC.
    #[must_use]
    pub const fn seconds_east(self) -> i32 {
        self.0
    }
}

impl fmt::Display for UtcOffset {
    /// `+00:00`, `-05:00`, as a record and a reader both expect.
    // Integer division is the operation, not an accident of one: an offset is
    // whole minutes by construction, and `from_seconds_east` has already bounded
    // the magnitude below 26 hours.
    #[allow(clippy::integer_division)]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let sign = if self.0 < 0 { '-' } else { '+' };
        let total = self.0.unsigned_abs();
        write!(f, "{sign}{:02}:{:02}", total / 3600, (total % 3600) / 60)
    }
}

/// A moment, in UTC, with the local offset if MCF knows it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Timestamp {
    /// Nanoseconds since 1970-01-01T00:00:00Z. Signed, so a machine whose
    /// clock is set before the epoch produces a timestamp rather than a
    /// wrapped one.
    utc_nanos: i128,
    offset: Attested<UtcOffset>,
}

impl Timestamp {
    /// A moment, stated.
    #[must_use]
    pub const fn from_utc_nanos(utc_nanos: i128, offset: Attested<UtcOffset>) -> Self {
        Self { utc_nanos, offset }
    }

    /// The moment the system clock reports now.
    ///
    /// The local offset is [`Attested::Unknown`]. Reading it requires a
    /// platform call the standard library does not offer, and B15 admits
    /// weight only against a stated cost — so until the condition-capture path
    /// (B-007) admits one, MCF records that it does not know rather than
    /// writing `+00:00` and being wrong for most of the world (A7). B-352
    /// registers the work.
    #[must_use]
    pub fn now() -> Self {
        let since_epoch = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH);
        // `i128` holds nanoseconds for about 5.4 × 10^21 years, so the
        // saturation below is unreachable on any clock a machine can hold. It
        // is written rather than assumed because the alternative is a
        // truncating cast, and a truncated timestamp is a confidently wrong
        // date rather than a missing one (P1).
        let utc_nanos = match since_epoch {
            Ok(elapsed) => i128::try_from(elapsed.as_nanos()).unwrap_or(i128::MAX),
            // The system clock is set before 1970. That is a real state of a
            // real machine, and the honest answer is a negative timestamp
            // rather than a clamp to the epoch.
            Err(before) => {
                i128::try_from(before.duration().as_nanos()).map_or(i128::MIN, |nanos| -nanos)
            }
        };
        Self {
            utc_nanos,
            offset: Attested::Unknown,
        }
    }

    /// Nanoseconds since the Unix epoch, UTC.
    #[must_use]
    pub const fn utc_nanos(self) -> i128 {
        self.utc_nanos
    }

    /// The local offset, if MCF read one.
    #[must_use]
    pub const fn offset(self) -> Attested<UtcOffset> {
        self.offset
    }

    /// The same moment, with a local offset attached.
    #[must_use]
    pub const fn with_offset(self, offset: UtcOffset) -> Self {
        Self {
            utc_nanos: self.utc_nanos,
            offset: Attested::Known(offset),
        }
    }

    /// The civil UTC date and time: year, month, day, hour, minute, second,
    /// nanosecond.
    ///
    /// Computed here rather than by a dependency. B15 admits weight only
    /// against a stated cost, and the cost of a date library is larger than
    /// the twenty lines below — which are Howard Hinnant's `civil_from_days`,
    /// exact for every representable day and tested against known dates (A19).
    // The casts below are bounded by the two `rem_euclid` calls immediately
    // above them and are proved rather than hoped: after the reductions,
    // `seconds` is in `0..86_400`, so hour is in `0..24`, minute and second in
    // `0..60`, and `nanosecond` is in `0..1_000_000_000`. Every one fits its
    // target type with room to spare, and none can be negative. The divisions
    // are the calendar arithmetic itself. `try_from` is unavailable here
    // because this is a `const fn`, which it is so that a timestamp can be
    // rendered without allocating in a failure path.
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::integer_division
    )]
    #[must_use]
    pub const fn civil_utc(self) -> Civil {
        const NANOS_PER_SECOND: i128 = 1_000_000_000;
        const SECONDS_PER_DAY: i128 = 86_400;

        // Floor division, so moments before the epoch land on the right day
        // rather than one day late.
        let mut seconds = self.utc_nanos.div_euclid(NANOS_PER_SECOND);
        let nanosecond = self.utc_nanos.rem_euclid(NANOS_PER_SECOND);
        let days = seconds.div_euclid(SECONDS_PER_DAY);
        seconds = seconds.rem_euclid(SECONDS_PER_DAY);

        let (year, month, day) = civil_from_days(days);
        Civil {
            year,
            month,
            day,
            hour: (seconds / 3600) as u8,
            minute: ((seconds % 3600) / 60) as u8,
            second: (seconds % 60) as u8,
            nanosecond: nanosecond as u32,
        }
    }
}

/// A civil date and time, as a record renders it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Civil {
    /// The proleptic Gregorian year.
    pub year: i64,
    /// The month, 1 through 12.
    pub month: u8,
    /// The day of the month, 1 through 31.
    pub day: u8,
    /// The hour, 0 through 23.
    pub hour: u8,
    /// The minute, 0 through 59.
    pub minute: u8,
    /// The second, 0 through 59. MCF does not represent leap seconds; the
    /// system clock does not hand them out.
    pub second: u8,
    /// The nanosecond within the second.
    pub nanosecond: u32,
}

/// Howard Hinnant's `civil_from_days`, exact for the proleptic Gregorian
/// calendar over the whole representable range.
///
/// Reproduced rather than depended on (B15), and checked against known dates
/// in the tests (A19).
// `mp` is in `0..12` and `day` in `1..=31` by the algorithm's own arithmetic,
// so both fit `u8`. The year is bounded by the range of a nanosecond timestamp:
// `i128::MAX` nanoseconds is about 5.4 × 10^18 years, which fits `i64`. The
// divisions are the algorithm.
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::integer_division
)]
const fn civil_from_days(days: i128) -> (i64, u8, u8) {
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let day_of_era = z - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let mp = (5 * day_of_year + 2) / 153;
    let day = (day_of_year - (153 * mp + 2) / 5 + 1) as u8;
    let month = (if mp < 10 { mp + 3 } else { mp - 9 }) as u8;
    let year = if month <= 2 { year + 1 } else { year };
    (year as i64, month, day)
}

impl fmt::Display for Timestamp {
    /// RFC 3339 in UTC, to nanosecond precision, with the local offset stated
    /// separately — never folded into the moment, because folding it in is how
    /// two machines' records stop being comparable (D9).
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let civil = self.civil_utc();
        write!(
            f,
            "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}.{:09}Z (local offset {})",
            civil.year,
            civil.month,
            civil.day,
            civil.hour,
            civil.minute,
            civil.second,
            civil.nanosecond,
            self.offset,
        )
    }
}
