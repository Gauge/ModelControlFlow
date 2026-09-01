//! The daemon: long-lived, restartable, and idle almost all the time (B-030,
//! B-031, B-036, D1).
//!
//! **What it is for at M2's start.** D1 settled that MCF is a process with
//! clients attached rather than a command that exits. This is that process. It
//! cannot serve a model — there is no engine — and it says so when asked, which
//! is the honest shape of a daemon that exists before the thing it will host
//! (A19).
//!
//! **Idle costs nothing, and that is structural rather than careful.** §3.13
//! asks that an idle MCF be indistinguishable from nothing, and D24 states it as
//! a prohibition: zero timer wakeups. The way to get that is not a small
//! interval — it is *no* interval. This daemon blocks in `accept` and does
//! nothing at all until somebody connects: no tick, no poll, no watcher, no
//! heartbeat. B-031's condition is measurable because of that shape, and
//! `checks/tests/soak.rs` measures it.
//!
//! **Local by construction, not by configuration.** The control plane is a Unix
//! socket in a directory only this user can enter (B-036, §6.12). There is no
//! bind address, no port and no flag to expose it: exposure is not something
//! MCF can do today, so it is not something a mistake can do either. When §XI's
//! remote surface arrives it arrives as a deliberate, recorded act with its own
//! decision behind it.
//!
//! **What survives a restart is what was written down.** The daemon keeps no
//! state a crash could lose: what it knows on start is what the record and the
//! model store say, both read fresh. A9's habit — a daemon that recovered from
//! its own memory would be a daemon whose memory is the record, and D20 already
//! settled that the journal is the record.

use std::io::{BufRead as _, BufReader, Write as _};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};

use mcf_core::build_identity::BuildIdentity;
use mcf_core::failure::{Attribution, Category, Disposition, Failure, Result, Subsystem};
use mcf_core::time::{Clock as _, Instant, Monotonic, SystemClock, Timestamp};
use mcf_record::journal::{Entry, EntryId, EntryKind};
use mcf_record::json::Value;

use mcf_hub::source::Source as _;

use crate::control::{Answer, REQUEST_CEILING, Request, VERSION};

const WHERE: Subsystem = Subsystem::new("mcf-serve::daemon");

/// How long the daemon waits for a client to say something.
///
/// Two seconds. Long enough that a slow client on a busy machine is not cut
/// off, short enough that one which says nothing is not worth waiting for —
/// and stated here rather than chosen at the call site, because it is the bound
/// on how long one client can delay another (B7).
pub const PATIENCE: std::time::Duration = std::time::Duration::from_secs(2);

/// What a daemon was told about where things are.
///
/// Passed in rather than discovered, for the reason every other path in MCF
/// takes one: a process that decides where to put its socket is a process a
/// test cannot put somewhere else, and the laboratory needs to run one without
/// touching the operator's own (B19).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Places {
    /// Where the control socket goes.
    pub socket: PathBuf,
    /// The record, which is what the daemon recovers from.
    pub journal: PathBuf,
    /// The model store, which is what it reports holding.
    pub models: PathBuf,
}

/// How many tokens each generation in a prompt report may produce.
///
/// Enough for an answer worth comparing and small enough that a report of nine
/// generations finishes: what is being compared is whether two answers differ,
/// which a short answer settles as well as a long one.
const PROMPT_REPORT_LIMIT: usize = 160;

/// A prompt report, as a client reads it.
fn prompt_report_value(report: &crate::prompt::Report, generations: usize) -> Value {
    Value::map([
        ("baseline", Value::text(report.baseline.clone())),
        (
            "floor_parts_per_million",
            Value::Integer(i64::try_from(report.floor).unwrap_or(i64::MAX)),
        ),
        (
            "clauses",
            Value::List(
                report
                    .clauses
                    .iter()
                    .map(|clause| {
                        Value::map([
                            ("text", Value::text(clause.text.clone())),
                            ("changed", Value::Bool(clause.changed)),
                            (
                                "moved_parts_per_million",
                                Value::Integer(i64::try_from(clause.moved).unwrap_or(i64::MAX)),
                            ),
                            ("without", Value::text(clause.without.clone())),
                        ])
                    })
                    .collect(),
            ),
        ),
        (
            "clauses_over_the_cap",
            Value::Integer(i64::try_from(report.clauses_over_the_cap).unwrap_or(i64::MAX)),
        ),
        (
            "seeds_asked",
            Value::Integer(i64::try_from(report.settled.asked).unwrap_or(i64::MAX)),
        ),
        (
            "distinct_answers",
            Value::Integer(i64::try_from(report.settled.distinct).unwrap_or(i64::MAX)),
        ),
        (
            "generations",
            Value::Integer(i64::try_from(generations).unwrap_or(i64::MAX)),
        ),
    ])
}

/// A model's header, from a bounded read of the front of the file.
///
/// **Never the whole file.** `gguf::read` reads every byte, which is right when
/// the weights are wanted and catastrophic when only the header is: this
/// machine holds 21 GB of models, and answering *what is held* by reading all
/// of them made the daemon appear to hang. The directory sits at the front, so
/// a few mebibytes answers every question this needs — and where a header is
/// unusually large the read grows once rather than giving up.
pub(crate) fn header_of(path: &std::path::Path) -> Option<mcf_standin::gguf::Model> {
    use std::io::Read as _;
    let held = std::fs::metadata(path).map_or(0, |meta| meta.len());
    for cap in [4_u64 << 20, 64 << 20] {
        let take = cap.min(held);
        let mut prefix = Vec::new();
        std::fs::File::open(path)
            .and_then(|handle| handle.take(take).read_to_end(&mut prefix))
            .ok()?;
        if let Ok(model) = mcf_standin::gguf::parse(&prefix) {
            return Some(model);
        }
        if take >= held {
            return None;
        }
    }
    None
}

/// Memory free for a new process, or `None` where the platform will not say.
///
/// Available rather than total: what matters is what a model could take now,
/// not what the machine has in principle.
fn system_memory_free() -> Option<u64> {
    let text = std::fs::read_to_string("/proc/meminfo").ok()?;
    for line in text.lines() {
        if let Some(rest) = line.strip_prefix("MemAvailable:") {
            let kibibytes: u64 = rest.split_whitespace().next()?.parse().ok()?;
            return Some(kibibytes * 1024);
        }
    }
    None
}

/// The last line of an acquisition: where the model went, and what was
/// written down about it.
fn acquired(file: &str, done: &mcf_hub::acquisition::Done) -> Answer {
    let count = |held: usize| Value::Integer(i64::try_from(held).unwrap_or(i64::MAX));
    let bytes = Value::Integer(i64::try_from(done.acquired.bytes).unwrap_or(i64::MAX));
    Answer::served(Value::map([
        ("acquiring", Value::text(file.to_owned())),
        ("doing", Value::text("done")),
        ("done", Value::Bool(true)),
        (
            "path",
            Value::text(done.acquired.path.display().to_string()),
        ),
        ("arrived", bytes.clone()),
        ("bytes", bytes),
        ("attempts", count(done.acquired.attempts)),
        (
            "recorded",
            match &done.recorded {
                Ok(where_) => Value::text(where_.display().to_string()),
                Err(_) => Value::Null,
            },
        ),
    ]))
}

/// The last line of a measurement: every reading, and what they were taken
/// under.
///
/// The conditions are not a footnote. A speed without them is a number nobody
/// can use or reproduce, and the engine that *ran* is the condition that
/// matters most — a timing taken from MCF's own stand-in measures the
/// stand-in, which is written to be read rather than to be fast (A6, B65, D31).
fn measured(
    named: &str,
    path: &Path,
    bytes: Option<u64>,
    asked: Option<&str>,
    ran_on: Option<&str>,
    readings: Vec<Value>,
) -> Answer {
    Answer::served(Value::map([
        ("measuring", Value::text(named.to_owned())),
        ("readings", Value::List(readings)),
        ("done", Value::Bool(true)),
        (
            "conditions",
            Value::map([
                ("model", Value::text(path.display().to_string())),
                (
                    "bytes",
                    bytes.map_or(Value::Null, |bytes| {
                        Value::Integer(i64::try_from(bytes).unwrap_or(i64::MAX))
                    }),
                ),
                (
                    "engine_asked",
                    asked.map_or(Value::Null, |engine| Value::text(engine.to_owned())),
                ),
                (
                    "engine_ran",
                    ran_on.map_or(Value::Null, |engine| Value::text(engine.to_owned())),
                ),
                (
                    "is_the_stand_in",
                    Value::Bool(ran_on.is_some_and(is_the_stand_in)),
                ),
                ("repeats", Value::Integer(i64::from(REPEATS))),
                (
                    "tokens_per_reading",
                    Value::Integer(i64::from(SETTLED_AS_F32)),
                ),
                (
                    "method",
                    Value::text(
                        "two runs a depth, one token and seventeen; the difference over sixteen, \
                         so that loading and prefill cancel",
                    ),
                ),
                ("loaded", Value::text("per_request")),
            ]),
        ),
    ]))
}

/// One timed generation: how long it took, and what actually ran it.
///
/// **Nanoseconds, as a whole number.** Floating point does not appear in a
/// shipped crate, because a NaN one division away from a record is how a
/// measurement starts lying (A6, A1) — and a duration is a count of ticks
/// anyway. Everything below divides and formats integers.
#[derive(Debug)]
struct Timed {
    /// Wall-clock nanoseconds.
    ns: u64,
    /// The engine the account says served it, which is not necessarily the
    /// one that was asked for.
    engine: Option<String>,
}

/// Nanoseconds as milliseconds, to three places, without a float.
fn as_milliseconds(ns: u64) -> String {
    #[expect(
        clippy::integer_division,
        reason = "nanoseconds to microseconds, then to milliseconds and a remainder; \
                  the discarded part is under a nanosecond"
    )]
    let micros = ns / 1_000;
    #[expect(
        clippy::integer_division,
        reason = "whole milliseconds and thousandths"
    )]
    let (whole, thousandths) = (micros / 1_000, micros % 1_000);
    format!("{whole}.{thousandths:03}")
}

/// Whether an engine name is MCF's own reference implementation.
fn is_the_stand_in(engine: &str) -> bool {
    engine == mcf_core::build_identity::stand_in_engine() || engine.contains("stand-in")
}

/// The median of a set of samples, sorting them on the way.
///
/// The median rather than the mean, because one run that met a busy machine
/// should not move the answer — and with three samples the mean would let it.
fn middle(samples: &mut [u64]) -> Option<u64> {
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

/// How many times each depth is measured.
///
/// Three: enough for a median to mean something, few enough that a ladder of
/// eight depths does not become a coffee break.
const REPEATS: u32 = 3;

/// How many tokens a reading is taken over, past the first.
///
/// Sixteen: enough that the difference between the two runs is dominated by
/// generation rather than by the noise in either, and few enough that a ladder
/// of eight depths is a minute rather than ten.
const SETTLED: u32 = 16;
/// The same, where arithmetic wants it.
const SETTLED_AS_F32: u16 = 16;

/// The shallowest depth worth measuring.
///
/// Below this the per-token cost is the same as at this depth to within the
/// noise, so a rung there would cost time and add nothing.
const SHALLOWEST: u64 = 512;

/// Every power of two from the shallowest up to and including `deepest`.
///
/// Powers of two because that is how context windows are asked for, and every
/// rung because a deep reading with no shallow one to compare against is not a
/// fall-off — it is a single number.
fn depth_ladder(deepest: u64) -> Vec<u64> {
    let mut ladder = Vec::new();
    let mut depth = SHALLOWEST;
    while depth <= deepest {
        ladder.push(depth);
        depth = depth.saturating_mul(2);
        if depth == 0 {
            break;
        }
    }
    ladder
}

/// Roughly how long a ladder will take, as a range.
///
/// The arithmetic below converts byte counts and depths into seconds. Both are
/// far inside f64's exact range — the largest model anybody holds is a few
/// hundred billion bytes and the mantissa runs out four thousand times higher
/// — and the result is clamped positive before it becomes a whole number.
///
/// Crude on purpose. What it is for is letting somebody decide not to wait,
/// and a range that is honestly wide serves that better than a single number
/// that is precisely wrong. MCF's estimates have been measured against what
/// runs actually take and land between 0.58× and 1.42× of them, so those are
/// the bounds rather than something invented here.
#[expect(
    clippy::integer_division,
    reason = "an estimate in whole seconds, and the remainder of a second is \
              far inside the range the answer is given as"
)]
fn estimated_seconds(ladder: &[u64], bytes: Option<u64>) -> (u64, u64) {
    // Milliseconds throughout, so that the arithmetic is whole numbers and
    // the rounding happens once, at the end.
    //
    // A model is read once per request at roughly half a gigabyte a second on
    // an ordinary machine, and a rung is two requests times the repeats.
    let per_load_ms = bytes.map_or(2_000, |bytes| bytes / 500_000);
    let prefill_ms: u64 = ladder.iter().map(|depth| depth / 4).sum();
    let runs = u64::from(REPEATS).saturating_mul(2);
    let middle_ms = (ladder.len() as u64)
        .saturating_mul(per_load_ms)
        .saturating_add(prefill_ms)
        .saturating_mul(runs);
    // MCF's estimates have been measured against what runs actually take and
    // land between 0.58× and 1.42× of them, so those are the bounds rather
    // than something invented here.
    let low = (middle_ms.saturating_mul(58) / 100_000).max(1);
    let high = (middle_ms.saturating_mul(142) / 100_000).max(2);
    (low, high)
}

/// A model MCF is holding for callers.
#[derive(Debug)]
struct Holding {
    /// The engine.
    ///
    /// Held and never read: dropping it is what stops the server, so the
    /// field's whole job is to be owned until somebody unhosts (A27).
    #[expect(dead_code, reason = "owning it is what keeps the engine alive")]
    served: crate::served::Served,
    /// Which model.
    model: PathBuf,
    /// What it was started under.
    settings: crate::hosting::Hosting,
    /// What MCF had recommended, so that what was chosen and what was advised
    /// can both be read back (§3.15).
    recommended: crate::hosting::Hosting,
    /// When it started.
    since: Timestamp,
}

/// Whether a header is describing the file it came from.
///
/// **A shape fetched from a hub is a claim, and this is the one part of it MCF
/// can check without the file.** The header's own tensor table says where the
/// last tensor ends, and that has to be inside the file the listing describes
/// and account for nearly all of it. Measured across seven architectures on
/// this machine, the declared extent is between 96.2% and 100.0% of the
/// published size.
///
/// So two things are caught. A header claiming *more* data than the file holds
/// is describing something else — definitively, since a file cannot contain
/// more than it contains. And a header claiming a fraction of it is the
/// deception that would matter here: a tiny shape against a large file makes a
/// forty-gigabyte model look like it needs almost nothing, and MCF would
/// answer *fits* (§3.7, B-022, A21).
///
/// What this cannot check is whether the weights are what the header says they
/// are. That needs the weights, and the point of reading a prefix is not to
/// fetch them — which is why the plan built on this says it rests on a
/// declaration.
pub(crate) fn header_describes_this_file(model: &mcf_standin::gguf::Model, published: u64) -> bool {
    let Some(extent) = model.data_bytes_required() else {
        // A quantization MCF cannot size is not a lie; it is a thing MCF
        // cannot check, and an unknown is not a failure (A7). The plan is
        // refused rather than built on something unexamined.
        return false;
    };
    if published == 0 || extent > published {
        return false;
    }
    // Half. The observed floor is 96%, and the margin is for a small model
    // whose metadata is a large share of it rather than for a header that is
    // describing something else.
    extent.saturating_mul(2) >= published
}

/// The plan, from whichever place this repository says how its model is shaped.
///
/// The configuration first, because it can say which blocks are full-attention
/// and a header cannot. The header second, because most repositories that
/// publish GGUFs publish no configuration at all — and *MCF cannot say* about
/// nearly everything anybody would download is a poor answer when the file
/// itself carries the numbers (B-413).
///
/// Returns the plan and whether the shape came from the header, because where
/// it came from is a condition of every verdict in it (A6).
fn plan_however_the_shape_can_be_found(
    hub: &mcf_hub::client::Hub,
    listing: &mcf_hub::source::Listing,
) -> (core::result::Result<mcf_hub::offer::Plan, String>, bool) {
    // What is free, read the way the daemon reads it for everything else — B4
    // keeps hardware sampling out of the serving path, so the plan is handed a
    // number rather than going and taking one.
    let Some(free) = system_memory_free() else {
        return (
            Err("this machine will not say how much memory is free".to_owned()),
            false,
        );
    };
    let available = mcf_core::measurement::Bytes(free);
    match mcf_hub::offer::shape_from_configuration(hub, listing) {
        Ok(shape) => (mcf_hub::offer::plan_with(listing, shape, available), false),
        Err(why) => match shape_from_a_published_header(hub, listing) {
            Some(shape) => (mcf_hub::offer::plan_with(listing, shape, available), true),
            None => (Err(why), false),
        },
    }
}

/// A model's shape, read from the header of the first GGUF a repository
/// publishes.
///
/// **A few megabytes, not the model.** The hub serves ranges, so the header
/// arrives without acquiring what is behind it — and the sizes tried are the
/// ones `header_of` uses for a file on this disk, for the same reason: a GGUF
/// puts its metadata first but a tokenizer vocabulary can be megabytes of it.
///
/// **Smallest first, and not every smallest file has a header.** A large model
/// is published in parts, and the second part of a split GGUF is smaller than
/// the first and carries no metadata at all — on a seventy-billion-parameter
/// repository the smallest file is exactly that. Rather than read the naming
/// convention for split parts, which would be guessing from a name again
/// (B-417), this simply tries the next candidate: a part with no header does
/// not parse, and one that does not parse is not the file to ask.
///
/// Four candidates at most. Every variant of one model shares its shape, so
/// the answer is in the first file that has a header, and a repository where
/// four in a row have none is one MCF says it cannot judge.
fn shape_from_a_published_header(
    hub: &mcf_hub::client::Hub,
    listing: &mcf_hub::source::Listing,
) -> Option<mcf_hub::fitment::Shape> {
    let mut candidates: Vec<&mcf_hub::source::Entry> = listing
        .entries
        .iter()
        .filter(|entry| {
            std::path::Path::new(&entry.path)
                .extension()
                .is_some_and(|extension| extension.eq_ignore_ascii_case("gguf"))
        })
        .collect();
    candidates.sort_by_key(|entry| entry.size);

    for entry in candidates.into_iter().take(4) {
        for prefix in [4_u64 << 20, 16 << 20] {
            let Ok(held) = hub.prefix_of(&listing.reference, entry, prefix) else {
                break;
            };
            if held.is_empty() {
                break;
            }
            if let Ok(model) = mcf_standin::gguf::parse(&held)
                && header_describes_this_file(&model, entry.size)
                && let Some(shape) = crate::engines::shape_of(&model)
            {
                return Some(shape);
            }
            // A prefix that did not reach the end of the metadata parses as
            // nothing rather than as something shorter, so a larger one is
            // the only way to tell *not yet* from *not there* (A7).
            if prefix >= entry.size {
                break;
            }
        }
    }
    None
}

/// The newest measurement of each model, from the record.
///
/// **Through the index and by kind**, not by replaying the journal: a daemon
/// start that parsed the whole history would cost seconds on a record that has
/// been measuring for a while (F14), and a listing that re-read it once a
/// model would cost the record's whole length once a row (F118). What this
/// reads is the entries whose kind says they are timings, newest last, and it
/// keeps one per model.
fn newest_timings(journal: &Path) -> std::collections::BTreeMap<PathBuf, Value> {
    let mut newest = std::collections::BTreeMap::new();
    if !journal.exists() {
        return newest;
    }
    let Ok(index) = mcf_record::journal::Index::over(
        journal,
        &mcf_record::journal::index::default_path(journal),
    ) else {
        // A record MCF cannot index is a record it reads nothing from, and
        // saying nothing about speed is the honest outcome — never a zero
        // (A7). The daemon's own start line already reports the loss.
        return newest;
    };
    for located in index.entries() {
        if located.kind() != EntryKind::ModelTimed {
            continue;
        }
        let Ok(entry) = index.read(located) else {
            continue;
        };
        let Some(model) = entry
            .body()
            .get("conditions")
            .and_then(|conditions| conditions.get("model"))
            .and_then(Value::as_text)
        else {
            continue;
        };
        // Later entries overwrite earlier ones, and the index is in order, so
        // what is left is the newest. Nothing is merged: two runs under
        // different settings are two facts (A1).
        let _replaced = newest.insert(PathBuf::from(model), entry.body().clone());
    }
    newest
}

/// The hub MCF reads when nobody has named another.
const DEFAULT_HUB: &str = "https://huggingface.co/";

/// A verdict about whether a published file will run here, in a sentence.
fn said_of(verdict: &mcf_hub::fitment::Verdict) -> String {
    match verdict {
        mcf_hub::fitment::Verdict::Fits { needs, .. } => format!("fits — needs {needs}"),
        mcf_hub::fitment::Verdict::FitsWithoutContextHeadroom {
            needs,
            longest_context,
        } => format!(
            "fits, but holds a shorter conversation — {longest_context} tokens, needing {needs}"
        ),
        mcf_hub::fitment::Verdict::DoesNotFit { needs, short_by } => {
            format!("needs {needs}, which is {short_by} more than this machine has free")
        }
    }
}

/// A running daemon.
#[derive(Debug)]
pub struct Daemon {
    /// The newest measurement of each model, read from the record when the
    /// daemon starts and added to as runs finish.
    ///
    /// Held rather than re-read: answering *what is held* walks every model,
    /// and re-reading a journal of thousands of entries once a model would
    /// make a listing cost the record's whole length (F118).
    timings: std::sync::Mutex<std::collections::BTreeMap<PathBuf, Value>>,
    /// The model being held for callers, if any, with what it was started
    /// under.
    ///
    /// One at a time. Two would be two models competing for the same card,
    /// and every figure either reported would be a figure about the other
    /// one being there too (A6).
    holding: std::sync::Mutex<Option<Holding>>,
    places: Places,
    /// The engines found when this daemon started, with what each can compute
    /// on. Asked once: finding them is a directory listing, but asking what
    /// devices they have means running them, and a status request that starts
    /// processes is a status request that costs something (§3.13).
    engines: Vec<(crate::engines::Engine, Vec<crate::engines::Device>)>,
    listener: UnixListener,
    started: Timestamp,
    since: Instant<Monotonic>,
    /// What the record said when this process started, which is what *recovered
    /// across a restart* means concretely.
    recovered: Recovered,
    /// The one model held between requests, if any (D41, §7.18).
    resident: std::sync::Mutex<Option<crate::generation::Resident>>,
    /// The provisioned engine's server, if one has been started (B-376).
    ///
    /// It holds its own model, so this is a second residency and not the same
    /// one: MCF's engine loads into `resident`, and llama.cpp loads into its
    /// own process. Dropping this stops that process (A27).
    server: std::sync::Mutex<Option<crate::served::Served>>,
}

/// What was there when the daemon started.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Recovered {
    /// How many entries the record held.
    pub entries: usize,
    /// What a replay could not read, where anything could not be.
    ///
    /// Kept rather than reduced to a count: B62 requires a replay report what
    /// was lost, and a daemon that recovered past a damaged record without
    /// saying so would be the silent failure A2 calls worse than a crash.
    pub unreadable: Option<String>,
    /// How many artifacts the store held.
    pub held: usize,
}

/// Why the daemon stopped.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Stopped {
    /// A client asked it to, and said why.
    Asked {
        /// The reason the client gave.
        reason: String,
    },
    /// The listener will not answer any more, and the failure says why.
    Broken {
        /// What went wrong.
        failure: Box<Failure>,
    },
}

impl Daemon {
    /// Starts a daemon: recovers what is on the disk, then listens.
    ///
    /// # Errors
    ///
    /// `config.conflict` when something is already listening on that socket —
    /// which is one MCF already running, and starting a second would give two
    /// processes one record (D20). `resource.disk.readonly` when the socket
    /// cannot be made.
    pub fn start(places: Places) -> Result<Self> {
        if let Some(parent) = places.socket.parent() {
            std::fs::create_dir_all(parent).map_err(|error| {
                unusable("the directory the control socket lives in", parent, &error)
            })?;
        }

        // A socket file left by a process that died is not a running daemon.
        // Distinguishing them is a *connection*, not a guess: if something
        // answers, MCF is already up; if nothing does, the file is a leftover
        // and removing it is safe (A27's habit from the other side).
        if places.socket.exists() {
            if UnixStream::connect(&places.socket).is_ok() {
                return Err(Failure::new(
                    Category::ConfigConflict,
                    Attribution::User,
                    Disposition::Refused,
                    WHERE,
                    "another MCF is already listening there",
                )
                .with_context("socket", places.socket.display().to_string())
                .with_context(
                    "what_to_do",
                    "two daemons would share one record, and D20 makes the record the thing \
                     MCF is: ask the running one to stop, or point this one somewhere else",
                ));
            }
            let _leftover = std::fs::remove_file(&places.socket);
        }

        let recovered = recover(&places)?;
        let listener = UnixListener::bind(&places.socket)
            .map_err(|error| unusable("the control socket", &places.socket, &error))?;

        let started = Timestamp::now();
        // Before the struct takes ownership of `places`: asked once, here, and
        // not again while this daemon is up.
        let engines = {
            let home = places.models.parent().unwrap_or(&places.models).to_owned();
            let free = system_memory_free();
            crate::engines::discover(&home)
                .into_iter()
                .map(|engine| {
                    let devices = engine.devices(free).unwrap_or_default();
                    (engine, devices)
                })
                .collect()
        };
        let daemon = Self {
            timings: std::sync::Mutex::new(newest_timings(&places.journal)),
            holding: std::sync::Mutex::new(None),
            places,
            listener,
            started,
            since: SystemClock.now(),
            recovered,
            engines,
            resident: std::sync::Mutex::new(None),
            server: std::sync::Mutex::new(None),
        };
        // An event, not a tick. *MCF was up between these two moments* is a
        // condition of anything measured in between (§3.4), and a daemon that
        // recorded nothing would leave it unanswerable — while one that
        // recorded on a timer would fail B-031's measurement. A record that
        // cannot be written does not stop the daemon: it is reported and the
        // daemon carries on, because a machine with a full disk still wants
        // MCF up (A4).
        daemon.note(
            EntryKind::DaemonStarted,
            started,
            daemon.recovered_as_value(),
        );
        Ok(daemon)
    }

    /// Writes one line to the record, or says why it could not.
    ///
    /// Deliberately not a `Result`: the caller is a lifecycle event rather than
    /// a request, and a daemon that refused to start because it could not
    /// write down that it had started would be trading a working MCF for a
    /// tidy record (A4, A2 — said, not swallowed).
    fn note(&self, kind: EntryKind, at: Timestamp, body: Value) -> Option<EntryId> {
        let Ok(mut journal) = mcf_record::journal::Journal::open(&self.places.journal) else {
            eprintln!(
                "mcf: the record at {} could not be opened, so this event is unrecorded",
                self.places.journal.display()
            );
            return None;
        };
        match journal.append(&Entry::new(kind, at, body)) {
            Ok(appended) => Some(appended.id),
            Err(failure) => {
                eprintln!("mcf: {kind} could not be recorded: {failure}");
                None
            }
        }
    }

    /// The engines this machine has, and what each can compute on.
    ///
    /// **Asked once, when the daemon starts.** Finding the engines is a
    /// directory listing, but finding their *devices* means running each one to
    /// ask — and a status request that starts two processes is a status request
    /// that costs something. §3.13 makes idle free; it would be a poor trade to
    /// make being asked expensive instead. What a build can compute on does not
    /// change while it sits on the disk.
    fn engines_as_value(&self) -> Value {
        Value::List(
            self.engines
                .iter()
                .map(|(engine, devices)| {
                    Value::map([
                        ("name", Value::text(engine.name.clone())),
                        ("commit", Value::text(engine.commit.clone())),
                        (
                            "devices",
                            Value::List(
                                devices
                                    .iter()
                                    .map(|device| {
                                        Value::map([
                                            ("name", Value::text(device.name.clone())),
                                            (
                                                "kind",
                                                Value::text(match device.kind {
                                                    crate::engines::Kind::Cpu => "cpu",
                                                    crate::engines::Kind::Gpu => "gpu",
                                                }),
                                            ),
                                            (
                                                "free_bytes",
                                                device.free.map_or(Value::Null, |free| {
                                                    Value::Integer(
                                                        i64::try_from(free).unwrap_or(i64::MAX),
                                                    )
                                                }),
                                            ),
                                        ])
                                    })
                                    .collect(),
                            ),
                        ),
                    ])
                })
                .collect(),
        )
    }

    /// The engines as they are now, for a question whose answer turns on free
    /// memory.
    ///
    /// **Which devices exist is asked once; how much is free is not.** What a
    /// build can compute on does not change while it sits on the disk, which
    /// is why `engines` is sampled at start-up and §3.13 keeps idle free. Free
    /// memory is the one property in there that does change: a resident model
    /// takes it and stopping one gives it back. Planning from the start-up
    /// figure is a declaration wearing an observation's clothes (A21) — on an
    /// idle machine it says a second model fits beside the first, because the
    /// first was not there when it was measured.
    ///
    /// Only the processor's figure is re-read, because that is a file read. A
    /// card's would cost a process launch per device, which is the expense
    /// §3.13 exists to avoid, and a card is not where a model MCF placed
    /// wrongly takes the machine down with it.
    fn engines_now(&self) -> Vec<(crate::engines::Engine, Vec<crate::engines::Device>)> {
        let free = system_memory_free();
        self.engines
            .iter()
            .map(|(engine, devices)| {
                let devices = devices
                    .iter()
                    .map(|device| match device.kind {
                        crate::engines::Kind::Cpu => crate::engines::Device {
                            free,
                            ..device.clone()
                        },
                        crate::engines::Kind::Gpu => device.clone(),
                    })
                    .collect();
                (engine.clone(), devices)
            })
            .collect()
    }

    /// What MCF worked out about running one model: its shape, and which
    /// engine and device would take it.
    ///
    /// **Answered here rather than by each surface.** A console that opened the
    /// model store itself would be a surface reaching past the wire, and two
    /// surfaces reading the same file could disagree about it. The daemon holds
    /// the models and the engines, so the daemon is where the question is
    /// answered — once, the same way, for everybody (A22).
    fn runs(&self, path: &std::path::Path, bytes: u64) -> Value {
        let Some(file) = header_of(path) else {
            return Value::map([
                ("known", Value::Bool(false)),
                ("why", Value::text("MCF could not read this file's header")),
            ]);
        };
        let architecture = file.architecture().map(str::to_owned);
        let trained = architecture.as_ref().and_then(|held| {
            file.get(&format!("{held}.context_length"))
                .and_then(mcf_standin::gguf::Value::as_integer)
                .and_then(|value| u64::try_from(value).ok())
        });
        let cache = crate::engines::cache_bytes_per_token(&file);
        let shape = |value: Option<u64>| {
            value.map_or(Value::Null, |held| {
                Value::Integer(i64::try_from(held).unwrap_or(i64::MAX))
            })
        };
        let resolved = match trained {
            Some(trained) => crate::engines::resolve(&self.engines_now(), bytes, cache, trained)
                .map_or_else(
                    |refused| {
                        Value::map([
                            ("known", Value::Bool(false)),
                            ("why", Value::text(refused.says())),
                        ])
                    },
                    |choice| {
                        Value::map([
                            ("known", Value::Bool(true)),
                            ("engine", Value::text(choice.engine)),
                            ("device", Value::text(choice.device.name)),
                            (
                                "device_kind",
                                Value::text(match choice.device.kind {
                                    crate::engines::Kind::Cpu => "cpu",
                                    crate::engines::Kind::Gpu => "gpu",
                                }),
                            ),
                            ("context", shape(Some(choice.context))),
                        ])
                    },
                ),
            None => Value::map([
                ("known", Value::Bool(false)),
                (
                    "why",
                    Value::text(crate::engines::Refused::HeaderIncomplete.says()),
                ),
            ]),
        };
        Value::map([
            (
                "architecture",
                architecture.map_or(Value::Null, Value::text),
            ),
            // What was measured about it, from the record. Absent where
            // nothing has been — never a zero, which would read as a model
            // that produces nothing (A7).
            (
                "measured",
                self.last_measurement(path).unwrap_or(Value::Null),
            ),
            ("trained_context", shape(trained)),
            ("cache_bytes_per_token", shape(cache)),
            ("resolved", resolved),
        ])
    }

    /// What this daemon cannot do, in words a person can act on.
    ///
    /// It used to say one sentence with two rule identifiers in it, and it said
    /// it whether or not an engine was there — which was how two provisioned
    /// engines sat on this machine while the daemon reported none. A citation is
    /// for the record, where it can be followed; on a screen it is noise nobody
    /// can use.
    fn cannot(&self) -> Value {
        if self.engines.is_empty() {
            return Value::List(vec![Value::text(
                "run a model: no engine is installed yet — MCF can build one for you",
            )]);
        }
        Value::List(Vec::new())
    }

    /// What this daemon recovered, in the record's own shape.
    fn recovered_as_value(&self) -> Value {
        Value::map([
            (
                "socket",
                Value::text(self.places.socket.display().to_string()),
            ),
            (
                "record_entries",
                Value::Integer(i64::try_from(self.recovered.entries).unwrap_or(i64::MAX)),
            ),
            (
                "record_unreadable",
                match &self.recovered.unreadable {
                    Some(what) => Value::text(what.clone()),
                    None => Value::Null,
                },
            ),
            (
                "models_held",
                Value::Integer(i64::try_from(self.recovered.held).unwrap_or(i64::MAX)),
            ),
        ])
    }

    /// What this daemon recovered when it started.
    #[must_use]
    pub const fn recovered(&self) -> &Recovered {
        &self.recovered
    }

    /// Where it is listening.
    #[must_use]
    pub fn socket(&self) -> &Path {
        &self.places.socket
    }

    /// Answers clients until one asks it to stop.
    ///
    /// Blocks in `accept`, which is the whole of the idle discipline: a daemon
    /// with nothing to do is a process the scheduler is not running (§3.13,
    /// D24's zero wakeups).
    ///
    /// One connection at a time, and deliberately: DEC-012 has not settled what
    /// several clients at once means, and a daemon that guessed would be
    /// answering a question nobody has asked yet (§3.13's refusal of
    /// generality).
    ///
    /// The cost of that choice is stated rather than hidden: a client that
    /// connects and says nothing delays every other client by at most
    /// [`PATIENCE`], and then the daemon carries on. It cannot hold MCF for
    /// ever, which is the property B7 asks for; it can make somebody wait, and
    /// what fixes *that* is DEC-012 rather than a smaller number here. The
    /// socket is reachable only by this user (B-036), so the client that could
    /// do it is the operator's own.
    pub fn serve(&mut self) -> Stopped {
        loop {
            let connection = match self.listener.accept() {
                Ok((stream, _)) => stream,
                Err(error) => {
                    return Stopped::Broken {
                        failure: Box::new(unusable(
                            "the control socket",
                            &self.places.socket,
                            &error,
                        )),
                    };
                }
            };
            if let Some(stopped) = self.answer_one(&connection) {
                // **A model being held is let go in writing, before the daemon
                // is.** The engine does stop — dropping what holds it is what
                // stops it — but a record carrying `model_hosted` and never
                // `model_unhosted` says a model is still being served, and
                // somebody reading it later would believe that. The record
                // keeps these in pairs for exactly this reason: after the stop
                // the entry is the only thing that says anything was ever
                // listening (A26, A1, B-210).
                self.let_go("the daemon stopped");
                // A26: a stop has an account, and the account is in the record
                // rather than only in what the client was told.
                self.note(
                    EntryKind::DaemonStopped,
                    Timestamp::now(),
                    match &stopped {
                        Stopped::Asked { reason } => Value::map([
                            ("how", Value::text("asked")),
                            (
                                "reason",
                                if reason.is_empty() {
                                    Value::Null
                                } else {
                                    Value::text(reason.clone())
                                },
                            ),
                        ]),
                        Stopped::Broken { failure } => Value::map([
                            ("how", Value::text("broken")),
                            ("why", mcf_record::encode::failure(failure)),
                        ]),
                    },
                );
                return stopped;
            }
        }
    }

    /// Reads one request, answers it, and says whether that was the last.
    fn answer_one(&self, connection: &UnixStream) -> Option<Stopped> {
        // A client that connects and says nothing must not hold the daemon:
        // B7 makes a hang a defined outcome, and this is the one place a
        // stranger could cause one.
        let _deadline = connection.set_read_timeout(Some(PATIENCE));
        let _writing = connection.set_write_timeout(Some(PATIENCE));

        let mut line = String::new();
        // Bounded before it is read, not after: a client that sends a gigabyte
        // without a newline must not become a gigabyte in this process (§3.7).
        let ceiling = u64::try_from(REQUEST_CEILING.saturating_add(1)).unwrap_or(u64::MAX);
        let read = BufReader::new(std::io::Read::take(connection, ceiling)).read_line(&mut line);
        let mut writer = connection;

        // A request that filled the ceiling without ending is a request MCF
        // did not receive, and it has to be *told so*. Before this, the daemon
        // read its 64 kibibytes, failed to parse the fragment, and closed while
        // the client was still writing — so the client saw a connection reset
        // and no reason at all, which is the silent failure A2 forbids. The
        // probe that found it reported *a line of the stream was unreadable*,
        // which was true and useless (B-055, F42).
        if line.len() >= REQUEST_CEILING && !line.ends_with('\n') {
            let failure = Failure::new(
                Category::ConfigInvalid,
                Attribution::User,
                Disposition::Refused,
                Subsystem::new("mcf-serve::daemon"),
                "a request longer than this build will read in one line",
            )
            .with_context("ceiling_bytes", REQUEST_CEILING.to_string())
            .with_context(
                "what_to_do",
                "a turn of token identifiers this long exceeds what the control protocol \
                 carries; ask for fewer, or a build with a larger ceiling",
            );
            let answer = Answer::refused(&failure);
            let _written = writeln!(writer, "{}", answer.to_line());
            let _flushed = writer.flush();
            // The client is still writing, and closing now would lose the
            // answer to a reset. Its own write timeout ends this; MCF reads
            // nothing further into memory.
            let _shutdown = writer.shutdown(std::net::Shutdown::Read);
            return None;
        }

        let answer = match read {
            Err(_) | Ok(0) => return None,
            Ok(_) => match Request::read(line.trim_end()) {
                Ok(Request::Generate {
                    model,
                    prompt,
                    limit,
                    whose,
                    seed,
                    tokens,
                    engine,
                }) => {
                    // A generation is one request and many lines, so it has
                    // its own path: nothing about it fits in one `Answer`.
                    self.generate(
                        &model,
                        &prompt,
                        limit,
                        seed,
                        tokens.as_deref(),
                        engine.as_deref(),
                        whose,
                        &mut writer,
                    );
                    return None;
                }
                Ok(Request::Measure {
                    model,
                    engine,
                    deepest,
                }) => {
                    self.measuring(&model, engine.as_deref(), deepest, &mut writer);
                    return None;
                }
                Ok(Request::PromptReport {
                    model,
                    prompt,
                    seed,
                }) => {
                    // Many generations and one report: a request that takes
                    // minutes says what it is doing as it goes, for the same
                    // reason a measurement does — a client cannot tell a long
                    // run from a hung one (B-227).
                    self.prompt_report(&model, &prompt, seed, &mut writer);
                    return None;
                }
                Ok(Request::Acquire {
                    reference,
                    file,
                    from,
                }) => {
                    self.acquiring(&reference, &file, from.as_deref(), &mut writer);
                    return None;
                }
                Ok(request) => {
                    let (answer, stop) = self.respond(&request);
                    let _written = writeln!(writer, "{}", answer.to_line());
                    let _flushed = writer.flush();
                    return stop;
                }
                Err(failure) => Answer::refused(&failure),
            },
        };
        let _written = writeln!(writer, "{}", answer.to_line());
        let _flushed = writer.flush();
        None
    }

    /// A model answers a prompt, one line per token, then the account (B-034,
    /// PR9).
    ///
    /// **Loaded per request and dropped after.** Whether a served model stays
    /// resident when nobody is looking is DEC-018 and open; until it is
    /// decided, the daemon holds nothing between requests, which is the answer
    /// that costs nothing while idle (§3.13) and hides no choice (§3.15). The
    /// price is paid at the start of every generation and stated in the
    /// account as `loaded: per_request`.
    ///
    /// **What is recorded is the terminating line.** The same object the
    /// client was sent (D20): a client that ignores the conditions still leaves
    /// them behind, and one that hangs up mid-stream leaves the account of what
    /// it got (A4, A26).
    #[allow(
        clippy::too_many_arguments,
        reason = "one request's conditions, each named in the account"
    )]
    fn generate(
        &self,
        named: &str,
        prompt: &str,
        limit: Option<usize>,
        seed: u64,
        tokens: Option<&[usize]>,
        engine: Option<&str>,
        whose: mcf_record::content::Whose,
        writer: &mut &UnixStream,
    ) {
        let at = Timestamp::now();
        let mcf_home = self
            .places
            .models
            .parent()
            .map_or_else(|| self.places.models.clone(), Path::to_path_buf);
        // Which build and which device this model resolves to, so the engine
        // that is started is the one MCF said would run it. Without this,
        // discovery matched a name, found the processor build every time, and
        // every generation ran there under a label that said otherwise
        // (F133).
        let picked = self.picked_engine(named);
        let produced = crate::generation::serve_generation(
            &self.places.models,
            &mcf_home,
            &self.resident,
            &self.server,
            self.places
                .socket
                .parent()
                .unwrap_or_else(|| Path::new("/tmp")),
            named,
            prompt,
            limit,
            seed,
            tokens,
            engine,
            picked,
            system_memory_free(),
            writer,
        );
        // The account goes to the record and what the model said goes to the
        // content store, filed under the entry the record just wrote (A25,
        // §6.8, F105). The record first, because the key is its identifier —
        // and a process that dies between the two leaves an entry saying how
        // many bytes were said with nothing filed under it, which
        // `disclose_kept` answers as absent. That is a state, and it is the one
        // A7 wants: *not kept* rather than a plausible empty string.
        // Whose text this was is a *condition* of the generation, not content:
        // a turn MCF asked for is a different experiment from one a person
        // asked for, and a reader of the record is entitled to tell them apart
        // (§3.4, B-146).
        let account = match produced.account {
            Value::Map(mut fields) => {
                if let Some(Value::Map(conditions)) = fields.get_mut("conditions") {
                    conditions.insert("asked_by".to_owned(), Value::text(whose.as_str()));
                }
                Value::Map(fields)
            }
            account => account,
        };
        let Some(id) = self.note(EntryKind::Generated, at, account) else {
            return;
        };
        let Some(said) = produced.said else { return };
        // Filed in the store its category names: a person's text and MCF's own
        // do not share a directory (§6.8, B-146, F114).
        let store = match mcf_record::content::ContentStore::open_for(
            &mcf_record::content::ContentStore::beside_for(&self.places.journal, whose),
            whose,
        ) {
            Ok(store) => store,
            Err(failure) => {
                eprintln!("mcf: what the model said could not be filed: {failure}");
                return;
            }
        };
        if let Err(failure) = store.keep(id.as_str(), &mcf_record::content::Content::new(said.text))
        {
            eprintln!("mcf: what the model said could not be filed: {failure}");
        }
    }

    /// Takes a prompt apart and says which of it reached the answer.
    ///
    /// **One generation per sentence, plus the seeds.** Expensive by
    /// construction and never done on the way past: what it buys is the one
    /// question a person cannot answer by looking at their own prompt — which
    /// of it the model actually used (§3.8).
    ///
    /// Each generation runs the same path a client's request runs, with the
    /// stream drained rather than suppressed — timing MCF rather than
    /// something that resembles it is the same reason a measurement does it
    /// this way (A11, A12).
    fn prompt_report(&self, named: &str, prompt: &str, seed: u64, writer: &mut &UnixStream) {
        let mcf_home = self
            .places
            .models
            .parent()
            .map_or_else(|| self.places.models.clone(), Path::to_path_buf);
        let picked = self.picked_engine(named);
        let mut asked = 0_usize;
        let mut ask = |prompt: &str, seed: u64| {
            asked = asked.saturating_add(1);
            let Ok((mine, theirs)) = UnixStream::pair() else {
                return String::new();
            };
            let drain = std::thread::spawn(move || {
                let mut end = &theirs;
                let _emptied = std::io::copy(&mut end, &mut std::io::sink());
            });
            let produced = {
                let mut into = &mine;
                crate::generation::serve_generation(
                    &self.places.models,
                    &mcf_home,
                    &self.resident,
                    &self.server,
                    self.places
                        .socket
                        .parent()
                        .unwrap_or_else(|| Path::new("/tmp")),
                    named,
                    prompt,
                    Some(PROMPT_REPORT_LIMIT),
                    seed,
                    None,
                    None,
                    picked.clone(),
                    system_memory_free(),
                    &mut into,
                )
            };
            drop(mine);
            let _joined = drain.join();
            produced.said.map(|held| held.text).unwrap_or_default()
        };
        let report = crate::prompt::measure(prompt, seed, &mut ask);
        let answer = Answer::served(prompt_report_value(&report, asked));
        let _written = writeln!(writer, "{}", answer.to_line());
        let _flushed = writer.flush();
    }

    /// What MCF says to each request.
    fn respond(&self, request: &Request) -> (Answer, Option<Stopped>) {
        match request {
            Request::Status => (Answer::served(self.status()), None),
            Request::Holding => (Answer::served(self.holding()), None),
            Request::Components => (Answer::served(self.components()), None),
            Request::Offered { reference, from } => {
                (Self::offered(reference, from.as_deref()), None)
            }
            Request::Settings { model } => (self.settings_for(model), None),
            Request::Host { model, settings } => (self.host(model, settings), None),
            Request::Hosted => (Answer::served(self.hosted()), None),
            Request::Unhost => (Answer::served(self.unhost()), None),
            // Handled before `respond` is reached; here so the match is
            // total and a future request type is a compile error rather than a
            // silent fall-through.
            Request::Generate { .. }
            | Request::Acquire { .. }
            | Request::Measure { .. }
            | Request::PromptReport { .. } => (
                Answer::refused(&crate::control::refused(
                    "a request that answers in many lines reached the one-answer path",
                    "generate, acquire or measure",
                )),
                None,
            ),
            Request::Stop { reason } => (
                Answer::served(Value::map([
                    ("stopping", Value::Bool(true)),
                    ("reason", Value::text(reason.clone())),
                ])),
                Some(Stopped::Asked {
                    reason: reason.clone(),
                }),
            ),
        }
    }

    /// Times a model on this machine, at doubling depths.
    ///
    /// **Two runs a depth, and the difference is the answer.** A single timed
    /// generation at depth *d* measures three things at once: loading the
    /// model, reading *d* tokens of prompt, and producing the tokens asked
    /// for. Only the third is what *speed at depth* means. So each depth is
    /// run twice — once producing one token, once producing seventeen — and
    /// the per-token cost is the difference over sixteen. Whatever the load
    /// and the prefill cost, they are in both and cancel.
    ///
    /// That matters more here than it usually would: the daemon loads a model
    /// for every request and drops it after (DEC-018), so an unsubtracted
    /// reading at a shallow depth would be mostly the loading.
    ///
    /// **The ladder is powers of two and every rung is climbed.** A run that
    /// measured 8 192 and skipped 2 048 would have no shallow point to read
    /// the deep one against, which is the whole of what a fall-off is.
    ///
    /// **An estimate goes out before any of it starts**, as a range, so that
    /// somebody can decide not to wait. MCF's own estimates land between
    /// 0.58× and 1.42× of what runs take, and a single number would be a
    /// promise it cannot keep.
    fn measuring(&self, named: &str, engine: Option<&str>, deepest: u64, writer: &mut &UnixStream) {
        let say = |writer: &mut &UnixStream, answer: &Answer| {
            let _written = writeln!(writer, "{}", answer.to_line());
            let _flushed = writer.flush();
        };

        let ladder = depth_ladder(deepest);
        if ladder.is_empty() {
            return say(
                writer,
                &Answer::refused(&crate::control::refused(
                    "a depth below the shallowest MCF measures",
                    &deepest.to_string(),
                )),
            );
        }
        let path = crate::generation::resolved(&self.places.models, named);
        let held = std::fs::metadata(&path).map(|about| about.len()).ok();

        // Before anything runs. The estimate is arithmetic over the ladder and
        // the model's size, and it is a range because it is an estimate (A6).
        let guess = estimated_seconds(&ladder, held);
        say(
            writer,
            &Answer::served(Value::map([
                ("measuring", Value::text(named.to_owned())),
                (
                    "depths",
                    Value::List(
                        ladder
                            .iter()
                            .map(|depth| Value::Integer(i64::try_from(*depth).unwrap_or(i64::MAX)))
                            .collect(),
                    ),
                ),
                (
                    "estimate_low_seconds",
                    Value::Integer(i64::try_from(guess.0).unwrap_or(i64::MAX)),
                ),
                (
                    "estimate_high_seconds",
                    Value::Integer(i64::try_from(guess.1).unwrap_or(i64::MAX)),
                ),
                ("done", Value::Bool(false)),
            ])),
        );

        // The build and the device the model resolves to decide how the
        // engine is started, so a measurement is taken on the device it says
        // it was taken on (A6, A12, F133).
        let picked = self.picked_engine(named);
        let mut readings: Vec<Value> = Vec::new();
        let mut ran_on: Option<String> = None;
        for depth in &ladder {
            let (reading, engine) = self.one_depth(named, engine, *depth, picked.as_ref());
            ran_on = ran_on.take().or(engine);
            readings.push(reading.clone());
            say(
                writer,
                &Answer::served(Value::map([
                    ("measuring", Value::text(named.to_owned())),
                    ("reading", reading),
                    (
                        "of",
                        Value::Integer(i64::try_from(ladder.len()).unwrap_or(i64::MAX)),
                    ),
                    (
                        "so_far",
                        Value::Integer(i64::try_from(readings.len()).unwrap_or(i64::MAX)),
                    ),
                    ("done", Value::Bool(false)),
                ])),
            );
        }

        let last = measured(named, &path, held, engine, ran_on.as_deref(), readings);
        // Written down as it is sent, the way a generation's account is. A
        // measurement nobody can find later is the same as one not taken
        // (A1), and until this existed a model's page said `Unknown` about
        // speed the moment a run finished.
        let _recorded = self.note(EntryKind::ModelTimed, Timestamp::now(), last.body.clone());
        // And into what the daemon holds, so the next listing has it without
        // re-reading the record.
        if let Ok(mut timings) = self.timings.lock() {
            let _replaced = timings.insert(path.clone(), last.body.clone());
        }
        say(writer, &last);
    }

    /// The engine and layer count this model resolves to.
    ///
    /// `None` where MCF cannot work it out — a header it could not read, no
    /// provisioned engine — and the generation path then falls back to its own
    /// discovery, which is what it did before there was anything to resolve.
    fn picked_engine(
        &self,
        named: &str,
    ) -> Option<(crate::adapters::ProvisionedLlama, u32, u64)> {
        let (recommended, _) = self.recommend(named).ok()?;
        let (engine, _) = self
            .engines
            .iter()
            .find(|(engine, _)| engine.name == recommended.engine)?;
        Some((
            crate::adapters::ProvisionedLlama {
                prefix: engine.prefix.clone(),
                commit: engine.commit.clone(),
                component: engine.name.clone(),
            },
            recommended.gpu_layers,
            // The window MCF resolved for this model on this machine, so the
            // engine opens the one that was planned against rather than the
            // whole trained context.
            recommended.context,
        ))
    }

    /// The most recent measurement of a model, from the record.
    ///
    /// The newest wins, and nothing is merged: two runs under different
    /// settings are two facts, and averaging them would produce a figure
    /// neither run produced (A1, A6).
    fn last_measurement(&self, model: &Path) -> Option<Value> {
        let held = self.timings.lock().ok()?;
        held.get(model).cloned()
    }

    /// One rung of the ladder: the repeats, the median, and what ran them.
    fn one_depth(
        &self,
        named: &str,
        engine: Option<&str>,
        depth: u64,
        picked: Option<&(crate::adapters::ProvisionedLlama, u32, u64)>,
    ) -> (Value, Option<String>) {
        // Repeats, because one pair is one sample and a fall-off read off
        // single samples is a reading of the noise. The median is taken
        // rather than the mean: a run that hit a scheduler hiccup should not
        // move the answer (F53).
        let mut samples: Vec<u64> = Vec::new();
        let mut first_token: Vec<u64> = Vec::new();
        let mut ran_on: Option<String> = None;
        for _ in 0..REPEATS {
            let one = self.timed_generation(named, engine, depth, 1, picked.cloned());
            let many = self.timed_generation(named, engine, depth, 1 + SETTLED, picked.cloned());
            if let Some(short) = &one {
                ran_on = ran_on.take().or_else(|| short.engine.clone());
            }
            if let (Some(short), Some(long)) = (&one, &many)
                && long.ns > short.ns
            {
                #[expect(
                    clippy::integer_division,
                    reason = "a difference in nanoseconds over sixteen tokens; the \
                              remainder is under a nanosecond a token"
                )]
                let per_token = (long.ns - short.ns) / u64::from(SETTLED);
                samples.push(per_token);
                first_token.push(short.ns);
            }
        }
        let at_depth = Value::Integer(i64::try_from(depth).unwrap_or(i64::MAX));
        let reading = match middle(&mut samples) {
            Some(per_token) => {
                let spread = samples
                    .last()
                    .copied()
                    .unwrap_or(per_token)
                    .saturating_sub(samples.first().copied().unwrap_or(per_token));
                Value::map([
                    ("depth", at_depth),
                    ("ms_per_token", Value::text(as_milliseconds(per_token))),
                    (
                        "first_token_ms",
                        middle(&mut first_token)
                            .map_or(Value::Null, |ns| Value::text(as_milliseconds(ns))),
                    ),
                    ("spread_ms", Value::text(as_milliseconds(spread))),
                    (
                        "samples",
                        Value::Integer(i64::try_from(samples.len()).unwrap_or(i64::MAX)),
                    ),
                    ("measured", Value::Bool(true)),
                ])
            }
            // No pair came back in the right order, so nothing here is a
            // per-token cost. The honest reading is that there is none —
            // never a zero, and never the unsubtracted number standing in for
            // the subtracted one (A7, A9).
            None => Value::map([
                ("depth", at_depth),
                ("measured", Value::Bool(false)),
                (
                    "why",
                    Value::text(
                        "no pair of runs at this depth separated: the longer one finished no \
                         later than the shorter, so their difference is not a cost",
                    ),
                ),
            ]),
        };
        (reading, ran_on)
    }

    /// One generation, timed, or `None` if it produced nothing.
    ///
    /// The prompt is a run of identifiers rather than text: what is being
    /// measured is depth, and depth is a count of tokens. Sending text would
    /// make the reading depend on how the text happened to segment.
    fn timed_generation(
        &self,
        named: &str,
        engine: Option<&str>,
        depth: u64,
        produce: u32,
        picked: Option<(crate::adapters::ProvisionedLlama, u32, u64)>,
    ) -> Option<Timed> {
        let how_many = usize::try_from(depth).ok()?;
        // Identifier 1 is inside every vocabulary MCF can address. What it
        // means does not matter; that there are `depth` of them does.
        let tokens: Vec<usize> = vec![1; how_many];
        let mcf_home = self
            .places
            .models
            .parent()
            .map_or_else(|| self.places.models.clone(), Path::to_path_buf);

        // A generation streams its tokens to whoever asked. Nothing is
        // asking here — what is wanted is how long it took — so the far end
        // of a socket pair is handed over and drained. This runs the
        // *same* generation path a client's request runs, rather than a
        // second one written to be measured, which is the difference between
        // timing MCF and timing something that resembles it (A11, A12).
        let (mine, theirs) = UnixStream::pair().ok()?;
        let drain = std::thread::spawn(move || {
            let mut end = &theirs;
            let _emptied = std::io::copy(&mut end, &mut std::io::sink());
        });

        let clock = SystemClock;
        let started = clock.now();
        let produced = {
            let mut writer = &mine;
            crate::generation::serve_generation(
                &self.places.models,
                &mcf_home,
                &self.resident,
                &self.server,
                self.places
                    .socket
                    .parent()
                    .unwrap_or_else(|| Path::new("/tmp")),
                named,
                "",
                Some(usize::try_from(produce).unwrap_or(1)),
                0,
                Some(&tokens),
                engine,
                picked,
                system_memory_free(),
                &mut writer,
            )
        };
        let took = clock.now().saturating_duration_since(started);
        drop(mine);
        let _joined = drain.join();

        produced.said.is_some().then(|| Timed {
            ns: took.as_nanos(),
            engine: produced
                .account
                .get("conditions")
                .and_then(|conditions| conditions.get("engine"))
                .and_then(Value::as_text)
                .map(str::to_owned),
        })
    }

    /// Fetches one published file, saying how far along it is as it goes.
    ///
    /// **Progress is read off the disk, not reported by the transfer.** The
    /// bytes land in a partial file whose size is the answer, so what a window
    /// shows is the file that exists rather than a count something kept. A
    /// transfer that claimed more than it had written would be exactly the
    /// kind of claim §3.7 says not to take on trust — and this cannot make
    /// that claim, because it is not the thing counting.
    ///
    /// One line a second while it runs, then one last line saying where the
    /// model went and what was written down about it.
    fn acquiring(&self, reference: &str, file: &str, from: Option<&str>, writer: &mut &UnixStream) {
        let say = |writer: &mut &UnixStream, answer: &Answer| {
            let _written = writeln!(writer, "{}", answer.to_line());
            let _flushed = writer.flush();
        };

        let parsed = match mcf_hub::reference::parse(reference) {
            Ok(parsed) => parsed,
            Err(failure) => return say(writer, &Answer::refused(&failure)),
        };
        let base = match mcf_hub::http::Url::parse(from.unwrap_or(DEFAULT_HUB)) {
            Ok(base) => base,
            Err(failure) => return say(writer, &Answer::refused(&failure)),
        };
        let base_again = base.clone();
        let wire = match mcf_hub::wire::for_url(&base) {
            Ok(wire) => wire,
            Err(failure) => return say(writer, &Answer::refused(&failure)),
        };
        let hub = mcf_hub::client::Hub::at(base, wire);
        let listing = match hub.list(&parsed) {
            Ok(listing) => listing,
            Err(failure) => return say(writer, &Answer::refused(&failure)),
        };
        let Some(entry) = listing.entry(file).cloned() else {
            return say(
                writer,
                &Answer::refused(&crate::control::refused(
                    "a file this repository does not publish",
                    file,
                )),
            );
        };
        let root = self.places.models.clone();
        let arriving = mcf_hub::acquisition::arriving_at(&root, &listing, &entry);
        let total = entry.size;

        say(
            writer,
            &Answer::served(Value::map([
                ("acquiring", Value::text(entry.path.clone())),
                (
                    "bytes",
                    Value::Integer(i64::try_from(total).unwrap_or(i64::MAX)),
                ),
                ("doing", Value::text("fetching")),
                ("done", Value::Bool(false)),
            ])),
        );

        // The transfer runs on its own thread so that this one can keep
        // saying how far it has got. The connection is not shared — a `Wire`
        // is not `Send`, and making one would have been a change to the
        // network layer for the sake of a progress bar. The thread opens its
        // own, which is one more connection to a hub that was going to be
        // asked for a file anyway.
        drop(hub);
        let (listing_for_thread, entry_for_thread) = (listing.clone(), entry.clone());
        let where_from = base_again.clone();
        let handle = std::thread::spawn(move || {
            let wire = mcf_hub::wire::for_url(&where_from)?;
            let hub = mcf_hub::client::Hub::at(where_from, wire);
            mcf_hub::acquisition::one(&hub, &listing_for_thread, &entry_for_thread, &root)
        });

        // The last size actually seen. A partial file that is not there is
        // not a transfer that has delivered nothing — at the start it has not
        // been created, and at the end it has been renamed to its
        // destination. Reporting zero for either would put a progress bar
        // back to the beginning at the moment it finished, which is the same
        // error as reporting an unknown as a measurement (A7).
        let mut furthest = 0_u64;
        while !handle.is_finished() {
            std::thread::sleep(std::time::Duration::from_millis(700));
            if let Ok(about) = std::fs::metadata(&arriving) {
                furthest = furthest.max(about.len());
            }
            // Once every byte is here the transfer is not over: what remains
            // is reading the whole file back to check its digest, which on a
            // large model takes longer than a person will wait without being
            // told what is happening (A2).
            let checking = furthest >= total && total > 0;
            say(
                writer,
                &Answer::served(Value::map([
                    ("acquiring", Value::text(entry.path.clone())),
                    (
                        "arrived",
                        Value::Integer(i64::try_from(furthest).unwrap_or(i64::MAX)),
                    ),
                    (
                        "bytes",
                        Value::Integer(i64::try_from(total).unwrap_or(i64::MAX)),
                    ),
                    (
                        "doing",
                        Value::text(if checking { "checking" } else { "fetching" }),
                    ),
                    ("done", Value::Bool(false)),
                ])),
            );
        }

        let answer = match handle.join() {
            Ok(Ok(done)) => acquired(&entry.path, &done),
            Ok(Err(failure)) => Answer::refused(&failure),
            // A thread that died left no failure to report, and saying
            // nothing would leave a window waiting forever (A2).
            Err(_) => Answer::refused(&crate::control::refused(
                "the transfer stopped without saying why",
                &entry.path,
            )),
        };
        say(writer, &answer);
    }

    /// What MCF would run a model under, and what it recommends.
    ///
    /// Nothing is started. A surface asks this to fill in a form.
    fn settings_for(&self, named: &str) -> Answer {
        match self.recommend(named) {
            Ok((recommended, _)) => Answer::served(Value::map([
                ("model", Value::text(named.to_owned())),
                ("recommended", recommended.to_value()),
                ("settings", recommended.to_value()),
                // **What the chosen window will actually reserve.** The
                // recommendation is the largest window that fits, and on a
                // large machine that is the model's whole trained context:
                // 262,144 tokens reserved 54.6 GiB of cache for a 17.6 GB
                // model, three times the model itself, and nothing on any
                // screen said so before it was spent. §3.15 asks that MCF
                // doing other than the plain thing be visible; a figure is
                // what makes it visible.
                (
                    "cache_bytes",
                    self.cache_for(named, recommended.context)
                        .map_or(Value::Null, |bytes| {
                            Value::Integer(i64::try_from(bytes).unwrap_or(i64::MAX))
                        }),
                ),
                (
                    "explains",
                    Value::List(
                        recommended
                            .listed(&recommended)
                            .into_iter()
                            .map(|setting| {
                                Value::map([
                                    ("name", Value::text(setting.name)),
                                    ("value", Value::text(setting.value)),
                                    ("recommended", Value::text(setting.recommended)),
                                    ("because", Value::text(setting.because)),
                                ])
                            })
                            .collect(),
                    ),
                ),
            ])),
            Err(failure) => Answer::refused(&failure),
        }
    }

    /// What MCF recommends for a model, and the path it resolved to.
    /// What a context window of this size costs this model in memory.
    ///
    /// `None` where the header does not say enough to work it out — which is a
    /// state, and better than a figure MCF assembled from a guess (A7).
    fn cache_for(&self, named: &str, context: u64) -> Option<u64> {
        let path = crate::generation::resolved(&self.places.models, named);
        let file = header_of(&path)?;
        let per_token = crate::engines::cache_bytes_per_token(&file)?;
        Some(per_token.saturating_mul(context))
    }

    fn recommend(&self, named: &str) -> Result<(crate::hosting::Hosting, PathBuf)> {
        let path = crate::generation::resolved(&self.places.models, named);
        let bytes = std::fs::metadata(&path)
            .map(|about| about.len())
            .map_err(|error| {
                crate::control::refused("a model this machine is not holding", &error.to_string())
            })?;
        let file = header_of(&path).ok_or_else(|| {
            crate::control::refused("a file whose header MCF could not read", named)
        })?;
        let architecture = file.architecture().map(str::to_owned);
        let trained = architecture.as_ref().and_then(|held| {
            file.get(&format!("{held}.context_length"))
                .and_then(mcf_standin::gguf::Value::as_integer)
                .and_then(|value| u64::try_from(value).ok())
        });
        let cache = crate::engines::cache_bytes_per_token(&file);
        let trained = trained.ok_or_else(|| {
            crate::control::refused(
                "a model whose header does not say how long a conversation it was trained for",
                named,
            )
        })?;
        let choice = crate::engines::resolve(&self.engines_now(), bytes, cache, trained)
            .map_err(|refused| crate::control::refused(&refused.says(), named))?;
        let on_a_card = matches!(choice.device.kind, crate::engines::Kind::Gpu);
        // Whether the whole thing fits where it is going: the weights plus
        // the cache at the window MCF settled on. This is the figure the
        // layer recommendation turns on, and it is arithmetic rather than a
        // guess (A6).
        let wanted = bytes.saturating_add(cache.unwrap_or(0).saturating_mul(choice.context));
        let fits = choice.device.free.is_none_or(|free| wanted <= free);
        Ok((
            crate::hosting::Hosting::recommended(
                &choice.engine,
                &choice.device.name,
                on_a_card,
                choice.context,
                std::thread::available_parallelism().ok().map(Into::into),
                fits,
            ),
            path,
        ))
    }

    /// Holds a model and answers on a port under these settings.
    fn host(&self, named: &str, asked: &Value) -> Answer {
        let (recommended, path) = match self.recommend(named) {
            Ok(held) => held,
            Err(failure) => return Answer::refused(&failure),
        };
        let settings = crate::hosting::Hosting::from_value(asked, &recommended);

        // **The engine named in the settings, not whichever one is found.**
        // Looking one up by shape returned the processor build while the
        // settings said the CUDA one — so MCF would have resolved a model to
        // a card, said so, and started the build that cannot use it. That is
        // the same defect as the hardcoded layer count, one level up (F133).
        let Some((engine, _)) = self
            .engines
            .iter()
            .find(|(engine, _)| engine.name == settings.engine)
        else {
            return Answer::refused(&crate::control::refused(
                "no provisioned engine by that name: `mcf provision` builds one",
                &settings.engine,
            ));
        };
        let llama = crate::adapters::ProvisionedLlama {
            prefix: engine.prefix.clone(),
            commit: engine.commit.clone(),
            component: engine.name.clone(),
        };
        // Whatever was held before goes first: two servers on one port is a
        // second that never starts, and two on one card is two figures each
        // about the other (A6). **And it is let go in writing**, through the
        // one place that does that — dropping it quietly here would leave a
        // record with two hostings and one release, which reads as a model
        // still being served (A1, A26, B-210).
        let _released = self.let_go("another model was hosted in its place");
        let mut holding = match self.holding.lock() {
            Ok(holding) => holding,
            Err(poisoned) => poisoned.into_inner(),
        };

        // And then the port, before anything is spawned. An engine that
        // cannot bind exits with a status and no sentence, and reporting
        // *the server stopped before it began answering* would be a true
        // report of the wrong thing (A2).
        if !settings.port_is_free() {
            return Answer::refused(&crate::control::refused(
                "something is already listening on that port, so MCF did not start a second \
                 thing there — choose another port, or stop what is on it",
                &settings.port.to_string(),
            ));
        }

        match crate::served::Served::hosted(&llama, &path, &settings) {
            Ok(served) => {
                let at = Timestamp::now();
                let moved = settings.differs_from(&recommended);
                // §6.12: network exposure is an explicit act rather than a
                // side effect, and an act nobody wrote down is
                // indistinguishable from a side effect. This is the one thing
                // MCF does that another program can see, and it goes on doing
                // it after the request that started it has returned.
                let _recorded = self.note(
                    EntryKind::ModelHosted,
                    at,
                    Value::map([
                        ("model", Value::text(path.display().to_string())),
                        ("address", Value::text(settings.address())),
                        // Stated rather than implied: what a hosted model is
                        // reachable from is the question §6.12 asks, and the
                        // answer is on this machine and nowhere else.
                        ("reachable_from", Value::text("this computer only")),
                        ("settings", settings.to_value()),
                        ("recommended", recommended.to_value()),
                        (
                            "changed",
                            Value::List(moved.iter().cloned().map(Value::text).collect()),
                        ),
                    ]),
                );
                *holding = Some(Holding {
                    served,
                    model: path.clone(),
                    settings: settings.clone(),
                    recommended: recommended.clone(),
                    since: at,
                });
                Answer::served(Value::map([
                    ("hosting", Value::text(path.display().to_string())),
                    ("address", Value::text(settings.address())),
                    ("settings", settings.to_value()),
                    ("recommended", recommended.to_value()),
                    // What was moved off the recommendation, in writing,
                    // because a run under a changed setting is not a run
                    // under the recommended one (§3.15, A6).
                    (
                        "changed",
                        Value::List(moved.into_iter().map(Value::text).collect()),
                    ),
                    ("since", Value::text(at.to_string())),
                ]))
            }
            Err(failure) => Answer::refused(&failure),
        }
    }

    /// What is being held, if anything.
    fn hosted(&self) -> Value {
        let holding = match self.holding.lock() {
            Ok(holding) => holding,
            Err(poisoned) => poisoned.into_inner(),
        };
        match holding.as_ref() {
            None => Value::map([("hosting", Value::Null)]),
            Some(held) => Value::map([
                ("hosting", Value::text(held.model.display().to_string())),
                ("address", Value::text(held.settings.address())),
                ("settings", held.settings.to_value()),
                ("recommended", held.recommended.to_value()),
                (
                    "changed",
                    Value::List(
                        held.settings
                            .differs_from(&held.recommended)
                            .into_iter()
                            .map(Value::text)
                            .collect(),
                    ),
                ),
                ("since", Value::text(held.since.to_string())),
            ]),
        }
    }

    /// Lets go of whatever is being held, and says so in the record.
    ///
    /// Returns what was let go, or `None` where nothing was.
    fn let_go(&self, why: &str) -> Option<String> {
        let mut holding = match self.holding.lock() {
            Ok(holding) => holding,
            Err(poisoned) => poisoned.into_inner(),
        };
        let was = holding.take().map(|held| held.model.display().to_string());
        if let Some(model) = was.clone() {
            let _recorded = self.note(
                EntryKind::ModelUnhosted,
                Timestamp::now(),
                Value::map([
                    ("model", Value::text(model)),
                    ("reason", Value::text(why.to_owned())),
                ]),
            );
        }
        was
    }

    /// Stops holding it, because somebody asked.
    fn unhost(&self) -> Value {
        let was = self.let_go("asked");
        Value::map([
            ("stopped", Value::Bool(was.is_some())),
            ("was", was.map_or(Value::Null, Value::text)),
        ])
    }

    /// What a repository publishes, and which of it will run on this machine.
    ///
    /// **It fetches nothing.** A listing and a configuration are read; no
    /// weights move. That is what makes it safe to send while somebody is
    /// still typing a name, and it is why the window can show what a
    /// repository holds before anybody has committed to a download.
    ///
    /// **A gated repository is a refusal with a reason**, not an empty list. A
    /// person who is told nothing is published concludes something false about
    /// the repository; a person told it needs a token knows what to do (A2,
    /// A7).
    fn offered(reference: &str, from: Option<&str>) -> Answer {
        let parsed = match mcf_hub::reference::parse(reference) {
            Ok(parsed) => parsed,
            Err(failure) => return Answer::refused(&failure),
        };
        let base = match mcf_hub::http::Url::parse(from.unwrap_or(DEFAULT_HUB)) {
            Ok(base) => base,
            Err(failure) => return Answer::refused(&failure),
        };
        let wire = match mcf_hub::wire::for_url(&base) {
            Ok(wire) => wire,
            Err(failure) => return Answer::refused(&failure),
        };
        let hub = mcf_hub::client::Hub::at(base, wire);
        let listing = match hub.list(&parsed) {
            Ok(listing) => listing,
            Err(failure) => return Answer::refused(&failure),
        };
        // What is free, read the way the daemon reads it for everything else
        // — B4 keeps hardware sampling out of the serving path, so the plan
        // is handed a number rather than going and taking one.
        let (planned, from_the_header) = plan_however_the_shape_can_be_found(&hub, &listing);

        // Every published file MCF can read, with what is known about each.
        // The verdict is attached where there is one and left absent where
        // there is not — a file with no verdict is not a file that will not
        // run (A7).
        let verdicts: std::collections::BTreeMap<String, &mcf_hub::fitment::Verdict> = planned
            .as_ref()
            .map(|plan| {
                plan.verdicts
                    .iter()
                    .map(|(name, verdict)| (name.clone(), verdict))
                    .collect()
            })
            .unwrap_or_default();

        let files: Vec<Value> = listing
            .entries
            .iter()
            .filter(|entry| {
                std::path::Path::new(&entry.path)
                    .extension()
                    .is_some_and(|extension| extension.eq_ignore_ascii_case("gguf"))
            })
            .map(|entry| {
                let verdict = verdicts.get(&entry.path);
                Value::map([
                    ("file", Value::text(entry.path.clone())),
                    (
                        "bytes",
                        Value::Integer(i64::try_from(entry.size).unwrap_or(i64::MAX)),
                    ),
                    (
                        "fits",
                        verdict.map_or(Value::Null, |verdict| {
                            Value::Bool(matches!(verdict, mcf_hub::fitment::Verdict::Fits { .. }))
                        }),
                    ),
                    (
                        "why",
                        verdict.map_or(Value::Null, |verdict| Value::text(said_of(verdict))),
                    ),
                ])
            })
            .collect();

        Answer::served(Value::map([
            ("repository", Value::text(listing.reference.repository())),
            (
                "revision",
                listing.revision.clone().map_or(Value::Null, Value::text),
            ),
            ("files", Value::List(files)),
            // **Before the download, not after it.** B-023 asks that terms be
            // surfaced before use, and downloading is a use: a person who
            // learns what a model's licence is once it is on their disk has
            // learned it too late to decide. The three states stay distinct
            // and none of them is a default — an identifier MCF recognises,
            // terms present that it could not identify, and nothing declared
            // at all, which is `Unknown` and never a plausible guess (A7).
            (
                "terms",
                Value::text(mcf_hub::licence::describe(
                    listing
                        .declared_licence
                        .as_deref()
                        .and_then(mcf_hub::licence::recognize)
                        .as_ref(),
                )),
            ),
            (
                "planned_at_context",
                Value::Integer(i64::try_from(mcf_hub::offer::PLANNING_CONTEXT).unwrap_or(i64::MAX)),
            ),
            (
                "no_plan",
                match &planned {
                    Ok(_) => Value::Null,
                    Err(why) => Value::text(why.clone()),
                },
            ),
            // Where the shape came from is a condition of every verdict above
            // it: a header read from a prefix says how many blocks cache, and
            // unlike a configuration it cannot say which of them are
            // full-attention — so a hybrid model's cache is overstated and the
            // verdict errs toward refusing something that would fit (A6, A7).
            // Where the shape came from, and `null` where none was found —
            // saying *the configuration* about a plan that was never made
            // would be naming a source for an answer that does not exist
            // (A7).
            (
                "shape_from",
                match (planned.is_ok(), from_the_header) {
                    (false, _) => Value::Null,
                    (true, true) => Value::text(
                        "what the model's own header declares, read from its first megabytes \
                         and checked against the published size — the weights have not been \
                         read",
                    ),
                    (true, false) => Value::text("what the repository's configuration declares"),
                },
            ),
        ]))
    }

    /// What this daemon is.
    ///
    /// Including what it will not do, because a status that listed only
    /// capabilities would leave a reader to infer the rest — and the thing to
    /// infer today is that MCF cannot serve a model (A19, C7).
    fn status(&self) -> Value {
        let identity = BuildIdentity::current();
        Value::map([
            ("protocol", Value::Integer(VERSION)),
            ("build", mcf_record::encode::build_identity(identity)),
            ("resident", self.resident_value()),
            // What another program can see. Everything else in this answer is
            // about what MCF is holding for itself; this is the one thing it
            // is holding for anybody else, and a status that omitted it would
            // leave the most consequential fact about the process to be found
            // by looking at the ports (§6.12, B-418).
            ("hosting", self.hosted()),
            ("started_at", mcf_record::encode::timestamp(self.started)),
            (
                "up_nanoseconds",
                Value::Integer(
                    i64::try_from(
                        SystemClock
                            .now()
                            .saturating_duration_since(self.since)
                            .as_nanos(),
                    )
                    .unwrap_or(i64::MAX),
                ),
            ),
            (
                "socket",
                Value::text(self.places.socket.display().to_string()),
            ),
            (
                "recovered",
                Value::map([
                    (
                        "record_entries",
                        Value::Integer(i64::try_from(self.recovered.entries).unwrap_or(i64::MAX)),
                    ),
                    (
                        "record_unreadable",
                        match &self.recovered.unreadable {
                            Some(what) => Value::text(what.clone()),
                            None => Value::Null,
                        },
                    ),
                    (
                        "models_held",
                        Value::Integer(i64::try_from(self.recovered.held).unwrap_or(i64::MAX)),
                    ),
                ]),
            ),
            ("engines", self.engines_as_value()),
            ("cannot", self.cannot()),
        ])
    }

    /// The model held between requests, or `Null` (D41).
    ///
    /// Said in status because memory held is the price of residency, and a
    /// price nobody can see is a hidden choice (§3.15).
    fn resident_value(&self) -> Value {
        let held = self
            .resident
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        match held.as_ref() {
            Some(resident) => resident.describe(),
            None => Value::Null,
        }
    }

    /// What this machine is holding, read from the disk rather than remembered.
    /// What MCF can build, and which of it is here.
    ///
    /// **The catalogue and the disk, paired.** The catalogue
    /// ([`mcf_core::component::COMPONENTS`]) is what MCF knows how to build;
    /// the prefix directory beside the model store is what has actually been
    /// built. A surface needs both, because *not provisioned* is a thing to
    /// say rather than an absence to leave a person guessing at (A7).
    ///
    /// Read-only. Building is a command, not a request.
    fn components(&self) -> Value {
        let mcf_home = self
            .places
            .models
            .parent()
            .map_or_else(|| self.places.models.clone(), Path::to_path_buf);
        let under = mcf_home.join("provisioned");
        let engines = crate::engines::discover(&mcf_home);
        Value::map([
            ("under", Value::text(under.display().to_string())),
            (
                "components",
                Value::List(
                    mcf_core::component::COMPONENTS
                        .iter()
                        .map(|component| {
                            let short: String =
                                component.commit.chars().take(12).collect();
                            let prefix = under.join(format!("{}@{short}", component.name));
                            // Complete means the provenance beside it, which
                            // is what the builder writes last — not merely a
                            // directory, which is what a run that stopped
                            // partway also leaves.
                            let present = prefix.is_dir();
                            let complete = prefix.join("mcf-provenance.json").is_file();
                            Value::map([
                                ("name", Value::text(component.name)),
                                ("commit", Value::text(component.commit)),
                                ("role", Value::text(component.role)),
                                ("image", Value::text(component.image)),
                                ("image_digest", Value::text(component.image_digest)),
                                ("source", Value::text(component.source)),
                                ("present", Value::Bool(present)),
                                ("provisioned", Value::Bool(complete)),
                                ("prefix", Value::text(prefix.display().to_string())),
                                // An engine MCF can actually reach. Not every
                                // component is one — a window library is not —
                                // so this is an extra fact about engines, never
                                // the test of whether a component is here.
                                (
                                    "usable_engine",
                                    Value::Bool(
                                        engines
                                            .iter()
                                            .any(|engine| engine.name == component.name),
                                    ),
                                ),
                            ])
                        })
                        .collect(),
                ),
            ),
        ])
    }

    fn holding(&self) -> Value {
        match mcf_hub::store::held(&self.places.models) {
            Err(failure) => Value::map([
                ("readable", Value::Bool(false)),
                ("why", mcf_record::encode::failure(&failure)),
            ]),
            Ok(holding) => Value::map([
                ("readable", Value::Bool(true)),
                (
                    "models",
                    Value::List(
                        holding
                            .iter()
                            .map(|held| {
                                Value::map([
                                    ("path", Value::text(held.path.display().to_string())),
                                    ("companion", Value::Bool(held.companion)),
                                    (
                                        "parts",
                                        Value::Integer(i64::from(held.parts)),
                                    ),
                                    (
                                        "bytes",
                                        Value::Integer(
                                            i64::try_from(held.bytes).unwrap_or(i64::MAX),
                                        ),
                                    ),
                                    (
                                        "provenance",
                                        match &held.provenance {
                                            Ok(provenance) => {
                                                mcf_record::encode::provenance(provenance)
                                            }
                                            Err(None) => Value::Null,
                                            Err(Some(failure)) => {
                                                mcf_record::encode::failure(failure)
                                            }
                                        },
                                    ),
                                    ("runs", self.runs(&held.path, held.bytes)),
                                ])
                            })
                            .collect(),
                    ),
                ),
            ]),
        }
    }
}

impl Drop for Daemon {
    /// The socket goes when the daemon does.
    ///
    /// A27: what MCF created, it removes. A leftover socket is not harmful — the
    /// next start connects to it, finds nothing and replaces it — but leaving
    /// one behind means the next start cannot tell *left over* from *running*
    /// without trying, and doing the tidying here keeps that check rare.
    fn drop(&mut self) {
        let _removed = std::fs::remove_file(&self.places.socket);
    }
}

/// What the disk says, read at start.
fn recover(places: &Places) -> Result<Recovered> {
    let (entries, unreadable) = if places.journal.exists() {
        // Through the index rather than a replay (B-300, D20): a daemon start
        // that parsed the whole history would cost seconds on a record that has
        // been measuring models for a while — 7.9 s at a million entries, where
        // the index takes 72 ms (F14) — and would do it at every start.
        let index = mcf_record::journal::Index::over(
            &places.journal,
            &mcf_record::journal::index::default_path(&places.journal),
        )?;
        (index.entries().len(), index.loss().map(ToString::to_string))
    } else {
        // No record is not a damaged record: a machine that has never run MCF
        // has nothing to recover, and saying so is different from saying it
        // recovered nothing (A7).
        (0, None)
    };

    let held = if places.models.exists() {
        mcf_hub::store::held(&places.models)
            .map(|holding| holding.len())
            .unwrap_or_default()
    } else {
        0
    };

    Ok(Recovered {
        entries,
        unreadable,
        held,
    })
}

fn unusable(what: &str, path: &Path, error: &std::io::Error) -> Failure {
    Failure::new(
        Category::ResourceDiskReadonly,
        Attribution::Machine,
        Disposition::Refused,
        WHERE,
        format!("{what} could not be made"),
    )
    .with_context("path", path.display().to_string())
    .with_context("reason", error.to_string())
}

#[cfg(test)]
mod tests;
