use core::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Band {
    pub fraction: u64,
    pub measured_on: &'static str,
}

pub const MEASURED: Band = Band {
    fraction: 300_000,
    measured_on: "one 32-thread machine, one model pair, 32 tokens (F95); interval width stayed \
                  inside that machine's own quiet variability to 30% of capacity and was 6× to \
                  30× wider by 51%",
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Headroom {
    pub competing: u64,
    pub capacity: u64,
    pub band: Band,
}

impl Headroom {
    #[must_use]
    pub fn taken(competing: u64) -> Self {
        let capacity = u64::try_from(std::thread::available_parallelism().map_or(1, Into::into))
            .unwrap_or(1)
            .saturating_mul(1_000);
        Self {
            competing,
            capacity,
            band: MEASURED,
        }
    }

    #[must_use]
    pub fn fraction(&self) -> u64 {
        self.competing
            .saturating_mul(1_000_000)
            .checked_div(self.capacity.max(1))
            .unwrap_or(0)
    }

    #[must_use]
    pub fn within_band(&self) -> bool {
        self.fraction() <= self.band.fraction
    }
}

impl fmt::Display for Headroom {
    #[allow(clippy::integer_division)]
    fn fmt(&self, form: &mut fmt::Formatter<'_>) -> fmt::Result {
        let per_cent = |held: u64| format!("{}.{}%", held / 10_000, (held % 10_000) / 1_000);
        write!(
            form,
            "{} of this machine was already busy ({}.{:02} of {} core(s))",
            per_cent(self.fraction()),
            self.competing / 1_000,
            (self.competing % 1_000) / 10,
            self.capacity / 1_000
        )?;
        if self.within_band() {
            write!(
                form,
                " — inside the band of {}, so the machine had room",
                per_cent(self.band.fraction)
            )
        } else {
            write!(
                form,
                " — OUTSIDE the band of {}, so this is a real measurement that is not fit to \
                 contribute (B-217, DEC-007). The band is DECLARED from {}; \
                 `prototypes/contention-band` measures it here (A21, A20)",
                per_cent(self.band.fraction),
                self.band.measured_on
            )
        }
    }
}

#[cfg(test)]
mod tests;
