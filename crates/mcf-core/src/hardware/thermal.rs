//! What the machine's own sensors say, read on demand (B-084, DEC-007, B4,
//! A7, §3.4, §3.8).
//!
//! **A recorded finding was wrong, and this exists because of it.** F53
//! concluded that *the only sensor this machine exposes reads sixteen degrees,
//! which is not a processor temperature*, and DEC-007's thermal half has been
//! blocked on that ever since. It was reading `/sys/class/thermal`, where this
//! board publishes one ACPI zone that does indeed read 16.8 °C. One directory
//! across, `/sys/class/hwmon` carries `k10temp` with the actual die sensors —
//! Tctl at 70 °C and a reading per compute die. The sensors were there the
//! whole time. F91.
//!
//! **Everything is reported and nothing is chosen.** A machine has many
//! thermometers and they measure different things: a processor die, a
//! board-level zone, a drive, an accelerator. MCF reads all of them, labels
//! each with the chip that published it, and classifies the ones it
//! recognises. It does not average them, does not pick a headline, and does
//! not drop the ones it cannot classify — an unrecognised sensor is still a
//! reading somebody may need, and dropping it would be the silent loss A1
//! forbids.
//!
//! **The classification is a table of driver names, and tables go stale.** The
//! chip name is the only signal there is, so a table is unavoidable; what is
//! avoidable is letting the table decide what gets reported.
//! [`Kind::Unclassified`] is a first-class outcome, everything is carried
//! whether or not it is recognised, and a machine whose drivers are unknown to
//! this list loses labels rather than data.
//!
//! **The critical point is often absent, and absence is not a default.**
//! Intel's `coretemp` publishes `tempN_crit`; the `k10temp` on this machine
//! publishes none. MCF reports what the chip publishes and leaves the rest
//! `None` — assuming a limit would be inventing the one number a thermal
//! criterion would be built on (A7, and the same wall
//! [F71](../doc/findings.md) stopped at).
//!
//! **On demand and never on a timer** (B4, F85). Reading these files is cheap
//! and reading them on a schedule would make MCF one of the competitors it
//! reports.
//!
//! **Portability.** This is Linux's sysfs. Windows exposes
//! `MSAcpi_ThermalZoneTemperature` through WMI, which is the same ACPI zone
//! that reads 16.8 °C here and is frequently absent altogether; the real
//! per-die registers need a kernel driver to reach, which MCF does not ship.
//! So a Windows build should expect `Unknown` for the processor and say so
//! rather than substituting a board sensor for a die one.

use core::fmt;

/// What a sensor is measuring, where MCF recognises the chip.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum Kind {
    /// A processor compute die — the closest thing to *how hot is the thing
    /// doing the work*.
    ProcessorDie,
    /// A processor package or control temperature, which may carry a vendor
    /// offset and is not always the same as a die reading.
    ProcessorPackage,
    /// An accelerator.
    Accelerator,
    /// A drive.
    Storage,
    /// A board-level zone: a chipset, a VRM, an ACPI thermal zone.
    ///
    /// Named separately because these are the least trustworthy as a proxy
    /// for anything: the ACPI zone on this machine reads 16.8 °C while the
    /// processor is at 70 °C.
    Board,
    /// A chip this build does not recognise.
    ///
    /// Still reported in full. A7: unrecognised is not absent.
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

/// One thermometer, as its chip published it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sensor {
    /// The driver that published it — `k10temp`, `coretemp`, `acpitz`.
    pub chip: String,
    /// The chip's own label for this channel, where it publishes one.
    pub label: Option<String>,
    /// The reading, in thousandths of a degree Celsius.
    ///
    /// Signed, because a sensor can read below zero and a machine in a cold
    /// room is not a broken machine.
    pub millidegrees: i64,
    /// The temperature the chip calls critical, where it publishes one.
    ///
    /// `None` where the chip publishes none, which is most AMD processors.
    /// Not a default: a limit MCF assumed would be the one number a thermal
    /// criterion rests on (A7).
    pub critical_millidegrees: Option<i64>,
    /// What MCF takes this to be measuring.
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
            // A7 in the rendering, not only in the type: a reader who is not
            // told the limit is missing will assume there is headroom.
            None => write!(form, ", critical point not published by this chip"),
        }
    }
}

/// Which chips measure what.
///
/// A driver-name table, which is the only signal sysfs offers. Ordered longest
/// first is unnecessary here because the match is exact; what matters is that
/// an unlisted chip falls through to [`Kind::Unclassified`] rather than being
/// guessed at.
fn kind_of(chip: &str, label: Option<&str>) -> Kind {
    // The label distinguishes a die from a package on the chips that publish
    // both. `Tccd1` is one of a Ryzen's compute dies; `Tctl` is the control
    // temperature the firmware derives, which on some parts carries an offset.
    let die = label.is_some_and(|held| {
        held.starts_with("Tccd") || held.starts_with("Core ") || held.starts_with("Tdie")
    });
    match chip {
        // x86 processors. `k10temp` is in every mainline kernel for AMD;
        // `zenpower` is the out-of-tree replacement some operators install.
        "k10temp" | "zenpower" | "zenpower3" | "coretemp" => {
            if die {
                Kind::ProcessorDie
            } else {
                Kind::ProcessorPackage
            }
        }
        // Systems-on-chip, where the die and the package are the same thing.
        // The names differ per silicon vendor and there is no common one.
        "cpu_thermal" | "cpu-thermal" | "soc_thermal" | "soc-thermal" | "bcm2835_thermal"
        | "rockchip_thermal" | "tegra_soctherm" | "imx_thermal" | "sun4i_ts" | "sun8i_ths"
        | "qcom_tsens" | "tsens" | "scpi_sensors" | "scmi_sensors" | "apple_soc_thermal" => {
            Kind::ProcessorDie
        }
        // Accelerators.
        "amdgpu" | "radeon" | "nouveau" | "i915" | "xe" | "nvidia" | "habanalabs" => {
            Kind::Accelerator
        }
        // Drives.
        "nvme" | "drivetemp" => Kind::Storage,
        // Board-level zones, chipsets, super-I/O and vendor embedded
        // controllers. Grouped because they share the property that matters:
        // none of them is measuring the thing doing the work, and reading one
        // as though it were is the mistake F91 exists to correct.
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

/// The chips on this machine that this build does not recognise.
///
/// **Not an error and not a gap to paper over** (A7, A2). A machine whose
/// sensors MCF cannot classify still gets every reading; what it loses is the
/// label that says which reading is the processor. That is worth telling the
/// operator about, because they are the only person who can close it: the
/// chip name is the whole of what MCF needs to widen the table, and it is on
/// their machine.
///
/// This is what a support request is assembled from. The table above is
/// unavoidably a list of driver names and lists go stale; the honest answer to
/// that is not a better list but a route by which an unknown name reaches
/// somebody who can add it.
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

/// Whether this machine published no processor temperature at all.
///
/// The state a Windows build should expect, and the one that most deserves a
/// support request: everything else MCF can still measure, and this is the
/// reading DEC-007's thermal half needs.
#[must_use]
pub fn processor_is_unreadable(sensors: &[Sensor]) -> bool {
    processor(sensors).is_none()
}

/// The range a silicon thermometer can plausibly report, in thousandths of a
/// degree.
///
/// **Why a bound at all**, when MCF's discipline is to report what it reads.
/// Because some chips publish a *sentinel* in the limit fields rather than
/// leaving them absent: an `NVMe` drive here reports `temp2_max` as 65261800,
/// which renders as *critical at 65261.8 °C* — a number no thermometer
/// produced, presented with the same confidence as one that was measured. An
/// out-of-range limit is not a hot machine, it is the chip's way of saying
/// nothing, and reading it as a limit is the same defect as the contention
/// instrument reporting more cores than the machine has (F90).
///
/// The bound is generous on purpose: −50 °C covers a cold room and a
/// mis-scaled sensor, and 200 °C is far above any silicon junction limit, so
/// nothing a real chip publishes falls outside it.
const PLAUSIBLE: core::ops::RangeInclusive<i64> = -50_000..=200_000;

/// Every thermometer this machine publishes, read now.
///
/// Empty where the platform exposes none, which is *unknown* rather than
/// *cold* — the caller renders it that way (A7).
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
                // A limit outside what a thermometer can report is a
                // sentinel, and absent is what it means (A7).
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

/// The hottest processor reading, where the machine publishes one.
///
/// **A die reading in preference to a package one**, because a package or
/// control temperature can carry a vendor offset and a die sensor is the thing
/// doing the work. `None` where no processor sensor was found at all — which
/// is what a Windows build should expect, and which must render as *unknown*
/// rather than as a machine that is cool (A7).
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
