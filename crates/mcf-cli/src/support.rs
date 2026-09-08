fn line(into: &mut String, said: &str) {
    into.push_str(said);
    into.push('\n');
}
use std::path::PathBuf;

use mcf_core::hardware::{thermal, utilisation};

use crate::Response;

pub(crate) struct Gaps {
    pub(crate) chips: Vec<String>,
    pub(crate) no_processor_temperature: bool,
    pub(crate) accelerators: Vec<String>,
}

impl Gaps {
    pub(crate) fn here() -> Self {
        let sensors = thermal::sensors();
        let cards = utilisation::accelerators();
        Self {
            chips: thermal::unrecognised(&sensors),
            no_processor_temperature: thermal::processor_is_unreadable(&sensors),
            accelerators: utilisation::unreadable(&cards),
        }
    }

    pub(crate) fn any(&self) -> bool {
        self.no_processor_temperature || !self.chips.is_empty() || !self.accelerators.is_empty()
    }

    pub(crate) fn one_line(&self) -> Option<String> {
        if !self.any() {
            return None;
        }
        let mut said = Vec::new();
        if self.no_processor_temperature {
            said.push("no processor temperature could be read on this machine".to_owned());
        }
        if !self.chips.is_empty() {
            said.push(format!(
                "{} sensor chip(s) MCF does not recognise ({})",
                self.chips.len(),
                self.chips.join(", ")
            ));
        }
        if !self.accelerators.is_empty() {
            said.push(format!(
                "{} accelerator driver(s) whose occupancy MCF cannot read ({})",
                self.accelerators.len(),
                self.accelerators.join(", ")
            ));
        }
        Some(format!(
            "  NOTE {}. `mcf support --into <path>` writes what a maintainer would need to \
             close that, for you to read before you send it (A7).",
            said.join("; ")
        ))
    }
}

pub(crate) fn run(into: Option<&str>) -> Response {
    let sensors = thermal::sensors();
    let cards = utilisation::accelerators();
    let gaps = Gaps::here();

    let mut report = String::new();
    report.push_str("# MCF hardware support request\n\n");
    report.push_str(
        "Written by `mcf support`. Nothing was sent: this is a file for you to read and send \
         if you choose.\n\nIt carries driver names, sensor labels and readings, and the paths \
         MCF looked in. It carries no prompt, no model output, no file content and no path \
         from a model directory.\n\n",
    );

    line(
        &mut report,
        &format!(
            "## What MCF is\n\n- build: {}\n",
            mcf_core::build_identity::BuildIdentity::current()
        ),
    );
    line(
        &mut report,
        &format!(
            "## What this machine is\n\n- architecture: {}\n- operating system: {}\n- kernel: {}\n",
            std::env::consts::ARCH,
            std::env::consts::OS,
            std::fs::read_to_string("/proc/sys/kernel/osrelease").map_or_else(
                |_| "not readable on this platform".to_owned(),
                |held| held.trim().to_owned(),
            )
        ),
    );

    report.push_str("## Temperature sensors\n\n");
    report.push_str("Read from `/sys/class/hwmon/*`.\n\n");
    if sensors.is_empty() {
        report.push_str("None. This platform published no `hwmon` sensors at all.\n\n");
    } else {
        for sensor in &sensors {
            line(&mut report, &format!("- {sensor}"));
        }
        report.push('\n');
    }

    report.push_str("## Accelerators\n\n");
    report.push_str("Read from `/sys/class/drm/card*/device`.\n\n");
    if cards.is_empty() {
        report.push_str("None found.\n\n");
    } else {
        for card in &cards {
            line(&mut report, &format!("- {card}"));
        }
        report.push('\n');
    }

    report.push_str("## What MCF could not account for\n\n");
    if gaps.any() {
        if gaps.no_processor_temperature {
            report.push_str(
                "- **No processor temperature.** MCF found no sensor it recognises as a \
                 processor die or package. This is the reading a thermal criterion needs, and \
                 the gap most worth closing.\n",
            );
        }
        for chip in &gaps.chips {
            line(
                &mut report,
                &format!(
                    "- **Unrecognised sensor chip `{chip}`.** Its readings are above; what is \
                 missing is which of them measures what."
                ),
            );
        }
        for driver in &gaps.accelerators {
            line(
                &mut report,
                &format!(
                    "- **Accelerator driver `{driver}` publishes no occupancy MCF can read.**"
                ),
            );
        }
        report.push('\n');
    } else {
        report.push_str(
            "Nothing. Every sensor was classified and every accelerator's occupancy was \
             readable. This report is still worth sending if something here looks wrong — a \
             reading MCF classified confidently and wrongly is harder to find than one it \
             could not classify at all.\n\n",
        );
    }

    let path = into.map_or_else(|| PathBuf::from("mcf-support.md"), PathBuf::from);
    match std::fs::write(&path, &report) {
        Ok(()) => Response {
            text: format!(
                "wrote {}\n  Read it before you send it — it is a file and nothing has left \
                 this machine (§3.20).\n{}",
                path.display(),
                gaps.one_line()
                    .unwrap_or_else(|| "  Nothing here was unreadable.".to_owned())
            ),
            served: true,
        },
        Err(why) => Response {
            text: format!("mcf: could not write {}: {why}", path.display()),
            served: false,
        },
    }
}
