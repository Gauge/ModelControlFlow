use mcf_core::attested::Attested;
use mcf_core::build_identity::BuildIdentity;
use mcf_core::capture;
use mcf_core::failure::Failure;
use mcf_core::hardware::{Attributability, Characterization, Machine, Watch};
use mcf_core::measurement::{Bytes, Conditions, Measurement};
use mcf_core::self_cost::{
    self, Budget, COLD_START, CORE_BINARY, EVENT_TRIALS, RECORD_WRITE, RESIDENT_IDLE, Verdict,
};
use mcf_core::time::{Duration, Monotonic, Timestamp};
use mcf_record::encode;
use mcf_record::journal::{Entry, EntryKind, Journal, default_path};
use mcf_record::json::Value;

#[derive(Debug)]
pub(crate) struct Report {
    pub(crate) mcf: BuildIdentity,
    pub(crate) at: Timestamp,
    pub(crate) machine: Machine,
    pub(crate) cost: Cost,
    pub(crate) recorded: Recorded,
}

#[derive(Debug)]
pub(crate) struct Cost {
    pub(crate) record_write: Option<Measurement<Duration<Monotonic>>>,
    pub(crate) cold_start_attributability: Attributability,
    pub(crate) record_write_attributability: Attributability,
    pub(crate) artifact: Attested<Bytes>,
    pub(crate) resident: Attested<Bytes>,
    pub(crate) cold_start: Option<Measurement<Duration<Monotonic>>>,
    pub(crate) conditions: Conditions,
}

#[derive(Debug)]
pub(crate) enum Recorded {
    Written {
        path: std::path::PathBuf,
        id: String,
        anomaly: Option<Failure>,
    },
    Refused(Failure),
    Declined,
}

#[must_use]
pub(crate) fn run(record: bool) -> Report {
    let mcf = BuildIdentity::current();
    let at = Timestamp::now();
    let machine = Machine::read();
    let cost = measure_cost(&machine, record);
    let body = body(&machine, &cost);
    let recorded = if record {
        write(&body, at)
    } else {
        Recorded::Declined
    };
    Report {
        mcf,
        at,
        machine,
        cost,
        recorded,
    }
}

fn measure_cost(machine: &Machine, recording: bool) -> Cost {
    let profile = if recording {
        "full — the record is being written"
    } else {
        "reduced — nothing is being recorded"
    };
    let binary_for_conditions = std::env::current_exe().ok();
    let conditions = capture::conditions(
        machine,
        None,
        "mcf doctor, no configuration",
        profile,
        binary_for_conditions.as_deref(),
    );

    let binary = std::env::current_exe().ok();
    let artifact = binary
        .as_deref()
        .map_or(Attested::Unknown, self_cost::artifact_bytes);
    let watch = Watch::start();
    let cold_start = binary.as_deref().and_then(|path| {
        self_cost::cold_start(path, &["--version"], EVENT_TRIALS, conditions.clone())
    });
    let cold_start_attributability = watch.finish();

    let watch = Watch::start();
    let record_write = if recording {
        mcf_record::journal::default_path().and_then(|path| {
            mcf_record::overhead::record_write_cost(&path, conditions.clone())
                .ok()
                .flatten()
        })
    } else {
        None
    };
    let record_write_attributability = watch.finish();

    Cost {
        artifact,
        resident: self_cost::resident_bytes(),
        cold_start,
        record_write,
        cold_start_attributability,
        record_write_attributability,
        conditions,
    }
}

fn body(machine: &Machine, cost: &Cost) -> Value {
    Value::map([
        ("mcf", encode::build_identity(BuildIdentity::current())),
        ("machine", encode::machine(machine)),
        (
            "self_cost",
            Value::map([
                (
                    "artifact_bytes",
                    match cost.artifact {
                        Attested::Known(Bytes(size)) => {
                            Value::Integer(i64::try_from(size).unwrap_or(i64::MAX))
                        }
                        Attested::Unknown => Value::Null,
                    },
                ),
                (
                    "resident_bytes",
                    match cost.resident {
                        Attested::Known(Bytes(size)) => {
                            Value::Integer(i64::try_from(size).unwrap_or(i64::MAX))
                        }
                        Attested::Unknown => Value::Null,
                    },
                ),
                (
                    "record_write",
                    match &cost.record_write {
                        Some(measured) => encode::measurement(measured, |d| {
                            i64::try_from(d.as_nanos()).unwrap_or(i64::MAX)
                        }),
                        None => Value::Null,
                    },
                ),
                (
                    "cold_start",
                    match &cost.cold_start {
                        Some(measured) => encode::measurement(measured, |d| {
                            i64::try_from(d.as_nanos()).unwrap_or(i64::MAX)
                        }),
                        None => Value::Null,
                    },
                ),
                (
                    "not_measurable_here",
                    Value::List(vec![
                        Value::text("idle CPU — needs a daemon"),
                        Value::text("timer wakeups while idle — needs a daemon"),
                        Value::text("memory growth over 30 simulated days — needs a daemon"),
                        Value::text("added request-to-first-token latency — needs a serving path"),
                    ]),
                ),
            ]),
        ),
    ])
}

fn write(body: &Value, at: Timestamp) -> Recorded {
    let Some(path) = default_path() else {
        return Recorded::Refused(no_record_location());
    };
    let mut journal = match Journal::open(&path) {
        Ok(journal) => journal,
        Err(failure) => return Recorded::Refused(failure),
    };
    let entry = Entry::new(EntryKind::MachineProfile, at, body.clone());
    match journal.append(&entry) {
        Ok(appended) => Recorded::Written {
            path,
            id: appended.id.as_str().to_owned(),
            anomaly: appended.anomaly,
        },
        Err(failure) => Recorded::Refused(failure),
    }
}

fn no_record_location() -> Failure {
    use mcf_core::failure::{Attribution, Category, Disposition, Subsystem};
    Failure::new(
        Category::RecordUnwritable,
        Attribution::Machine,
        Disposition::Degraded,
        Subsystem::new("mcf-cli::doctor"),
        "neither XDG_DATA_HOME nor HOME is set, so there is no place to keep the record",
    )
    .with_context("tried", "$XDG_DATA_HOME/mcf, $HOME/.local/share/mcf")
}

fn sensor_lines() -> String {
    let mut said: Vec<String> = mcf_core::hardware::thermal::sensors()
        .iter()
        .map(|sensor| format!("  {sensor}"))
        .collect();
    said.extend(
        mcf_core::hardware::utilisation::accelerators()
            .iter()
            .map(|card| format!("  {card}")),
    );
    said.extend(crate::support::Gaps::here().one_line());
    said.push(String::new());
    said.join("\n")
}

fn uncharacterized(machine: &Machine) -> String {
    let mut said: Vec<String> = Vec::new();
    for device in &machine.accelerators {
        if let Characterization::AttemptedUncharacterized { missing } = device.characterization() {
            let names: Vec<&str> = missing.iter().map(|m| m.as_str()).collect();
            said.push(format!(
                "\n  ⚠ accelerator #{} is present and not characterized.\n\
                     \x20   MCF will attempt to use it and will mark every result taken on it\n\
                     \x20   as degraded, because it cannot read: {}.\n\
                     \x20   Those readings are what §3.8 needs to tell a slow model from a\n\
                     \x20   busy machine, so results taken here are not comparable with\n\
                     \x20   characterized ones and are not contributable (D25, A5, A8).",
                device.index(),
                names.join(", "),
            ));
        }
    }
    said.join("")
}

impl core::fmt::Display for Report {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        writeln!(f, "{}", self.mcf)?;
        writeln!(f, "recorded at {}", self.at)?;
        writeln!(f, "{}", self.record_line())?;

        writeln!(f, "\nMACHINE")?;
        for line in self.machine.to_string().lines() {
            writeln!(f, "  {line}")?;
        }
        write!(f, "{}", sensor_lines())?;
        write!(f, "{}", uncharacterized(&self.machine))?;

        writeln!(f, "\nWHAT MCF COSTS HERE")?;
        write!(f, "{}", Self::cost_line(&CORE_BINARY, self.cost.artifact))?;
        write!(f, "{}", Self::cost_line(&RESIDENT_IDLE, self.cost.resident))?;
        match &self.cost.cold_start {
            Some(measured) => {
                writeln!(
                    f,
                    "  {:<38} {} — {}",
                    COLD_START.name,
                    COLD_START.statistic(measured),
                    COLD_START.read_measurement(measured, &self.cost.cold_start_attributability),
                )?;
                writeln!(f, "  {:<38} ceiling {}", "", COLD_START.ceiling)?;
                writeln!(f, "  {:<38} {}", "", self.cost.cold_start_attributability)?;
            }
            None => writeln!(f, "  {:<38} {}", COLD_START.name, Verdict::NotMeasured)?,
        }
        match &self.cost.record_write {
            Some(measured) => {
                writeln!(
                    f,
                    "  {:<38} {} — {}",
                    RECORD_WRITE.name,
                    RECORD_WRITE.statistic(measured),
                    RECORD_WRITE
                        .read_measurement(measured, &self.cost.record_write_attributability),
                )?;
                writeln!(f, "  {:<38} this is what MCF's own observation costs", "")?;
                writeln!(f, "  {:<38} {}", "", self.cost.record_write_attributability)?;
            }
            None => writeln!(f, "  {:<38} {}", RECORD_WRITE.name, Verdict::NotMeasured)?,
        }
        writeln!(
            f,
            "\n  Not measured here, and named rather than left out:\n\
             \x20   idle CPU, timer wakeups while idle, memory growth over 30 simulated days,\n\
             \x20   added request-to-first-token latency — all D24 figures about a *running*\n\
             \x20   daemon. One exists (`mcf serve`) and `mcf doctor` does not start it: a\n\
             \x20   report that started a daemon to measure one would be changing the machine\n\
             \x20   it is describing. `scripts/ci.sh --with-soak` and `--with-budget` measure\n\
             \x20   them against a daemon of their own (B-031, B-035)."
        )?;

        writeln!(f, "\n  Taken under: {}", self.cost.conditions)?;

        write!(f, "{}", where_models_go())?;

        writeln!(f, "\nWHAT MCF PROMISES HERE")?;
        for (held, promise) in self.promises() {
            writeln!(f, "  {} {promise}", if held { "✓" } else { "✗" })?;
        }
        Ok(())
    }
}

impl Report {
    #[must_use]
    pub(crate) fn render(&self) -> String {
        self.to_string()
    }

    fn cost_line<Q: mcf_core::measurement::Quantity>(
        budget: &Budget<Q>,
        measured: Attested<Q>,
    ) -> String {
        format!(
            "  {:<38} {} — ceiling {} — {}\n",
            budget.name,
            measured,
            budget.ceiling,
            budget.read(measured),
        )
    }

    fn record_line(&self) -> String {
        match &self.recorded {
            Recorded::Written {
                path,
                id,
                anomaly: None,
            } => {
                format!("record: {} · wrote {id}", path.display())
            }
            Recorded::Written {
                path,
                id,
                anomaly: Some(anomaly),
            } => format!(
                "record: {} · wrote {id}\n\
                 ⚠ THE CLOCK MOVED while this was written: {anomaly}\n\
                 \x20 Anything measured across it is unsound (D9, §3.4). The record is \
                 written\n\x20 and the anomaly is recorded beside it; neither is smoothed away.",
                path.display()
            ),
            Recorded::Declined => "record: not written — asked not to".to_owned(),
            Recorded::Refused(failure) => format!("record: NOT WRITTEN — {failure}"),
        }
    }

    #[must_use]
    pub(crate) fn promises(&self) -> Vec<(bool, String)> {
        let recorded = matches!(self.recorded, Recorded::Written { .. });
        let mut promises = vec![
            (
                true,
                "Every failure is classified against the taxonomy, attributed and \
                 carries its context"
                    .to_owned(),
            ),
            (
                true,
                "Every measurement carries its conditions, its sample count and its spread"
                    .to_owned(),
            ),
            (
                true,
                "Every artifact carries its provenance, or records it as unknown".to_owned(),
            ),
            (
                recorded,
                if recorded {
                    "This report is in the record, and the record is append-only".to_owned()
                } else {
                    "Nothing was written to the record on this run".to_owned()
                },
            ),
        ];

        if self.machine.accelerators.is_empty() {
            promises.push((
                false,
                "No accelerator claim of any kind — there is no accelerator here".to_owned(),
            ));
        } else if self.machine.every_accelerator_is_characterized() {
            promises.push((
                true,
                "Every accelerator present is characterized, so results taken on one \
                 carry the conditions §3.4 needs"
                    .to_owned(),
            ));
        } else {
            promises.push((
                false,
                "At least one accelerator is uncharacterized, so results taken on it \
                 are degraded and not comparable"
                    .to_owned(),
            ));
        }

        promises.push((
            false,
            "Nothing about model quality, speed or fitness — that is work for later milestones"
                .to_owned(),
        ));
        promises
    }

    #[must_use]
    pub(crate) fn to_value(&self) -> Value {
        body(&self.machine, &self.cost)
    }
}

fn where_models_go() -> String {
    let mut lines = vec![String::new(), "WHERE MODELS GO".to_owned()];
    for (position, store) in crate::models::stores().iter().enumerate() {
        let room = match room_for(store) {
            mcf_core::attested::Attested::Known(space) => {
                format!("{} free of {}", space.available, space.total)
            }
            mcf_core::attested::Attested::Unknown => {
                "how much room it has is not something this platform will say".to_owned()
            }
        };
        lines.push(format!(
            "  {}{} — {room}",
            store.display(),
            if position == 0 {
                " (new models go here)"
            } else {
                ""
            }
        ));
    }
    lines.push(format!(
        "  {} says where; unset, models go where the platform keeps a user's data.",
        crate::models::STORES
    ));
    lines.push("  `mcf pull --into <directory>` overrides it for one acquisition.".to_owned());
    for ignored in crate::models::ignored_stores() {
        lines.push(format!(
            "  IGNORED: {} is not an absolute path, and MCF will not resolve a store against \
             whatever directory it was started in (A7)",
            ignored.display()
        ));
    }
    lines.push(String::new());
    lines.join("\n")
}

fn room_for(store: &std::path::Path) -> mcf_core::attested::Attested<mcf_core::hardware::Space> {
    let mut asking = store;
    loop {
        if asking.exists() {
            return mcf_core::hardware::space_on(asking);
        }
        match asking.parent() {
            Some(parent) => asking = parent,
            None => return mcf_core::attested::Attested::Unknown,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Recorded, run};
    use mcf_core::failure::{Attribution, Category, Disposition, Failure, Subsystem};

    #[test]
    fn a_report_is_produced_and_is_complete() {
        let report = run(false);
        let rendered = report.render();
        for section in [
            "MACHINE",
            "WHAT MCF COSTS HERE",
            "WHAT MCF PROMISES HERE",
            "processor:",
            "power profile:",
        ] {
            assert!(rendered.contains(section), "{rendered}");
        }
    }

    #[test]
    fn what_cannot_be_measured_here_is_named() {
        let rendered = run(false).render();
        for absent in [
            "idle CPU",
            "timer wakeups while idle",
            "memory growth over 30 simulated days",
            "added request-to-first-token latency",
        ] {
            assert!(rendered.contains(absent), "{rendered} omits {absent:?}");
        }
        assert!(
            !rendered.contains("no daemon"),
            "the report still says MCF has no daemon: {rendered}"
        );
        assert!(rendered.contains("mcf serve"), "{rendered}");
    }

    #[test]
    fn the_promises_include_the_ones_mcf_cannot_make() {
        let report = run(false);
        let promises = report.promises();
        assert!(
            promises.iter().any(|(held, _)| *held),
            "nothing is promised"
        );
        assert!(
            promises.iter().any(|(held, _)| !*held),
            "every promise is held, which cannot be true at the first milestone"
        );
        assert!(
            promises
                .iter()
                .any(|(_, text)| text.contains("later milestones")),
            "the report claims something about model quality"
        );
    }

    #[test]
    fn declining_to_record_is_stated() {
        let report = run(false);
        assert!(matches!(report.recorded, Recorded::Declined));
        assert!(report.render().contains("record: not written"));
    }

    #[test]
    fn a_record_that_could_not_be_written_is_reported_at_full_volume() {
        let mut report = run(false);
        report.recorded = Recorded::Refused(
            Failure::new(
                Category::RecordUnwritable,
                Attribution::Machine,
                Disposition::Degraded,
                Subsystem::new("mcf-cli::doctor::tests"),
                "the volume is read-only",
            )
            .with_context("path", "/nowhere"),
        );
        let rendered = report.render();
        assert!(rendered.contains("NOT WRITTEN"), "{rendered}");
        assert!(rendered.contains("record.unwritable"), "{rendered}");
        assert!(rendered.contains("MACHINE"), "the report was thrown away");
    }

    #[test]
    fn every_cost_line_carries_its_ceiling() {
        let rendered = run(false).render();
        assert!(rendered.contains("ceiling"), "{rendered}");
        for figure in [
            "core binary, no engines",
            "resident memory, nothing loaded",
            "cold start to first command response",
        ] {
            assert!(rendered.contains(figure), "{rendered} omits {figure:?}");
        }
    }

    #[test]
    fn an_event_class_figure_is_reported_at_the_percentile_d27_names() {
        let report = run(false);
        if report.cost.cold_start.is_some() {
            let rendered = report.render();
            assert!(rendered.contains("p99"), "{rendered}");
            assert!(rendered.contains("median"), "{rendered}");
        }
    }

    #[test]
    fn each_reading_gets_its_own_verdict() {
        let report = run(false);
        let rendered = report.render();
        if report.cost.cold_start.is_some() {
            assert!(
                rendered.contains("attributable") || rendered.contains("UNATTRIBUTABLE"),
                "{rendered}"
            );
        }
        let _ = &report.cost.record_write_attributability;
        let _ = &report.cost.cold_start_attributability;
    }

    #[test]
    fn the_cost_of_observation_is_reported_or_stated_absent() {
        let report = run(false);
        assert!(report.cost.record_write.is_none());
        assert!(report.render().contains("record write, per event"));
    }

    #[test]
    fn the_reports_record_form_round_trips() {
        let value = run(false).to_value();
        let line = value.to_line();
        assert_eq!(mcf_record::json::parse(&line), Ok(value));
    }

    #[test]
    fn unread_conditions_are_null_in_the_record() {
        let value = run(false).to_value();
        let line = value.to_line();
        assert!(line.contains("null"), "nothing was recorded as unknown");
        assert!(
            !line.contains(r#""unknown""#),
            "an unknown was written as the word rather than as null"
        );
    }
}
