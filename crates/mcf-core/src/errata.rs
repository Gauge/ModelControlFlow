use core::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Erratum {
    pub instrument: &'static str,
    pub defect: &'static str,
    pub effect: &'static str,
    pub finding: &'static str,
    pub corrected_at_utc_nanos: i64,
    pub corrected_at: &'static str,
}

impl fmt::Display for Erratum {
    fn fmt(&self, form: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            form,
            "{}: {} — {} (corrected {}, {})",
            self.instrument, self.defect, self.effect, self.corrected_at, self.finding
        )
    }
}

pub const KNOWN: [Erratum; 4] = [
    Erratum {
        instrument: "mcf_core::hardware::contention",
        defect: "the rate divided accumulated processor ticks by the interval the sampler \
                 intended to wait rather than the one that elapsed, and walking `/proc` \
                 happens inside that interval",
        effect: "competing-processor readings are high by roughly 8% on a quiet machine and \
                 22% under load, and could report more cores than the machine has",
        finding: "F90",
        corrected_at_utc_nanos: 1_787_953_140_000_000_000,
        corrected_at: "2026-08-28T21:39:00Z",
    },
    Erratum {
        instrument: "mcf_core::hardware::thermal",
        defect: "no processor temperature was read at all; the thermal condition carried \
                 accelerator readings only, and `/sys/class/thermal` was consulted where the \
                 die sensors live in `/sys/class/hwmon`",
        effect: "every measurement recorded before this has no processor thermal state, so a \
                 run taken on a hot machine cannot be told from one taken cold",
        finding: "F91",
        corrected_at_utc_nanos: 1_787_956_020_000_000_000,
        corrected_at: "2026-08-28T22:27:00Z",
    },
    Erratum {
        instrument: "mcf_bench::enough",
        defect: "the effect size was a point estimate beside a sign test that discards \
                 magnitudes, in a sentence claiming the statistic covered the size",
        effect: "the reported difference carried no measure of its own reliability; the \
                 interval is recomputed from the trials on read, so an affected entry renders \
                 correctly now",
        finding: "F92",
        corrected_at_utc_nanos: 1_787_958_120_000_000_000,
        corrected_at: "2026-08-28T23:02:00Z",
    },
    Erratum {
        instrument: "mcf_serve::generation",
        defect: "a generation's engine condition named MCF's version, which is the same string \
                 for every build, so the account could not say which binary produced the answer \
                 — and a daemon serves from its own binary rather than the one that asked",
        effect: "a generation recorded before this cannot be attributed to the build that took \
                 it, and one served by a daemon started from older source is indistinguishable \
                 from one the current binary produced; every entry now carries the digest",
        finding: "F104",
        corrected_at_utc_nanos: 1_787_984_045_000_000_000,
        corrected_at: "2026-08-29T06:14:05Z",
    },
];

#[must_use]
pub fn affecting(utc_nanos: i64) -> Vec<&'static Erratum> {
    KNOWN
        .iter()
        .filter(|held| utc_nanos < held.corrected_at_utc_nanos)
        .collect()
}

#[cfg(test)]
mod tests;
