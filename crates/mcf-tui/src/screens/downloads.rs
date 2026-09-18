//! The queue of files MCF is bringing here, with no display attached.
//!
//! The same queue the window and the command line show, read off the same daemon. A file
//! asked for over SSH is paused in the window, and it is one transfer either way.

use crate::screen::{Ink, Screen};
use crate::screens::{UNKNOWN, gigabytes};

/// One transfer, as this screen reads it off the queue.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Arriving {
    pub id: u64,
    pub reference: String,
    pub file: String,
    pub part: u64,
    pub parts: u64,
    pub arrived: u64,
    pub whole: u64,
    pub state: String,
    pub why: Option<String>,
}

impl Arriving {
    #[must_use]
    pub fn name(&self) -> String {
        self.file
            .rsplit('/')
            .next()
            .unwrap_or(&self.file)
            .trim_end_matches(".gguf")
            .to_owned()
    }

    #[must_use]
    pub fn under_way(&self) -> bool {
        matches!(self.state.as_str(), "queued" | "fetching" | "checking")
    }

    #[must_use]
    pub fn settled(&self) -> bool {
        matches!(self.state.as_str(), "done" | "failed" | "cancelled")
    }

    /// How far along, in whole per cent, or nothing while the whole is unknown. A share of
    /// a total nobody has stated is a number made up.
    #[must_use]
    pub fn per_cent(&self) -> Option<u64> {
        self.arrived.saturating_mul(100).checked_div(self.whole)
    }

    #[must_use]
    pub fn ink(&self) -> Ink {
        match self.state.as_str() {
            "failed" => Ink::Refusal,
            "done" => Ink::Held,
            "fetching" | "checking" => Ink::Figure,
            _ => Ink::Quiet,
        }
    }

    #[must_use]
    pub fn said(&self) -> String {
        match self.state.as_str() {
            "queued" => "waiting its turn".to_owned(),
            "fetching" if self.parts > 1 => {
                format!("part {} of {}", self.part.max(1), self.parts)
            }
            "fetching" => "arriving".to_owned(),
            "checking" => "reading it back against its digest".to_owned(),
            "paused" => "stopped where it stood".to_owned(),
            "done" => "here, and checked".to_owned(),
            "failed" => self
                .why
                .clone()
                .unwrap_or_else(|| "it did not arrive".to_owned()),
            "cancelled" => "given up".to_owned(),
            other => other.to_owned(),
        }
    }
}

/// The keys this screen answers to. Shown on it, because a key that is not written down
/// is a key nobody presses.
pub const KEYS: &str = "p pause · c carry on · x give up · d drop the finished · r refresh";

pub fn draw(into: &mut Screen, from: usize, queue: &[Arriving], at: usize) {
    let left = 2;
    let under_way = queue.iter().filter(|held| held.under_way()).count();
    let heading = match under_way {
        0 => "Nothing on its way".to_owned(),
        1 => "One file on its way".to_owned(),
        many => format!("{many} files on their way"),
    };
    into.put(left, from + 1, &heading, Ink::Heading);

    if queue.is_empty() {
        into.put(
            left,
            from + 3,
            "`mcf downloads add <owner/name> <file>` asks for one",
            Ink::Quiet,
        );
        into.put(
            left,
            from + 4,
            "it keeps arriving whether or not anything is watching",
            Ink::Quiet,
        );
        return;
    }

    let mut row = from + 3;
    for (index, transfer) in queue.iter().enumerate() {
        if row + 2 >= into.height().saturating_sub(3) {
            break;
        }
        if index == at {
            into.select_row(row);
        }
        into.put(
            left,
            row,
            &format!("{:>3}  {}", transfer.id, transfer.name()),
            if index == at {
                Ink::Selected
            } else {
                Ink::Plain
            },
        );
        let far = transfer
            .per_cent()
            .map_or_else(|| UNKNOWN.to_owned(), |share| format!("{share}%"));
        let of = if transfer.whole > 0 {
            format!(" of {}", gigabytes(transfer.whole))
        } else {
            String::new()
        };
        into.put(
            left + 5,
            row + 1,
            &format!("{:<9} {far}{of}", transfer.state),
            transfer.ink(),
        );
        into.put(left + 5, row + 2, &transfer.said(), Ink::Quiet);
        row += 4;
    }
    let last = into.height().saturating_sub(3);
    into.put(left, last, KEYS, Ink::Quiet);
}

#[cfg(test)]
mod tests;
