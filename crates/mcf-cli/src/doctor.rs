//! `mcf doctor` — the M0 product.
//!
//! It reports what this machine is, what MCF costs on it, and what MCF will
//! and will not promise here, and it writes the whole thing to the record
//! because running it is an event (B4, §3.3).
//!
//! **What it refuses to do is the design.** It does not guess at hardware it
//! cannot read (A7), does not report a budget it did not measure as passing
//! (A7 again — *not measured* is its own verdict), does not claim a promise it
//! has not built, and does not fail because the machine is unusual: a machine
//! with no accelerator, no `/proc`, no writable record path and no network is a
//! valid subject, and the report says exactly that. B19 requires the whole
//! thing work on a laptop, offline.
//!
//! **Exit status is zero even when the report is bleak.** `doctor` reports; it
//! does not fail because the machine did. A non-zero status is reserved for
//! `doctor` itself being unable to report.

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

/// What the laboratory demonstrated on this machine, now.
///
/// §VIII puts MCF's confidence in the laboratory rather than in ambient
/// observation, and A13 says an untested claim is not made. `doctor` therefore
/// *runs* the catalogue rather than reporting that one exists: the difference
/// between "there is a laboratory" and "every failure MCF claims to handle was
/// reproduced on this machine a moment ago" is the whole of §VIII.
#[derive(Debug)]
pub(crate) struct Laboratory {
    /// How many scenarios ran.
    pub(crate) scenarios: usize,
    /// How many produced the category they declare.
    pub(crate) reproduced: usize,
    /// The ones that did not, and what happened instead.
    pub(crate) divergences: Vec<String>,
    /// How many distinct taxonomy categories those scenarios reproduce.
    pub(crate) categories: usize,
}

impl Laboratory {
    fn run() -> Self {
        let mut reproduced = 0;
        let mut divergences = Vec::new();
        for scenario in mcf_lab::CATALOGUE {
            let outcome = mcf_lab::run(scenario);
            if outcome.matches(scenario.produces) {
                reproduced += 1;
            } else {
                divergences.push(format!("{} → {outcome}", scenario.id));
            }
        }
        let mut codes: Vec<&str> = mcf_lab::CATALOGUE
            .iter()
            .map(|scenario| scenario.produces.code())
            .collect();
        codes.sort_unstable();
        codes.dedup();

        Self {
            scenarios: mcf_lab::CATALOGUE.len(),
            reproduced,
            divergences,
            categories: codes.len(),
        }
    }
}

/// Everything `doctor` found.
#[derive(Debug)]
pub(crate) struct Report {
    /// What built the binary that produced this report.
    pub(crate) mcf: BuildIdentity,
    /// When it was produced.
    pub(crate) at: Timestamp,
    /// What the machine is.
    pub(crate) machine: Machine,
    /// What MCF costs here.
    pub(crate) cost: Cost,
    /// What happened when the report was written to the record.
    pub(crate) recorded: Recorded,
    /// What the laboratory demonstrated here.
    pub(crate) laboratory: Laboratory,
}

/// What MCF costs on this machine, against D24.
#[derive(Debug)]
pub(crate) struct Cost {
    /// What one recorded event costs — MCF's own observation (B-012, §3.8).
    ///
    /// `None` when it could not be measured, which is a state and not a zero:
    /// a machine with nowhere to write a record has an unmeasured observation
    /// cost, not a free one (A7).
    pub(crate) record_write: Option<Measurement<Duration<Monotonic>>>,
    /// Whether the cold-start reading was about MCF (D30).
    pub(crate) cold_start_attributability: Attributability,
    /// Whether the record-write reading was.
    ///
    /// Separate verdicts, because D30 makes attributability a property of a
    /// *reading* rather than of the machine: two measurements taken seconds
    /// apart can differ, and reporting one verdict for both would be reporting
    /// the machine again.
    pub(crate) record_write_attributability: Attributability,
    /// The binary's size on disk.
    pub(crate) artifact: Attested<Bytes>,
    /// This process's resident set.
    pub(crate) resident: Attested<Bytes>,
    /// Cold start to first command response, or `None` if it could not be
    /// measured.
    pub(crate) cold_start: Option<Measurement<Duration<Monotonic>>>,
    /// The conditions all of the above were taken under.
    pub(crate) conditions: Conditions,
}

/// What became of the record write.
#[derive(Debug)]
pub(crate) enum Recorded {
    /// Written, at this path, under this identifier.
    Written {
        /// Where.
        path: std::path::PathBuf,
        /// The entry's identifier.
        id: String,
        /// A clock anomaly noticed while writing it (D9, B37).
        ///
        /// The record is still written — A1 forbids losing the event — and
        /// what an anomaly invalidates is anything that was being measured
        /// across it, which the report says at full volume rather than in a
        /// footnote.
        anomaly: Option<Failure>,
    },
    /// Not written, and why. §3.2: MCF degrades and says so — a report that
    /// could not be recorded is still a report, and pretending otherwise would
    /// throw away the reading to protect the filing.
    Refused(Failure),
    /// Not attempted, because the operator asked for it not to be.
    Declined,
}

/// Runs the whole thing.
///
/// `record` is false when the operator asked for a report without a write.
/// That is a real need — inspecting a machine without touching its record —
/// and it is not a gated category, so B1 lets it flow.
#[must_use]
pub(crate) fn run(record: bool) -> Report {
    let mcf = BuildIdentity::current();
    let at = Timestamp::now();
    let machine = Machine::read();
    let cost = measure_cost(&machine, record);
    let laboratory = Laboratory::run();
    let body = body(&machine, &cost, &laboratory);
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
        laboratory,
    }
}

fn measure_cost(machine: &Machine, recording: bool) -> Cost {
    // The conditions these figures were taken under, captured from the live
    // machine rather than assembled here (B-007). What the machine does not
    // report stays unknown; what nothing runs a model to supply — quantization,
    // context length, batch shape, realized placement — stays unknown too, and
    // says so (A7).
    // B3: the instrumentation profile is part of what a figure was taken
    // under. `--no-record` is the reduced arm and the default is the full one,
    // and the delta between them is what B-012 asks MCF to report about itself.
    let profile = if recording {
        "full — the record is being written"
    } else {
        "reduced — nothing is being recorded"
    };
    // The artifact under measurement at M0 is MCF itself, so the storage that
    // matters is the one its own binary was read from (B-193, F5). `None` when
    // the platform will not say which file is running: unknown rather than a
    // guess (A7).
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
    // The subject is `mcf --version`, the shortest complete command MCF has.
    // Measuring `doctor` itself would measure a process that profiles hardware
    // and writes a record, which is not what D24's figure is about — and, as
    // F1 records, an instrument that measures itself measuring itself does not
    // terminate.
    // D27 reads an event-class figure at the 99th percentile over at least a
    // hundred trials, because a p99 of twenty is the maximum wearing a
    // percentile's name.
    // D30: the verdict brackets the measurement, because the question is
    // whether *this reading* was affected rather than whether the machine is
    // busy.
    let watch = Watch::start();
    let cold_start = binary.as_deref().and_then(|path| {
        self_cost::cold_start(path, &["--version"], EVENT_TRIALS, conditions.clone())
    });
    let cold_start_attributability = watch.finish();

    // What MCF's own observation costs, measured beside the real record so it
    // sees the same filesystem. Not measured when nothing is being recorded:
    // the reduced arm's observation cost is zero by construction, and
    // measuring it would be measuring the probe.
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

fn body(machine: &Machine, cost: &Cost, laboratory: &Laboratory) -> Value {
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
                // What is absent is named, rather than being absent silently.
                // A7 governs values MCF could not read; this is the same
                // instinct one level up, for quantities MCF cannot yet measure
                // at all.
                (
                    "not_measurable_here",
                    Value::List(vec![
                        Value::text("idle CPU — needs a daemon (B-030, B-031)"),
                        Value::text("timer wakeups while idle — needs a daemon (B-031)"),
                        Value::text("memory growth over 30 simulated days — needs a daemon"),
                        Value::text(
                            "added request-to-first-token latency — needs a serving path (B-035)",
                        ),
                    ]),
                ),
            ]),
        ),
        (
            "laboratory",
            Value::map([
                (
                    "scenarios",
                    Value::Integer(i64::try_from(laboratory.scenarios).unwrap_or(i64::MAX)),
                ),
                (
                    "reproduced",
                    Value::Integer(i64::try_from(laboratory.reproduced).unwrap_or(i64::MAX)),
                ),
                (
                    "categories",
                    Value::Integer(i64::try_from(laboratory.categories).unwrap_or(i64::MAX)),
                ),
                (
                    "divergences",
                    Value::List(
                        laboratory
                            .divergences
                            .iter()
                            .map(|what| Value::text(what.clone()))
                            .collect(),
                    ),
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
        // D9: a clock anomaly noticed while writing is an *event*, not a
        // correction. The entry was written and the anomaly was written beside
        // it, and the report says so rather than only that the record was
        // written.
        // The identifier comes back from the write rather than off the entry:
        // a writer mints it, so an entry nobody has appended has none
        // (DEC-037).
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

impl core::fmt::Display for Report {
    /// The report, rendered for a terminal.
    ///
    /// C3: a minimalist surface shows less decoration, not less information.
    /// Every figure here carries what it is read against, every condition that
    /// could not be read says so, and every promise MCF cannot make on this
    /// machine is listed beside the ones it can.
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        writeln!(f, "{}", self.mcf)?;
        writeln!(f, "recorded at {}", self.at)?;
        writeln!(f, "{}", self.record_line())?;

        writeln!(f, "\nMACHINE")?;
        for line in self.machine.to_string().lines() {
            writeln!(f, "  {line}")?;
        }
        for device in &self.machine.accelerators {
            if let Characterization::AttemptedUncharacterized { missing } =
                device.characterization()
            {
                let names: Vec<&str> = missing.iter().map(|m| m.as_str()).collect();
                writeln!(
                    f,
                    "\n  ⚠ accelerator #{} is present and not characterized.\n\
                     \x20   MCF will attempt to use it and will mark every result taken on it\n\
                     \x20   as degraded, because it cannot read: {}.\n\
                     \x20   Those readings are what §3.8 needs to tell a slow model from a\n\
                     \x20   busy machine, so results taken here are not comparable with\n\
                     \x20   characterized ones and are not contributable (D25, A5, A8).",
                    device.index(),
                    names.join(", "),
                )?;
            }
        }

        writeln!(f, "\nWHAT MCF COSTS HERE")?;
        write!(f, "{}", Self::cost_line(&CORE_BINARY, self.cost.artifact))?;
        write!(f, "{}", Self::cost_line(&RESIDENT_IDLE, self.cost.resident))?;
        match &self.cost.cold_start {
            Some(measured) => {
                // D27: the ceiling is about the 99th percentile, and the
                // median is shown beside it because the gap between them is
                // what a busy machine looks like.
                writeln!(
                    f,
                    "  {:<38} p99 {} over n={} — {}",
                    COLD_START.name,
                    COLD_START.statistic(measured),
                    measured.n(),
                    COLD_START.read_measurement(measured, &self.cost.cold_start_attributability),
                )?;
                writeln!(
                    f,
                    "  {:<38} median {} · ceiling {}",
                    "",
                    measured.spread().median,
                    COLD_START.ceiling,
                )?;
                writeln!(f, "  {:<38} {}", "", self.cost.cold_start_attributability)?;
            }
            None => writeln!(f, "  {:<38} {}", COLD_START.name, Verdict::NotMeasured)?,
        }
        match &self.cost.record_write {
            Some(measured) => {
                writeln!(
                    f,
                    "  {:<38} p99 {} over n={} — {}",
                    RECORD_WRITE.name,
                    RECORD_WRITE.statistic(measured),
                    measured.n(),
                    RECORD_WRITE
                        .read_measurement(measured, &self.cost.record_write_attributability),
                )?;
                writeln!(
                    f,
                    "  {:<38} this is what MCF's own observation costs (§3.8, B3)",
                    "",
                )?;
                writeln!(f, "  {:<38} {}", "", self.cost.record_write_attributability)?;
            }
            None => writeln!(f, "  {:<38} {}", RECORD_WRITE.name, Verdict::NotMeasured)?,
        }
        writeln!(
            f,
            "\n  Not measurable here, and named rather than left out:\n\
             \x20   idle CPU, timer wakeups while idle, memory growth over 30 simulated days,\n\
             \x20   added request-to-first-token latency — all D24 figures about a daemon,\n\
             \x20   and there is no daemon until M2 (B-030, B-031, B-035)."
        )?;

        // A6: the conditions travel with the figures, on the surface and not
        // only in the record.
        writeln!(f, "\n  Taken under: {}", self.cost.conditions)?;

        writeln!(
            f,
            "\nTHE LABORATORY, RUN HERE JUST NOW\n  {} scenarios · {} taxonomy categories · {} reproduced",
            self.laboratory.scenarios, self.laboratory.categories, self.laboratory.reproduced,
        )?;
        for divergence in &self.laboratory.divergences {
            writeln!(f, "  ⚠ {divergence}")?;
        }
        writeln!(
            f,
            "  Every category MCF's own code can produce has a scenario here, and no\n\
             \x20 more: the taxonomy's remaining codes are classifications waiting for the\n\
             \x20 code that will use them (A13, D26)."
        )?;

        writeln!(f, "\nWHAT MCF PROMISES HERE")?;
        for (held, promise) in self.promises() {
            writeln!(f, "  {} {promise}", if held { "✓" } else { "✗" })?;
        }
        Ok(())
    }
}

impl Report {
    /// The report, rendered for a terminal.
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

    /// What MCF will and will not promise on this machine.
    ///
    /// Each entry is a claim MCF can either make here or cannot, and the ones
    /// it cannot are listed rather than omitted — a report that showed only the
    /// ticks would be a report that read as complete.
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
            "Nothing about model quality, speed or fitness — that is M5 onward".to_owned(),
        ));
        let laboratory = &self.laboratory;
        promises.push((
            laboratory.divergences.is_empty() && laboratory.scenarios > 0,
            if laboratory.divergences.is_empty() && laboratory.scenarios > 0 {
                format!(
                    "Every failure MCF claims to handle was reproduced on this machine \
                     just now — {} scenarios, {} categories (A13, §VIII)",
                    laboratory.scenarios, laboratory.categories,
                )
            } else {
                format!(
                    "The laboratory did not reproduce what it claims here: {:?}",
                    laboratory.divergences
                )
            },
        ));
        promises
    }

    /// The report as the record holds it.
    #[must_use]
    pub(crate) fn to_value(&self) -> Value {
        body(&self.machine, &self.cost, &self.laboratory)
    }
}

#[cfg(test)]
mod tests {
    use super::{Recorded, run};
    use mcf_core::failure::{Attribution, Category, Disposition, Failure, Subsystem};

    /// B-014's condition, and B19's: the report is produced on whatever machine
    /// the suite is running on — with or without an accelerator, offline, with
    /// no model present — and it is complete.
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

    /// The report names what it cannot measure rather than leaving it out. A
    /// list of five figures that shows three and stops reads as five.
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
    }

    /// The laboratory runs as part of the report, and what it demonstrated is
    /// on the surface. §VIII puts confidence there rather than in ambient
    /// observation, and the difference between "there is a laboratory" and
    /// "every failure MCF claims was reproduced here a moment ago" is the whole
    /// of it.
    #[test]
    fn the_laboratory_runs_and_reports_what_it_demonstrated() {
        let report = run(false);
        assert!(report.laboratory.scenarios > 0, "no scenario ran");
        assert_eq!(
            report.laboratory.reproduced, report.laboratory.scenarios,
            "the laboratory did not reproduce what it claims: {:?}",
            report.laboratory.divergences
        );
        let rendered = report.render();
        assert!(
            rendered.contains("THE LABORATORY, RUN HERE JUST NOW"),
            "{rendered}"
        );
        assert!(rendered.contains("taxonomy categories"), "{rendered}");
    }

    /// The promises MCF cannot make are listed alongside the ones it can. A
    /// report showing only the ticks would read as complete.
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
            "every promise is held, which cannot be true at M0"
        );
        assert!(
            promises.iter().any(|(_, text)| text.contains("M5 onward")),
            "the report claims something about model quality"
        );
    }

    /// `--no-record` writes nothing, and the report says so rather than
    /// leaving a reader to assume either way.
    #[test]
    fn declining_to_record_is_stated() {
        let report = run(false);
        assert!(matches!(report.recorded, Recorded::Declined));
        assert!(report.render().contains("record: not written"));
    }

    /// §3.2: a record MCF could not write is a degradation, not a reason to
    /// throw the reading away. The report is still produced and says at full
    /// volume what happened.
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

    /// A6: every figure the report shows carries what it is read against, and
    /// a figure with no reading says *not measured* rather than nothing.
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

    /// D27: an event-class figure is read at the 99th percentile, with the
    /// median beside it — the gap between the two is what a busy machine looks
    /// like, and hiding it would hide the reason the reading may not be usable.
    #[test]
    fn an_event_class_figure_is_reported_at_the_percentile_d27_names() {
        let report = run(false);
        if report.cost.cold_start.is_some() {
            let rendered = report.render();
            assert!(rendered.contains("p99"), "{rendered}");
            assert!(rendered.contains("median"), "{rendered}");
        }
    }

    /// D30: the verdict is about the *reading*, and each reading gets its own.
    /// A single verdict for the whole report would be reporting the machine
    /// again, which F3 established the load average already does badly.
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
        // The two are answered separately even when they agree.
        let _ = &report.cost.record_write_attributability;
        let _ = &report.cost.cold_start_attributability;
    }

    /// §3.8 and B-012: what MCF's own observation costs is measured and
    /// reported, and where it could not be measured that is a state rather
    /// than a zero (A7).
    #[test]
    fn the_cost_of_observation_is_reported_or_stated_absent() {
        let report = run(false);
        // With nothing being recorded there is no observation to cost, and the
        // report says so rather than showing a zero.
        assert!(report.cost.record_write.is_none());
        assert!(report.render().contains("record write, per event"));
    }

    /// The record's own form of the report is valid, self-describing JSON —
    /// A22's headless path is complete only if something other than a person
    /// can read it.
    #[test]
    fn the_reports_record_form_round_trips() {
        let value = run(false).to_value();
        let line = value.to_line();
        assert_eq!(mcf_record::json::parse(&line), Ok(value));
    }

    /// A7: an unread condition is `null` in the record, never a substitute.
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
