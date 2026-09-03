//! Sampling parameters, carried exactly.
//!
//! D18 puts these in the identity: a model cannot run without them, §6.6 lists
//! them among what MCF tunes, and §XV cannot reproduce behaviour without them.
//! Two configurations differing only in temperature are two configurations.
//!
//! **They are carried as thousandths, not as floats.** Identity is an equality
//! question. `0.7` is not a value a binary float holds exactly, so two
//! configurations that should be identical would depend on how each was
//! parsed — and `Hash` and `Eq` are not even available on `f64`. Thousandths
//! are exact, orderable, hashable, and enough resolution for every sampler
//! parameter a model publisher states.

use core::fmt;

use crate::attested::Attested;

/// A value in thousandths: `700` is `0.7`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Thousandths(pub u32);

impl core::str::FromStr for Thousandths {
    type Err = ();

    /// A decimal as a person or a publisher writes one: `0.7` is 700, `1` is
    /// 1000, `0.9500` is 950.
    ///
    /// More than three decimal places is refused rather than rounded: the
    /// unit is thousandths, and silently dropping a digit is deciding a value
    /// somebody else stated (A7). Read here rather than through a float, so
    /// that what was written is what is held (A1).
    fn from_str(written: &str) -> Result<Self, ()> {
        let written = written.trim();
        let (whole, rest) = match written.split_once('.') {
            Some((whole, rest)) => (whole, rest),
            None => (written, ""),
        };
        if whole.is_empty() && rest.is_empty() {
            return Err(());
        }
        let whole: u32 = if whole.is_empty() {
            0
        } else {
            whole.parse().map_err(|_| ())?
        };
        if rest.len() > 3
            && rest
                .get(3..)
                .is_some_and(|tail| !tail.chars().all(|d| d == '0'))
        {
            return Err(());
        }
        let mut thousandths = 0_u32;
        for at in 0..3 {
            let digit = rest
                .chars()
                .nth(at)
                .map_or(Some(0), |held| held.to_digit(10))
                .ok_or(())?;
            thousandths = thousandths
                .checked_mul(10)
                .and_then(|held| held.checked_add(digit))
                .ok_or(())?;
        }
        whole
            .checked_mul(1_000)
            .and_then(|held| held.checked_add(thousandths))
            .map(Self)
            .ok_or(())
    }
}

impl fmt::Display for Thousandths {
    /// Rendered as the decimal a publisher wrote, so a reader recognizes it.
    // Integer division is the conversion, and both operands are bounded by the
    // type: a `u32` in thousandths cannot overflow or lose a sign.
    #[allow(clippy::integer_division)]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{:03}", self.0 / 1000, self.0 % 1000)
    }
}

/// How the sampler was told to draw.
///
/// Every field is [`Attested`] and `Unknown` means *this configuration does not
/// set it, so whatever the engine defaults to is what ran, and MCF does not
/// claim to know what that is*. A7 forbids the plausible substitute, and an
/// engine default guessed at would be exactly one — it would also make two
/// configurations compare equal that were never run the same way.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Sampling {
    /// Temperature.
    pub temperature: Attested<Thousandths>,
    /// Nucleus sampling threshold.
    pub top_p: Attested<Thousandths>,
    /// Top-k cutoff.
    pub top_k: Attested<u32>,
    /// The floor on a token's probability, as a fraction of the most likely
    /// token's — the third truncation a model file can recommend, and the one
    /// the provisioned engine applies at a house value of its own where the
    /// file says nothing (F157).
    pub min_p: Attested<Thousandths>,
    /// Repetition penalty.
    pub repetition_penalty: Attested<Thousandths>,
    /// The cap on generated tokens.
    pub max_output_tokens: Attested<u32>,
}

impl Sampling {
    /// A sampling configuration that sets nothing.
    ///
    /// Not a default — deliberately not named one. It is the honest state of a
    /// configuration whose sampler settings MCF has not established: every
    /// parameter asked about, none answered. D18 makes the model's own
    /// recommendation the value MCF adopts, marked *declared, unverified*
    /// (A21), and that arrives from calibration rather than from here.
    #[must_use]
    pub const fn nothing_set() -> Self {
        Self {
            temperature: Attested::Unknown,
            top_p: Attested::Unknown,
            top_k: Attested::Unknown,
            min_p: Attested::Unknown,
            repetition_penalty: Attested::Unknown,
            max_output_tokens: Attested::Unknown,
        }
    }

    /// Each parameter, paired with the question it answers.
    #[must_use]
    pub fn entries(&self) -> [(&'static str, String); 6] {
        [
            ("temperature", self.temperature.to_string()),
            ("top_p", self.top_p.to_string()),
            ("top_k", self.top_k.to_string()),
            ("min_p", self.min_p.to_string()),
            ("repetition_penalty", self.repetition_penalty.to_string()),
            ("max_output_tokens", self.max_output_tokens.to_string()),
        ]
    }
}

impl fmt::Display for Sampling {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let rendered: Vec<String> = self
            .entries()
            .into_iter()
            .map(|(name, value)| format!("{name}={value}"))
            .collect();
        f.write_str(&rendered.join(" "))
    }
}
