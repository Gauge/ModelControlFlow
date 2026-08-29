//! What a reading of a given kind *tends* to mean, in words, marked as
//! guidance and never as a result (B-380, DEC-002, A21, §3.15, §3.18).
//!
//! **The hazard is the whole of the design.** A rule of thumb printed beside a
//! measured number in the same typeface becomes a measured number to a reader
//! who is not looking for the difference — and the readers this exists for are
//! exactly those readers. The operator's addition to DEC-002 says so, and the
//! shape MCF already has for it is the one it applies to a model's own claims:
//! *declared*, *verified*, *unknown* are three states and never four (A21).
//! Turned on MCF's own sentences, that makes a touchstone a fourth thing which
//! is none of them — a statement about the world that this machine has not
//! measured — and it has to be impossible to mistake for the other three.
//!
//! **Three structural properties, not three conventions.**
//!
//! 1. **A touchstone contains no digit.** It describes a *relation* in words;
//!    every number on the screen came from a measurement. This is checked
//!    rather than asked for — `checks/tests/a_touchstone_is_never_a_result.rs`
//!    reads the catalogue — because "don't put numbers in it" is exactly the
//!    kind of rule that holds until somebody is in a hurry.
//! 2. **A touchstone cannot exist without saying what MCF has not measured.**
//!    Both halves are required by the constructor and both are rendered. A
//!    guidance sentence whose limits are optional is a guidance sentence whose
//!    limits are absent by the second edit.
//! 3. **A touchstone cannot reach the record.** There is no encoding of it into
//!    a record value and no module of `mcf-record` names this type, for the
//!    reason A25 keeps content out of the record: a filter can be
//!    misconfigured, and a thing that was never written cannot be read back as
//!    a measurement by a later reader who has forgotten where it came from.
//!
//! **And a touchstone is temporary by construction.** Each one names the
//! register item whose laboratory would replace it with a measurement. When
//! that item is done the touchstone is a defect — MCF would be offering a rule
//! of thumb about something it can now measure — and the check fails until it
//! is removed. That is B-380's *the replacement is visible as a change*, held
//! by a machine rather than by whoever remembers.

use core::fmt;

/// A plain-language note about what a kind of reading tends to mean.
///
/// Constructed only from the catalogue below: a touchstone somebody assembles
/// at a call site is one nothing checked.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Touchstone {
    /// What this is about, in the reader's words rather than the record's.
    subject: &'static str,
    /// What a reading of that kind tends to mean.
    tends: &'static str,
    /// What MCF has *not* measured about the relation just described.
    ///
    /// Required, because the sentence is only honest with it. A touchstone is
    /// the one place MCF says something it did not measure, and the price of
    /// saying it is saying so.
    unmeasured: &'static str,
    /// The register item whose work would replace this with a measurement.
    ///
    /// When it is done, this touchstone is removed rather than kept beside the
    /// measurement it was standing in for.
    until: &'static str,
}

impl Touchstone {
    /// What the touchstone is about.
    #[must_use]
    pub const fn subject(&self) -> &'static str {
        self.subject
    }

    /// What MCF has not measured about it.
    #[must_use]
    pub const fn unmeasured(&self) -> &'static str {
        self.unmeasured
    }

    /// The item that would replace it.
    #[must_use]
    pub const fn until(&self) -> &'static str {
        self.until
    }

    /// The words themselves, without the marking.
    ///
    /// Named for what it is so that a caller rendering a touchstone without its
    /// mark has written the word `bare` and can be found (A5's shape: a
    /// degraded thing cannot be rendered as an undegraded one by accident).
    #[must_use]
    pub const fn bare(&self) -> &'static str {
        self.tends
    }
}

impl fmt::Display for Touchstone {
    /// Always marked, and always with its limits.
    ///
    /// There is no rendering of a touchstone that omits either half. The
    /// `RULE OF THUMB` prefix is uppercase for the same reason `DEGRADED` is:
    /// the reader who most needs to see it is the one skimming.
    fn fmt(&self, form: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            form,
            "RULE OF THUMB, not a result — {}: {}. MCF has not measured {}",
            self.subject, self.tends, self.unmeasured
        )
    }
}

/// Every touchstone MCF has.
///
/// A catalogue rather than sentences scattered through the surfaces, so that
/// what MCF says without having measured it is one list somebody can read in
/// one sitting — and so that the check can hold all of it at once.
///
/// It is deliberately short. Every entry is a thing MCF says on no evidence of
/// its own, and §3.15's rule about hidden choices applies twice as hard to
/// sentences the reader cannot check.
pub const CATALOGUE: [Touchstone; 3] = [
    Touchstone {
        subject: "a smaller quantization",
        tends: "usually runs faster and needs less memory, and sometimes answers worse — the \
                loss is uneven across tasks rather than a steady decline",
        unmeasured: "whether this model answers worse at anything you care about, which needs a \
                     laboratory and a graded task",
        until: "B-110",
    },
    Touchstone {
        subject: "a difference this small",
        tends: "is often below what a person notices while waiting for text, and a difference \
                that is easy to measure is not always one anybody feels",
        unmeasured: "what you would notice, which is a fact about you and this machine together \
                     and not about either model",
        until: "B-122",
    },
    Touchstone {
        subject: "two arms that did not separate",
        tends: "means no difference was found *here* — on this prompt, on this machine, in this \
                sitting — rather than that the two are alike everywhere",
        unmeasured: "how they compare on work unlike this, which is what your own workload in \
                     the bench would answer",
        until: "B-205",
    },
];

#[cfg(test)]
mod tests;
