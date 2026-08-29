//! How busy each accelerator is, from whatever interface its vendor offers
//! (B-216, DEC-007, A7, §3.4, §3.8).
//!
//! **Every vendor exposes this differently, and one of them not at all.** AMD
//! publishes `gpu_busy_percent` in sysfs, which any process can read. NVIDIA
//! publishes nothing in sysfs and requires NVML, which MCF already loads for
//! memory and temperature. Intel's `i915` has no equivalent single figure —
//! utilisation there is derived from perf counters that need elevated
//! privilege — so on an Intel graphics machine this is honestly `Unknown`.
//!
//! **Unknown is a reading.** A machine whose accelerator MCF cannot poll is
//! not an idle machine, and rendering it as zero would be the same failure
//! `Energy` exists to prevent: a plausible number with nothing behind it
//! (B39, A7). Every absence here carries the reason, so an operator can tell
//! *nothing was competing* from *MCF could not see*.
//!
//! **On demand only** (B4, F85). Nothing here runs on a timer.
//!
//! **Cross-checked by test:** `accelerator_occupancy_agrees_with_the_vendor` — the
//! vendor's own tool, where one is installed.

use core::fmt;

use crate::attested::Attested;

/// One accelerator's occupancy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Busy {
    /// Which card, as the kernel numbers them.
    pub card: String,
    /// The driver that owns it.
    pub driver: String,
    /// How busy, in per cent, where the vendor publishes it.
    pub percent: Attested<u64>,
    /// Why it is not known, where it is not.
    pub because: Option<String>,
}

impl fmt::Display for Busy {
    fn fmt(&self, form: &mut fmt::Formatter<'_>) -> fmt::Result {
        match (&self.percent, &self.because) {
            (Attested::Known(held), _) => {
                write!(form, "{} ({}) {held}% busy", self.card, self.driver)
            }
            (Attested::Unknown, Some(why)) => {
                write!(form, "{} ({}) unknown: {why}", self.card, self.driver)
            }
            (Attested::Unknown, None) => write!(
                form,
                "{} ({}) unknown, and no reason was recorded",
                self.card, self.driver
            ),
        }
    }
}

/// Every accelerator this machine has, and how busy each is.
///
/// Reads sysfs only. NVIDIA cards appear here with `Unknown` and the reason;
/// their occupancy comes through NVML on the [`crate::hardware::Machine`],
/// which is where the vendor put it.
#[must_use]
pub fn accelerators() -> Vec<Busy> {
    let mut found = Vec::new();
    let Ok(entries) = std::fs::read_dir("/sys/class/drm") else {
        return found;
    };
    let mut cards: Vec<String> = entries
        .flatten()
        .filter_map(|entry| {
            let name = entry.file_name().into_string().ok()?;
            // `card1` is a device; `card1-DP-1` is a connector on it.
            (name.starts_with("card") && !name.contains('-')).then_some(name)
        })
        .collect();
    cards.sort();
    for card in cards {
        let at = std::path::Path::new("/sys/class/drm")
            .join(&card)
            .join("device");
        let driver = std::fs::read_to_string(at.join("uevent"))
            .ok()
            .and_then(|held| {
                held.lines()
                    .find_map(|line| line.strip_prefix("DRIVER=").map(str::to_owned))
            })
            .unwrap_or_else(|| "unknown".to_owned());
        let busy = std::fs::read_to_string(at.join("gpu_busy_percent"))
            .ok()
            .and_then(|held| held.trim().parse::<u64>().ok());
        let because = busy.is_none().then(|| match driver.as_str() {
            "nvidia" => "this driver publishes no occupancy in sysfs; NVML is where the vendor \
                         put it"
                .to_owned(),
            "i915" | "xe" => "this driver derives occupancy from perf counters rather than \
                              publishing a figure"
                .to_owned(),
            other => format!("{other} publishes no `gpu_busy_percent`"),
        });
        found.push(Busy {
            card,
            driver,
            percent: busy.map_or(Attested::Unknown, Attested::Known),
            because,
        });
    }
    found
}

/// The drivers here whose occupancy MCF cannot read.
///
/// What a support request is assembled from, for the same reason
/// [`crate::hardware::thermal::unrecognised`] exists: the driver name is the
/// whole of what is needed to close the gap, and it is on the operator's
/// machine rather than on MCF's.
#[must_use]
pub fn unreadable(found: &[Busy]) -> Vec<String> {
    let mut named: Vec<String> = found
        .iter()
        .filter(|held| matches!(held.percent, Attested::Unknown))
        .map(|held| held.driver.clone())
        .collect();
    named.sort();
    named.dedup();
    named
}

#[cfg(test)]
mod tests;
