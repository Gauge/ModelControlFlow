use core::fmt;

use crate::attested::Attested;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Thousandths(pub u32);

impl core::str::FromStr for Thousandths {
    type Err = ();

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
    #[allow(clippy::integer_division)]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{:03}", self.0 / 1000, self.0 % 1000)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Sampling {
    pub temperature: Attested<Thousandths>,
    pub top_p: Attested<Thousandths>,
    pub top_k: Attested<u32>,
    pub min_p: Attested<Thousandths>,
    pub repetition_penalty: Attested<Thousandths>,
    pub max_output_tokens: Attested<u32>,
}

impl Sampling {
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
