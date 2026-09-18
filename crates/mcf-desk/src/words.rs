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

pub const WORDS_PER_TOKEN: f64 = 0.75;

pub const READING_SPEED: f64 = 240.0;

#[must_use]
pub fn basis() -> String {
    format!(
        "Words are counted at about {WORDS_PER_TOKEN} to a token, and reading at \
         {READING_SPEED:.0} words a minute."
    )
}

#[must_use]
pub fn grouped_signed(number: i64) -> String {
    let digits = grouped(number.unsigned_abs());
    if number < 0 {
        format!("-{digits}")
    } else {
        digits
    }
}

#[must_use]
pub fn grouped(number: u64) -> String {
    mcf_tui::screens::grouped(number)
}

#[must_use]
pub fn speed_in_words(tokens_a_second: Option<f64>) -> Option<String> {
    let rate = tokens_a_second?;
    if !rate.is_finite() || rate <= 0.0 {
        return None;
    }
    let words = rate * WORDS_PER_TOKEN;
    Some(if words >= 10.0 {
        format!("{words:.0} words a second")
    } else {
        format!("{words:.1} words a second")
    })
}

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

#[must_use]
pub fn remembers(context_tokens: Option<u64>) -> Option<String> {
    let tokens = context_tokens?;
    if tokens == 0 {
        return None;
    }
    let words = tokens as f64 * WORDS_PER_TOKEN;
    let rounded = round_to_two(words);
    Some(format!("about {} words", grouped(rounded)))
}

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

#[must_use]
pub fn size_in_words(bytes: Option<u64>) -> Option<String> {
    let bytes = bytes?;
    let giga = bytes as f64 / 1_000_000_000.0;
    // Terabytes above a thousand gigabytes. A disk said to hold "1900 GB" is a disk
    // nobody reads at a glance, and the page about a disk is where that figure lands.
    Some(if giga >= 1_000.0 {
        format!("{:.1} TB", giga / 1_000.0)
    } else if giga >= 10.0 {
        format!("{giga:.0} GB")
    } else if giga >= 1.0 {
        format!("{giga:.1} GB")
    } else {
        format!("{:.0} MB", bytes as f64 / 1_000_000.0)
    })
}

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

pub const UNMEASURED: &str = "Not measured yet";

#[cfg(test)]
mod tests;
