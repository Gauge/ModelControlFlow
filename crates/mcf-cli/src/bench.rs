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

/// The discipline, the method and the pinned budget, from what was asked.
///
/// D19's timing discipline made explicit — the seed is held still and the
/// generation length is pinned, both travelling into every trial and into the
/// record so a reader can never mistake this run's fixed seed for a behaviour
/// run's mistake (B61, B-290). And the *method*, which the conditions do not
/// say: §II asks that somebody else be able to repeat this, and a floor full
/// of hardware does not tell them what to run (PR2, B30, B-211).
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
            // The text, not how it arrived: an operator who types the standard
            // question verbatim has asked the standard question, and a run that
            // took it because nothing was given has too (B-160, B42).
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

/// What the run has so far, said while it is still going (B-227, A4, §3.1).
///
/// **To standard error, and only there.** The result of a benchmark is one
/// thing and goes to standard output; these are the run talking about itself
/// while it works. A pipeline reading the verdict must not have to filter
/// progress out of it.
///
/// **Marked as partial in the line itself**, not by where it appeared. A
/// reader who scrolls back to an interim line must not be able to mistake it
/// for the answer — so it is prefixed and it always says how many pairs it
/// rests on. It is not an estimate (A20): it is a real finding over fewer
/// pairs, which is a different thing and is labelled as the different thing.
///
/// **Nothing is computed here that the run does not already compute.** The
/// finding at this many pairs is what the stopping condition asks for anyway,
/// so reporting costs a rendering and not a measurement, and this cannot
/// change what the run does (A18: a benchmark has no pass condition, and
/// progress is not one).
fn so_far(held: &Comparison<Monotonic>, resolving: PartsPerMillion) {
    let pairs = held.pairs().len();
    // Two is the floor below which there is no comparison to report.
    if pairs < 2 {
        return;
    }
    let reached = held.finding(resolving);
    // The medians rather than the verdict's own sentence: while a run is going
    // *not decided* is the answer at nearly every pair, and repeating it forty
    // times says nothing. What moves is the two arms, so that is what a
    // watching operator is shown — labelled *so far*, which is what keeps it
    // from reading as the result.
    let said = match (reached.verdict(), held.medians()) {
        (Some(mcf_bench::enough::Verdict::NotYet { .. }) | None, Some((left, right))) => {
            format!("{} against {}", milliseconds(left), milliseconds(right))
        }
        (Some(verdict), _) => verdict.to_string(),
        (None, None) => return,
    };
    eprintln!("  … after {pairs} pair(s), so far: {said}");
}

/// The prompt as each model's own vocabulary produces it, done **once**.
///
// The prompt as each model's own vocabulary produces it, done **once**
/// rather than per trial: tokenizing is not generating, and a benchmark that
/// paid for it every trial would be timing MCF's tokenizer alongside the
/// engine (B-090, F64).
///
/// Unless the operator asked for a cold run. **Why that option exists**
/// (F65): the provisioned engine's server holds one model at a time
/// (DEC-001), and a paired comparison of two models alternates them — so
/// most trials reload and some do not, which is a *mixed* run and not one
/// measurement (§6.13). The text path is uniformly cold, because it is a
/// fresh process per request, and uniformly cold is a measurement even
/// though it includes what F64 measured as forty-six milliseconds of
/// overhead. Uniform and honest beats warm and mixed.
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

/// What a run declared it would do, and what that was expected to take.
///
/// The two are separate fields on purpose (B-224). The work is a fact about
/// the run and is true on any machine; the expectation is a derived
/// [`Estimate`] about *this* machine, and lives or dies with the history it
/// came from.
///
/// [`Estimate`]: mcf_core::measurement::Estimate
pub(crate) struct Planned {
    /// The declaration, in countable units — already reduced where a budget
    /// bought less than the ceiling, and never reduced silently.
    work: mcf_bench::planned::Work,
    /// What that much work was expected to take here, already rendered — or
    /// why there was nothing to derive it from.
    expected: String,
    /// What a time budget proposed, where one was given.
    proposal: Option<String>,
    /// Why no run was possible under the budget asked for.
    ///
    /// A budget that buys less than a comparison, or one MCF has no measured
    /// rate to plan against, is a refusal with a reason — never a quietly
    /// smaller run (§3.1, B47).
    refused: Option<String>,
}

/// Declares the work, and derives what it should take from local history.
///
/// **The band brackets both arms.** Two files of different sizes have two
/// per-generation bands, and the run alternates between them; the enclosing
/// band — the faster arm's floor to the slower arm's ceiling — is wider than
/// either and contains what the run will actually do. Widening rather than
/// splitting the difference is the direction an estimate is allowed to be
/// wrong in (B46).
///
/// **Absent rather than invented.** A machine with no history at this budget
/// gets the reason it has no expectation, and still gets the declaration:
/// B-224's countable units are what a laboratory owes, and the minutes are the
/// part MCF may be unable to supply.
fn declared(
    left: &Path,
    right: &Path,
    tokens: Option<u32>,
    within: Option<Duration<Monotonic>>,
) -> Planned {
    // B-226: the budget produces a proposal, and the proposal names both
    // halves. Two pairs is the smallest thing that is a paired comparison at
    // all — the same floor `bench_where` refuses below.
    const LEAST: usize = 2;

    let work = mcf_bench::planned::Work {
        trials: CEILING,
        arms: 2,
        tokens: tokens.unwrap_or(0),
    };
    // A budget MCF cannot plan against is a refusal, not a run that hopes.
    // Truncating against a rate it does not have would be inventing the rate
    // (A7); running the full ceiling anyway would be ignoring what was asked.
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
    // B-385: the band inherits the conditions of what it was read between, and
    // says so. A projection that drops them is the defect F74 found.
    //
    // The two arms are usually read between the same two points, in which case
    // the sentence is the same sentence and printing it twice tells a reader
    // nothing except that MCF is repeating itself.
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

/// A duration band as a person reads it, without a float (A6).
///
/// Scaled to the size of the thing: a whole run's expected duration in
/// milliseconds is six digits nobody reads, and *two and a half minutes* is
/// the answer to the question actually being asked. The scale is chosen from
/// the value and never rounded away — a figure under a minute keeps its
/// tenths, because that is where a benchmark's differences live.
fn span(held: &mcf_core::measurement::Estimate<Duration<Monotonic>>) -> String {
    format!("{} to {}", scaled(held.low()), scaled(held.high()))
}

/// One duration, at whichever scale reads.
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

/// How much of this machine was already busy while a run happened
/// (B-217, F95).
///
/// The busier of the two readings taken either side, because a run is only as
/// measurable as its worst moment — and the band DEC-007 left open is measured
/// now, so a run outside it can be marked. Marked rather than refused: the
/// operator's decision of 2026-08-28, and refusing would deny a result to
/// anyone whose machine is simply busy.
fn headroom_of(machine: &MachineHeld) -> mcf_core::hardware::headroom::Headroom {
    mcf_core::hardware::headroom::Headroom::taken(machine.before.max(machine.after))
}

/// The machine either side of the run, from the reading taken before it and
/// one taken now (B-217).
///
/// Taken here rather than at the top so that the second reading is genuinely
/// after the last trial: a reading interleaved with the run would be MCF
/// measuring itself measuring (§3.8, B3).
fn watched(before: &mcf_core::hardware::Steadiness) -> MachineHeld {
    let after = mcf_core::hardware::steadiness(WATCHED);
    MachineHeld {
        before: before.middle,
        after: after.middle,
        steady_before: before.spread,
        steady_after: after.spread,
    }
}

/// How many readings the machine is watched for, either side of a run.
///
/// Two, which is the fewest that can show a spread at all, and costs two
/// stated intervals each side. A chosen number, stated in one line: more
/// readings buy a firmer answer about the machine and are paid for in wall
/// clock the operator is waiting through.
const WATCHED: usize = 2;

/// The difference a benchmark looks for when nobody says.
///
/// Five percent. Also chosen, also stated in one line: it is the caller's
/// question — *how much is a difference* is about their purpose and not about
/// the machine — and this is only what MCF asks when they have not.
const RESOLVING: PartsPerMillion = PartsPerMillion(50_000);

/// How long the wire may be silent before it is called dead: not how long
/// a generation may take, which is as long as it takes (D48). The daemon
/// says how far the engine has got every ten seconds while it works.
const PATIENCE: std::time::Duration = std::time::Duration::from_secs(3600);

/// Compares two models, and says what it found.
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

/// The same, told where a daemon would be.
///
/// Where the daemon is, is an *input* rather than something looked up in the
/// middle, for the reason `run_where` gives: a suite whose answer depends on
/// whether a daemon happens to be running is a suite that reports on the
/// machine (F46, §3.12).
/// A benchmark needs the daemon: this process cannot time an engine it did
/// not start, and MCF's own engine is not one that can be timed (B65, D31).
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
    // B65, asked rather than assumed. One request per arm, and the account
    // says which engine ran.
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

    // What the machine was doing before the first trial (B-217, §3.8). Two
    // readings, because two is the fewest that can show a spread — and neither
    // during the run, because sampling while measuring would make MCF one of
    // the competitors it reports.
    let before = mcf_core::hardware::steadiness(WATCHED);
    // B-224: what this run will do, counted, and — separately and derived —
    // what that would take here. Taken *before* the run: an expectation formed
    // afterwards is hindsight wearing the word *expected*.
    let planned = declared(&left_path, &right_path, discipline.pinned_tokens(), within);
    // §3.1 and B47: a budget that buys less than a comparison, or one there is
    // no measured rate to plan against, refuses and says why. It never becomes
    // a quietly smaller run.
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
    // A4: a run that was interrupted before it had two pairs has no comparison
    // to report, and *that* is a refusal. One with two or more has a result,
    // and the interruption travels with it rather than replacing it.
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
    // B24, PR5: a run that could not decide is one whose next question is
    // *what was competing with it*, and MCF is the only thing positioned to
    // answer — it was here when it happened. Sampled **after** the run, so
    // that MCF is not one of the competitors it reports (§3.8, B3), and only
    // when there is something to explain: B4 refuses ambient sampling.
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
    );
    let competing_written = competing
        .as_ref()
        .map(|held| keep_contention(held, mcf_core::time::Timestamp::now()));
    // Every verdict is served. A18: a benchmark has no pass condition, and a
    // command that exited non-zero on *not decided* would be a pass condition
    // wearing an exit status.
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

/// Writes a contention snapshot beside the comparison it explains.
///
/// PR5 requires the snapshot persist with the record rather than being a
/// transient thing on a screen, so that the finding survives the terminal it
/// was printed in (§3.1).
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

/// The two arms of a comparison: where each model is, and the prompt as each
/// one's own vocabulary produces it.
#[derive(Clone, Copy)]
struct Arms<'a> {
    left: (&'a Path, Option<&'a Vec<usize>>),
    right: (&'a Path, Option<&'a Vec<usize>>),
}

/// What the operator asked for, held together so that the runner's signature
/// is about the run rather than about its options.
#[derive(Clone, Copy)]
struct Asked<'a> {
    prompt: &'a str,
    limit: Option<usize>,
    seed: u64,
    engine: Option<&'a str>,
    resolving: PartsPerMillion,
    /// The most pairs this run will take — the declared ceiling, or the
    /// smaller number a time budget proposed (B-226).
    ceiling: usize,
    /// What the engine is started with beyond the plain load, the same for
    /// both arms (B-463).
    started: mcf_serve::declared::Started,
}

/// Runs the pairs until the comparison decides or the ceiling is reached.
///
/// The comparison is built the only way one can be (B-250) and finished here,
/// which is where what the run reused becomes a condition of both arms (§6.13,
/// B-081): known only once the run is over, because it is a fact about what
/// happened rather than about what MCF intended.
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
                    // A run that did not happen is not a trial (A4, A1): no
                    // pair is recorded, the pairs already taken are kept, and
                    // what stopped the run is written down below.
                    failed.get_or_insert(text);
                    None
                }
            }
        });
        if !ran {
            // The declared seed set ran out. Every pair taken is kept, and
            // what stopped the run is said rather than left as a smaller
            // number nobody can explain (A4, B-087, B61).
            running.stopped_short(
                "the declared seed set ran out, and repeating it would repeat a trajectory (B61)",
            );
            break;
        }
        if let Some(text) = &failed {
            // **Nine of ten trials completing is nine data points** (A4). The
            // pairs already taken are two runs of two arms under the same
            // conditions apiece, and a trial failing afterwards does not reach
            // back and unmake them.
            running.stopped_short(text.clone());
            break;
        }
        // B-227: what it has so far, as it goes. A run that takes minutes and
        // says nothing until it is finished is one an operator cannot tell
        // from a hung one, and a run interrupted at pair forty should not be
        // the first time they learn what forty pairs said.
        so_far(running.comparison(), asked.resolving);
        // A run whose trials have stopped being alike cannot produce a delta
        // however long it goes on (§6.13), so it stops as soon as that is
        // true rather than spending the ceiling to arrive at the same refusal.
        //
        // **And it says so.** This was the one exit that broke without
        // writing down why: a comparison that ended here reported "no reason
        // was recorded", which is the sentence A7 exists to prevent, and left
        // an operator to guess at a run that had in fact stopped for a
        // perfectly good reason.
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

/// Whether both arms would run on an engine that can be timed (B65, D31).
///
/// Asked rather than assumed: one request per arm, and the daemon's account
/// says which engine actually ran. A stand-in is refused **by name** rather
/// than marked, because a marked number is a number somebody will quote
/// without its mark.
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
    // How long the prompt actually was, in this model's own identifiers. Two
    // models given the same text may receive different numbers of tokens, and
    // that is a condition of what each was asked rather than a fault (§3.4).
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

/// The prompt, as identifiers this model's own vocabulary produces.
///
/// **Why a benchmark sends identifiers rather than text** (B-090, F64). A
/// prompt routes to the provisioned engine's *completion tool*, which is a
/// fresh process per request; a turn of identifiers routes to its **server**,
/// which stays up and holds the model (B-376). F64 measured what the
/// difference is worth: forty-six milliseconds of process start and model load
/// against a quarter of a millisecond per token, which was three fifths of
/// every default trial.
///
/// Each arm is tokenized by **its own** vocabulary, because that is what
/// giving two models the same prompt means. The identifier counts may
/// therefore differ, and that is a condition rather than a problem — it is
/// recorded.
///
/// `None` where the file will not parse or the vocabulary will not encode the
/// prompt, which is not a refusal: the run falls back to sending text, comes
/// out cold, and says so in its own conditions (§6.13).
fn as_identifiers(path: &Path, prompt: &str) -> Option<Vec<usize>> {
    let bytes = read_prefix(path)?;
    let file = mcf_standin::gguf::parse(&bytes).ok()?;
    let vocabulary = mcf_standin::tokenizer::Vocabulary::read(&file).ok()?;
    vocabulary.encode(prompt, true).ok()
}

/// Enough of a model file to hold its directory and vocabulary.
///
/// The same three sizes `run` uses, because reading sixteen gigabytes to
/// tokenize four words would make a benchmark's first act its slowest
/// (B-372).
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

/// How a model file's weights are encoded, counted by kind.
///
/// From a bounded prefix, growing only if the directory did not fit — the same
/// three sizes `run` uses, because reading sixteen gigabytes to name a
/// quantization would make a benchmark's first act its slowest (B-372).
/// `None` where the file will not parse, which is `Unknown` rather than a
/// guess (A7): the benchmark still runs, and reports that it could not tell
/// what it isolated.
fn quantization_of(path: &Path) -> Option<String> {
    let bytes = read_prefix(path)?;
    let file = mcf_standin::gguf::parse(&bytes).ok()?;
    Some(crate::explain::quantizations(&file))
}

/// Resolves a model reference, or the sentence explaining why it did not.
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

/// Which engine the daemon would actually use for this model.
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

/// Whether a named engine is MCF's own stand-in.
///
/// By name, because that is what the account carries. The stand-in says so in
/// its own words and no other engine does.
fn is_a_stand_in(named: &str) -> bool {
    let named = named.to_lowercase();
    named.contains("stand-in") || named.contains("stand in")
}

/// One generation, timed.
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
    // The pin, proven: the request said how many tokens, and the account
    // says how many there were. A trial that produced fewer is not a trial
    // under this discipline — its duration is a duration of something else,
    // and dividing it by the pinned count would put a per-token figure on
    // the record that no token cost (B-396, A21).
    if let Some(pinned) = limit {
        held_the_pin(&account, pinned)?;
    }
    // §6.13: what the trial reused is a condition of it, and the daemon
    // already says so in its account. Reading it is what makes a warm
    // measurement distinguishable from a cold one (B-081).
    let warmth = Warmth::from_account(
        account
            .get("conditions")
            .and_then(|conditions| conditions.get("loaded"))
            .and_then(Value::as_text),
    );
    Ok((took, warmth))
}

/// Whether the account proves the trial produced what it was pinned to.
///
/// The count is the engine's, read back rather than assumed from the request
/// (A21). A path that hands back text and no count — the completion tool —
/// says so under `conditions.length`, and a pin through it is asked and not
/// proven, which is not the same as held.
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

/// One generation: how long the whole request took, and its account.
///
/// The monotonic clock, always (D9), and the whole request rather than the
/// tokens alone — what an operator waits for is the request, and a figure that
/// excluded the parts MCF chose not to count would be a figure about a
/// subset nobody named (§3.4).
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

    // Identifiers where MCF could produce them, because that is what reaches
    // the provisioned engine's server rather than a fresh process per request
    // (B-376, F64). Where it could not, the text goes and the run says it was
    // cold.
    let request = Request::Generate {
        // `mcf bench --prompt` is the operator's own text, held still across
        // both arms — theirs, not MCF's (§6.8, B-146).
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
        // A timing divides by the count, so the count is pinned, and the
        // account's own count is read back below to prove it was (B-396).
        pinned: true,
        turn: None,
        image: None,
        // What the engine is started with beyond the plain load, held the
        // same across both arms: a timing under a draft head is a timing of
        // that condition, and the two arms must be under one (A6, B-463).
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

/// Writes the comparison to the record, and says where or why not.
fn keep(
    held: &Comparison<Monotonic>,
    finding: &mcf_bench::compare::Finding,
    method: &Method,
    machine: Option<&MachineHeld>,
    at: Timestamp,
) -> Result<PathBuf, String> {
    let Some(path) = mcf_record::journal::default_path() else {
        return Err("there is nowhere to write a record on this machine".to_owned());
    };
    let mut journal =
        Journal::open(&path).map_err(|failure| format!("the record would not open — {failure}"))?;
    // The finding the operator was shown, not one recomputed here. Recomputing
    // it took the *default* resolution rather than the one they asked about, so
    // a caller who asked about half a percent was shown one verdict and the
    // record kept another — two answers to one question (A6).
    let body = record::comparison(held, finding, method, machine);
    let appended = journal
        .append(&Record::new(EntryKind::Comparison, at, body))
        .map_err(|failure| format!("the comparison would not append — {failure}"))?;
    // The prompt is a condition and it is content, so the record keeps its
    // length and its digest and the text goes beside it, under this entry's
    // identifier (A25, A6, F105). A prompt that cannot be filed is said rather
    // than swallowed: the comparison is written either way, and what is missing
    // is what a bundle would have disclosed.
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

/// A duration as a person reads it, without a float.
///
/// Milliseconds and tenths, from integer arithmetic: this crate holds no
/// floating-point number and a rendering is not a reason to introduce one (A6).
fn milliseconds(held: Duration<Monotonic>) -> String {
    let tenths = held.as_nanos().wrapping_div(100_000);
    format!("{}.{} ms", tenths.wrapping_div(10), tenths.wrapping_rem(10))
}

/// The rules of thumb that apply to what is on the screen (B-380, DEC-002).
///
/// **Placed here because a number teaches by contrast.** The operator's answer
/// to DEC-002 put touchstones in the comparison view first: *0.77 tokens per
/// character means nothing alone and everything beside 2.00*, so a reader
/// placed in front of two values sees the difference a touchstone describes
/// instead of being asked to believe it.
///
/// **Only where the subject is on the screen.** A rule of thumb about
/// quantization beside a comparison of two unrelated models is a sentence about
/// something the reader is not looking at, and the way a guidance section stops
/// being read is by containing things that do not apply. So each is chosen by
/// what this comparison actually isolated and what verdict it reached.
///
/// They are separated by their own rule and their own heading, and each renders
/// through `Touchstone`'s `Display`, which cannot omit the mark or the limits.
fn rules_of_thumb(finding: &mcf_bench::compare::Finding, planned: &Planned) -> Vec<String> {
    use mcf_bench::enough::Verdict;
    use mcf_core::touchstone::CATALOGUE;

    let mut apt: Vec<&mcf_core::touchstone::Touchstone> = Vec::new();
    let about = |subject: &str| CATALOGUE.iter().find(|held| held.subject() == subject);

    // What the comparison isolated: a quantization frontier is the case this
    // touchstone is for, and it is read from what the arms differ in rather
    // than from what the caller called them.
    let said = format!("{} {}", planned.work, finding.isolation()).to_lowercase();
    if said.contains("quantization") {
        apt.extend(about("a smaller quantization"));
    }
    match finding.verdict() {
        // A difference that was established: the reader now has a size, and
        // the size is the thing a rule of thumb is about.
        Some(Verdict::Differ { .. } | Verdict::Apart { .. }) => {
            apt.extend(about("a difference this small"));
        }
        // A null result, which is a result (A9) and the one most often
        // over-read: *no difference* is about this prompt on this machine.
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
            "    A laboratory would replace this with a measurement: {} (B-380)",
            held.until()
        ));
    }
    lines
}

/// What the operator reads.
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
        // What both engines were started with beyond the plain load: a
        // comparison under a draft head is a comparison of that condition,
        // and two runs are only comparable if each says which it was (A6,
        // B-463).
        format!("  started  {}", started.said()),
    ];
    // Everything that stops this travelling, all of it (A1): a run can be
    // outside the band *and* fail to establish its size, and a reader told
    // only one will fix that one and be surprised again.
    for why in finding.not_fit_to_contribute() {
        lines.push(format!("  NOT FIT TO CONTRIBUTE — {why}"));
    }
    if let Some(proposal) = planned.proposal.as_ref() {
        // §3.1: *ran 6 of 20* is always accompanied by the fourteen, and the
        // fourteen are in the record rather than only on the screen.
        lines.push(format!("  budget   {proposal}"));
    }
    if let Some((left, right)) = held.medians() {
        // The absolute, beside the comparison and never instead of it (§3.27):
        // it answers *will this fit in my latency budget*, which a ratio
        // cannot, and it is the figure that does not travel.
        lines.push(format!(
            "  medians  {} and {} — local figures, which do not travel (§3.27)",
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
        lines.push("           back and unmake them (A4).".to_owned());
    }
    lines.extend(rules_of_thumb(finding, planned));
    lines.push(match written {
        Ok(path) => format!("  recorded {}", path.display()),
        // A6 and A2: the measurement stands, and the fact that it was not kept
        // is said rather than swallowed.
        Err(why) => format!("  NOT RECORDED — {why}"),
    });
    if !held.reuse().is_uniform() {
        lines.push(String::new());
        for said in [
            "  The trials were not alike, so there is no delta (§6.13). One model is",
            "  resident at a time and a paired comparison alternates them, so most",
            "  trials reload and some do not — which of them is a property of the",
            "  drawn order rather than of either arm (F65).",
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
        lines.push(
            "  This run could not decide, and B24 says MCF must not attribute that".to_owned(),
        );
        lines.push(
            "  to the arms. What it can do is say what else was here (PR5, §3.8).".to_owned(),
        );
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
