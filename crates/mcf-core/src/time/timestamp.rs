use core::fmt;

use crate::attested::Attested;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct UtcOffset(i32);

impl UtcOffset {
    pub const UTC: Self = Self(0);

    #[must_use]
    pub const fn from_seconds_east(seconds: i32) -> Option<Self> {
        const LIMIT: i32 = 26 * 3600;
        if seconds < -LIMIT || seconds > LIMIT {
            None
        } else {
            Some(Self(seconds))
        }
    }

    #[must_use]
    pub const fn seconds_east(self) -> i32 {
        self.0
    }
}

impl fmt::Display for UtcOffset {
    #[allow(clippy::integer_division)]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let sign = if self.0 < 0 { '-' } else { '+' };
        let total = self.0.unsigned_abs();
        write!(f, "{sign}{:02}:{:02}", total / 3600, (total % 3600) / 60)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Timestamp {
    utc_nanos: i128,
    offset: Attested<UtcOffset>,
}

impl Timestamp {
    #[must_use]
    pub const fn from_utc_nanos(utc_nanos: i128, offset: Attested<UtcOffset>) -> Self {
        Self { utc_nanos, offset }
    }

    #[must_use]
    pub fn now() -> Self {
        let since_epoch = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH);
        let utc_nanos = match since_epoch {
            Ok(elapsed) => i128::try_from(elapsed.as_nanos()).unwrap_or(i128::MAX),
            Err(before) => {
                i128::try_from(before.duration().as_nanos()).map_or(i128::MIN, |nanos| -nanos)
            }
        };
        let moment = Self {
            utc_nanos,
            offset: Attested::Unknown,
        };
        Self {
            utc_nanos,
            offset: super::zone::offset_at(moment),
        }
    }

    #[must_use]
    pub const fn utc_nanos(self) -> i128 {
        self.utc_nanos
    }

    #[must_use]
    pub const fn offset(self) -> Attested<UtcOffset> {
        self.offset
    }

    #[must_use]
    pub const fn with_offset(self, offset: UtcOffset) -> Self {
        Self {
            utc_nanos: self.utc_nanos,
            offset: Attested::Known(offset),
        }
    }

    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::integer_division
    )]
    #[must_use]
    pub const fn civil_utc(self) -> Civil {
        const NANOS_PER_SECOND: i128 = 1_000_000_000;
        const SECONDS_PER_DAY: i128 = 86_400;

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Civil {
    pub year: i64,
    pub month: u8,
    pub day: u8,
    pub hour: u8,
    pub minute: u8,
    pub second: u8,
    pub nanosecond: u32,
}

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
