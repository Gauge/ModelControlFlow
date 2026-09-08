use core::fmt;

use crate::attested::Attested;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Busy {
    pub card: String,
    pub driver: String,
    pub percent: Attested<u64>,
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
