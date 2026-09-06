//! Measurements, said in words somebody who has never heard of a token can use.
//!
//! **This is a presentation and never a record.** Every function here takes a
//! measurement and returns a sentence; nothing returns a number that anything
//! else computes with, and nothing MCF stores passes through this file. The
//! record keeps milliseconds and tokens (A1), the window shows words a second,
//! and the two do not have to agree about vocabulary because only one of them
//! is evidence.
//!
//! **Every conversion here has a constant in it, and a constant is a
//! condition** (A6). They are named below rather than buried in the
//! arithmetic, and [`basis`] states them in a sentence that the interface can
//! show — because a person who is told "twenty-five times faster than you can
//! read" is owed the reading speed that was assumed.

#![allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "the values converted here are token counts, byte counts and \
              rates. A context length is at most a few million, the largest \
              model anybody holds is a few hundred billion bytes, and both are \
              exact in f64 — the mantissa runs out at 9,007,199,254,740,992, \
              which is four thousand times the largest file. Every f64 that \
              becomes an integer has already been checked finite and positive"
)]

/// Words per token, averaged over English prose.
///
/// Tokenisers differ and so do texts; this is the round number in the middle of
/// the range the common vocabularies produce, and it is why every phrasing
/// built on it says *about*.
pub const WORDS_PER_TOKEN: f64 = 0.75;

/// Words a minute, silent adult reading of ordinary prose.
pub const READING_SPEED: f64 = 240.0;

/// The sentence naming the two constants above, for the interface to show
/// wherever it has used them.
#[must_use]
pub fn basis() -> String {
    format!(
        "Words are counted at about {WORDS_PER_TOKEN} to a token, and reading at \
         {READING_SPEED:.0} words a minute."
    )
}

/// A whole number with its sign, its digits grouped in threes.
#[must_use]
pub fn grouped_signed(number: i64) -> String {
    let digits = grouped(number.unsigned_abs());
    if number < 0 {
        format!("-{digits}")
    } else {
        digits
    }
}

/// A count with its thousands separated — the console's (B-072).
#[must_use]
pub fn grouped(number: u64) -> String {
    mcf_tui::screens::grouped(number)
}

/// How quickly a model produces text, in words a second.
///
/// Returns `None` when there is no measurement, because there is no honest
/// sentence to write in that case — the interface says *not measured yet* and
/// offers the button that measures it (A7).
#[must_use]
pub fn speed_in_words(tokens_a_second: Option<f64>) -> Option<String> {
    let rate = tokens_a_second?;
    if !rate.is_finite() || rate <= 0.0 {
        return None;
    }
    let words = rate * WORDS_PER_TOKEN;
    // A tenth of a word a second is meaningless above ten; below it, the
    // difference between 3 and 3.4 is the difference between two experiences.
    Some(if words >= 10.0 {
        format!("{words:.0} words a second")
    } else {
        format!("{words:.1} words a second")
    })
}

/// The bare figure, for a screen that puts the unit somewhere else.
#[must_use]
pub fn speed_figure(tokens_a_second: Option<f64>) -> Option<String> {
    let rate = tokens_a_second?;
    if !rate.is_finite() || rate <= 0.0 {
        return None;
    }
    let words = rate * WORDS_PER_TOKEN;
    Some(if words >= 10.0 {
        format!("{words:.0}")
    } else {
        format!("{words:.1}")
    })
}

/// How that speed compares with reading it.
///
/// The comparison people actually have a feel for. Below reading speed it says
/// so plainly rather than reporting a fraction, because "0.4× as fast as you
/// can read" is a number pretending to be an intuition.
#[must_use]
pub fn against_reading(tokens_a_second: Option<f64>) -> Option<String> {
    let rate = tokens_a_second?;
    if !rate.is_finite() || rate <= 0.0 {
        return None;
    }
    let words_a_second = rate * WORDS_PER_TOKEN;
    let reading = READING_SPEED / 60.0;
    let times = words_a_second / reading;
    Some(if times >= 2.0 {
        format!("about {times:.0}× faster than you can read")
    } else if times >= 1.1 {
        "a little faster than you can read".to_owned()
    } else if times >= 0.9 {
        "about as fast as you can read".to_owned()
    } else {
        "slower than you can read".to_owned()
    })
}

/// How much conversation a model can hold, in words.
#[must_use]
pub fn remembers(context_tokens: Option<u64>) -> Option<String> {
    let tokens = context_tokens?;
    if tokens == 0 {
        return None;
    }
    // Rounded to two significant figures: the conversion is approximate and a
    // figure like "24,576 words" claims a precision the constant cannot carry.
    let words = tokens as f64 * WORDS_PER_TOKEN;
    let rounded = round_to_two(words);
    Some(format!("about {} words", grouped(rounded)))
}

/// Rounds to two significant figures, which is all an approximate conversion
/// has earned.
fn round_to_two(value: f64) -> u64 {
    if !value.is_finite() || value <= 0.0 {
        return 0;
    }
    let magnitude = value.log10().floor();
    let step = 10_f64.powf(magnitude - 1.0);
    if step <= 0.0 {
        return value.round().max(0.0) as u64;
    }
    ((value / step).round() * step).max(0.0) as u64
}

/// A size in bytes, as a person would say it.
#[must_use]
pub fn size_in_words(bytes: Option<u64>) -> Option<String> {
    let bytes = bytes?;
    let giga = bytes as f64 / 1_000_000_000.0;
    Some(if giga >= 10.0 {
        format!("{giga:.0} GB")
    } else if giga >= 1.0 {
        format!("{giga:.1} GB")
    } else {
        format!("{:.0} MB", bytes as f64 / 1_000_000.0)
    })
}

/// How long a model takes to become ready.
#[must_use]
pub fn wakes_in(seconds: Option<f64>) -> Option<String> {
    let seconds = seconds?;
    if !seconds.is_finite() || seconds < 0.0 {
        return None;
    }
    Some(if seconds < 1.0 {
        "under a second".to_owned()
    } else if seconds < 90.0 {
        format!("{seconds:.0} seconds")
    } else {
        format!("{:.0} minutes", seconds / 60.0)
    })
}

/// Whether a model fits, and what that leaves.
///
/// Says the same thing whether it fits or not, so that a person reads one
/// sentence in the same place either way rather than hunting for a warning.
#[must_use]
pub fn does_it_fit(needs: Option<u64>, free: Option<u64>, where_: &str) -> String {
    let (Some(needs), Some(free)) = (needs, free) else {
        return "Not measured yet".to_owned();
    };
    let needed = size_in_words(Some(needs)).unwrap_or_else(|| "?".to_owned());
    let room = size_in_words(Some(free)).unwrap_or_else(|| "?".to_owned());
    if needs <= free {
        format!("Uses {needed} of your {room} of {where_}")
    } else {
        format!("Needs {needed}, and there is {room} of {where_}")
    }
}

/// How much the fall-off matters, said as whether it matters.
///
/// The measurement is a slope in milliseconds per token per token. Nobody
/// wants that. What they want to know is whether a long conversation goes
/// slow, and the honest answers to that are three.
#[must_use]
pub fn holds_up(fastest: Option<f64>, slowest: Option<f64>) -> Option<String> {
    let (fastest, slowest) = (fastest?, slowest?);
    if !fastest.is_finite() || !slowest.is_finite() || fastest <= 0.0 {
        return None;
    }
    let ratio = slowest / fastest;
    Some(if ratio <= 1.25 {
        "Stays fast".to_owned()
    } else if ratio <= 2.0 {
        "Slows a little".to_owned()
    } else {
        format!("Slows to about a {}th of its speed", ratio.round() as u64)
    })
}

/// What the interface says where it has no measurement.
///
/// One phrase, everywhere, and never a zero or a dash: a person reading "0"
/// concludes something false, and a person reading "—" concludes nothing at
/// all. This says which it is and implies the remedy (A7).
pub const UNMEASURED: &str = "Not measured yet";

#[cfg(test)]
mod tests;
