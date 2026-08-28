//! Instruments that were found to be wrong, and what they did to readings
//! (F93, A1, A2, §3.4, §6.16).
//!
//! **Why this has to exist.** MCF found three defects in its own measuring
//! instruments in a single working day — a contention reading high by a fifth
//! under load, a processor temperature it never read at all, and an effect
//! size with no measure of itself. Each was corrected. None of the
//! measurements already recorded could be told apart from the ones taken
//! afterwards, because the condition that was supposed to identify the
//! instrument — MCF's version — does not change when an instrument does.
//!
//! **The rate will not be zero.** A repository that finds three in one day
//! will find more, and each one silently partitions the record into a before
//! and an after. Without a mechanism, that partition exists only in somebody's
//! memory and in a git log nobody reading a measurement will consult.
//!
//! **Keyed on time, because time is what the record reliably carries.** Going
//! forward a measurement carries the digest of the binary that took it
//! (`build_identity::instrument`), which identifies the instrument exactly.
//! Every measurement taken *before* that field existed cannot be attributed
//! that way — but `recorded_at` is on every entry, precise, and trustworthy.
//! So an erratum names the moment the defect was corrected, and everything
//! recorded before it is affected.
//!
//! **Nothing is rewritten** (A1). An erratum is appended and rendered beside
//! the entries it concerns. The measurements stay exactly as they were taken,
//! because they are what the instrument said, and a record that edits its own
//! history to look better is not a record.
//!
//! **This is §6.16 pointed at MCF.** *The instrument does not get to grade
//! itself* — so when it turns out to have been wrong, the correction is part
//! of the record rather than a commit message.

use core::fmt;

/// An instrument found to have been wrong, and until when.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Erratum {
    /// Which instrument, by the module that implements it.
    pub instrument: &'static str,
    /// What was wrong, in one line and in the present tense.
    pub defect: &'static str,
    /// What it did to the readings, so a reader can judge what to do about a
    /// measurement rather than only be told to distrust it.
    pub effect: &'static str,
    /// The finding that establishes it.
    pub finding: &'static str,
    /// Everything recorded strictly before this moment is affected.
    ///
    /// UTC nanoseconds, matching an entry's own `recorded_at_utc_nanos`, so
    /// the comparison is integer and needs no calendar arithmetic (D9, B37).
    pub corrected_at_utc_nanos: i64,
    /// The same moment, readable.
    ///
    /// Two spellings of one fact, which is a thing that drifts — the first
    /// version of this list had nanoseconds two days from the date beside
    /// them, computed by hand and plausible on sight. The test cross-checks
    /// them against each other through [`crate::time::Timestamp`], which is
    /// A19: a reported quantity checked against an independently known value.
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

/// Every instrument defect MCF knows about in its own history.
///
/// **Carried in the source rather than only in the record**, so that a binary
/// reading a record written elsewhere still knows what was wrong with the
/// instrument that wrote it. The list travels with the tool; the entries in
/// the journal are how it travels with an exported record.
///
/// Ordered oldest first. Adding to it is part of fixing an instrument, and
/// `checks/tests/a_corrected_instrument_leaves_an_erratum.rs` is what makes
/// that not optional.
pub const KNOWN: [Erratum; 3] = [
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
];

/// The errata affecting something recorded at `utc_nanos`.
///
/// Everything corrected *after* a measurement was taken applies to it.
#[must_use]
pub fn affecting(utc_nanos: i64) -> Vec<&'static Erratum> {
    KNOWN
        .iter()
        .filter(|held| utc_nanos < held.corrected_at_utc_nanos)
        .collect()
}

#[cfg(test)]
mod tests;
