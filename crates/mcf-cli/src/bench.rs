//! `mcf bench`: the runner that produces measurements and cannot fail
//! (A18, §6.7, B-080).
//!
//! **Two systems, never conflated.** A18: *tests gate correctness — fast,
//! deterministic, hermetic, green. Benchmarks produce measurements — slow,
//! stochastic, hardware-bound, and with no pass condition. A benchmark that
//! "fails" has usually just told you something true.* Its violation is a
//! throughput assertion in the test suite, which is how suites become flaky and
//! then get ignored.
//!
//! So this command has **no verdict that makes it fail**. *They differ*, *they
//! are the same to a stated resolution* and *not decided* all exit
//! successfully, because all three are things the machine told MCF and none of
//! them is MCF being wrong. What does fail is MCF being unable to run the
//! benchmark at all — no such model, no daemon, an engine that cannot report a
//! speed — and those are refusals rather than results.
//!
//! **The engine is asked, not assumed.** B65 and D31: MCF's own stand-in is
//! written to be read rather than to be fast, and a timing taken from it
//! measures the stand-in. So before any trial is timed, one request is sent to
//! each arm and the *account* is read for which engine actually ran. A
//! stand-in on either side is a refusal by name — not a mark on a number that
//! somebody will quote without it.
//!
//! **The comparison is built the only way one can be** (B-250): paired,
//! interleaved, with the order drawn per pair, stopping when this run's own
//! arithmetic separates the arms or establishes they are the same to the
//! resolution asked for (F55, F57). The count is part of the answer because
//! the count is a property of the sitting (F53).
//!
//! **What it leaves behind** is a `comparison` entry with every pair's raw
//! durations, both arms' conditions and the verdict — a result whichever way it
//! came out (A9, B-086).

use std::io::{BufRead as _, BufReader, Write as _};
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};

use mcf_bench::compare::{Comparison, Discipline, Interleaving, UnderTest};
use mcf_bench::record;
use mcf_core::attested::Attested;
use mcf_core::build_identity::BuildIdentity;
use mcf_core::measurement::{ConditionValue, Conditions, Floor, PartsPerMillion};
use mcf_core::time::{Clock as _, Duration, Monotonic, SystemClock, Timestamp};
use mcf_core::trial::{Arm, SessionId};
use mcf_record::journal::{Entry as Record, EntryKind, Journal};
use mcf_record::json::Value;
use mcf_serve::control::{Request as Ask, Streamed};

use crate::Response;

/// The most paired trials a run will take before reporting what it has.
///
/// A ceiling rather than a target, and it is a chosen number — stated here so
/// that it is one line to find. A run that reaches it reports *not decided*,
/// which is a result about this machine and this half-hour rather than about
/// the two arms (A7, F53).
const CEILING: usize = 200;

/// The generation length a benchmark pins when nobody says.
///
/// **Pinned rather than left to the model** (D19, B-290): *a seed changes which
/// tokens are produced and therefore possibly how many, and a timing that
/// varies because one run stopped earlier is measuring the stop, not the
/// speed.* A benchmark whose arms stopped where they liked would report the
/// models' verbosity as the machine's throughput. So `mcf bench` always pins a
/// length, and where the operator did not name one it uses this and says so.
///
/// A chosen number, stated in one line: long enough that process start does not
/// dominate on a small model, short enough that a hundred paired trials is
/// minutes rather than an afternoon.
const TOKENS: u32 = 128;

/// The difference a benchmark looks for when nobody says.
///
/// Five percent. Also chosen, also stated in one line: it is the caller's
/// question — *how much is a difference* is about their purpose and not about
/// the machine — and this is only what MCF asks when they have not.
const RESOLVING: PartsPerMillion = PartsPerMillion(50_000);

/// How long a single generation may take before the wire is called dead.
const PATIENCE: std::time::Duration = std::time::Duration::from_secs(600);

/// Compares two models, and says what it found.
pub(crate) fn bench(
    left: &str,
    right: &str,
    prompt: &str,
    limit: Option<usize>,
    seed: u64,
    engine: Option<&str>,
    resolving: Option<PartsPerMillion>,
) -> Response {
    bench_where(
        crate::serve::socket_path(),
        left,
        right,
        prompt,
        limit,
        seed,
        engine,
        resolving,
    )
}

/// The same, told where a daemon would be.
///
/// Where the daemon is, is an *input* rather than something looked up in the
/// middle, for the reason `run_where` gives: a suite whose answer depends on
/// whether a daemon happens to be running is a suite that reports on the
/// machine (F46, §3.12).
#[expect(
    clippy::too_many_arguments,
    reason = "every one is a condition of the measurement, and a struct of them \
              would be the same list with a name (§3.4)"
)]
pub(crate) fn bench_where(
    socket: Option<PathBuf>,
    left: &str,
    right: &str,
    prompt: &str,
    limit: Option<usize>,
    seed: u64,
    engine: Option<&str>,
    resolving: Option<PartsPerMillion>,
) -> Response {
    let resolving = resolving.unwrap_or(RESOLVING);
    // D19's timing discipline, made explicit: the seed is held still and the
    // generation length is pinned. Both travel into every trial and into the
    // record, so a reader can never mistake this run's fixed seed for a
    // behaviour run's mistake (B61, B-290).
    let tokens = limit
        .and_then(|held| u32::try_from(held).ok())
        .unwrap_or(TOKENS);
    let limit = Some(usize::try_from(tokens).unwrap_or(usize::MAX));
    let discipline = Discipline::Timing { seed, tokens };
    let (left_path, right_path) = match (located(left), located(right)) {
        (Ok(one), Ok(other)) => (one, other),
        (Err(text), _) | (_, Err(text)) => {
            return Response {
                text,
                served: false,
            };
        }
    };

    let Some(socket) = socket.filter(|socket| UnixStream::connect(socket).is_ok()) else {
        return Response {
            text: "mcf: a benchmark runs through the daemon, and none is listening\n  `mcf serve` \
                   starts one"
                .to_owned(),
            served: false,
        };
    };

    // B65, asked rather than assumed. One request per arm, and the account
    // says which engine ran.
    for path in [&left_path, &right_path] {
        match engine_of(&socket, path, prompt, seed, engine) {
            Err(text) => {
                return Response {
                    text,
                    served: false,
                };
            }
            Ok(named) if is_a_stand_in(&named) => {
                return Response {
                    text: format!(
                        "mcf: {} would run on {named}, and a stand-in's answer can never be a \
                         speed (B65, D31)\n  MCF's own engine is written to be read rather than \
                         to be fast, so a timing taken from it measures the stand-in.\n  \
                         `mcf provision llama.cpp` builds an engine that can be timed; \
                         `mcf run` answers behaviour questions on this one",
                        path.display()
                    ),
                    served: false,
                };
            }
            Ok(_) => {}
        }
    }

    let mut failed: Option<String> = None;
    let mut running = Interleaving::<Monotonic>::new(
        arm(&left_path, engine, limit, seed, &discipline),
        arm(&right_path, engine, limit, seed, &discipline),
        SessionId::new(format!("bench-{}", std::process::id())),
        seed,
        discipline.clone(),
    );
    let named = Arm::new(left_path.display().to_string());
    for _ in 0..CEILING {
        let ran = running.round(|which, _drew| {
            let path = if *which == named {
                &left_path
            } else {
                &right_path
            };
            match timed(&socket, path, prompt, limit, seed, engine) {
                Ok(took) => took,
                Err(text) => {
                    failed.get_or_insert(text);
                    Duration::from_nanos(0)
                }
            }
        });
        if !ran {
            break;
        }
        if let Some(text) = failed {
            return Response {
                text,
                served: false,
            };
        }
        if running
            .finding(resolving)
            .verdict()
            .is_some_and(|verdict| !matches!(verdict, mcf_bench::enough::Verdict::NotYet { .. }))
        {
            break;
        }
    }

    let held = running.finish();
    let finding = held.finding(resolving);
    let written = keep(&held, mcf_core::time::Timestamp::now());
    // Every verdict is served. A18: a benchmark has no pass condition, and a
    // command that exited non-zero on *not decided* would be a pass condition
    // wearing an exit status.
    Response {
        text: report(&finding, &held, &written),
        served: true,
    }
}

/// One arm, as a configuration.
///
/// What MCF can state here it states: the file under test, the engine asked
/// for, and the budget. What it cannot read is `Unknown` rather than filled in
/// (A7), so the comparison reports its isolation as undetermined until the
/// condition producers exist (B-007, B-013) — which is the truth (F56).
fn arm(
    path: &Path,
    engine: Option<&str>,
    limit: Option<usize>,
    seed: u64,
    discipline: &Discipline,
) -> UnderTest {
    let mut floor = Floor::nothing_known();
    floor.mcf_configuration = Attested::Known(ConditionValue::text(format!(
        "engine={}, limit={}, seed={seed}",
        engine.unwrap_or("as the daemon chooses"),
        limit.map_or_else(
            || "as the daemon chooses".to_owned(),
            |held| held.to_string()
        ),
    )));
    // Read from the file's own tensor types rather than from its name (A21).
    // This is the condition two quantizations of one model differ in, so it is
    // what turns a benchmark of them from *undetermined* into *isolated* — and
    // it is the first of F56's ten unread conditions to be filled in here.
    floor.quantization = match quantization_of(path) {
        Some(held) => Attested::Known(ConditionValue::text(held)),
        None => Attested::Unknown,
    };
    // D19 makes the seed set a condition, and a timing run's answer is *none,
    // and here is what it pinned instead* — which is a thing MCF knows rather
    // than a thing it failed to read (B-290).
    floor.seed_set = Attested::Known(ConditionValue::text(discipline.seed_set()));
    UnderTest::new(
        Arm::new(path.display().to_string()),
        Conditions::new(BuildIdentity::current(), floor),
    )
}

/// How a model file's weights are encoded, counted by kind.
///
/// From a bounded prefix, growing only if the directory did not fit — the same
/// three sizes `run` uses, because reading sixteen gigabytes to name a
/// quantization would make a benchmark's first act its slowest (B-372).
/// `None` where the file will not parse, which is `Unknown` rather than a
/// guess (A7): the benchmark still runs, and reports that it could not tell
/// what it isolated.
fn quantization_of(path: &Path) -> Option<String> {
    use std::io::Read as _;

    let held = std::fs::metadata(path).map_or(0, |meta| meta.len());
    for cap in [16_u64 << 20, 256 << 20, u64::MAX] {
        let take = cap.min(held);
        let mut prefix = Vec::new();
        let opened =
            std::fs::File::open(path).and_then(|handle| handle.take(take).read_to_end(&mut prefix));
        opened.ok()?;
        if let Ok(file) = mcf_standin::gguf::parse(&prefix) {
            return Some(crate::explain::quantizations(&file));
        }
        if take >= held {
            return None;
        }
    }
    None
}

/// Resolves a model reference, or the sentence explaining why it did not.
fn located(model: &str) -> Result<PathBuf, String> {
    match crate::run::resolve(model) {
        Ok(Some(path)) => Ok(path),
        Ok(None) => Err(format!(
            "mcf: there is no model at {model}\n  `mcf list` says what this machine is holding; a \
             path to a file works too"
        )),
        Err(found) => Err(crate::run::ambiguous(model, &found)),
    }
}

/// Which engine the daemon would actually use for this model.
fn engine_of(
    socket: &Path,
    path: &Path,
    prompt: &str,
    seed: u64,
    engine: Option<&str>,
) -> Result<String, String> {
    let account = generate(socket, path, prompt, Some(1), seed, engine)?.1;
    Ok(account
        .get("conditions")
        .and_then(|conditions| conditions.get("engine"))
        .and_then(Value::as_text)
        .unwrap_or("an engine the daemon did not name")
        .to_owned())
}

/// Whether a named engine is MCF's own stand-in.
///
/// By name, because that is what the account carries. The stand-in says so in
/// its own words and no other engine does.
fn is_a_stand_in(named: &str) -> bool {
    let named = named.to_lowercase();
    named.contains("stand-in") || named.contains("stand in")
}

/// One generation, timed.
fn timed(
    socket: &Path,
    path: &Path,
    prompt: &str,
    limit: Option<usize>,
    seed: u64,
    engine: Option<&str>,
) -> Result<Duration<Monotonic>, String> {
    generate(socket, path, prompt, limit, seed, engine).map(|(took, _)| took)
}

/// One generation: how long the whole request took, and its account.
///
/// The monotonic clock, always (D9), and the whole request rather than the
/// tokens alone — what an operator waits for is the request, and a figure that
/// excluded the parts MCF chose not to count would be a figure about a
/// subset nobody named (§3.4).
fn generate(
    socket: &Path,
    path: &Path,
    prompt: &str,
    limit: Option<usize>,
    seed: u64,
    engine: Option<&str>,
) -> Result<(Duration<Monotonic>, Value), String> {
    let mut connection = UnixStream::connect(socket).map_err(|error| {
        format!(
            "mcf: the daemon at {} would not answer\n  {error}",
            socket.display()
        )
    })?;
    let _deadline = connection.set_read_timeout(Some(PATIENCE));
    let _writing = connection.set_write_timeout(Some(PATIENCE));

    let request = Ask::Generate {
        model: path.display().to_string(),
        prompt: prompt.to_owned(),
        limit,
        seed,
        tokens: None,
        engine: engine.map(str::to_owned),
    };
    let clock = SystemClock;
    let began = clock.now();
    writeln!(connection, "{}", request.to_line())
        .and_then(|()| connection.flush())
        .map_err(|error| {
            format!(
                "mcf: the daemon at {} would not take the request\n  {error}",
                socket.display()
            )
        })?;

    let mut account = None;
    let reader = BufReader::new(&connection);
    for line in reader.lines() {
        let Ok(line) = line else { break };
        match Streamed::read(line.trim_end()) {
            Ok(Streamed::Token { .. }) => {}
            Ok(Streamed::Done(done)) => {
                account = Some(done);
                break;
            }
            Err(_) => break,
        }
    }
    let took = clock.now().saturating_duration_since(began);

    let Some(account) = account else {
        return Err(format!(
            "mcf: the stream from the daemon at {} ended before its account, so this trial has no \
             conditions and is not a trial (A4, A7)",
            socket.display()
        ));
    };
    if let Some(failure) = account.get("failure") {
        return Err(format!(
            "mcf: {} did not run\n  the daemon refused it: {}",
            path.display(),
            failure.to_line()
        ));
    }
    Ok((took, account))
}

/// Writes the comparison to the record, and says where or why not.
fn keep(held: &Comparison<Monotonic>, at: Timestamp) -> Result<PathBuf, String> {
    let Some(path) = mcf_record::journal::default_path() else {
        return Err("there is nowhere to write a record on this machine".to_owned());
    };
    let mut journal =
        Journal::open(&path).map_err(|failure| format!("the record would not open — {failure}"))?;
    let body = record::comparison(held, &held.finding(RESOLVING));
    journal
        .append(&Record::new(EntryKind::Comparison, at, body))
        .map(|_id| path)
        .map_err(|failure| format!("the comparison would not append — {failure}"))
}

/// What the operator reads.
fn report(
    finding: &mcf_bench::compare::Finding,
    held: &Comparison<Monotonic>,
    written: &Result<PathBuf, String>,
) -> String {
    let (left_first, right_first) = held.order_balance();
    let mut lines = vec![
        format!("{finding}"),
        String::new(),
        "── how it was taken ─────────────────────────────────────────".to_owned(),
        format!("  {}", held.discipline()),
        format!(
            "  pairs    {} interleaved, order drawn per pair",
            held.pairs().len()
        ),
        format!("  order    {left_first} left-first, {right_first} right-first"),
    ];
    if let Some(differences) = held.paired_differences() {
        lines.push("  paired differences, in interleaving order:".to_owned());
        for (at, difference) in differences.iter().enumerate() {
            lines.push(format!("    #{at}: {difference}"));
        }
    }
    lines.push(match written {
        Ok(path) => format!("  recorded {}", path.display()),
        // A6 and A2: the measurement stands, and the fact that it was not kept
        // is said rather than swallowed.
        Err(why) => format!("  NOT RECORDED — {why}"),
    });
    lines.push(String::new());
    lines.push(
        "  A benchmark has no pass condition (A18): every verdict above is\n  something this \
         machine said, and none of them is MCF being wrong."
            .to_owned(),
    );
    lines.join("\n")
}

/// A percentage as parts per million, as `bench`'s own option reader reads it.
///
/// Re-exported here so that the reading and the thing it configures are tested
/// together: the option lives with the parser and the unit lives with the
/// measurement, and a test of one that could not see the other would be
/// testing a convention.
#[cfg(test)]
fn per_cent_of(written: &str) -> Option<PartsPerMillion> {
    crate::per_cent(written).map(PartsPerMillion)
}

#[cfg(test)]
mod tests;
