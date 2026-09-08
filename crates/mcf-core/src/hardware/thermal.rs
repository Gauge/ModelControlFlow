use core::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum Kind {
    ProcessorDie,
    ProcessorPackage,
    Accelerator,
    Storage,
    Board,
    Unclassified,
}

impl fmt::Display for Kind {
    fn fmt(&self, form: &mut fmt::Formatter<'_>) -> fmt::Result {
        form.write_str(match self {
            Self::ProcessorDie => "processor die",
            Self::ProcessorPackage => "processor package",
            Self::Accelerator => "accelerator",
            Self::Storage => "storage",
            Self::Board => "board",
            Self::Unclassified => "unclassified",
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sensor {
    pub chip: String,
    pub label: Option<String>,
    pub millidegrees: i64,
    pub critical_millidegrees: Option<i64>,
    pub kind: Kind,
}

impl fmt::Display for Sensor {
    #[allow(clippy::integer_division)]
    fn fmt(&self, form: &mut fmt::Formatter<'_>) -> fmt::Result {
        let named = self.label.as_deref().unwrap_or("unlabelled");
        write!(
            form,
            "{}/{named} {}.{} °C ({})",
            self.chip,
            self.millidegrees / 1000,
            (self.millidegrees.abs() % 1000) / 100,
            self.kind
        )?;
        match self.critical_millidegrees {
            Some(limit) => write!(
                form,
                ", critical at {}.{} °C",
                limit / 1000,
                (limit.abs() % 1000) / 100
            ),
            None => write!(form, ", critical point not published by this chip"),
        }
    }
}

fn kind_of(chip: &str, label: Option<&str>) -> Kind {
    let die = label.is_some_and(|held| {
        held.starts_with("Tccd") || held.starts_with("Core ") || held.starts_with("Tdie")
    });
    match chip {
        "k10temp" | "zenpower" | "zenpower3" | "coretemp" => {
            if die {
                Kind::ProcessorDie
            } else {
                Kind::ProcessorPackage
            }
        }
        "cpu_thermal" | "cpu-thermal" | "soc_thermal" | "soc-thermal" | "bcm2835_thermal"
        | "rockchip_thermal" | "tegra_soctherm" | "imx_thermal" | "sun4i_ts" | "sun8i_ths"
        | "qcom_tsens" | "tsens" | "scpi_sensors" | "scmi_sensors" | "apple_soc_thermal" => {
            Kind::ProcessorDie
        }
        "amdgpu" | "radeon" | "nouveau" | "i915" | "xe" | "nvidia" | "habanalabs" => {
            Kind::Accelerator
        }
        "nvme" | "drivetemp" => Kind::Storage,
        "acpitz" | "applesmc" | "dell_smm" | "thinkpad_acpi" | "asus_wmi_sensors"
        | "asus_ec_sensors" | "system76_acpi" | "pch_cannonlake" | "pch_skylake"
        | "intel_pch_thermal" | "jc42" | "spd5118" => Kind::Board,
        other
            if other.ends_with("_wmi")
                || other.starts_with("nct")
                || other.starts_with("it87")
                || other.starts_with("w836")
                || other.starts_with("f718")
                || other.starts_with("ipmi") =>
        {
            Kind::Board
        }
        _ => Kind::Unclassified,
    }
}

#[must_use]
pub fn unrecognised(sensors: &[Sensor]) -> Vec<String> {
    let mut found: Vec<String> = sensors
        .iter()
        .filter(|held| held.kind == Kind::Unclassified)
        .map(|held| held.chip.clone())
        .collect();
    found.sort();
    found.dedup();
    found
}

#[must_use]
pub fn processor_is_unreadable(sensors: &[Sensor]) -> bool {
    processor(sensors).is_none()
}

const PLAUSIBLE: core::ops::RangeInclusive<i64> = -50_000..=200_000;

#[must_use]
pub fn sensors() -> Vec<Sensor> {
    let mut found = Vec::new();
    let Ok(entries) = std::fs::read_dir("/sys/class/hwmon") else {
        return found;
    };
    for entry in entries.flatten() {
        let at = entry.path();
        let chip = std::fs::read_to_string(at.join("name"))
            .map(|held| held.trim().to_owned())
            .unwrap_or_default();
        if chip.is_empty() {
            continue;
        }
        let Ok(files) = std::fs::read_dir(&at) else {
            continue;
        };
        let mut channels: Vec<String> = files
            .flatten()
            .filter_map(|file| {
                let name = file.file_name().into_string().ok()?;
                name.strip_prefix("temp")?
                    .strip_suffix("_input")
                    .map(str::to_owned)
            })
            .collect();
        channels.sort();
        for channel in channels {
            let read = |suffix: &str| {
                std::fs::read_to_string(at.join(format!("temp{channel}_{suffix}")))
                    .ok()
                    .and_then(|held| held.trim().parse::<i64>().ok())
            };
            let Some(millidegrees) = read("input") else {
                continue;
            };
            let label = std::fs::read_to_string(at.join(format!("temp{channel}_label")))
                .ok()
                .map(|held| held.trim().to_owned())
                .filter(|held| !held.is_empty());
            found.push(Sensor {
                kind: kind_of(&chip, label.as_deref()),
                chip: chip.clone(),
                label,
                millidegrees,
                critical_millidegrees: read("crit")
                    .or_else(|| read("max"))
                    .filter(|held| PLAUSIBLE.contains(held)),
            });
        }
    }
    found.sort_by(|one, other| {
        one.kind
            .cmp(&other.kind)
            .then_with(|| one.chip.cmp(&other.chip))
            .then_with(|| one.label.cmp(&other.label))
    });
    found
}

#[must_use]
pub fn processor(sensors: &[Sensor]) -> Option<&Sensor> {
    sensors
        .iter()
        .filter(|held| held.kind == Kind::ProcessorDie)
        .max_by_key(|held| held.millidegrees)
        .or_else(|| {
            sensors
                .iter()
                .filter(|held| held.kind == Kind::ProcessorPackage)
                .max_by_key(|held| held.millidegrees)
        })
}

#[cfg(test)]
mod tests;
