use core::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Touchstone {
    subject: &'static str,
    tends: &'static str,
    unmeasured: &'static str,
    until: &'static str,
}

impl Touchstone {
    #[must_use]
    pub const fn subject(&self) -> &'static str {
        self.subject
    }

    #[must_use]
    pub const fn unmeasured(&self) -> &'static str {
        self.unmeasured
    }

    #[must_use]
    pub const fn until(&self) -> &'static str {
        self.until
    }

    #[must_use]
    pub const fn bare(&self) -> &'static str {
        self.tends
    }
}

impl fmt::Display for Touchstone {
    fn fmt(&self, form: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            form,
            "RULE OF THUMB, not a result — {}: {}. MCF has not measured {}",
            self.subject, self.tends, self.unmeasured
        )
    }
}

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
