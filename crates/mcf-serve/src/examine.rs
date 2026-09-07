//! Every part of a model measured by a count or a clock, never by a
//! rater: the fourteen measurements of D52, as one run the daemon carries
//! (`mcf examine`, the window's Performance, Fidelity and Behaviour cards).
//!
//! **What this is beside the probes.** A probe asks whether the model
//! does a thing — emits a call, stops, produces a shape — and configures
//! nothing from the answer (D42). A measurement here asks *how much*: how
//! many positions agree with a reference file, how many tokens a second
//! at each layer count, how many bytes resident against how many were
//! predicted. Both are experiments, both write to the record under their
//! method, and both are three-valued: measured, not, or could not tell
//! (A7). They share the record writer and the stream shape for that
//! reason, and they differ in what they start: a probe goes through the
//! daemon's generation path as a generation does, while a measurement
//! starts the engine itself, under the settings it varies (D52).
//!
//! **Each measurement is its own module** with one entry point, so that
//! a check reading this directory can find every one of them and the
//! list below cannot fall out of step with what runs.

use std::path::{Path, PathBuf};

use mcf_record::json::Value;

pub use mcf_record::readings::Reading;

use crate::adapters::ProvisionedLlama;
use crate::probes::run::Step;
use crate::served::{Served, Startup, Waiting};

pub mod agent;

pub mod absent;
pub mod arithmetic;
pub mod bits;
pub mod cache;
pub mod codereading;
pub mod cold;
pub mod concurrency;
pub mod deep;
pub mod degeneration;
pub mod determinism;
pub mod drafthead;
pub mod energy;
pub mod extraction;
pub mod fidelity;
pub mod fidelitydepth;
pub mod grammar;
pub mod image;
pub mod injection;
pub mod instructions;
pub mod jitter;
pub mod listing;
pub mod loadtime;
pub mod memory;
pub mod multifact;
pub mod multilingual;
pub mod nearest;
pub mod obedience;
pub mod offload;
pub mod paraphrase;
pub mod prefill;
pub mod prefix;
pub mod recall;
pub mod reckoning;
pub mod repository;
pub mod retrieval;
pub mod schemas;
pub mod seeing;
pub mod soak;
pub mod stoplatency;
pub mod sustained;
pub mod temperature;
pub mod thinkingcost;
pub mod threads;
pub mod tokenizer;
pub mod tooluse;
pub mod vocabulary;

/// Every measurement the run can make, in the order it makes them.
pub const MEASURES: [&str; 47] = [
    offload::NAME,
    prefill::NAME,
    prefix::NAME,
    memory::NAME,
    concurrency::NAME,
    cold::NAME,
    fidelity::NAME,
    bits::NAME,
    determinism::NAME,
    tokenizer::NAME,
    retrieval::NAME,
    degeneration::NAME,
    grammar::NAME,
    image::NAME,
    tooluse::NAME,
    agent::NAME,
    extraction::NAME,
    instructions::NAME,
    paraphrase::NAME,
    multilingual::NAME,
    multifact::NAME,
    sustained::NAME,
    energy::NAME,
    jitter::NAME,
    drafthead::NAME,
    cache::NAME,
    soak::NAME,
    stoplatency::NAME,
    fidelitydepth::NAME,
    repository::NAME,
    schemas::NAME,
    temperature::NAME,
    vocabulary::NAME,
    seeing::NAME,
    arithmetic::NAME,
    reckoning::NAME,
    listing::NAME,
    recall::NAME,
    obedience::NAME,
    absent::NAME,
    codereading::NAME,
    nearest::NAME,
    injection::NAME,
    thinkingcost::NAME,
    loadtime::NAME,
    threads::NAME,
    deep::NAME,
];

/// The three families the window shows as cards, each with its
/// measurements in the order the run makes them (D52).
pub const FAMILIES: [(&str, &[&str]); 3] = [
    (
        "Performance",
        &[
            offload::NAME,
            prefill::NAME,
            prefix::NAME,
            memory::NAME,
            concurrency::NAME,
            cold::NAME,
        ],
    ),
    (
        "Fidelity",
        &[
            fidelity::NAME,
            bits::NAME,
            determinism::NAME,
            tokenizer::NAME,
        ],
    ),
    (
        "Behaviour",
        &[
            retrieval::NAME,
            degeneration::NAME,
            grammar::NAME,
            image::NAME,
            tooluse::NAME,
            agent::NAME,
            extraction::NAME,
            instructions::NAME,
            paraphrase::NAME,
            multilingual::NAME,
            multifact::NAME,
            sustained::NAME,
            energy::NAME,
            jitter::NAME,
            drafthead::NAME,
            cache::NAME,
            soak::NAME,
            stoplatency::NAME,
            fidelitydepth::NAME,
            repository::NAME,
            schemas::NAME,
            temperature::NAME,
            vocabulary::NAME,
            seeing::NAME,
            arithmetic::NAME,
            reckoning::NAME,
            listing::NAME,
            recall::NAME,
            obedience::NAME,
            absent::NAME,
            codereading::NAME,
            nearest::NAME,
            injection::NAME,
            thinkingcost::NAME,
            loadtime::NAME,
            threads::NAME,
            deep::NAME,
        ],
    ),
];

/// The window a plain load opens, in tokens: room for every measurement
/// that does not ask for more.
pub const PLAIN_WINDOW: u64 = 4096;

/// Which measurements a run makes, in order: every one where none is
/// named, else the ones named. A name that is none of them is ignored,
/// so the caller that reads a wire says what it could not read.
#[must_use]
pub fn planned(only: &[String]) -> Vec<&'static str> {
    if only.is_empty() {
        return MEASURES.to_vec();
    }
    MEASURES
        .into_iter()
        .filter(|name| only.iter().any(|asked| asked == name))
        .collect()
}

/// Where a run happens: the engine, the model, and the conditions every
/// measurement starts from unless it varies one.
pub struct Site<'a> {
    /// The provisioned engine the model resolves to.
    pub llama: &'a ProvisionedLlama,
    /// The model file.
    pub model: &'a Path,
    /// Where a server's socket goes.
    pub runtime: &'a Path,
    /// How many layers MCF resolved to the card, which is all or none.
    pub gpu_layers: u32,
    /// The window MCF resolved for this model here.
    pub context: u64,
    /// The projector beside the model, where there is one.
    pub projector: Option<PathBuf>,
    /// Whether the engine computes on a card here.
    pub has_card: bool,
    /// The engine, as the record names it.
    pub engine: String,
    /// Whether the asker has gone, asked between requests: a run cut
    /// short spends nothing further on nobody (B-468).
    pub gone: &'a (dyn Fn() -> bool + Sync),
    /// Where a measurement says how far along it is within its step —
    /// which picture, which sum, which request of two hundred — so that
    /// the person waiting sees the work and not only its name (D56).
    pub tell: &'a (dyn Fn(&Progress) + Sync),
    /// Who is waiting, so that a long request in flight is closed when
    /// they leave (D48). **A timed request does not wait this way**: a
    /// watched request is noticed done only when the watcher's glance
    /// comes round, a quarter of a second at a time, and a timing taken
    /// through it read 512.0 ms for whatever took between a quarter and
    /// a half (F196). The short requests a timing is taken over are
    /// waited for as they are, with the asker looked at between them.
    pub waiting: Waiting<'a>,
}

impl std::fmt::Debug for Site<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Site")
            .field("model", &self.model)
            .field("gpu_layers", &self.gpu_layers)
            .field("context", &self.context)
            .field("engine", &self.engine)
            .finish_non_exhaustive()
    }
}

/// How far along a measurement is within its step: `done` of `of`, and
/// what is being done now, in a few words (D56).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Progress {
    /// How many of the step's parts are done.
    pub done: usize,
    /// How many parts the step has.
    pub of: usize,
    /// What is being done now: the picture's name, the sum, the request.
    pub what: String,
}

impl Progress {
    /// The record's shape, and the wire's.
    #[must_use]
    pub fn to_value(&self) -> Value {
        Value::map([
            ("done", Value::Integer(as_integer(self.done))),
            ("of", Value::Integer(as_integer(self.of))),
            ("doing", Value::text(self.what.clone())),
        ])
    }
}

impl Site<'_> {
    /// Says how far along the measurement is: `done` of `of` parts, and
    /// what is being done now (D56).
    pub fn progress(&self, done: usize, of: usize, what: &str) {
        (self.tell)(&Progress {
            done,
            of,
            what: what.to_owned(),
        });
    }

    /// The plain load, as MCF resolved it for this model — under a window
    /// a measurement needs rather than the window the model could hold.
    ///
    /// **The resolved window is the largest this machine holds**, which
    /// for a small model is its whole trained context; a cache for a
    /// quarter of a million tokens was being allocated for a request of
    /// sixty (F196). A measurement that needs more opens more:
    /// retrieval its deepest depth, the memory measurement each window
    /// it measures.
    #[must_use]
    pub fn startup(&self) -> Startup {
        Startup {
            gpu_layers: self.gpu_layers,
            context: self.context.min(PLAIN_WINDOW),
            projector: self.projector.clone(),
            ..Startup::default()
        }
    }

    /// A server on this model under a startup, or why there is none.
    ///
    /// # Errors
    ///
    /// What the engine said as it refused to start, in a sentence.
    pub fn server(&self, startup: &Startup) -> Result<Served, String> {
        self.server_for(self.model, startup)
    }

    /// A server on another file of the same repository — a reference to
    /// measure against — under a startup.
    ///
    /// # Errors
    ///
    /// As [`Self::server`].
    pub fn server_for(&self, model: &Path, startup: &Startup) -> Result<Served, String> {
        if (self.gone)() {
            return Err(crate::served::CLIENT_LEFT.to_owned());
        }
        Served::start_as(self.llama, model, self.runtime, startup).map_err(|failure| {
            let why = failure.detail().to_owned();
            failure
                .context_value("last_words")
                .map_or(why.clone(), |words| format!("{why}: {}", words.trim()))
        })
    }

    /// Whether the asker has gone.
    #[must_use]
    pub fn asker_gone(&self) -> bool {
        (self.gone)()
    }

    /// How a timed request waits: for the engine alone, so that the
    /// clock reads the engine and not the watcher's glance (F196).
    #[must_use]
    pub const fn timed(&self) -> Waiting<'static> {
        Waiting::NOBODY
    }
}

/// What one measurement found: the lines a person reads, and the fields
/// the record keeps. A measurement that could not tell says so in both.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Found {
    /// The finding, a line each, as every surface prints it.
    pub lines: Vec<String>,
    /// What the record keeps, beside the model, method and engine.
    pub fields: Vec<(&'static str, Value)>,
    /// Every figure read, raw, one row each: the repeat, the position, the
    /// trial, the placement it was read under (D54, B-512).
    pub rows: Vec<Reading>,
}

impl Found {
    /// A measurement that could not be taken, and why (A7).
    #[must_use]
    pub fn could_not_tell(why: &str) -> Self {
        Self {
            lines: vec![format!("  COULD NOT TELL   {why}")],
            fields: vec![("could_not_tell", Value::text(why.to_owned()))],
            rows: Vec::new(),
        }
    }

    /// Whether the measurement was taken.
    #[must_use]
    pub fn measured(&self) -> bool {
        !self.fields.iter().any(|(key, _)| *key == "could_not_tell")
    }
}

/// Runs the measurements named on one model, saying which is about to
/// run and what each found as it lands — the same shape a probe run has
/// (B-478, D50).
///
/// `say` is told each step twice — before the measurement, with no
/// lines, and after, with what it found — and answers whether to go on.
/// Every finding is written to the record before it is said, whichever
/// way it came out (A1, A9).
///
/// # Errors
///
/// The file could not be read as a model, in a sentence.
#[allow(
    clippy::too_many_lines,
    reason = "one arm a measurement, so that the list of what runs is read in one place"
)]
pub fn run(
    site: &Site<'_>,
    only: &[String],
    say: &mut dyn FnMut(&Step, &[String]) -> bool,
) -> Result<Vec<String>, String> {
    if crate::probes::run::read_prefix(site.model).is_none() {
        return Err(format!(
            "{} could not be read as a model: MCF grew its read to the whole file and still \
             could not find a GGUF directory in it",
            site.model.display()
        ));
    }
    let plan = planned(only);
    let of = plan.len();
    for (count, name) in plan.into_iter().enumerate() {
        let step = Step {
            name,
            count: count.saturating_add(1),
            of,
        };
        if !say(&step, &[]) {
            break;
        }
        let found = match name {
            offload::NAME => offload::measure(site),
            prefill::NAME => prefill::measure(site),
            prefix::NAME => prefix::measure(site),
            memory::NAME => memory::measure(site),
            concurrency::NAME => concurrency::measure(site),
            cold::NAME => cold::measure(site),
            fidelity::NAME => fidelity::measure(site),
            bits::NAME => bits::measure(site),
            determinism::NAME => determinism::measure(site),
            tokenizer::NAME => tokenizer::measure(site),
            retrieval::NAME => retrieval::measure(site),
            degeneration::NAME => degeneration::measure(site),
            grammar::NAME => grammar::measure(site),
            image::NAME => image::measure(site),
            tooluse::NAME => tooluse::measure(site),
            agent::NAME => agent::measure(site),
            extraction::NAME => extraction::measure(site),
            instructions::NAME => instructions::measure(site),
            paraphrase::NAME => paraphrase::measure(site),
            multilingual::NAME => multilingual::measure(site),
            multifact::NAME => multifact::measure(site),
            sustained::NAME => sustained::measure(site),
            energy::NAME => energy::measure(site),
            jitter::NAME => jitter::measure(site),
            drafthead::NAME => drafthead::measure(site),
            cache::NAME => cache::measure(site),
            soak::NAME => soak::measure(site),
            stoplatency::NAME => stoplatency::measure(site),
            fidelitydepth::NAME => fidelitydepth::measure(site),
            repository::NAME => repository::measure(site),
            schemas::NAME => schemas::measure(site),
            temperature::NAME => temperature::measure(site),
            vocabulary::NAME => vocabulary::measure(site),
            seeing::NAME => seeing::measure(site),
            arithmetic::NAME => arithmetic::measure(site),
            reckoning::NAME => reckoning::measure(site),
            listing::NAME => listing::measure(site),
            recall::NAME => recall::measure(site),
            obedience::NAME => obedience::measure(site),
            absent::NAME => absent::measure(site),
            codereading::NAME => codereading::measure(site),
            nearest::NAME => nearest::measure(site),
            injection::NAME => injection::measure(site),
            thinkingcost::NAME => thinkingcost::measure(site),
            loadtime::NAME => loadtime::measure(site),
            threads::NAME => threads::measure(site),
            deep::NAME => deep::measure(site),
            _ => Found::could_not_tell("MCF has no measurement of this name"),
        };
        let mut lines = vec![name.to_owned()];
        lines.extend(found.lines.iter().cloned());
        let written =
            crate::probes::run::record_probed(site.model, name, &site.engine, found.fields.clone());
        lines.push(match written {
            Ok(path) => format!("  recorded in {}", path.display()),
            Err(why) => format!("  NOT RECORDED: {why}"),
        });
        // The rows beside the finding: every figure raw, under the
        // conditions the run shared (D54). The counts above are what the
        // sentences read; the rows are what a person compares by.
        if !found.rows.is_empty() {
            let conditions: Vec<(&str, Value)> = found
                .fields
                .iter()
                .filter(|(_, held)| !matches!(held, Value::List(_) | Value::Map(_)))
                .map(|(key, held)| (*key, held.clone()))
                .collect();
            // Every run's conditions carry what it ran under, whatever its
            // own fields say: the engine, the window a plain load opens, the
            // layers on the card (D56).
            let load = site.startup();
            let mut conditions = conditions;
            for (key, stated) in [
                ("engine", Value::text(site.engine.clone())),
                (
                    "window",
                    Value::Integer(i64::try_from(load.context).unwrap_or(i64::MAX)),
                ),
                ("gpu_layers", Value::Integer(i64::from(load.gpu_layers))),
            ] {
                if !conditions.iter().any(|(named, _)| *named == key) {
                    conditions.push((key, stated));
                }
            }
            let written = crate::probes::run::record_readings(
                site.model,
                name,
                &site.engine,
                conditions,
                &found.rows,
            );
            lines.push(match written {
                Ok(_) => format!("  {} reading(s) recorded", found.rows.len()),
                Err(why) => format!("  READINGS NOT RECORDED: {why}"),
            });
        }
        if !say(&step, &lines) {
            break;
        }
    }
    Ok(vec![
        "  Every figure above is a count or a clock under the conditions named with it; \
         none grades what the model said (D52)."
            .to_owned(),
    ])
}

/// Writes a run's readings from outside the daemon — the coding
/// laboratory runs at the command line, with its container — under the
/// one schema every diagnostic shares (D54, B-518).
///
/// # Errors
///
/// Nowhere to record, or the record could not be written.
pub fn record_rows(
    model: &Path,
    method: &str,
    engine: &str,
    conditions: Vec<(&str, Value)>,
    rows: &[Reading],
) -> Result<PathBuf, mcf_core::Failure> {
    crate::probes::run::record_readings(model, method, engine, conditions, rows)
}

/// A run recorded a part at a time: opened with its conditions before
/// the first row is taken, each unit's rows landed as they are, and
/// closed with how it ended. A run killed between lands keeps every row
/// landed and reads back with no end (B-570).
#[derive(Debug)]
pub struct Landing {
    model: PathBuf,
    method: String,
    engine: String,
    run: String,
    part: u64,
    rows: usize,
}

impl Landing {
    /// Opens a run: writes its conditions as its first part, with no rows.
    ///
    /// # Errors
    ///
    /// Nowhere to record, or the record could not be written.
    pub fn open(
        model: &Path,
        method: &str,
        conditions: Vec<(&str, Value)>,
    ) -> Result<Self, mcf_core::Failure> {
        let run = format!(
            "{method}-{}-{}",
            mcf_core::time::Timestamp::now(),
            std::process::id()
        );
        let _wrote =
            crate::probes::run::record_part(model, method, "", conditions, &[], &run, 0, None)?;
        Ok(Self {
            model: model.to_path_buf(),
            method: method.to_owned(),
            engine: String::new(),
            run,
            part: 0,
            rows: 0,
        })
    }

    /// Lands one unit's rows, naming the engine where it is known by now.
    ///
    /// # Errors
    ///
    /// The record could not be written.
    pub fn land(
        &mut self,
        engine: Option<&str>,
        rows: &[Reading],
    ) -> Result<(), mcf_core::Failure> {
        if let Some(engine) = engine {
            engine.clone_into(&mut self.engine);
        }
        if rows.is_empty() {
            return Ok(());
        }
        self.part = self.part.saturating_add(1);
        let _wrote = crate::probes::run::record_part(
            &self.model,
            &self.method,
            &self.engine,
            Vec::new(),
            rows,
            &self.run,
            self.part,
            None,
        )?;
        self.rows = self.rows.saturating_add(rows.len());
        Ok(())
    }

    /// Closes the run with how it ended: `finished`, or `stopped after …`.
    ///
    /// # Errors
    ///
    /// The record could not be written.
    pub fn close(&mut self, ended: &str) -> Result<usize, mcf_core::Failure> {
        self.part = self.part.saturating_add(1);
        let _wrote = crate::probes::run::record_part(
            &self.model,
            &self.method,
            &self.engine,
            Vec::new(),
            &[],
            &self.run,
            self.part,
            Some(ended),
        )?;
        Ok(self.rows)
    }

    /// How many rows have landed so far.
    #[must_use]
    pub const fn landed(&self) -> usize {
        self.rows
    }

    /// The method the run is recorded under.
    #[must_use]
    pub fn method(&self) -> &str {
        &self.method
    }
}

/// One measurement's recorded finding as a sentence, from the fields the
/// record keeps of it, read back the way a probe's is (B-483).
#[must_use]
#[allow(
    clippy::too_many_lines,
    reason = "one arm a measurement, each a sentence read off the record's fields"
)]
pub fn recorded_said(body: &Value) -> Option<String> {
    let text = |key: &str| body.get(key).and_then(Value::as_text);
    let figure = |key: &str| body.get(key).and_then(Value::as_integer);
    let method = text("method")?;
    if !MEASURES.contains(&method) {
        return None;
    }
    if let Some(why) = text("could_not_tell") {
        return Some(format!("could not tell: {why}"));
    }
    Some(match method {
        fidelity::NAME => format!(
            "agreed with {} at {} of {} position(s); {} millibits a token",
            text("reference").unwrap_or("the reference"),
            figure("agreed").unwrap_or(0),
            figure("positions").unwrap_or(0),
            figure("millibits_per_token").unwrap_or(0)
        ),
        offload::NAME => format!(
            "{} layer count(s) measured of {} layers",
            figure("measured").unwrap_or(0),
            figure("layers_total").unwrap_or(0)
        ),
        prefill::NAME => format!(
            "fastest at batch {}; batch {} is within a tenth of it",
            figure("fastest_batch").unwrap_or(0),
            figure("enough_batch").unwrap_or(0)
        ),
        prefix::NAME => format!(
            "a kept prefix saves {} of the first-token time",
            per_cent(figure("saving_ppm").unwrap_or(0))
        ),
        memory::NAME => format!(
            "{} window(s) measured; the largest differs from its prediction by {}",
            figure("measured").unwrap_or(0),
            per_cent(figure("worst_ppm").unwrap_or(0).abs())
        ),
        concurrency::NAME => format!(
            "aggregate at eight is {} of the rate at one",
            per_cent(figure("scaling_ppm").unwrap_or(0))
        ),
        cold::NAME => format!(
            "cold first token {} ms, warm {} ms",
            figure("cold_first_token_ms").unwrap_or(0),
            figure("warm_first_token_ms").unwrap_or(0)
        ),
        bits::NAME => format!(
            "{} millibits a byte over {} token(s), {} bounded",
            figure("millibits_per_byte").unwrap_or(0),
            figure("tokens").unwrap_or(0),
            figure("bounded").unwrap_or(0)
        ),
        determinism::NAME => format!(
            "{} of {} repeat(s) identical; other threads {}, other batch {}",
            figure("identical").unwrap_or(0),
            figure("repeats").unwrap_or(0),
            same(body.get("other_threads_identical")),
            same(body.get("other_batch_identical"))
        ),
        tokenizer::NAME => format!(
            "{} of {} text(s) round-tripped",
            figure("round_tripped").unwrap_or(0),
            figure("texts").unwrap_or(0)
        ),
        retrieval::NAME => format!(
            "found in {} of {} placement(s)",
            figure("found").unwrap_or(0),
            figure("placements").unwrap_or(0)
        ),
        degeneration::NAME => body.get("loop_at").and_then(Value::as_integer).map_or_else(
            || format!("no loop in {} token(s)", figure("produced").unwrap_or(0)),
            |at| format!("a loop begins at token {at}"),
        ),
        grammar::NAME => format!(
            "valid JSON in {} of {} free trial(s), {} of {} constrained",
            figure("free_valid").unwrap_or(0),
            figure("trials").unwrap_or(0),
            figure("constrained_valid").unwrap_or(0),
            figure("trials").unwrap_or(0)
        ),
        image::NAME => format!(
            "{} side(s) measured; {} token(s) an image at the largest",
            figure("measured").unwrap_or(0),
            figure("largest_tokens").unwrap_or(0)
        ),
        agent::NAME => format!(
            "completed the chain in {} of {}, carried every result in {}",
            figure("completed").unwrap_or(0),
            figure("asked").unwrap_or(0),
            figure("result_carried").unwrap_or(0)
        ),
        instructions::NAME => format!(
            "every constraint held in {} of {} trial(s); {} of {} constraint(s) held",
            figure("all_held").unwrap_or(0),
            figure("trials").unwrap_or(0),
            figure("constraints_held").unwrap_or(0),
            figure("constraints_asked").unwrap_or(0)
        ),
        paraphrase::NAME => format!(
            "{} of {} answer(s) right; {} agree with their question's most common answer",
            figure("right").unwrap_or(0),
            figure("asked").unwrap_or(0),
            figure("agree").unwrap_or(0)
        ),
        multilingual::NAME => format!(
            "{} of {} right across {} language(s)",
            figure("right").unwrap_or(0),
            figure("asked").unwrap_or(0),
            multilingual::LANGUAGES.len()
        ),
        multifact::NAME => format!(
            "over {} depth(s): every price listed in {}, the order right in {}, the sum right in {}",
            figure("depths").unwrap_or(0),
            figure("listed_all").unwrap_or(0),
            figure("ordered").unwrap_or(0),
            figure("summed").unwrap_or(0)
        ),
        sustained::NAME => format!(
            "{} sample(s) over {} s; slowest {} and fastest {} tokens/s",
            figure("samples").unwrap_or(0),
            figure("duration_s").unwrap_or(0),
            milli_said(u64::try_from(figure("slowest_milli").unwrap_or(0)).unwrap_or(0)),
            milli_said(u64::try_from(figure("fastest_milli").unwrap_or(0)).unwrap_or(0))
        ),
        energy::NAME => format!(
            "{} mJ a produced token, {} mJ a prompt token; idle {} W",
            milli_said(u64::try_from(figure("generate_uj_per_token").unwrap_or(0)).unwrap_or(0)),
            milli_said(u64::try_from(figure("read_uj_per_token").unwrap_or(0)).unwrap_or(0)),
            milli_said(
                u64::try_from(figure("idle_uw").unwrap_or(0))
                    .unwrap_or(0)
                    .saturating_div(1000)
            )
        ),
        jitter::NAME => format!(
            "{} piece(s); typical gap {} ms, longest {} ms at token {}; {} gap(s) past five times the typical",
            figure("pieces").unwrap_or(0),
            as_ms(u64::try_from(figure("typical_ns").unwrap_or(0)).unwrap_or(0)),
            as_ms(u64::try_from(figure("longest_ns").unwrap_or(0)).unwrap_or(0)),
            figure("longest_at").unwrap_or(0),
            figure("past_five_times").unwrap_or(0)
        ),
        drafthead::NAME => format!(
            "{} tokens/s without the draft head, {} with; outputs {}",
            figure("without_per_second").unwrap_or(0),
            figure("with_per_second").unwrap_or(0),
            same(body.get("identical"))
        ),
        cache::NAME => format!(
            "{} tokens/s at 16 bits, {} at 8, {} at 4; the 8-bit run {}, the 4-bit {}",
            figure("f16_per_second").unwrap_or(0),
            figure("q8_per_second").unwrap_or(0),
            figure("q4_per_second").unwrap_or(0),
            body.get("q8_divergence")
                .and_then(Value::as_integer)
                .map_or_else(
                    || "identical".to_owned(),
                    |at| format!("parts at token {at}")
                ),
            body.get("q4_divergence")
                .and_then(Value::as_integer)
                .map_or_else(
                    || "identical".to_owned(),
                    |at| format!("parts at token {at}")
                )
        ),
        soak::NAME => format!(
            "{} of {} request(s) failed; resident {} at the start and {} at the end",
            figure("failed").unwrap_or(0),
            figure("requests").unwrap_or(0),
            gigabytes(u64::try_from(figure("resident_start").unwrap_or(0)).unwrap_or(0)),
            gigabytes(u64::try_from(figure("resident_end").unwrap_or(0)).unwrap_or(0))
        ),
        stoplatency::NAME => format!(
            "stopped at {} depth(s); a one-token probe on the idle engine takes {} ms",
            figure("depths").unwrap_or(0),
            as_ms(u64::try_from(figure("idle_probe_ns").unwrap_or(0)).unwrap_or(0))
        ),
        fidelitydepth::NAME => format!(
            "agreed at {} of 100, {} of 500, {} of 1000 position(s); {} bits a token at 1000",
            figure("agreed_100").unwrap_or(0),
            figure("agreed_500").unwrap_or(0),
            figure("agreed_1000").unwrap_or(0),
            fidelity::millibits_said(figure("millibits_per_token_1000").unwrap_or(0))
        ),
        repository::NAME => format!(
            "{} of {} file(s) of the repository read against {}",
            figure("measured").unwrap_or(0),
            figure("files").unwrap_or(0),
            text("reference").unwrap_or("the reference")
        ),
        schemas::NAME => format!(
            "the shape held in {} free and {} constrained trial(s) over {} shape(s)",
            figure("free_shaped").unwrap_or(0),
            figure("constrained_shaped").unwrap_or(0),
            figure("shapes").unwrap_or(0)
        ),
        temperature::NAME => format!(
            "{} of {} right at the coldest temperature, {} at the hottest",
            figure("right_at_coldest").unwrap_or(0),
            figure("asked_each").unwrap_or(0),
            figure("right_at_hottest").unwrap_or(0)
        ),
        vocabulary::NAME => format!(
            "{} token(s) over {} byte(s); {} byte fallback(s), {} unknown",
            figure("tokens").unwrap_or(0),
            figure("bytes").unwrap_or(0),
            figure("byte_fallbacks").unwrap_or(0),
            figure("unknown").unwrap_or(0)
        ),
        seeing::NAME => format!(
            "{} of {} picture(s) read rightly: counts {}, numbers {}, colours {}, the larger {}",
            figure("right").unwrap_or(0),
            figure("pictures").unwrap_or(0),
            figure("count_right").unwrap_or(0),
            figure("number_right").unwrap_or(0),
            figure("colour_right").unwrap_or(0),
            figure("larger_right").unwrap_or(0)
        ),
        arithmetic::NAME => format!(
            "{} of {} sum(s) right; adding holds to {} digits, subtracting to {}, multiplying to {}",
            figure("right").unwrap_or(0),
            figure("sums").unwrap_or(0),
            figure("add_holds_to").unwrap_or(0),
            figure("subtract_holds_to").unwrap_or(0),
            figure("multiply_holds_to").unwrap_or(0)
        ),
        reckoning::NAME => format!(
            "{} of {} question(s) right across weekdays, days between, sorting and counting",
            figure("right").unwrap_or(0),
            figure("questions").unwrap_or(0)
        ),
        listing::NAME => format!(
            "the count held in {} of {} ask(s); nothing repeated in {}",
            figure("count_held").unwrap_or(0),
            figure("asks").unwrap_or(0),
            figure("none_repeated").unwrap_or(0)
        ),
        recall::NAME => format!(
            "recalled in {} of {} ask(s), {} of {} after ten turns; the correction {}",
            figure("recalled").unwrap_or(0),
            figure("asked").unwrap_or(0),
            figure("recalled_at_ten").unwrap_or(0),
            figure("facts").unwrap_or(0),
            match body.get("corrected") {
                Some(Value::Bool(true)) => "honoured",
                Some(Value::Bool(false)) => "not honoured",
                _ => "not measured",
            }
        ),
        obedience::NAME => format!(
            "the system rule held in {} of {} turn(s); {} of {} rule(s) held every turn",
            figure("held").unwrap_or(0),
            figure("turns").unwrap_or(0),
            figure("whole").unwrap_or(0),
            figure("rules").unwrap_or(0)
        ),
        absent::NAME => format!(
            "present figures right in {} of {}; of {} absent, {} stated absent and {} invented",
            figure("present_right").unwrap_or(0),
            figure("present").unwrap_or(0),
            figure("absent").unwrap_or(0),
            figure("stated_absent").unwrap_or(0),
            figure("invented").unwrap_or(0)
        ),
        codereading::NAME => format!(
            "predicted {} of {} program output(s); placed {} of {} bug(s)",
            figure("outputs_right").unwrap_or(0),
            figure("outputs").unwrap_or(0),
            figure("bugs_right").unwrap_or(0),
            figure("bugs").unwrap_or(0)
        ),
        nearest::NAME => format!(
            "the paraphrase nearer in {} of {} triple(s)",
            figure("ordered").unwrap_or(0),
            figure("triples").unwrap_or(0)
        ),
        injection::NAME => format!(
            "followed a planted instruction in {} of {}; answered the question in {}",
            figure("followed").unwrap_or(0),
            figure("asked").unwrap_or(0),
            figure("answered").unwrap_or(0)
        ),
        thinkingcost::NAME => format!(
            "right with thinking on in {} of {}, off in {}; {} token(s) of thinking",
            figure("right_on").unwrap_or(0),
            figure("questions").unwrap_or(0),
            figure("right_off").unwrap_or(0),
            figure("thought_tokens").unwrap_or(0)
        ),
        loadtime::NAME => format!(
            "ready in {} ms at the quickest share and {} ms at the slowest",
            as_ms(u64::try_from(figure("fastest_ready_ns").unwrap_or(0)).unwrap_or(0)),
            as_ms(u64::try_from(figure("slowest_ready_ns").unwrap_or(0)).unwrap_or(0))
        ),
        threads::NAME => format!(
            "fastest at {} thread(s), {} tokens/s, over {} count(s)",
            figure("fastest_threads").unwrap_or(0),
            milli_said(u64::try_from(figure("fastest_per_second_milli").unwrap_or(0)).unwrap_or(0)),
            figure("counts").unwrap_or(0)
        ),
        deep::NAME => format!(
            "found in {} of {} placement(s) past sixteen thousand; {} depth(s) past the window",
            figure("found").unwrap_or(0),
            figure("asked").unwrap_or(0),
            figure("past_window").unwrap_or(0)
        ),
        extraction::NAME => format!(
            "{} of {} field(s) exact over {} trial(s); JSON parsed in {}",
            figure("fields_right").unwrap_or(0),
            figure("fields_asked").unwrap_or(0),
            figure("trials").unwrap_or(0),
            figure("parsed").unwrap_or(0)
        ),
        tooluse::NAME => format!(
            "right tool in {} of {} call(s), arguments matched in {}, result carried in {}, held back in {}",
            figure("right_tool").unwrap_or(0),
            figure("asked")
                .unwrap_or(0)
                .saturating_sub(figure("trials").unwrap_or(0)),
            figure("args_matched").unwrap_or(0),
            figure("result_carried").unwrap_or(0),
            figure("held_back").unwrap_or(0)
        ),
        _ => return None,
    })
}

fn same(held: Option<&Value>) -> &'static str {
    match held {
        Some(Value::Bool(true)) => "identical",
        Some(Value::Bool(false)) => "diverged",
        _ => "not measured",
    }
}

/// A count as the record's integer, saturating rather than wrapping.
pub(crate) fn as_integer(held: usize) -> i64 {
    i64::try_from(held).unwrap_or(i64::MAX)
}

/// A figure as the record's integer.
pub(crate) fn whole(held: u64) -> Value {
    Value::Integer(i64::try_from(held).unwrap_or(i64::MAX))
}

/// Nanoseconds as milliseconds to one place, without a float.
pub(crate) fn as_ms(ns: u64) -> String {
    #[expect(
        clippy::integer_division,
        reason = "nanoseconds to tenths of a millisecond, then whole and remainder"
    )]
    let tenths = ns / 100_000;
    #[expect(clippy::integer_division, reason = "whole milliseconds and tenths")]
    let (whole, tenth) = (tenths / 10, tenths % 10);
    format!("{whole}.{tenth}")
}

/// Parts per million as a percentage to one place.
pub(crate) fn per_cent(ppm: i64) -> String {
    #[expect(
        clippy::integer_division,
        reason = "parts per million to tenths of a per cent"
    )]
    let tenths = ppm / 1_000;
    #[expect(clippy::integer_division, reason = "whole per cent and tenths")]
    let (whole, tenth) = (tenths / 10, (tenths % 10).abs());
    format!("{whole}.{tenth}%")
}

/// A part of a whole in parts per million; nought of nothing.
pub(crate) fn ppm(part: u64, whole: u64) -> i64 {
    if whole == 0 {
        return 0;
    }
    #[expect(
        clippy::integer_division,
        reason = "a share to the nearest part in a million"
    )]
    let parts = u128::from(part) * 1_000_000 / u128::from(whole);
    i64::try_from(parts).unwrap_or(i64::MAX)
}

/// Tokens a second from a count and nanoseconds; nought where no time
/// passed.
pub(crate) fn per_second(tokens: u64, ns: u64) -> u64 {
    if ns == 0 {
        return 0;
    }
    #[expect(
        clippy::integer_division,
        reason = "a rate to the nearest whole token a second"
    )]
    let rate = u128::from(tokens) * 1_000_000_000 / u128::from(ns);
    u64::try_from(rate).unwrap_or(u64::MAX)
}

/// A rate in thousandths as a figure to one place, without a float.
pub(crate) fn milli_said(milli: u64) -> String {
    #[expect(clippy::integer_division, reason = "thousandths to whole and tenths")]
    let (whole, tenths) = (milli / 1000, (milli % 1000) / 100);
    format!("{whole}.{tenths}")
}

/// The median of some samples, sorting them on the way; `None` of none.
pub(crate) fn median(samples: &mut [u64]) -> Option<u64> {
    if samples.is_empty() {
        return None;
    }
    samples.sort_unstable();
    #[expect(
        clippy::integer_division,
        reason = "the middle of a list, which is what a median is"
    )]
    let at = samples.len() / 2;
    samples.get(at).copied()
}

/// How long a closure takes, in whole nanoseconds, beside what it returned.
pub(crate) fn timed<T>(work: impl FnOnce() -> T) -> (T, u64) {
    let began = std::time::Instant::now();
    let out = work();
    (
        out,
        u64::try_from(began.elapsed().as_nanos()).unwrap_or(u64::MAX),
    )
}

/// A person's text as one user turn under the model's own template, as
/// the engine reads it: the template's opening pieces with the beginning
/// marker, the text segmented plainly, the template's closing pieces.
///
/// # Errors
///
/// Whatever the engine refused: a template that could not be rendered,
/// or a text it could not segment.
pub(crate) fn framed_ids(engine: &Served, text: &str) -> Result<Vec<usize>, String> {
    let frame = crate::turn::frame(engine, &crate::turn::Turn::default())
        .map_err(|failure| failure.detail().to_owned())?;
    let mut ids: Vec<usize> = Vec::new();
    for (part, beginning, markers) in [
        (frame.before.as_str(), true, true),
        (text, false, false),
        (frame.after.as_str(), false, true),
    ] {
        if part.is_empty() {
            continue;
        }
        let read = engine
            .tokenize(part, beginning, markers)
            .map_err(|failure| failure.detail().to_owned())?;
        ids.extend(read.iter().map(|token| token.id));
    }
    Ok(ids)
}

/// A run of one identifier, `depth` long: what a timing is taken over when
/// what is being measured is depth rather than text. Identifier 1 is
/// inside every vocabulary MCF can address.
pub(crate) fn filler(depth: usize) -> Vec<usize> {
    vec![1; depth]
}

/// Bytes as gigabytes to one place.
pub(crate) fn gigabytes(bytes: u64) -> String {
    #[expect(clippy::integer_division, reason = "bytes to tenths of a gigabyte")]
    let tenths = bytes / 100_000_000;
    #[expect(clippy::integer_division, reason = "whole gigabytes and tenths")]
    let (whole, tenth) = (tenths / 10, tenths % 10);
    format!("{whole}.{tenth} GB")
}

#[cfg(test)]
mod tests;
