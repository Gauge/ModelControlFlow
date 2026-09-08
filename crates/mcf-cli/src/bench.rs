use std::io::{BufRead as _, BufReader, Write as _};
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};

use mcf_bench::compare::{Comparison, Discipline, Interleaving, MachineHeld, Method, UnderTest};
use mcf_bench::record;
use mcf_bench::warmth::Warmth;
use mcf_core::attested::Attested;
use mcf_core::build_identity::BuildIdentity;
use mcf_core::measurement::{ConditionValue, Conditions, Floor, PartsPerMillion};
use mcf_core::time::{Clock as _, Duration, Monotonic, SystemClock, Timestamp};
use mcf_core::trial::{Arm, SessionId};
use mcf_record::journal::{Entry as Record, EntryKind, Journal};
use mcf_record::json::Value;
use mcf_serve::control::{Request, Streamed};

use crate::Response;

const CEILING: usize = 200;

const TOKENS: u32 = 128;

fn asked_for(
    prompt: &str,
    limit: Option<usize>,
    seed: u64,
    engine: Option<&str>,
    resolving: PartsPerMillion,
    cold: bool,
) -> (Discipline, Method, Option<usize>) {
    let tokens = limit
        .and_then(|held| u32::try_from(held).ok())
        .unwrap_or(TOKENS);
    (
        Discipline::Timing { seed, tokens },
        Method {
            prompt: prompt.to_owned(),
            resolving,
            workload: if prompt == mcf_bench::STANDARD_QUESTION {
                mcf_core::contribution::Workload::Declared
            } else {
                mcf_core::contribution::Workload::Custom
            },
            ceiling: CEILING,
            engine: engine.map(str::to_owned),
            cold,
        },
        Some(usize::try_from(tokens).unwrap_or(usize::MAX)),
    )
}

fn so_far(held: &Comparison<Monotonic>, resolving: PartsPerMillion) {
    let pairs = held.pairs().len();
    if pairs < 2 {
        return;
    }
    let reached = held.finding(resolving);
    let said = match (reached.verdict(), held.medians()) {
        (Some(mcf_bench::enough::Verdict::NotYet { .. }) | None, Some((left, right))) => {
            format!("{} against {}", milliseconds(left), milliseconds(right))
        }
        (Some(verdict), _) => verdict.to_string(),
        (None, None) => return,
    };
    eprintln!("  … after {pairs} pair(s), so far: {said}");
}

fn vocabularies(
    left: &Path,
    right: &Path,
    prompt: &str,
    cold: bool,
) -> (Option<Vec<usize>>, Option<Vec<usize>>) {
    if cold {
        (None, None)
    } else {
        (as_identifiers(left, prompt), as_identifiers(right, prompt))
    }
}

pub(crate) struct Planned {
    work: mcf_bench::planned::Work,
    expected: String,
    proposal: Option<String>,
    refused: Option<String>,
}

fn declared(
    left: &Path,
    right: &Path,
    tokens: Option<u32>,
    within: Option<Duration<Monotonic>>,
) -> Planned {
    const LEAST: usize = 2;

    let work = mcf_bench::planned::Work {
        trials: CEILING,
        arms: 2,
        tokens: tokens.unwrap_or(0),
    };
    let unplannable = |why: String| Planned {
        expected: format!("no expected duration: {why}"),
        refused: within.map(|_| {
            format!(
                "mcf: a time budget needs a measured rate to plan against, and there is none \
                 here — {why}\n  run without --within to take the comparison and give this \
                 machine that history"
            )
        }),
        work,
        proposal: None,
    };
    let Some(tokens) = tokens else {
        return unplannable(
            "a behaviour run pins no generation length, so there is no budget to project at \
             (D19)"
                .to_owned(),
        );
    };
    let history = crate::history::read();
    let sized = |path: &Path| std::fs::metadata(path).map_or(0, |meta| meta.len());
    let bands = [
        mcf_bench::project::band(&history.points, sized(left), tokens),
        mcf_bench::project::band(&history.points, sized(right), tokens),
    ];
    let (one, other) = match &bands {
        [Ok(one), Ok(other)] => (one, other),
        [Err(why), _] | [_, Err(why)] => return unplannable(why.to_string()),
    };
    let widest = mcf_core::measurement::Estimate::band(
        one.band().low().min(other.band().low()),
        one.band().high().max(other.band().high()),
        one.band().basis().clone(),
    );
    let rested = if one.rested_on() == other.rested_on() {
        one.rested_on().to_string()
    } else {
        format!(
            "{}; and the other arm {}",
            one.rested_on(),
            other.rested_on()
        )
    };
    let expected = format!(
        "expected {} at that ceiling — an ESTIMATE from {} measured arm(s) of local history, \
         never a measurement and never a declaration (B-224, A20); {rested}",
        span(&work.expected(&widest)),
        history.points.len()
    );
    let Some(budget) = within else {
        return Planned {
            work,
            expected,
            proposal: None,
            refused: None,
        };
    };
    let proposal = mcf_bench::planned::Proposal::within(work, &widest, budget, LEAST);
    let running = proposal.running();
    Planned {
        work: running.unwrap_or(work),
        expected: running.map_or_else(
            || expected.clone(),
            |held| {
                format!(
                    "expected {} at that ceiling — an ESTIMATE from {} measured arm(s) of local \
                     history, never a measurement and never a declaration (B-224, A20); \
                     {rested}",
                    span(&held.expected(&widest)),
                    history.points.len()
                )
            },
        ),
        refused: running.is_none().then(|| format!("mcf: {proposal}")),
        proposal: Some(proposal.to_string()),
    }
}

fn span(held: &mcf_core::measurement::Estimate<Duration<Monotonic>>) -> String {
    format!("{} to {}", scaled(held.low()), scaled(held.high()))
}

fn scaled(at: Duration<Monotonic>) -> String {
    const SECOND: u64 = 1_000_000_000;
    let nanos = at.as_nanos();
    if nanos < SECOND {
        return milliseconds(at);
    }
    let seconds = nanos.wrapping_div(SECOND);
    if seconds < 60 {
        let tenths = nanos.wrapping_div(SECOND.wrapping_div(10)).wrapping_rem(10);
        return format!("{seconds}.{tenths} s");
    }
    format!(
        "{}m {}s",
        seconds.wrapping_div(60),
        seconds.wrapping_rem(60)
    )
}

fn headroom_of(machine: &MachineHeld) -> mcf_core::hardware::headroom::Headroom {
    mcf_core::hardware::headroom::Headroom::taken(machine.before.max(machine.after))
}

fn watched(before: &mcf_core::hardware::Steadiness) -> MachineHeld {
    let after = mcf_core::hardware::steadiness(WATCHED);
    MachineHeld {
        before: before.middle,
        after: after.middle,
        steady_before: before.spread,
        steady_after: after.spread,
    }
}

const WATCHED: usize = 2;

const RESOLVING: PartsPerMillion = PartsPerMillion(50_000);

const PATIENCE: std::time::Duration = std::time::Duration::from_secs(3600);

#[expect(
    clippy::too_many_arguments,
    reason = "every one is a condition of the measurement, and a struct of them \
              would be the same list with a name (§3.4)"
)]
pub(crate) fn bench(
    left: &str,
    right: &str,
    prompt: &str,
    limit: Option<usize>,
    seed: u64,
    engine: Option<&str>,
    resolving: Option<PartsPerMillion>,
    cold: bool,
    within: Option<Duration<Monotonic>>,
    started: mcf_serve::declared::Started,
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
        cold,
        within,
        started,
    )
}

fn no_daemon() -> Response {
    Response {
        text: "mcf: a benchmark runs through the daemon, and none is listening\n  `mcf serve` \
               starts one"
            .to_owned(),
        served: false,
    }
}

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
    cold: bool,
    within: Option<Duration<Monotonic>>,
    started: mcf_serve::declared::Started,
) -> Response {
    let resolving = resolving.unwrap_or(RESOLVING);
    let (discipline, method, limit) = asked_for(prompt, limit, seed, engine, resolving, cold);
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
        return no_daemon();
    };

    let (left_identifiers, right_identifiers) = vocabularies(&left_path, &right_path, prompt, cold);
    if let Err(text) = timeable(
        &socket,
        [
            (&left_path, left_identifiers.as_ref()),
            (&right_path, right_identifiers.as_ref()),
        ],
        prompt,
        seed,
        engine,
        started,
    ) {
        return Response {
            text,
            served: false,
        };
    }

    let before = mcf_core::hardware::steadiness(WATCHED);
    let planned = declared(&left_path, &right_path, discipline.pinned_tokens(), within);
    if let Some(why) = planned.refused {
        return Response {
            text: why,
            served: false,
        };
    }
    let held = interleave(
        &socket,
        Arms {
            left: (&left_path, left_identifiers.as_ref()),
            right: (&right_path, right_identifiers.as_ref()),
        },
        &discipline,
        Asked {
            prompt,
            limit,
            seed,
            engine,
            resolving,
            ceiling: planned.work.trials,
            started,
        },
    );
    if held.pairs().len() < 2 {
        return Response {
            text: format!(
                "mcf: the benchmark produced {} paired trial(s), which is not a comparison\n  {}",
                held.pairs().len(),
                held.cut_short().unwrap_or("no reason was recorded")
            ),
            served: false,
        };
    }

    let machine = watched(&before);
    let held = held.on_a_machine_with(headroom_of(&machine));
    let finding = held.finding(resolving);
    let competing = matches!(
        finding.verdict(),
        Some(mcf_bench::enough::Verdict::NotYet { .. })
    )
    .then(mcf_core::hardware::contention);
    let written = keep(
        &held,
        &finding,
        &method,
        Some(&machine),
        mcf_core::time::Timestamp::now(),
        (left, right),
        engine.unwrap_or("the engine the daemon chose"),
    );
    let competing_written = competing
        .as_ref()
        .map(|held| keep_contention(held, mcf_core::time::Timestamp::now()));
    Response {
        text: report(
            &finding,
            &held,
            &written,
            &machine,
            &planned,
            competing.as_ref(),
            competing_written.as_ref(),
            started,
        ),
        served: true,
    }
}

fn keep_contention(held: &mcf_core::hardware::Snapshot, at: Timestamp) -> Result<PathBuf, String> {
    let Some(path) = mcf_record::journal::default_path() else {
        return Err("there is nowhere to write a record on this machine".to_owned());
    };
    let mut journal =
        Journal::open(&path).map_err(|failure| format!("the record would not open — {failure}"))?;
    journal
        .append(&Record::new(
            EntryKind::ContentionSnapshot,
            at,
            mcf_record::encode::contention(held),
        ))
        .map(|_id| path)
        .map_err(|failure| format!("the snapshot would not append — {failure}"))
}

#[derive(Clone, Copy)]
struct Arms<'a> {
    left: (&'a Path, Option<&'a Vec<usize>>),
    right: (&'a Path, Option<&'a Vec<usize>>),
}

#[derive(Clone, Copy)]
struct Asked<'a> {
    prompt: &'a str,
    limit: Option<usize>,
    seed: u64,
    engine: Option<&'a str>,
    resolving: PartsPerMillion,
    ceiling: usize,
    started: mcf_serve::declared::Started,
}

fn interleave(
    socket: &Path,
    arms: Arms<'_>,
    discipline: &Discipline,
    asked: Asked<'_>,
) -> Comparison<Monotonic> {
    let mut failed: Option<String> = None;
    let mut running = Interleaving::<Monotonic>::new(
        arm(
            arms.left.0,
            asked.engine,
            asked.limit,
            asked.seed,
            discipline,
            arms.left.1,
        ),
        arm(
            arms.right.0,
            asked.engine,
            asked.limit,
            asked.seed,
            discipline,
            arms.right.1,
        ),
        SessionId::new(format!("bench-{}", std::process::id())),
        asked.seed,
        discipline.clone(),
    );
    let named = Arm::new(arms.left.0.display().to_string());
    for _ in 0..asked.ceiling {
        let ran = running.round(|which, _drew| {
            let (path, identifiers) = if *which == named {
                arms.left
            } else {
                arms.right
            };
            match timed(
                socket,
                path,
                asked.prompt,
                identifiers,
                asked.limit,
                asked.seed,
                asked.engine,
                asked.started,
            ) {
                Ok(held) => Some(held),
                Err(text) => {
                    failed.get_or_insert(text);
                    None
                }
            }
        });
        if !ran {
            running.stopped_short(
                "the declared seed set ran out, and repeating it would repeat a trajectory",
            );
            break;
        }
        if let Some(text) = &failed {
            running.stopped_short(text.clone());
            break;
        }
        so_far(running.comparison(), asked.resolving);
        if !running.comparison().reuse().is_uniform() {
            let mixed = running.comparison().reuse().condition();
            running.stopped_short(format!(
                "the trials stopped being alike: reuse was {mixed}, and a delta \
                 across trials that did not all load the model would be measuring \
                 the load rather than the model (§6.13, B-081). `--cold` makes \
                 every trial load it for itself."
            ));
            break;
        }
        if running
            .finding(asked.resolving)
            .verdict()
            .is_some_and(|verdict| !matches!(verdict, mcf_bench::enough::Verdict::NotYet { .. }))
        {
            break;
        }
    }
    running.finish()
}

fn timeable(
    socket: &Path,
    arms: [(&Path, Option<&Vec<usize>>); 2],
    prompt: &str,
    seed: u64,
    engine: Option<&str>,
    started: mcf_serve::declared::Started,
) -> Result<(), String> {
    for (path, identifiers) in arms {
        let named = engine_of(socket, path, prompt, identifiers, seed, engine, started)?;
        if is_a_stand_in(&named) {
            return Err(format!(
                "mcf: {} would run on {named}, and a stand-in's answer can never be a speed \
                 (B65, D31)\n  MCF's own engine is written to be read rather than to be fast, so \
                 a timing taken from it measures the stand-in.\n  `mcf provision llama.cpp` \
                 builds an engine that can be timed; `mcf run` answers behaviour questions on \
                 this one",
                path.display()
            ));
        }
    }
    Ok(())
}

fn arm(
    path: &Path,
    engine: Option<&str>,
    limit: Option<usize>,
    seed: u64,
    discipline: &Discipline,
    identifiers: Option<&Vec<usize>>,
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
    floor.quantization = match quantization_of(path) {
        Some(held) => Attested::Known(ConditionValue::text(held)),
        None => Attested::Unknown,
    };
    floor.seed_set = Attested::Known(ConditionValue::text(discipline.seed_set()));
    floor.batch_shape = match identifiers {
        Some(held) => Attested::Known(ConditionValue::integer(
            i64::try_from(held.len()).unwrap_or(i64::MAX),
        )),
        None => Attested::Unknown,
    };
    UnderTest::new(
        Arm::new(path.display().to_string()),
        Conditions::new(BuildIdentity::current(), floor),
    )
}

fn as_identifiers(path: &Path, prompt: &str) -> Option<Vec<usize>> {
    let bytes = read_prefix(path)?;
    let file = mcf_standin::gguf::parse(&bytes).ok()?;
    let vocabulary = mcf_standin::tokenizer::Vocabulary::read(&file).ok()?;
    vocabulary.encode(prompt, true).ok()
}

pub(crate) fn read_prefix(path: &Path) -> Option<Vec<u8>> {
    use std::io::Read as _;

    let held = std::fs::metadata(path).map_or(0, |meta| meta.len());
    for cap in [16_u64 << 20, 256 << 20, u64::MAX] {
        let take = cap.min(held);
        let mut prefix = Vec::new();
        std::fs::File::open(path)
            .and_then(|handle| handle.take(take).read_to_end(&mut prefix))
            .ok()?;
        if mcf_standin::gguf::parse(&prefix).is_ok() {
            return Some(prefix);
        }
        if take >= held {
            return None;
        }
    }
    None
}

fn quantization_of(path: &Path) -> Option<String> {
    let bytes = read_prefix(path)?;
    let file = mcf_standin::gguf::parse(&bytes).ok()?;
    Some(crate::explain::quantizations(&file))
}

pub(crate) fn located(model: &str) -> Result<PathBuf, String> {
    match crate::run::resolve(model) {
        Ok(Some(path)) => Ok(path),
        Ok(None) => Err(format!(
            "mcf: there is no model at {model}\n  `mcf list` says what this machine is holding; a \
             path to a file works too"
        )),
        Err(found) => Err(crate::run::ambiguous(model, &found)),
    }
}

fn engine_of(
    socket: &Path,
    path: &Path,
    prompt: &str,
    identifiers: Option<&Vec<usize>>,
    seed: u64,
    engine: Option<&str>,
    started: mcf_serve::declared::Started,
) -> Result<String, String> {
    let account = generate(
        socket,
        path,
        prompt,
        identifiers,
        Some(1),
        seed,
        engine,
        started,
    )?
    .1;
    Ok(account
        .get("conditions")
        .and_then(|conditions| conditions.get("engine"))
        .and_then(Value::as_text)
        .unwrap_or("an engine the daemon did not name")
        .to_owned())
}

fn is_a_stand_in(named: &str) -> bool {
    let named = named.to_lowercase();
    named.contains("stand-in") || named.contains("stand in")
}

#[allow(
    clippy::too_many_arguments,
    reason = "one trial's conditions, each named in its account"
)]
fn timed(
    socket: &Path,
    path: &Path,
    prompt: &str,
    identifiers: Option<&Vec<usize>>,
    limit: Option<usize>,
    seed: u64,
    engine: Option<&str>,
    started: mcf_serve::declared::Started,
) -> Result<(Duration<Monotonic>, Warmth), String> {
    let (took, account) = generate(
        socket,
        path,
        prompt,
        identifiers,
        limit,
        seed,
        engine,
        started,
    )?;
    if let Some(pinned) = limit {
        held_the_pin(&account, pinned)?;
    }
    let warmth = Warmth::from_account(
        account
            .get("conditions")
            .and_then(|conditions| conditions.get("loaded"))
            .and_then(Value::as_text),
    );
    Ok((took, warmth))
}

fn held_the_pin(account: &Value, pinned: usize) -> Result<(), String> {
    let length = account
        .get("conditions")
        .and_then(|conditions| conditions.get("length"))
        .and_then(Value::as_text);
    if length == Some("exactly_but_uncounted") {
        return Err(
            "mcf: the trial went through an engine path that hands back text and no token \
             count, so the pinned length cannot be proven; a timing needs the count (B-396)"
                .to_owned(),
        );
    }
    let produced = account
        .get("tokens")
        .and_then(Value::as_integer)
        .and_then(|held| usize::try_from(held).ok());
    if produced == Some(pinned) {
        return Ok(());
    }
    let stopped = account
        .get("stopped")
        .and_then(Value::as_text)
        .unwrap_or("something it did not name");
    Err(format!(
        "mcf: the trial produced {} of the {pinned} tokens pinned and stopped at {stopped} — a \
         run that did not produce what it pinned is not a trial, because the cost a token is \
         read off the count (B-396)",
        produced.map_or_else(|| "an unsaid number".to_owned(), |count| count.to_string()),
    ))
}

#[allow(
    clippy::too_many_arguments,
    reason = "one trial's conditions, each named in its account"
)]
fn generate(
    socket: &Path,
    path: &Path,
    prompt: &str,
    identifiers: Option<&Vec<usize>>,
    limit: Option<usize>,
    seed: u64,
    engine: Option<&str>,
    started: mcf_serve::declared::Started,
) -> Result<(Duration<Monotonic>, Value), String> {
    let mut connection = UnixStream::connect(socket).map_err(|error| {
        format!(
            "mcf: the daemon at {} would not answer\n  {error}",
            socket.display()
        )
    })?;
    let _deadline = connection.set_read_timeout(Some(PATIENCE));
    let _writing = connection.set_write_timeout(Some(PATIENCE));

    let request = Request::Generate {
        whose: mcf_record::content::Whose::User,
        model: path.display().to_string(),
        prompt: match identifiers {
            Some(_) => String::new(),
            None => prompt.to_owned(),
        },
        limit,
        seed,
        tokens: identifiers.cloned(),
        pieces: None,
        engine: engine.map(str::to_owned),
        pinned: true,
        turn: None,
        image: None,
        started,
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
            Ok(Streamed::Token { .. } | Streamed::Progress { .. }) => {}
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

#[allow(
    clippy::too_many_arguments,
    reason = "one comparison's record: what was held, what was found, how, where, when, of which two, on what"
)]
fn keep(
    held: &Comparison<Monotonic>,
    finding: &mcf_bench::compare::Finding,
    method: &Method,
    machine: Option<&MachineHeld>,
    at: Timestamp,
    (left_model, right_model): (&str, &str),
    engine_said: &str,
) -> Result<PathBuf, String> {
    let Some(path) = mcf_record::journal::default_path() else {
        return Err("there is nowhere to write a record on this machine".to_owned());
    };
    let mut journal =
        Journal::open(&path).map_err(|failure| format!("the record would not open — {failure}"))?;
    let body = record::comparison(held, finding, method, machine);
    let appended = journal
        .append(&Record::new(EntryKind::Comparison, at, body.clone()))
        .map_err(|failure| format!("the comparison would not append — {failure}"))?;
    let pair_rows = comparison_rows(&body);
    if !pair_rows.is_empty() {
        for (side, model) in [("left", left_model), ("right", right_model)] {
            let rows: Vec<mcf_serve::examine::Reading> = pair_rows
                .iter()
                .filter(|row| row.dim("side") == side)
                .cloned()
                .collect();
            let _recorded = mcf_serve::examine::record_rows(
                std::path::Path::new(model),
                "comparison",
                engine_said,
                vec![(
                    "against",
                    Value::text(if side == "left" {
                        right_model
                    } else {
                        left_model
                    }),
                )],
                &rows,
            );
        }
    }
    let content = mcf_record::content::ContentStore::beside(&path);
    match mcf_record::content::ContentStore::open(&content).and_then(|store| {
        store.keep(
            appended.id.as_str(),
            &mcf_record::content::Content::new(method.prompt.clone()),
        )
    }) {
        Ok(()) => Ok(path),
        Err(failure) => Err(format!(
            "the comparison is recorded at {} and its prompt could not be filed beside it — \
             {failure}",
            path.display()
        )),
    }
}

fn milliseconds(held: Duration<Monotonic>) -> String {
    let tenths = held.as_nanos().wrapping_div(100_000);
    format!("{}.{} ms", tenths.wrapping_div(10), tenths.wrapping_rem(10))
}

fn rules_of_thumb(finding: &mcf_bench::compare::Finding, planned: &Planned) -> Vec<String> {
    use mcf_bench::enough::Verdict;
    use mcf_core::touchstone::CATALOGUE;

    let mut apt: Vec<&mcf_core::touchstone::Touchstone> = Vec::new();
    let about = |subject: &str| CATALOGUE.iter().find(|held| held.subject() == subject);

    let said = format!("{} {}", planned.work, finding.isolation()).to_lowercase();
    if said.contains("quantization") {
        apt.extend(about("a smaller quantization"));
    }
    match finding.verdict() {
        Some(Verdict::Differ { .. } | Verdict::Apart { .. }) => {
            apt.extend(about("a difference this small"));
        }
        Some(Verdict::Same { .. }) => apt.extend(about("two arms that did not separate")),
        Some(Verdict::Ordered { .. } | Verdict::NotYet { .. }) | None => {}
    }
    if apt.is_empty() {
        return Vec::new();
    }

    let mut lines = vec![
        String::new(),
        "── rules of thumb, which are not results ────────────────────".to_owned(),
    ];
    for held in apt {
        lines.push(format!("  {held}"));
        lines.push(format!(
            "    A laboratory would replace this with a measurement: {}",
            held.until()
        ));
    }
    lines
}

#[expect(
    clippy::too_many_arguments,
    reason = "every one is a condition of the comparison, and the report's job is \
              to say all of them (§3.4)"
)]
fn report(
    finding: &mcf_bench::compare::Finding,
    held: &Comparison<Monotonic>,
    written: &Result<PathBuf, String>,
    machine: &MachineHeld,
    planned: &Planned,
    competing: Option<&mcf_core::hardware::Snapshot>,
    competing_written: Option<&Result<PathBuf, String>>,
    started: mcf_serve::declared::Started,
) -> String {
    let (left_first, right_first) = held.order_balance();
    let mut lines = vec![
        format!("{finding}"),
        String::new(),
        "── how it was taken ─────────────────────────────────────────".to_owned(),
        format!("  {}", held.discipline()),
        format!("  reuse    {}", held.reuse()),
        format!("  machine  {machine}"),
        format!("  work     {}", planned.work),
        format!("           {}", planned.expected),
        format!(
            "  pairs    {} interleaved, order drawn per pair",
            held.pairs().len()
        ),
        format!("  order    {left_first} left-first, {right_first} right-first"),
        format!("  started  {}", started.said()),
    ];
    for why in finding.not_fit_to_contribute() {
        lines.push(format!("  NOT FIT TO CONTRIBUTE — {why}"));
    }
    if let Some(proposal) = planned.proposal.as_ref() {
        lines.push(format!("  budget   {proposal}"));
    }
    if let Some((left, right)) = held.medians() {
        lines.push(format!(
            "  medians  {} and {} — local figures, which do not travel",
            milliseconds(left),
            milliseconds(right)
        ));
    }
    if let Some(differences) = held.paired_differences() {
        lines.push("  paired differences, in interleaving order:".to_owned());
        for (at, difference) in differences.iter().enumerate() {
            lines.push(format!("    #{at}: {difference}"));
        }
    }
    if let Some(because) = held.cut_short() {
        lines.push(format!("  CUT SHORT after {} pair(s)", held.pairs().len()));
        lines.push(format!("           {because}"));
        lines.push(
            "           The pairs above are kept: each is two runs of two arms taken".to_owned(),
        );
        lines.push(
            "           back to back, and an interruption afterwards does not reach".to_owned(),
        );
        lines.push("           back and unmake them.".to_owned());
    }
    lines.extend(rules_of_thumb(finding, planned));
    lines.push(match written {
        Ok(path) => format!("  recorded {}", path.display()),
        Err(why) => format!("  NOT RECORDED — {why}"),
    });
    if !held.reuse().is_uniform() {
        lines.push(String::new());
        for said in [
            "  The trials were not alike, so there is no delta. One model is",
            "  resident at a time and a paired comparison alternates them, so most",
            "  trials reload and some do not — which of them is a property of the",
            "  drawn order rather than of either arm.",
            "",
            "  `--cold` makes every trial load the model, which is uniform and",
            "  measurable; comparing a model with itself is uniform too.",
        ] {
            lines.push(said.to_owned());
        }
    }
    if let Some(snapshot) = competing {
        lines.push(String::new());
        lines.push("── what was competing, sampled after the run ─────────────────".to_owned());
        lines.push(format!("  {snapshot}"));
        for one in &snapshot.competitors {
            lines.push(format!("    {one}"));
        }
        lines.push("  This run could not decide, and MCF does not attribute that".to_owned());
        lines.push("  to the arms. What it can do is say what else was here.".to_owned());
        if let Some(Err(why)) = competing_written {
            lines.push(format!("  THE SNAPSHOT WAS NOT RECORDED — {why}"));
        }
    }
    lines.push(String::new());
    lines.push(
        "  A benchmark has no pass condition (A18): every verdict above is\n  something this \
         machine said, and none of them is MCF being wrong."
            .to_owned(),
    );
    lines.join("\n")
}

#[cfg(test)]
fn per_cent_of(written: &str) -> Option<PartsPerMillion> {
    crate::per_cent(written).map(PartsPerMillion)
}

#[cfg(test)]
mod tests;

fn comparison_rows(body: &Value) -> Vec<mcf_serve::examine::Reading> {
    use mcf_serve::examine::Reading;
    let mut rows = Vec::new();
    let pairs = body.get("pairs").and_then(Value::as_list).unwrap_or(&[]);
    for (at, pair) in pairs.iter().enumerate() {
        let pair_at = Value::Integer(i64::try_from(at).unwrap_or(i64::MAX));
        for side in ["left", "right"] {
            let dims = [("side", Value::text(side)), ("pair", pair_at.clone())];
            if let Some(ns) = pair.get(&format!("{side}_ns")).and_then(Value::as_integer) {
                rows.push(Reading::new(&dims, "ns", ns, "ns"));
            }
            if let Some(position) = pair
                .get(&format!("{side}_position"))
                .and_then(Value::as_integer)
            {
                rows.push(Reading::new(&dims, "position", position, "count"));
            }
            if let Some(first) = pair.get("first").and_then(Value::as_text) {
                rows.push(Reading::new(
                    &dims,
                    "went_first",
                    i64::from(first == side),
                    "bool",
                ));
            }
        }
    }
    rows
}
