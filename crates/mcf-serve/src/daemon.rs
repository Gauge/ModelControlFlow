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

/// How long a stopping daemon waits for the requests it closed to finish
/// closing before it records its stop. Each closes as soon as the engine
/// notices its connection gone, which the pinned server polls for once a
/// second.
const CLOSING: std::time::Duration = std::time::Duration::from_secs(10);

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
/// **A hundred and sixty was too few, and the report did not say it was
/// cutting.** The reasoning was that comparing whether two answers differ is
/// settled as well by a short answer as a long one. It is not, when the answer
/// is code: asked for a C# class, every generation stopped inside the import
/// block, so what was compared was six lines of preamble that shift wholesale
/// when anything before them changes. That is a measurement of the preamble,
/// and it put the floor at 94.3% — a run that could separate nothing (F147).
///
/// Six hundred is enough for a small class or function and still finishes: a
/// prompt of six sentences is ten generations, which is the cost this command
/// has always had. The report now says what the limit is and whether an answer
/// reached it, because an answer that was cut is a condition of every figure
/// computed from it (§3.4, A6).
const PROMPT_REPORT_LIMIT: usize = 600;

/// Where the model ranked each token of a prompt, or why it was not read.
struct RankedPrompt {
    /// One row a position, empty where the reading was not taken.
    rows: Vec<Value>,
    /// Why, where it was not.
    refused: Option<String>,
    /// What the prompt was addressed as while it was read.
    under: Option<String>,
}

/// The floor at every position, as a client reads it: null where one draw
/// was taken (B-434).
fn floors_value(floors: Option<&[crate::prompt::FloorAt]>) -> Value {
    let count = |held: usize| Value::Integer(i64::try_from(held).unwrap_or(i64::MAX));
    floors.map_or(Value::Null, |floors| {
        Value::List(
            floors
                .iter()
                .map(|at| {
                    Value::map([
                        ("position", count(at.position)),
                        (
                            "moved_parts_per_million",
                            Value::Integer(i64::try_from(at.moved).unwrap_or(i64::MAX)),
                        ),
                        ("held", held_value(at.held)),
                    ])
                })
                .collect(),
        )
    })
}

/// One reading of a variant prompt, as a client reads it: null where none
/// was taken (B-435).
fn reading_value(reading: Option<&crate::prompt::Reading>) -> Value {
    reading.map_or(Value::Null, |read| {
        Value::map([
            (
                "moved_parts_per_million",
                Value::Integer(i64::try_from(read.moved).unwrap_or(i64::MAX)),
            ),
            ("held", held_value(read.held)),
            ("answer", Value::text(read.answer.clone())),
        ])
    })
}

/// A list of readings, one a part: null where none was taken.
fn readings_value(readings: Option<&[crate::prompt::Reading]>) -> Value {
    readings.map_or(Value::Null, |readings| {
        Value::List(
            readings
                .iter()
                .map(|read| reading_value(Some(read)))
                .collect(),
        )
    })
}

/// The same parts in each form, as a client reads them: a form's name and
/// its reading, or the form and why it was not rendered (B-444). Null
/// where the forms were not asked, which is not *no form moved it* (A7).
fn forms_value(forms: Option<&[crate::prompt::Formed]>) -> Value {
    forms.map_or(Value::Null, |forms| {
        Value::List(
            forms
                .iter()
                .map(|formed| {
                    let mut fields = vec![(
                        "form".to_owned(),
                        Value::text(formed.form.name().to_owned()),
                    )];
                    match &formed.outcome {
                        crate::prompt::Rendering::Read(read) => {
                            if let Value::Map(reading) = reading_value(Some(read)) {
                                fields.extend(reading);
                            }
                        }
                        crate::prompt::Rendering::NotRendered(why) => {
                            fields
                                .push(("not_rendered".to_owned(), Value::text((*why).to_owned())));
                        }
                    }
                    Value::map(fields)
                })
                .collect(),
        )
    })
}

/// A served form's name and figures and none of its text, for the record
/// (A25): a form not rendered keeps why (A7). Null stays null.
fn forms_figures(served: Option<&Value>) -> Value {
    let Some(Value::List(forms)) = served else {
        return Value::Null;
    };
    Value::List(
        forms
            .iter()
            .map(|formed| {
                let kept = |key: &str| formed.get(key).cloned().unwrap_or(Value::Null);
                Value::map([
                    ("form", kept("form")),
                    ("not_rendered", kept("not_rendered")),
                    ("moved_parts_per_million", kept("moved_parts_per_million")),
                    ("held", kept("held")),
                ])
            })
            .collect(),
    )
}

/// A served reading's figures and none of its text, for the record (A25):
/// null stays null.
fn reading_figures(served: Option<&Value>) -> Value {
    let figures = |read: &Value| {
        Value::map([
            (
                "moved_parts_per_million",
                read.get("moved_parts_per_million")
                    .cloned()
                    .unwrap_or(Value::Null),
            ),
            ("held", read.get("held").cloned().unwrap_or(Value::Null)),
        ])
    };
    match served {
        Some(Value::List(readings)) => Value::List(readings.iter().map(figures).collect()),
        Some(read @ Value::Map(_)) => figures(read),
        _ => Value::Null,
    }
}

/// The least, middle and most of the floors: null where there is one.
fn spread_value(spread: Option<crate::prompt::Spread>) -> Value {
    let ppm = |held: u64| Value::Integer(i64::try_from(held).unwrap_or(i64::MAX));
    spread.map_or(Value::Null, |spread| {
        Value::map([
            ("least_parts_per_million", ppm(spread.least)),
            ("middle_parts_per_million", ppm(spread.middle)),
            ("most_parts_per_million", ppm(spread.most)),
        ])
    })
}

/// A forced reading, as a client reads it: null where none was taken.
fn held_value(held: Option<crate::prompt::Held>) -> Value {
    held.map_or(Value::Null, |held| {
        Value::map([
            (
                "first_rank",
                held.first.map_or(Value::Null, |rank| {
                    Value::Integer(i64::try_from(rank).unwrap_or(i64::MAX))
                }),
            ),
            (
                "kept",
                Value::Integer(i64::try_from(held.kept).unwrap_or(i64::MAX)),
            ),
            (
                "of",
                Value::Integer(i64::try_from(held.of).unwrap_or(i64::MAX)),
            ),
        ])
    })
}

/// Who decided what the document was taken apart into (B-430, §3.15).
///
/// A report by paragraph and one by sentence are different measurements of
/// the same text, and which it was is a condition of the run.
fn unit_chosen_by(report: &crate::prompt::Report) -> &'static str {
    if report.unit_chosen {
        "chosen"
    } else if report.unit == crate::prompt::Unit::Paragraph {
        "text: blank lines"
    } else {
        "text: no blank line"
    }
}

/// What several seeds made of the prompt at a stated temperature, as the
/// wire carries it: the condition first, then the figures under it (§3.4).
fn settled_value(settled: Option<&crate::prompt::Settled>) -> Value {
    let Some(settled) = settled else {
        return Value::Null;
    };
    let count = |held: usize| Value::Integer(i64::try_from(held).unwrap_or(i64::MAX));
    let ppm = |held: u64| Value::Integer(i64::try_from(held).unwrap_or(i64::MAX));
    Value::map(
        [
            (
                "temperature_thousandths",
                Value::Integer(i64::from(settled.temperature.0)),
            ),
            ("temperature", Value::text(settled.temperature.to_string())),
            ("seeds_asked", count(settled.asked)),
            ("distinct_answers", count(settled.distinct)),
            ("spread_parts_per_million", ppm(settled.spread)),
            ("from_greedy_parts_per_million", ppm(settled.from_greedy)),
        ]
        .into_iter()
        .chain(settled.truncation.entries())
        .collect::<Vec<(&str, Value)>>(),
    )
}

/// One ablated part, as a client reads it.
/// A count where one was taken, and null where none was: nought is a
/// figure and *not counted* is not one (A7).
fn counted(held: Option<usize>) -> Value {
    held.map_or(Value::Null, |held| {
        Value::Integer(i64::try_from(held).unwrap_or(i64::MAX))
    })
}

fn clause_value(clause: &crate::prompt::Clause) -> Value {
    Value::map([
        ("text", Value::text(clause.text.clone())),
        ("changed", Value::Bool(clause.changed)),
        (
            "moved_parts_per_million",
            Value::Integer(i64::try_from(clause.moved).unwrap_or(i64::MAX)),
        ),
        ("without", Value::text(clause.without.clone())),
        ("held", held_value(clause.held)),
        // What the model spent thinking without this part. Null is *not
        // counted* — a turn with no marker to think inside — and not
        // nought (A7, B-455).
        ("thought", counted(clause.thought)),
    ])
}

/// The figures of a served prompt report, for the record, and none of its
/// text (A25, B-432).
///
/// Built by naming what is kept rather than by removing what is not: a key
/// added to the served report later is absent from the record until somebody
/// decides it belongs there, which is the direction a mistake should fall.
/// The prompt is present as its length, its parts and its digest — enough to
/// tell a second run of the same text from a run of a changed one — and the
/// answer as its length.
/// The conditions a recorded prompt report carries (§3.4).
fn prompt_report_conditions(
    served: &Value,
    model: &Path,
    seed: u64,
    engines: &std::collections::BTreeSet<String>,
) -> Value {
    let kept = |key: &str| served.get(key).cloned().unwrap_or(Value::Null);
    Value::map([
        ("model", Value::text(model.display().to_string())),
        (
            "engines",
            Value::List(engines.iter().cloned().map(Value::text).collect()),
        ),
        (
            "seed",
            Value::Integer(i64::try_from(seed).unwrap_or(i64::MAX)),
        ),
        ("unit", kept("unit")),
        ("unit_chosen_by", kept("unit_chosen_by")),
        ("most", kept("most")),
        ("token_limit", kept("token_limit")),
        ("sampler", kept("sampler")),
        ("addressed_as", kept("addressed_as")),
        ("asked_as", kept("asked_as")),
        ("ranked_under", kept("ranked_under")),
        ("read_by", kept("read_by")),
        ("forced_depth", kept("forced_depth")),
        ("ranked_depth", kept("ranked_depth")),
    ])
}

fn prompt_report_entry(
    served: &Value,
    model: &Path,
    prompt: &str,
    seed: u64,
    engines: &std::collections::BTreeSet<String>,
) -> Value {
    let kept = |key: &str| served.get(key).cloned().unwrap_or(Value::Null);
    let count = |held: usize| Value::Integer(i64::try_from(held).unwrap_or(i64::MAX));
    let characters = |key: &str| {
        served
            .get(key)
            .and_then(Value::as_text)
            .map_or(Value::Null, |text| count(text.chars().count()))
    };
    let clauses = served
        .get("clauses")
        .and_then(Value::as_list)
        .unwrap_or(&[])
        .iter()
        .map(|clause| {
            Value::map([
                (
                    "characters",
                    clause
                        .get("text")
                        .and_then(Value::as_text)
                        .map_or(Value::Null, |text| count(text.chars().count())),
                ),
                (
                    "changed",
                    clause.get("changed").cloned().unwrap_or(Value::Null),
                ),
                (
                    "moved_parts_per_million",
                    clause
                        .get("moved_parts_per_million")
                        .cloned()
                        .unwrap_or(Value::Null),
                ),
                ("held", clause.get("held").cloned().unwrap_or(Value::Null)),
            ])
        })
        .collect();
    // The words the model did not expect are words of the prompt; what is
    // kept is how many positions were read and how many were first choice.
    let expected = served
        .get("expected")
        .and_then(Value::as_list)
        .unwrap_or(&[]);
    let first_choice = expected
        .iter()
        .filter(|row| matches!(row.get("rank"), Some(Value::Integer(1))))
        .count();
    Value::map([
        (
            "conditions",
            prompt_report_conditions(served, model, seed, engines),
        ),
        (
            "prompt",
            Value::map([
                ("characters", count(prompt.chars().count())),
                ("parts", count(clauses_counted(served))),
                ("tokens", kept("prompt_tokens")),
                ("tokens_refused", kept("prompt_tokens_refused")),
                (
                    "sha256",
                    Value::text(mcf_core::digest::sha256(prompt.as_bytes()).hex()),
                ),
            ]),
        ),
        ("answer_characters", characters("baseline")),
        ("answer_tokens", kept("answer_tokens")),
        ("answer_stopped", kept("answer_stopped")),
        ("floor_parts_per_million", kept("floor_parts_per_million")),
        ("floor_held", kept("floor_held")),
        ("held_refused", kept("held_refused")),
        ("floors", kept("floors")),
        ("floor_spread", kept("floor_spread")),
        ("alone", reading_figures(served.get("alone"))),
        ("alone_floor", reading_figures(served.get("alone_floor"))),
        ("prefixes", reading_figures(served.get("prefixes"))),
        ("swaps", reading_figures(served.get("swaps"))),
        ("forms", forms_figures(served.get("forms"))),
        ("clauses", Value::List(clauses)),
        ("clauses_over_the_cap", kept("clauses_over_the_cap")),
        ("settled", kept("settled")),
        ("generations", kept("generations")),
        (
            "expected_read",
            count(
                expected
                    .iter()
                    .filter(|row| row.get("read").and_then(Value::as_bool) != Some(false))
                    .count(),
            ),
        ),
        ("expected_first_choice", count(first_choice)),
        ("expected_by_part", kept("expected_by_part")),
        ("expected_by_word", words_counted(served)),
        ("expected_refused", kept("expected_refused")),
    ])
}

/// The word reading as figures for the record: how many words, how many
/// in more than one piece, how many whose first piece was past the depth
/// read — the words themselves are the prompt's and stay out (A25).
fn words_counted(served: &Value) -> Value {
    let Some(words) = served
        .get("expected_by_word")
        .and_then(|held| held.get("words"))
        .and_then(Value::as_list)
    else {
        return Value::Null;
    };
    let count = |held: usize| Value::Integer(i64::try_from(held).unwrap_or(i64::MAX));
    let integer = |word: &Value, key: &str| word.get(key).and_then(Value::as_integer);
    Value::map([
        ("words", count(words.len())),
        (
            "in_pieces",
            count(
                words
                    .iter()
                    .filter(|word| integer(word, "pieces").unwrap_or(0) > 1)
                    .count(),
            ),
        ),
        (
            "past_depth",
            count(
                words
                    .iter()
                    .filter(|word| {
                        integer(word, "pieces").unwrap_or(0) > 0 && integer(word, "rank").is_none()
                    })
                    .count(),
            ),
        ),
    ])
}

/// How many parts the prompt had: the ones ablated and the ones over the cap.
fn clauses_counted(served: &Value) -> usize {
    let ablated = served
        .get("clauses")
        .and_then(Value::as_list)
        .map_or(0, <[Value]>::len);
    let over = served
        .get("clauses_over_the_cap")
        .and_then(Value::as_integer)
        .and_then(|held| usize::try_from(held).ok())
        .unwrap_or(0);
    ablated.saturating_add(over)
}

/// Tells a client its request filled the ceiling without ending.
///
/// Before this, the daemon read its 64 kibibytes, failed to parse the
/// fragment, and closed while the client was still writing — so the client
/// saw a connection reset and no reason at all, which is the silent failure
/// A2 forbids. The probe that found it reported *a line of the stream was
/// unreadable*, which was true and useless (B-055, F42).
fn refuse_an_unending_request(writer: &mut &UnixStream) {
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
        "a turn of token identifiers this long exceeds what the control protocol carries; ask \
         for fewer, or a build with a larger ceiling",
    );
    let answer = Answer::refused(&failure);
    let _written = writeln!(writer, "{}", answer.to_line());
    let _flushed = writer.flush();
    // The client is still writing, and closing now would lose the answer to
    // a reset. Its own write timeout ends this; MCF reads nothing further
    // into memory.
    let _shutdown = writer.shutdown(std::net::Shutdown::Read);
}

/// A served rank row as the placement reads it: a rank, or a null rank
/// that is past the depth read, or a null rank the row itself says was
/// never read — the first piece of a turn nothing preceded (F160).
fn rank_rows(ranked: &[Value]) -> Vec<(String, crate::prompt::Rank)> {
    use crate::prompt::Rank;
    ranked
        .iter()
        .map(|row| {
            let text = row
                .get("text")
                .and_then(Value::as_text)
                .unwrap_or_default()
                .to_owned();
            let rank = match row
                .get("rank")
                .and_then(Value::as_integer)
                .and_then(|held| usize::try_from(held).ok())
            {
                Some(at) => Rank::At(at),
                None if row.get("read").and_then(Value::as_bool) == Some(false) => Rank::NoContext,
                None => Rank::PastDepth,
            };
            (text, rank)
        })
        .collect()
}

/// The rank reading grouped by part (B-433): which part the model least
/// expected, spending no generation. Null where no reading was taken, so
/// that an absent reading is not served as *all expected* (A7).
fn expected_by_part_value(parts: &[crate::prompt::Part], ranked: &[Value]) -> Value {
    if ranked.is_empty() {
        return Value::Null;
    }
    let pairs = rank_rows(ranked);
    let (found, nowhere) = crate::prompt::surprise_by_part(parts, &pairs);
    let count = |held: usize| Value::Integer(i64::try_from(held).unwrap_or(i64::MAX));
    Value::map([
        (
            "parts",
            Value::List(
                found
                    .iter()
                    .map(|held| {
                        Value::map([
                            ("tokens", count(held.tokens)),
                            ("first_choice", count(held.first_choice)),
                            ("past_depth", count(held.past_depth)),
                            ("no_context", count(held.no_context)),
                        ])
                    })
                    .collect(),
            ),
        ),
        ("unplaced", count(nowhere)),
    ])
}

/// The rank reading by word (B-443): each word's pieces and the rank of
/// its first, in the order written, for a reader asking *which words*. The
/// words are the prompt's and go to the client, not the record (A25). Null
/// where no reading was taken (A7).
fn expected_by_word_value(parts: &[crate::prompt::Part], ranked: &[Value]) -> Value {
    if ranked.is_empty() {
        return Value::Null;
    }
    let pairs = rank_rows(ranked);
    let (found, nowhere) = crate::prompt::expected_by_word(parts, &pairs);
    let count = |held: usize| Value::Integer(i64::try_from(held).unwrap_or(i64::MAX));
    Value::map([
        (
            "words",
            Value::List(
                found
                    .into_iter()
                    .map(|word| {
                        Value::map([
                            ("text", Value::text(word.text)),
                            ("pieces", count(word.pieces)),
                            ("rank", word.rank.map_or(Value::Null, count)),
                            ("first_choice", count(word.first_choice)),
                            ("unread", count(word.unread)),
                            ("part", word.part.map_or(Value::Null, count)),
                        ])
                    })
                    .collect(),
            ),
        ),
        ("unplaced", count(nowhere)),
    ])
}

/// A prompt report, as a client reads it.
fn prompt_report_value(
    report: &crate::prompt::Report,
    parts: &[crate::prompt::Part],
    generations: usize,
    tokens: core::result::Result<usize, String>,
    ranked: RankedPrompt,
    read_by: String,
    not_held: Option<String>,
) -> Value {
    let RankedPrompt {
        rows: ranked,
        refused: no_ranking,
        under: ranked_under,
    } = ranked;
    let by_part = expected_by_part_value(parts, &ranked);
    let by_word = expected_by_word_value(parts, &ranked);
    Value::map([
        ("baseline", Value::text(report.baseline.clone())),
        // What the prompt as written cost in thinking before its answer
        // began, where the turn had a marker to think inside (B-455).
        ("baseline_thought", counted(report.baseline_thought)),
        (
            "floor_parts_per_million",
            Value::Integer(i64::try_from(report.floor).unwrap_or(i64::MAX)),
        ),
        // **The forced reading** (B-429): whether the model would still have
        // begun the baseline's answer under each shortened prompt, and under
        // the inert one. `forced_depth` bounds a null `first_rank`: outside
        // the sixty read is a bound, not an absence (A7).
        ("floor_held", held_value(report.floor_held)),
        ("floor_thought", counted(report.floor_thought)),
        // Why the forced reading was not taken, where it was not: the first
        // reason met, since every part's reading is the same operation and
        // fails the same way (A2, F160).
        ("held_refused", not_held.map_or(Value::Null, Value::text)),
        // **The floor at every position, where asked** (B-434): null where
        // one draw was taken, which a reader must not read as a spread of
        // nothing (A7).
        ("floors", floors_value(report.floors.as_deref())),
        ("floor_spread", spread_value(report.floor_spread())),
        // **Each part alone, where asked** (B-435): null where not, which is
        // not *no part carries the answer on its own* (A7).
        ("alone", readings_value(report.alone.as_deref())),
        ("alone_floor", reading_value(report.alone_floor.as_ref())),
        // **The prompt grown from the front, where asked** (B-436).
        ("prefixes", readings_value(report.prefixes.as_deref())),
        // **Neighbouring parts swapped, where asked** (B-437).
        ("swaps", readings_value(report.swaps.as_deref())),
        // **The same parts in each form, where asked** (B-444).
        ("forms", forms_value(report.forms.as_deref())),
        (
            "forced_depth",
            Value::Integer(i64::try_from(crate::generation::HOW_DEEP).unwrap_or(i64::MAX)),
        ),
        (
            "ranked_under",
            ranked_under.map_or(Value::Null, Value::text),
        ),
        (
            "clauses",
            Value::List(report.clauses.iter().map(clause_value).collect()),
        ),
        (
            "clauses_over_the_cap",
            Value::Integer(i64::try_from(report.clauses_over_the_cap).unwrap_or(i64::MAX)),
        ),
        ("unit", Value::text(report.unit.name().to_owned())),
        (
            "unit_chosen_by",
            Value::text(unit_chosen_by(report).to_owned()),
        ),
        (
            "most",
            Value::Integer(i64::try_from(report.most).unwrap_or(i64::MAX)),
        ),
        // Filled in by the caller from the generations' own accounts: what
        // they were addressed as is theirs to say (A21, F160).
        ("addressed_as", Value::Null),
        // What was asked of the model's own template, where anything was: a
        // report taken under a system turn is a report of the prompt inside
        // that turn, and the figures are not the bare prompt's (B-455, D43).
        ("asked_as", Value::Null),
        // **Settledness, under its condition or not at all.** The seeds are
        // drawn at a temperature the caller stated, and where none was the
        // question was not asked: `settled` is then null, which a reader
        // must not read as *settled* (A7, B-431).
        ("settled", settled_value(report.settled.as_ref())),
        (
            "generations",
            Value::Integer(i64::try_from(generations).unwrap_or(i64::MAX)),
        ),
        // **What each answer was allowed to be, and whether any hit it.** A
        // figure computed from an answer that was cut is a figure about a
        // prefix, and a reader comparing two prefixes of long answers is
        // measuring the preamble. The condition travels with the report
        // (§3.4, A6, F147).
        (
            "token_limit",
            Value::Integer(i64::try_from(PROMPT_REPORT_LIMIT).unwrap_or(i64::MAX)),
        ),
        (
            "sampler",
            Value::text(
                "greedy · temperature 0 · seed held · as written, every removal, the control",
            ),
        ),
        (
            "prompt_tokens",
            tokens.as_ref().map_or(Value::Null, |held| {
                Value::Integer(i64::try_from(*held).unwrap_or(i64::MAX))
            }),
        ),
        (
            "prompt_tokens_refused",
            tokens.err().map_or(Value::Null, Value::text),
        ),
        // **Who read the prompt** (B-441): the engine that generated, so the
        // count and the ranks are of the prompt the answers were given.
        ("read_by", Value::text(read_by)),
        ("expected", Value::List(ranked)),
        ("expected_by_part", by_part),
        ("expected_by_word", by_word),
        (
            "expected_refused",
            no_ranking.map_or(Value::Null, Value::text),
        ),
        (
            "ranked_depth",
            Value::Integer(i64::try_from(crate::generation::HOW_DEEP).unwrap_or(i64::MAX)),
        ),
    ])
}

/// What a prompt report's generations were addressed as, from their own
/// accounts: the addressing on file where one was applied, and the bare
/// prompt said as such where none was — a report over a prompt sent with
/// no turn markers used to say *one user turn* (A21, F160).
fn addressed_as(seen: &std::collections::BTreeSet<String>) -> String {
    if seen.is_empty() {
        return format!("{} · mcf probe sets one", crate::generation::BARE_PROMPT);
    }
    seen.iter().cloned().collect::<Vec<_>>().join(" · ")
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

/// The engines under this daemon's home, each asked once what it computes on.
fn discover_engines(places: &Places) -> Vec<(crate::engines::Engine, Vec<crate::engines::Device>)> {
    let home = places.models.parent().unwrap_or(&places.models).to_owned();
    let free = system_memory_free();
    crate::engines::discover(&home)
        .into_iter()
        .map(|engine| {
            let devices = engine.devices(free).unwrap_or_default();
            (engine, devices)
        })
        .collect()
}

/// The engine a model on this machine would need built: decided by whether
/// an accelerator's driver is loaded, which is a directory listing.
fn needed_engine() -> Option<&'static mcf_core::component::Component> {
    crate::engines::required(crate::engines::accelerator_driver_present())
}

/// Memory free for a new process, or `None` where the platform will not say.
///
/// Available rather than total: what matters is what a model could take now,
/// not what the machine has in principle.
///
/// **And what this process may take, which is not always what the machine
/// has.** Under a container or a scope with a memory limit, `/proc/meminfo`
/// reports the host's free memory — true, and about somewhere else. Read
/// directly here, MCF sized a context against 119 GiB while running under a
/// 40 GiB limit and the kernel ended the engine sixteen seconds in (F144).
/// `mcf_core::hardware` reads both and returns the smaller, and it is asked
/// here rather than repeated so that the console and the daemon cannot come
/// to different numbers about one machine (B-072).
fn system_memory_free() -> Option<u64> {
    mcf_core::hardware::memory_available_now()
}

/// What an account's failure says, as `detail (category)`, for a generation
/// that produced nothing.
fn failure_said(account: &Value) -> String {
    let failure = account.get("failure");
    let said = |key: &str| {
        failure
            .and_then(|failure| failure.get(key))
            .and_then(Value::as_text)
    };
    match (said("detail"), said("category")) {
        (Some(detail), Some(category)) => format!("{detail} ({category})"),
        (Some(detail), None) => detail.to_owned(),
        (None, Some(category)) => category.to_owned(),
        (None, None) => "the account carries no failure".to_owned(),
    }
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
#[allow(
    clippy::too_many_arguments,
    reason = "one measurement's conditions, each of which it writes down"
)]
fn measured(
    named: &str,
    path: &Path,
    bytes: Option<u64>,
    asked: Option<&str>,
    ran_on: Option<&str>,
    readings: Vec<Value>,
    planned: &crate::ladder::Planned,
    started: crate::declared::Started,
) -> Answer {
    // Two measurements the run took and used to throw away: what reading a
    // token of prompt costs, and the time to a first token (A7).
    let prompt_reading = crate::ladder::prompt_reading(&readings, as_milliseconds);
    let first_token = crate::ladder::first_token(&readings, as_milliseconds);
    // And a third, read off the engine's process: what a token of window
    // costs in memory, against what the header planned before the run (B-424).
    let memory = crate::ladder::memory(&readings, planned);
    // And the fall-off: what a token costs more for every token of depth,
    // against what the header says that depth re-reads (B-400).
    let fall_off = crate::ladder::fall_off(&readings, planned, as_milliseconds);
    Answer::served(Value::map([
        ("measuring", Value::text(named.to_owned())),
        ("readings", Value::List(readings)),
        ("prompt_reading", prompt_reading),
        ("first_token", first_token),
        ("memory", memory),
        ("fall_off", fall_off),
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
                // What the engine was started with beyond the plain load: a
                // timing under a draft head is a timing of that condition,
                // and two runs are only comparable if each says which it was
                // (A6, B-463).
                ("started_with", started.to_value()),
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
                // What this machine read at, measured at the working set the
                // cache takes at each depth before the engine was up, so the
                // fall-off's prediction can be checked against it (B-427).
                (
                    "read_bandwidth",
                    Value::List(
                        planned
                            .bandwidth
                            .iter()
                            .map(crate::bandwidth::Reading::as_value)
                            .collect(),
                    ),
                ),
            ]),
        ),
    ]))
}

/// What the header and the machine say about memory before a run, for the
/// measured figure to be set against.
///
/// Taken before the ladder runs: the engine that serves the last rung is
/// still resident when the last line is composed, and the memory it holds is
/// not free memory the plan may count twice. The weights are the whole set,
/// not the part the model is named by (F138); where the store cannot say,
/// the one file's length stands, which is what the conditions say too.
fn planned_memory(path: &Path, held: Option<u64>, ladder: &[u64]) -> crate::ladder::Planned {
    let file = header_of(path);
    let declared = |key: &str| {
        let file = file.as_ref()?;
        let architecture = file.architecture()?;
        file.get(&format!("{architecture}.{key}"))
            .and_then(mcf_standin::gguf::Value::as_integer)
            .and_then(|value| u64::try_from(value).ok())
    };
    let per_token = file
        .as_ref()
        .and_then(crate::engines::cache_bytes_per_token);
    crate::ladder::Planned {
        per_token,
        weights: mcf_hub::store::bytes_of_the_whole(path).ok().or(held),
        free: system_memory_free(),
        trained: declared("context_length"),
        sliding_window: declared("attention.sliding_window"),
        attending_blocks: file
            .as_ref()
            .and_then(crate::engines::shape_of)
            .map(|shape| shape.blocks),
        // Measured now, with no engine up to share the memory bus: the rate
        // the engine's re-read of the cache is set against (B-427).
        bandwidth: crate::bandwidth::along_a_ladder(ladder, per_token),
    }
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
    /// The most memory the engine's process held resident, where the account
    /// says (B-424).
    peak_resident: Option<u64>,
    /// The window the engine ran in, which is what its cache was sized to.
    window: Option<u64>,
    /// How many tokens the account says were produced — the engine's count,
    /// which is what proves the pin held (B-396).
    produced: Option<u64>,
    /// Why the engine stopped, in the account's word.
    stopped: Option<String>,
}

impl Timed {
    /// Whether the run produced exactly what it was pinned to.
    ///
    /// The engine's count, not the request's: a run whose end of text came
    /// early produced fewer tokens than the difference divides by, and a
    /// per-token cost read off it would be a per-token cost of nothing in
    /// particular (B-396, A21).
    fn held_the_pin(&self, pinned: u32) -> bool {
        self.produced == Some(u64::from(pinned))
    }

    /// The pin it fell short of, said.
    fn short_of(&self, pinned: u32) -> String {
        format!(
            "the engine produced {} of the {pinned} tokens pinned, and stopped at {}",
            self.produced
                .map_or_else(|| "an unsaid number".to_owned(), |count| count.to_string()),
            self.stopped
                .as_deref()
                .unwrap_or("something it did not name"),
        )
    }
}

/// A rung with no per-token cost, and why.
///
/// No pair came back in the right order, so nothing here is a per-token
/// cost. The honest reading is that there is none — never a zero, and never
/// the unsubtracted number standing in for the subtracted one (A7, A9). And
/// where the engine refused the depth, that is the reason, not the pair
/// (F152); where it ran but did not produce the pinned count, that is
/// (B-396).
fn not_measured(depth: Value, refused: Option<String>, fell_short: Option<String>) -> Value {
    Value::map([
        ("depth", depth),
        ("measured", Value::Bool(false)),
        (
            "why",
            Value::text(match (refused, fell_short) {
                (Some(why), _) => format!("the engine produced nothing at this depth: {why}"),
                (None, Some(why)) => format!(
                    "{why} — a run that did not produce what it pinned is not a sample, \
                     because the cost a token is read off the count (B-396)"
                ),
                (None, None) => "no pair of runs at this depth separated: the longer one \
                                 finished no later than the shorter, so their difference is \
                                 not a cost"
                    .to_owned(),
            }),
        ),
    ])
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

/// How long a cross-check is expected to take, as a range in seconds.
///
/// **The stand-in is the cost.** The provisioned engine reads the model once
/// and produces a hundred-odd tokens, which is seconds; MCF's own engine then
/// holds the model dequantized and pays one forward pass — every active
/// weight read once — per position, prompt and produced. So the middle is the
/// load, plus the dequantized bytes over a stated bandwidth once per
/// position. The bandwidth is a round figure from one run — Seed-Coder-8B,
/// 33 GB dequantized, 129 positions read in 155 s on this machine's
/// processor, which is 27 GB/s — and the bounds are the ladder's measured
/// spread (0.58× to 1.42×) borrowed until this run has its own — which is
/// why it is a range and not a promise (A6, A20).
#[expect(clippy::integer_division, reason = "a bound on a duration")]
fn cross_check_seconds(bytes: Option<u64>, dequantized: Option<u64>, prompt: usize) -> (u64, u64) {
    // A gigabyte a second to load, twice: the file is in the page cache the
    // second time, and the first is a stated figure and not a measurement.
    let per_load_ms = bytes.map_or(2_000, |bytes| bytes / 1_000_000);
    // Milliseconds a pass, at twenty-five gigabytes a second through the
    // dequantized weights; a directory that does not size itself is read as
    // the file eight times over, which is what four-bit weights dequantize to.
    let pass_ms = dequantized
        .or_else(|| bytes.map(|bytes| bytes.saturating_mul(8)))
        .map_or(1_000, |bytes| bytes / 25_000_000);
    let positions = (crate::crosscheck::POSITIONS as u64).saturating_add(prompt as u64);
    let middle_ms = per_load_ms
        .saturating_mul(2)
        .saturating_add(pass_ms.saturating_mul(positions));
    let low = (middle_ms.saturating_mul(58) / 100_000).max(1);
    let high = (middle_ms.saturating_mul(142) / 100_000).max(2);
    (low, high)
}

/// A duration as whole milliseconds, for a record.
#[expect(
    clippy::integer_division,
    reason = "whole milliseconds; the rest is noise"
)]
fn milliseconds(took: mcf_core::time::Duration<Monotonic>) -> Value {
    Value::Integer(i64::try_from(took.as_nanos() / 1_000_000).unwrap_or(i64::MAX))
}

/// Why two engines could not be compared on a model, as a refusal.
fn could_not_compare(path: &Path, why: &str) -> Failure {
    Failure::new(
        Category::ProbeInconclusive,
        Attribution::Machine,
        Disposition::Refused,
        WHERE,
        "the two engines could not be compared",
    )
    .with_context("model", path.display().to_string())
    .with_context("why", why.to_owned())
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
    /// What the engine said it takes for this model, where it answered.
    takes: Option<crate::takes::Takes>,
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

/// The newest entry of one kind about each model, from the record: the
/// timings, or the cross-checks.
///
/// **Through the index and by kind**, not by replaying the journal: a daemon
/// start that parsed the whole history would cost seconds on a record that has
/// been measuring for a while (F14), and a listing that re-read it once a
/// model would cost the record's whole length once a row (F118). What this
/// reads is the entries whose kind says they are the one asked for, newest
/// last, and it keeps one per model — the model being what the entry's
/// conditions name.
fn newest_of(journal: &Path, kind: EntryKind) -> std::collections::BTreeMap<PathBuf, Value> {
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
        if located.kind() != kind {
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
    /// The newest cross-check of each model, from the record, kept the way
    /// the timings are and for the same reason: a listing that re-read the
    /// record once a model would cost its whole length once a row (F118).
    cross_checks: std::sync::Mutex<std::collections::BTreeMap<PathBuf, Value>>,
    /// The newest prompt report of each model, from the record, kept the
    /// same way (B-432).
    prompt_reports: std::sync::Mutex<std::collections::BTreeMap<PathBuf, Value>>,
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
    /// processes is a status request that costs something (§3.13). Asked
    /// again only after this daemon has built one itself, which is the one
    /// moment the set is known to have changed.
    engines: std::sync::Mutex<Vec<(crate::engines::Engine, Vec<crate::engines::Device>)>>,
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
    /// Raised when a stop was asked for: every request in flight is closed,
    /// so that a stop is not waited on behind a generation (D48).
    stopping: std::sync::atomic::AtomicBool,
    /// The requests being carried on their own threads right now, so that
    /// a status asked for while one runs can say so, with how far the
    /// engine has got (D48, B-460).
    running: std::sync::Mutex<std::collections::BTreeMap<u64, std::sync::Arc<Running>>>,
    /// Numbers the requests as they arrive.
    arrivals: std::sync::atomic::AtomicU64,
}

/// One request being carried on its own thread.
#[derive(Debug)]
struct Running {
    /// What kind of request it is, in a word.
    what: &'static str,
    /// The model it names.
    model: String,
    /// When it arrived.
    since: Timestamp,
    /// Since when, on the monotonic clock, for how long it has run.
    began: Instant<Monotonic>,
    /// How far the engine has got, where the engine is the served one.
    progress: crate::served::Progress,
}

impl Running {
    fn to_value(&self) -> Value {
        Value::map([
            ("doing", Value::text(self.what)),
            ("model", Value::text(self.model.clone())),
            ("since", mcf_record::encode::timestamp(self.since)),
            (
                "nanoseconds",
                Value::Integer(
                    i64::try_from(
                        SystemClock
                            .now()
                            .saturating_duration_since(self.began)
                            .as_nanos(),
                    )
                    .unwrap_or(i64::MAX),
                ),
            ),
            ("engine", self.progress.to_value()),
        ])
    }
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
        let engines = discover_engines(&places);
        let daemon = Self {
            timings: std::sync::Mutex::new(newest_of(&places.journal, EntryKind::ModelTimed)),
            cross_checks: std::sync::Mutex::new(newest_of(
                &places.journal,
                EntryKind::CrossChecked,
            )),
            prompt_reports: std::sync::Mutex::new(newest_of(
                &places.journal,
                EntryKind::PromptReported,
            )),
            holding: std::sync::Mutex::new(None),
            places,
            listener,
            started,
            since: SystemClock.now(),
            recovered,
            engines: std::sync::Mutex::new(engines),
            resident: std::sync::Mutex::new(None),
            server: std::sync::Mutex::new(None),
            stopping: std::sync::atomic::AtomicBool::new(false),
            running: std::sync::Mutex::new(std::collections::BTreeMap::new()),
            arrivals: std::sync::atomic::AtomicU64::new(0),
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
            self.engines_held()
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
    ///
    /// **What the daemon's own server holds is free for this question.**
    /// A prompt report's second run on a 30B model was told the model did
    /// not fit, on the machine that had answered its first run a moment
    /// before: the memory group's headroom had been taken by the server
    /// holding that very model, and the arithmetic counted it as gone
    /// (F160). It is not gone. A request for the model it holds reuses the
    /// server, and a request for another stops it first (D41) — either way
    /// what it holds resident comes back to the request, so it is added
    /// here and nowhere else. A server another thread is generating with is
    /// spoken for, and counts as nothing.
    fn engines_now(&self) -> Vec<(crate::engines::Engine, Vec<crate::engines::Device>)> {
        let held = self
            .server
            .try_lock()
            .ok()
            .and_then(|slot| {
                slot.as_ref()
                    .and_then(crate::served::Served::resident_bytes)
            })
            .unwrap_or(0);
        let free = system_memory_free().map(|free| free.saturating_add(held));
        self.engines_held()
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

    /// The engines as sampled, copied out from under the lock.
    ///
    /// A poisoned lock is a thread that panicked while holding it, which this
    /// crate's lints forbid; the list is taken anyway rather than reported
    /// empty, because *no engine* is a claim about the disk (A7).
    fn engines_held(&self) -> Vec<(crate::engines::Engine, Vec<crate::engines::Device>)> {
        self.engines
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
    }

    /// Looks for the engines again, after this daemon has built one.
    ///
    /// The one exception to *asked once*: a build this daemon just finished
    /// is a change to the disk it knows about, and an engine it built and
    /// then could not see would be F31 by MCF's own hand — the prefix there
    /// and the daemon saying no engine is. Each engine is run once to ask its
    /// devices, as at start-up.
    fn rediscover(&self) {
        let found = discover_engines(&self.places);
        *self
            .engines
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = found;
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
            (
                "cross_checked",
                self.cross_checks
                    .lock()
                    .ok()
                    .and_then(|held| held.get(path).cloned())
                    .unwrap_or(Value::Null),
            ),
            (
                "prompt_reported",
                self.prompt_reports
                    .lock()
                    .ok()
                    .and_then(|held| held.get(path).cloned())
                    .unwrap_or(Value::Null),
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
        if self.engines_held().is_empty() {
            return Value::List(vec![Value::text(
                "run a model: no engine is installed yet — MCF builds one when a model is held",
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
        // **A request that takes minutes is carried on its own thread, and
        // the socket keeps answering** (D48, B-460). One thread answered
        // everything in turn, and a generation that ran for hours had every
        // status behind it time out: a person could not be told that the
        // daemon was busy, because the only thing that could tell them was
        // busy. The short questions are answered here; the long ones are
        // spawned; and a stop raises the flag that closes every request in
        // flight before the threads are joined, so a stop is not waited for
        // behind a generation either.
        std::thread::scope(|scope| {
            let stopped = loop {
                let connection = match self.listener.accept() {
                    Ok((stream, _)) => stream,
                    Err(error) => {
                        break Stopped::Broken {
                            failure: Box::new(unusable(
                                "the control socket",
                                &self.places.socket,
                                &error,
                            )),
                        };
                    }
                };
                if let Some(stopped) = self.answer_one(scope, &connection) {
                    break stopped;
                }
            };
            self.stopping
                .store(true, std::sync::atomic::Ordering::Relaxed);
            self.let_the_carried_close();
            self.stopped(&stopped);
            stopped
        })
    }

    /// Waits for the requests in flight to close, which the raised flag
    /// makes them do, so that their accounts land in the record before the
    /// daemon's own stop does: a record that said *stopped* and then
    /// accounted for a generation would read as a daemon that ran after it
    /// had stopped (A1). Bounded, because a thread that will not close is
    /// not a reason to never record the stop.
    fn let_the_carried_close(&self) {
        let began = std::time::Instant::now();
        while began.elapsed() < CLOSING {
            let carrying = self
                .running
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .is_empty();
            if carrying {
                return;
            }
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
    }

    /// Says goodbye in the record, before the threads are joined.
    fn stopped(&self, stopped: &Stopped) {
        {
            {
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
            }
        }
    }

    /// Reads one request and answers it, or hands it to a thread of its
    /// own; says whether it was the last.
    fn answer_one<'scope>(
        &'scope self,
        scope: &'scope std::thread::Scope<'scope, '_>,
        connection: &UnixStream,
    ) -> Option<Stopped> {
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
        // did not receive, and it has to be *told so* (A2, B-055, F42).
        if line.len() >= REQUEST_CEILING && !line.ends_with('\n') {
            refuse_an_unending_request(&mut writer);
            return None;
        }

        let answer = match read {
            Err(_) | Ok(0) => return None,
            Ok(_) => match Request::read(line.trim_end()) {
                Ok(request) if Self::carried(&request).is_some() => {
                    // A request that answers in many lines over minutes or
                    // hours gets a thread, so that the socket keeps answering
                    // (D48). The connection goes with it.
                    let connection = connection.try_clone();
                    match connection {
                        Ok(connection) => {
                            scope.spawn(move || self.carry(request, &connection));
                        }
                        Err(error) => {
                            let failure =
                                unusable("the client's connection", &self.places.socket, &error);
                            let _written =
                                writeln!(writer, "{}", Answer::refused(&failure).to_line());
                        }
                    }
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

    /// What a request is called while it runs, and the model it names, for
    /// the requests that are carried on their own threads; `None` for one
    /// answered in a line.
    fn carried(request: &Request) -> Option<(&'static str, String)> {
        match request {
            Request::Generate { model, .. } => Some(("generation", model.clone())),
            Request::Measure { model, .. } => Some(("measurement", model.clone())),
            Request::CrossCheck { model } => Some(("cross-check", model.clone())),
            Request::PromptReport { model, .. } => Some(("prompt report", model.clone())),
            Request::Acquire { reference, .. } => Some(("acquisition", reference.clone())),
            Request::Provision { component } => Some((
                "provisioning",
                component
                    .clone()
                    .unwrap_or_else(|| "the engine this machine needs".to_owned()),
            )),
            Request::Status
            | Request::Holding
            | Request::Components
            | Request::Offered { .. }
            | Request::Settings { .. }
            | Request::Anatomy { .. }
            | Request::Host { .. }
            | Request::Hosted
            | Request::Unhost
            | Request::Stop { .. } => None,
        }
    }

    /// Carries one long request on the thread it was given: registers it as
    /// running, answers it down the connection, and forgets it.
    fn carry(&self, request: Request, connection: &UnixStream) {
        let Some((what, model)) = Self::carried(&request) else {
            return;
        };
        let number = self
            .arrivals
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let running = std::sync::Arc::new(Running {
            what,
            model,
            since: Timestamp::now(),
            began: SystemClock.now(),
            progress: crate::served::Progress::default(),
        });
        self.running
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .insert(number, std::sync::Arc::clone(&running));
        let waiting = crate::served::Waiting {
            client: Some(connection),
            told: None,
            stopping: Some(&self.stopping),
            progress: Some(&running.progress),
        };
        let mut writer = connection;
        self.carrying(request, waiting, &mut writer);
        self.running
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .remove(&number);
    }

    /// The long requests, each answered in many lines.
    fn carrying(
        &self,
        request: Request,
        waiting: crate::served::Waiting<'_>,
        writer: &mut &UnixStream,
    ) {
        match request {
            Request::Generate {
                model,
                prompt,
                limit,
                whose,
                seed,
                tokens,
                engine,
                pinned,
                turn,
                image,
                started,
            } => {
                // A generation is one request and many lines, so it has
                // its own path: nothing about it fits in one `Answer`. Its
                // stream has room for the engine's progress (B-458).
                let waiting = crate::served::Waiting {
                    told: waiting.client,
                    ..waiting
                };
                self.generate(
                    &model,
                    &prompt,
                    limit,
                    seed,
                    tokens.as_deref(),
                    engine.as_deref(),
                    whose,
                    pinned,
                    turn.as_ref(),
                    image.as_deref().map(Path::new),
                    started,
                    waiting,
                    writer,
                );
            }
            Request::Measure {
                model,
                engine,
                deepest,
                started,
            } => self.measuring(&model, engine.as_deref(), deepest, started, waiting, writer),
            Request::CrossCheck { model } => self.cross_checking(&model, waiting, writer),
            Request::PromptReport {
                model,
                prompt,
                by,
                most,
                extras,
                turn,
                temperature,
                seed,
            } => {
                // Many generations and one report: a request that takes
                // minutes says what it is doing as it goes, for the same
                // reason a measurement does — a client cannot tell a long
                // run from a hung one (B-227).
                self.prompt_report(
                    &model,
                    &crate::prompt::Taken {
                        text: &prompt,
                        by,
                        most,
                        extras,
                    },
                    seed,
                    temperature,
                    turn.as_ref(),
                    waiting,
                    writer,
                );
            }
            Request::Acquire {
                reference,
                file,
                from,
            } => self.acquiring(&reference, &file, from.as_deref(), writer),
            Request::Provision { component } => self.provisioning(component.as_deref(), writer),
            // Answered in a line, never carried; here so the match is total.
            Request::Status
            | Request::Holding
            | Request::Components
            | Request::Offered { .. }
            | Request::Settings { .. }
            | Request::Anatomy { .. }
            | Request::Host { .. }
            | Request::Hosted
            | Request::Unhost
            | Request::Stop { .. } => {}
        }
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
        pinned: bool,
        turn: Option<&crate::turn::Turn>,
        picture: Option<&Path>,
        started: crate::declared::Started,
        waiting: crate::served::Waiting<'_>,
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
            crate::generation::Draw::greedy(seed),
            tokens,
            engine,
            picked,
            system_memory_free(),
            pinned,
            turn,
            picture,
            started,
            waiting,
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
    /// Each token of the prompt, and where the model ranked it.
    ///
    /// Empty where the model cannot be read or no server can be started: a
    /// reading MCF could not take is absent from the report rather than shown
    /// as a row of nothing (A7).
    fn ranked_prompt(
        &self,
        tokenizer: &Result<crate::generation::Tokenizer<'_>>,
        named: &str,
        prompt: &str,
        picked: core::result::Result<(crate::adapters::ProvisionedLlama, u32, u64), String>,
    ) -> RankedPrompt {
        // **Why it is missing, where it is missing.** An empty list and a list
        // MCF could not take look the same on the page, and the first draft of
        // this swallowed every failure into `Vec::new()` — the silent failure
        // A2 forbids, written by the hand that had just spent a day finding
        // them.
        let refused = |why: &str| RankedPrompt {
            rows: Vec::new(),
            refused: Some(why.to_owned()),
            under: None,
        };
        // **The engine this report already resolved, not a fresh answer.**
        // Asking again here asked after the report's own generations had taken
        // the memory, so `resolve` refused a model it had just run — and the
        // ranking was reported as *no engine resolves this model* on a machine
        // that had been running it for two minutes. Which engine serves a
        // report is one question, settled once at the top of it (F144, §3.15).
        let (llama, gpu_layers, context) = match picked {
            Ok(picked) => picked,
            Err(why) => return refused(&format!("no engine resolves this model: {why}")),
        };
        let path = crate::generation::resolved(&self.places.models, named);
        let tokenizer = match tokenizer {
            Ok(tokenizer) => tokenizer,
            Err(failure) => return refused(&failure.to_string()),
        };
        // **Under the addressing the answer was given.** The ablation's
        // generations go through the derived addressing, and the first cut of
        // this ranked the bare prompt: position one was *what follows the
        // word You with nothing before it*, and the two readings were of two
        // different prompts with nothing on the page to say so (§3.4, B-429).
        // What follows the prompt is left open, since that is what is asked.
        let received =
            match crate::generation::received(tokenizer, &self.mcf_home(), &path, prompt, false) {
                Ok(received) => received,
                Err(failure) => return refused(&failure.to_string()),
            };
        let tokens = received.tokens();
        let crate::generation::Received {
            read,
            before,
            under,
        } = received;
        let runtime = self
            .places
            .socket
            .parent()
            .unwrap_or_else(|| Path::new("/tmp"));
        let where_it_lives = crate::generation::Where {
            store: &self.places.models,
            llama: &llama,
            runtime,
            named,
            gpu_layers,
            context,
            // Reading a turn asks nothing of the engine beyond the plain
            // load: a draft head guesses tokens and does not change how
            // they are spelled (B-456).
            started: crate::declared::Started::default(),
        };
        let ranked = match crate::generation::ranks_over(
            &where_it_lives,
            &self.server,
            &tokens,
            before,
            crate::generation::MOST_RANKED,
        ) {
            Ok(ranked) => ranked,
            Err(failure) => return refused(&failure.to_string()),
        };
        // **The first piece of a turn nothing preceded is a row too.** With
        // no addressing on file and a vocabulary that adds no beginning
        // marker, the prompt's own first piece is at position nought, which
        // has nothing to be ranked against. Leaving it out left the reading
        // one piece short of the prompt, and the placement that walks the
        // prompt piece by piece never found where to start: every word was
        // *in no word* (A7, F160). It is served unread, not past the depth.
        let unread = (before..before.max(1)).map(|position| {
            let text = read
                .get(position)
                .map(|held| held.piece.clone())
                .unwrap_or_default();
            Value::map([
                ("text", Value::text(text)),
                ("rank", Value::Null),
                ("read", Value::Bool(false)),
                ("engine_said", Value::Null),
            ])
        });
        let rows = unread
            .chain(ranked.into_iter().enumerate().map(|(at, (rank, said))| {
                let position = at.saturating_add(before.max(1));
                let text = read
                    .get(position)
                    .map(|held| held.piece.clone())
                    .unwrap_or_default();
                Value::map([
                    ("text", Value::text(text)),
                    (
                        "rank",
                        rank.map_or(Value::Null, |held| {
                            Value::Integer(i64::try_from(held).unwrap_or(i64::MAX))
                        }),
                    ),
                    ("read", Value::Bool(true)),
                    ("engine_said", said.map_or(Value::Null, Value::text)),
                ])
            }))
            .collect();
        RankedPrompt {
            rows,
            refused: None,
            under: Some(under),
        }
    }

    /// Where the derived configurations live: beside the model store.
    fn mcf_home(&self) -> std::path::PathBuf {
        self.places
            .models
            .parent()
            .map_or_else(|| self.places.models.clone(), Path::to_path_buf)
    }

    /// Where the model ranks an answer's opening after a prompt it was not
    /// written to (B-429).
    ///
    /// The prompt goes as the generation sent it — addressed, with the turn
    /// closed — and the opening's own identifiers after it; at each of them
    /// the model is asked where it ranks the token that stood there. Nothing
    /// is generated. Where no reading could be taken, the sentence saying
    /// why: the report shows that apart from a rank, and a reading that was
    /// not taken says so in words rather than in a dash (A2, A7).
    fn forced(
        &self,
        tokenizer: &Result<crate::generation::Tokenizer<'_>>,
        named: &str,
        prompt: &str,
        opening: &[usize],
        picked: core::result::Result<(crate::adapters::ProvisionedLlama, u32, u64), String>,
        turn: Option<&crate::turn::Turn>,
    ) -> core::result::Result<crate::prompt::Held, String> {
        let (llama, gpu_layers, context) =
            picked.map_err(|why| format!("no engine resolves this model: {why}"))?;
        let path = crate::generation::resolved(&self.places.models, named);
        let reads = tokenizer.as_ref().map_err(ToString::to_string)?;
        // **Read under the frame the answers were read under** (B-455). A
        // rank taken over a bare prompt while every answer was framed by the
        // model's own template would be two conditions in one report, and
        // the rank would be the one nobody could act on (A6, §3.4).
        let mut tokens = match turn {
            Some(turn) => {
                let frame = reads.framed(turn).map_err(|failure| failure.to_string())?;
                crate::generation::framed_as(reads, prompt, &frame)
                    .map_err(|failure| failure.to_string())?
            }
            None => crate::generation::received(reads, &self.mcf_home(), &path, prompt, true)
                .map_err(|failure| failure.to_string())?
                .tokens(),
        };
        let from = tokens.len();
        tokens.extend_from_slice(opening);
        let runtime = self
            .places
            .socket
            .parent()
            .unwrap_or_else(|| Path::new("/tmp"));
        let where_it_lives = crate::generation::Where {
            store: &self.places.models,
            llama: &llama,
            runtime,
            named,
            gpu_layers,
            context,
            // Reading a turn asks nothing of the engine beyond the plain
            // load: a draft head guesses tokens and does not change how
            // they are spelled (B-456).
            started: crate::declared::Started::default(),
        };
        let ranked = crate::generation::ranks_over(
            &where_it_lives,
            &self.server,
            &tokens,
            from,
            opening.len(),
        )
        .map_err(|failure| failure.to_string())?;
        Ok(crate::prompt::Held {
            first: ranked.first().and_then(|(rank, _)| *rank),
            kept: ranked.iter().filter(|(rank, _)| *rank == Some(1)).count(),
            of: ranked.len(),
        })
    }

    /// How many tokens the prompt's own text makes, by the tokenizer of the
    /// engine that answers it (B-441).
    ///
    /// The prompt's own: a beginning marker or a turn's opening is not the
    /// prompt, and the PARTS table this count sits above sums to the same
    /// figure. Why not, in words, where it could not be read (A7).
    fn tokens_in(
        tokenizer: &Result<crate::generation::Tokenizer<'_>>,
        text: &str,
    ) -> core::result::Result<usize, String> {
        tokenizer
            .as_ref()
            .map_err(ToString::to_string)?
            .encode(text, false)
            .map(|held| held.len())
            .map_err(|failure| failure.to_string())
    }

    /// The tokenizer a prompt report reads with: the engine that answers it
    /// (B-441). The provisioned server where one was resolved, MCF's own
    /// where its own engine answers — and where its own cannot read the
    /// file, that is the report's reason, carried into every reading that
    /// needed it rather than each reading finding out for itself.
    fn tokenizer_for<'a>(
        &'a self,
        named: &'a str,
        picked: Option<&'a (crate::adapters::ProvisionedLlama, u32, u64)>,
    ) -> Result<crate::generation::Tokenizer<'a>> {
        match picked {
            Some((llama, gpu_layers, context)) => Ok(crate::generation::Tokenizer::Engine {
                where_it_lives: crate::generation::Where {
                    store: &self.places.models,
                    llama,
                    runtime: self
                        .places
                        .socket
                        .parent()
                        .unwrap_or_else(|| Path::new("/tmp")),
                    named,
                    started: crate::declared::Started::default(),
                    gpu_layers: *gpu_layers,
                    context: *context,
                },
                server: &self.server,
            }),
            None => crate::generation::Tokenizer::own(&crate::generation::resolved(
                &self.places.models,
                named,
            )),
        }
    }

    /// **How the seeded draws of a prompt report are cut is decided here,
    /// once, and stated on every one of them** (B-440). Left to the engine,
    /// a draw above nought is cut with the file's `general.sampling.*` or
    /// the engine's own house values, and the report never says which
    /// (F157). The file's recommendation is what is adopted where there is
    /// one (B60), and *off* where there is not — never the engine's own. A
    /// file whose header cannot be read recommends nothing knowable, and
    /// that is a refusal rather than a guess (A2).
    fn settle_for(
        &self,
        named: &str,
        temperature: mcf_core::configuration::Thousandths,
    ) -> Result<crate::prompt::Settle> {
        let path = crate::generation::resolved(&self.places.models, named);
        let Some(file) = header_of(&path) else {
            return Err(Failure::new(
                Category::ArtifactMissing,
                Attribution::Machine,
                Disposition::Refused,
                Subsystem::new("mcf-serve::daemon"),
                "the model file's header could not be read, so what it recommends for a \
                 seeded draw is unknown",
            )
            .with_context("path", path.display().to_string()));
        };
        let recommended = mcf_standin::recommended::read(&file);
        Ok(crate::prompt::Settle {
            temperature,
            truncation: crate::prompt::Truncation::recommended(recommended.sampling()),
        })
    }

    /// One generation for a report, with its stream drained rather than
    /// written anywhere: what the report keeps is the account and the words
    /// at the end of it. `None` where no pair of sockets could be had.
    fn generated_quietly(
        &self,
        named: &str,
        prompt: &str,
        draw: crate::prompt::Draw,
        picked: Option<(crate::adapters::ProvisionedLlama, u32, u64)>,
        turn: Option<&crate::turn::Turn>,
        waiting: crate::served::Waiting<'_>,
    ) -> Option<crate::generation::Produced> {
        let (mine, theirs) = UnixStream::pair().ok()?;
        let drain = std::thread::spawn(move || {
            let mut end = &theirs;
            let _emptied = std::io::copy(&mut end, &mut std::io::sink());
        });
        let produced = {
            let mut into = &mine;
            crate::generation::serve_generation(
                &self.places.models,
                &self.mcf_home(),
                &self.resident,
                &self.server,
                self.places
                    .socket
                    .parent()
                    .unwrap_or_else(|| Path::new("/tmp")),
                named,
                prompt,
                Some(PROMPT_REPORT_LIMIT),
                draw,
                None,
                None,
                picked,
                system_memory_free(),
                // What the model says to a prompt, ended where the model
                // ends it: a report on the prompt is not a timing.
                false,
                turn,
                None,
                crate::declared::Started::default(),
                waiting,
                &mut into,
            )
        };
        drop(mine);
        let _joined = drain.join();
        Some(produced)
    }

    /// The settling this model would be read under, where a temperature was
    /// asked for at all.
    fn settle_asked(
        &self,
        named: &str,
        settle: Option<mcf_core::configuration::Thousandths>,
    ) -> core::result::Result<Option<crate::prompt::Settle>, mcf_core::Failure> {
        match settle {
            Some(temperature) => self.settle_for(named, temperature).map(Some),
            None => Ok(None),
        }
    }

    /// What an account says the model spent before its answer began, where
    /// it counted anything (B-451).
    fn thought_in(account: &Value) -> Option<usize> {
        account
            .get("before_the_answer")
            .and_then(|before| before.get("tokens"))
            .and_then(Value::as_integer)
            .and_then(|tokens| usize::try_from(tokens).ok())
    }

    #[allow(
        clippy::too_many_arguments,
        reason = "one report's conditions, each named in what it writes"
    )]
    fn prompt_report(
        &self,
        named: &str,
        taken: &crate::prompt::Taken<'_>,
        seed: u64,
        settle: Option<mcf_core::configuration::Thousandths>,
        turn: Option<&crate::turn::Turn>,
        waiting: crate::served::Waiting<'_>,
        writer: &mut &UnixStream,
    ) {
        let picked = self.picked_engine_or_why(named);
        let settle = match self.settle_asked(named, settle) {
            Ok(settle) => settle,
            Err(failure) => {
                let answer = Answer::refused(&failure);
                let _written = writeln!(writer, "{}", answer.to_line());
                let _flushed = writer.flush();
                return;
            }
        };
        let mut asked = 0_usize;
        // Which engine answered is a condition of every figure below, and
        // the account of each generation names it; the report keeps the
        // names and drains the rest (§3.4).
        let mut engines = std::collections::BTreeSet::new();
        // What the generations were addressed as, from their own accounts:
        // the report said *one user turn* over a prompt that went bare
        // (A21, F160).
        let mut addressed = std::collections::BTreeSet::new();
        // **A generation that was refused refuses the report.** Before this,
        // a model name no engine resolved gave seven empty answers, and the
        // report read them as *every part removed gave the SAME answer* —
        // a finding printed over a failure, and then recorded (A2, F: seen
        // with a name that was a directory rather than a file). The first
        // refusal is kept whole and passed on as the answer.
        let mut refused: Option<Value> = None;
        let mut baseline_account: Option<(i64, String)> = None;
        let mut ask = |prompt: &str, draw: crate::prompt::Draw| {
            asked = asked.saturating_add(1);
            let Some(produced) =
                self.generated_quietly(named, prompt, draw, picked.clone().ok(), turn, waiting)
            else {
                return crate::prompt::Answered::default();
            };
            // The first ask is the prompt as written. How long its answer
            // was and what ended it are the account's, and a page that
            // printed an empty answer under a 600-token cap with neither
            // left a reader to guess between a model that said nothing and
            // a report that lost what it said (A7, F160).
            if asked == 1 {
                baseline_account = Some(Self::length_and_ending(&produced.account));
            }
            if produced.said.is_none()
                && let Some(failure) = produced.account.get("failure")
                && refused.is_none()
            {
                refused = Some(failure.clone());
            }
            if let Some(engine) = Self::condition_of(&produced.account, "engine") {
                let _seen = engines.insert(engine);
            }
            if let Some(under) = Self::condition_of(&produced.account, "addressed_as") {
                let _seen = addressed.insert(under);
            }
            // What the model spent before its answer, from the account
            // that counted it: a persona that makes the model think for
            // three hundred tokens costs that on every turn it is used
            // (B-455, B-451).
            let thought = Self::thought_in(&produced.account);
            produced
                .said
                .map(|held| crate::prompt::Answered {
                    text: held.text,
                    tokens: held.tokens,
                    thought,
                })
                .unwrap_or_default()
        };
        let tokenizer = self.tokenizer_for(named, picked.as_ref().ok());
        let mut not_held: Option<String> = None;
        let mut force = |prompt: &str, opening: &[usize]| match self.forced(
            &tokenizer,
            named,
            prompt,
            opening,
            picked.clone(),
            turn,
        ) {
            Ok(held) => Some(held),
            Err(why) => {
                let _first = not_held.get_or_insert(why);
                None
            }
        };
        let report = crate::prompt::measure(taken, seed, settle, &mut ask, &mut force);
        if let Some(failure) = refused {
            let answer = Answer::refused_as(failure);
            let _written = writeln!(writer, "{}", answer.to_line());
            let _flushed = writer.flush();
            return;
        }
        // What the model was asked, as the baseline was: the parts put back
        // together, which is what the counts below are of.
        let parts = taken.parts();
        let prompt = crate::prompt::joined(&parts);
        let prompt = prompt.as_str();
        // **How the model actually receives the prompt.** The figures above
        // are about answers; this is about the question, it costs no
        // generation, and a reader asking which parts of their prompt carry
        // weight wants to know that `c#` reached the model as two pieces.
        // `mcf segment` shows every fragment; what belongs in a report about
        // one prompt is how many there were (§3.15, B-381).
        let tokens = Self::tokens_in(&tokenizer, prompt);
        // **Where the model ranked each word of the question.** A second
        // reading that does not compare two answers, so the drift that makes
        // the ablation an ordering does not touch it (§3.8).
        let ranked = self.ranked_prompt(&tokenizer, named, prompt, picked.clone());
        let read_by = tokenizer.as_ref().map_or_else(
            |failure| format!("nothing: {failure}"),
            crate::generation::Tokenizer::named,
        );
        let mut served =
            prompt_report_value(&report, &parts, asked, tokens, ranked, read_by, not_held);
        if let Value::Map(fields) = &mut served {
            let _was = fields.insert(
                "addressed_as".to_owned(),
                Value::text(addressed_as(&addressed)),
            );
            let _was = fields.insert(
                "asked_as".to_owned(),
                turn.map_or(Value::Null, |turn| Value::text(turn.said())),
            );
            if let Some((tokens, stopped)) = baseline_account {
                let _was = fields.insert("answer_tokens".to_owned(), Value::Integer(tokens));
                let _was = fields.insert("answer_stopped".to_owned(), Value::text(stopped));
            }
        }
        let served = self.record_prompt_report(served, named, prompt, seed, &engines);
        let answer = Answer::served(served);
        let _written = writeln!(writer, "{}", answer.to_line());
        let _flushed = writer.flush();
    }

    /// One text condition of a generation's account, where it has one.
    fn condition_of(account: &Value, key: &str) -> Option<String> {
        account
            .get("conditions")
            .and_then(|conditions| conditions.get(key))
            .and_then(Value::as_text)
            .map(str::to_owned)
    }

    /// How many tokens a generation produced and what ended it, from its
    /// account; nought and *unknown* where the account has neither (A7).
    fn length_and_ending(account: &Value) -> (i64, String) {
        (
            account
                .get("tokens")
                .and_then(Value::as_integer)
                .unwrap_or(0),
            account
                .get("stopped")
                .and_then(Value::as_text)
                .unwrap_or("unknown")
                .to_owned(),
        )
    }

    /// **The figures go to the record; the text does not** (A25, B-432).
    /// Every other diagnostic leaves an entry, and a report that lived only
    /// in the terminal it was printed in was a measurement nobody could find
    /// again. The entry is built from what was served, by naming each figure
    /// kept, so that what the record holds is what the caller saw and
    /// nothing the caller typed. What is served gains the record's id.
    fn record_prompt_report(
        &self,
        mut served: Value,
        named: &str,
        prompt: &str,
        seed: u64,
        engines: &std::collections::BTreeSet<String>,
    ) -> Value {
        let path = crate::generation::resolved(&self.places.models, named);
        let entry = prompt_report_entry(&served, &path, prompt, seed, engines);
        let recorded = self.note(EntryKind::PromptReported, Timestamp::now(), entry.clone());
        if let Ok(mut reports) = self.prompt_reports.lock() {
            let _replaced = reports.insert(path, entry);
        }
        if let Value::Map(fields) = &mut served {
            let _added = fields.insert(
                "recorded".to_owned(),
                recorded.map_or(Value::Null, |id| Value::text(id.as_str().to_owned())),
            );
        }
        served
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
            Request::Anatomy { model } => (self.anatomy_of(model), None),
            Request::Host { model, settings } => (self.host(model, settings), None),
            Request::Hosted => (Answer::served(self.hosted()), None),
            Request::Unhost => (Answer::served(self.unhost()), None),
            // Handled before `respond` is reached; here so the match is
            // total and a future request type is a compile error rather than a
            // silent fall-through.
            Request::Generate { .. }
            | Request::Acquire { .. }
            | Request::Measure { .. }
            | Request::CrossCheck { .. }
            | Request::Provision { .. }
            | Request::PromptReport { .. } => (
                Answer::refused(&crate::control::refused(
                    "a request that answers in many lines reached the one-answer path",
                    "generate, acquire, measure, cross-check or provision",
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
    #[allow(
        clippy::too_many_arguments,
        reason = "one measurement's conditions, each named in what it writes"
    )]
    fn measuring(
        &self,
        named: &str,
        engine: Option<&str>,
        deepest: u64,
        started: crate::declared::Started,
        waiting: crate::served::Waiting<'_>,
        writer: &mut &UnixStream,
    ) {
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
        // A switch this file cannot honour is refused before the ladder is
        // climbed rather than once a rung of it: an hour spent to say no is
        // an hour (A2, B-463).
        if let Err(failure) = started.against(&crate::declared::Declared::of(&path)) {
            return say(writer, &Answer::refused(&failure));
        }

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
        let planned = planned_memory(&path, held, &ladder);
        let mut readings: Vec<Value> = Vec::new();
        let mut ran_on: Option<String> = None;
        for depth in &ladder {
            let (reading, engine) =
                self.one_depth(named, engine, *depth, picked.as_ref(), started, waiting);
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

        let last = measured(
            named,
            &path,
            held,
            engine,
            ran_on.as_deref(),
            readings,
            &planned,
            started,
        );
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

    /// Reads what the provisioned engine produces from a model with MCF's
    /// own engine, and says whether the two agree (B-362, B-424, §II).
    ///
    /// **The other engine generates freely; MCF reads what it produced.**
    /// Asking both to generate and comparing texts is the thing that does
    /// not work (F27, F40): past the first close call they are writing
    /// different sentences. So the provisioned engine is asked for
    /// [`crate::crosscheck::POSITIONS`] tokens of
    /// [`crate::crosscheck::PROMPT`] through the same generation path a
    /// client's request runs, and [`crate::crosscheck::against`] is then made
    /// to read those tokens position by position.
    ///
    /// **Weighed before anything runs.** MCF's own engine holds the model
    /// dequantized, and a model this machine cannot hold that way is refused
    /// from its directory with both numbers, rather than discovered by the
    /// kernel ending the daemon (B-372, F136).
    ///
    /// **Three lines and a record.** What is about to run and what it is
    /// expected to cost; what the provisioned engine produced, so a window
    /// can say which half is running; and the agreement, in figures and in
    /// the sentences every surface prints. The last line is written down as
    /// it is sent (A1): until this existed the one check that answers §II
    /// was printed to a terminal and kept nowhere.
    fn cross_checking(
        &self,
        named: &str,
        waiting: crate::served::Waiting<'_>,
        writer: &mut &UnixStream,
    ) {
        let say = |writer: &mut &UnixStream, answer: &Answer| {
            let _written = writeln!(writer, "{}", answer.to_line());
            let _flushed = writer.flush();
        };
        let path = crate::generation::resolved(&self.places.models, named);
        let file = match crate::crosscheck::examined(&path, system_memory_free()) {
            Ok(file) => file,
            Err(failure) => return say(writer, &Answer::refused(&failure)),
        };
        let prompt_tokens = match mcf_standin::tokenizer::Vocabulary::read(&file)
            .and_then(|vocabulary| vocabulary.encode(crate::crosscheck::PROMPT, true))
        {
            Ok(tokens) => tokens,
            Err(failure) => return say(writer, &Answer::refused(&failure)),
        };
        let count = |held: usize| Value::Integer(i64::try_from(held).unwrap_or(i64::MAX));
        let held = std::fs::metadata(&path).map(|about| about.len()).ok();
        let guess = cross_check_seconds(held, file.dequantized_bytes(), prompt_tokens.len());
        say(
            writer,
            &Answer::served(Value::map([
                ("cross_checking", Value::text(named.to_owned())),
                ("prompt", Value::text(crate::crosscheck::PROMPT.to_owned())),
                ("prompt_tokens", count(prompt_tokens.len())),
                ("positions", count(crate::crosscheck::POSITIONS)),
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

        let (tokens, engine_ran, took) =
            match self.other_engines_tokens(named, &prompt_tokens, waiting) {
                Ok(produced) => produced,
                Err(why) => return say(writer, &Answer::refused(&could_not_compare(&path, &why))),
            };
        say(
            writer,
            &Answer::served(Value::map([
                ("cross_checking", Value::text(named.to_owned())),
                ("produced", count(tokens.len())),
                ("engine_ran", engine_ran.clone()),
                ("reading", Value::Bool(true)),
                ("done", Value::Bool(false)),
            ])),
        );

        let started = SystemClock.now();
        let read = std::fs::read(&path).map_err(|error| {
            could_not_compare(&path, &format!("the model file could not be read: {error}"))
        });
        let agreement = read.and_then(|bytes| {
            crate::crosscheck::against(&bytes, &prompt_tokens, &tokens, system_memory_free())
        });
        let agreement = match agreement {
            Ok(agreement) => agreement,
            Err(failure) => return say(writer, &Answer::refused(&failure)),
        };
        let reading_took = SystemClock.now().saturating_duration_since(started);
        let conditions = Value::map([
            ("model", Value::text(path.display().to_string())),
            ("engine_ran", engine_ran),
            ("own_engine", Value::text("mcf-standin".to_owned())),
            // The prompt is MCF's constant, not a person's, and the text the
            // engine produced is drained rather than filed — but the entry says
            // whose question it was, as every generation does (§6.8, B-146).
            (
                "asked_by",
                Value::text(mcf_record::content::Whose::Fixture.as_str()),
            ),
            ("prompt", Value::text(crate::crosscheck::PROMPT.to_owned())),
            ("prompt_tokens", count(prompt_tokens.len())),
            ("positions_asked", count(crate::crosscheck::POSITIONS)),
            ("produced", count(tokens.len())),
            ("generating_ms", milliseconds(took)),
            ("reading_ms", milliseconds(reading_took)),
        ]);
        let last = Answer::served(Value::map([
            ("cross_checked", Value::text(named.to_owned())),
            ("conditions", conditions),
            ("agreement", agreement.to_value()),
            (
                "said",
                Value::List(agreement.said().into_iter().map(Value::text).collect()),
            ),
            ("done", Value::Bool(true)),
        ]));
        let _recorded = self.note(EntryKind::CrossChecked, Timestamp::now(), last.body.clone());
        if let Ok(mut checks) = self.cross_checks.lock() {
            let _replaced = checks.insert(path, last.body.clone());
        }
        say(writer, &last);
    }

    /// What the provisioned engine produced from these tokens: the
    /// identifiers, which engine ran, and how long it took — or why there is
    /// nothing to read.
    fn other_engines_tokens(
        &self,
        named: &str,
        prompt_tokens: &[usize],
        waiting: crate::served::Waiting<'_>,
    ) -> std::result::Result<(Vec<usize>, Value, mcf_core::time::Duration<Monotonic>), String> {
        let picked = self.picked_engine(named);
        let (produced, took) = self.drained_generation(
            named,
            Some("provisioned"),
            prompt_tokens,
            crate::crosscheck::POSITIONS,
            // Not pinned: the cross-check compares what two engines say from
            // the same prefix, and where one ends its turn is part of that.
            false,
            picked,
            // The plain load: a cross-check reads what two engines spell,
            // and a draft head changes which tokens are drawn rather than
            // how they are spelled (B-463).
            crate::declared::Started::default(),
            waiting,
        )?;
        let Some(said) = produced.said else {
            return Err(format!(
                "the provisioned engine did not generate: {}",
                failure_said(&produced.account)
            ));
        };
        if said.tokens.is_empty() {
            return Err(
                "this engine does not hand back the identifiers it produced, so there is nothing \
                 to read with MCF's own (B-362, B-376)"
                    .to_owned(),
            );
        }
        let engine_ran = produced
            .account
            .get("conditions")
            .and_then(|conditions| conditions.get("engine"))
            .and_then(Value::as_text)
            .map_or(Value::Null, |engine| Value::text(engine.to_owned()));
        Ok((said.tokens, engine_ran, took))
    }

    /// The engine and layer count this model resolves to.
    ///
    /// `None` where MCF cannot work it out — a header it could not read, no
    /// provisioned engine — and the generation path then falls back to its own
    /// discovery, which is what it did before there was anything to resolve.
    fn picked_engine(&self, named: &str) -> Option<(crate::adapters::ProvisionedLlama, u32, u64)> {
        self.picked_engine_or_why(named).ok()
    }

    /// The engine this model resolves to, or the sentence saying why none
    /// does.
    ///
    /// A report that says *no engine resolves this model* and nothing else
    /// sent a reader to a machine that had just run it; the reason — it did
    /// not fit, the header did not say, nothing is provisioned — was found
    /// and dropped on the way (A2, F160). It is kept here and printed where
    /// a reading was not taken because of it.
    fn picked_engine_or_why(
        &self,
        named: &str,
    ) -> core::result::Result<(crate::adapters::ProvisionedLlama, u32, u64), String> {
        let (recommended, _) = self.recommend(named).map_err(|failure| {
            failure
                .context_value("wanted")
                .map_or_else(|| failure.detail().to_owned(), str::to_owned)
        })?;
        let (engine, _) = self
            .engines_held()
            .into_iter()
            .find(|(engine, _)| engine.name == recommended.engine)
            .ok_or_else(|| {
                format!(
                    "the engine this model resolves to, {}, is not provisioned",
                    recommended.engine
                )
            })?;
        Ok((
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
    #[allow(
        clippy::too_many_arguments,
        reason = "one rung's conditions, each named in what it writes"
    )]
    fn one_depth(
        &self,
        named: &str,
        engine: Option<&str>,
        depth: u64,
        picked: Option<&(crate::adapters::ProvisionedLlama, u32, u64)>,
        started: crate::declared::Started,
        waiting: crate::served::Waiting<'_>,
    ) -> (Value, Option<String>) {
        // Repeats, because one pair is one sample and a fall-off read off
        // single samples is a reading of the noise. The median is taken
        // rather than the mean: a run that hit a scheduler hiccup should not
        // move the answer (F53).
        let mut samples: Vec<u64> = Vec::new();
        let mut first_token: Vec<u64> = Vec::new();
        let mut peak_resident: Option<u64> = None;
        let mut window: Option<u64> = None;
        let mut ran_on: Option<String> = None;
        // Why a run produced nothing, where the engine said: a rung the
        // engine refused is not a rung whose pair did not separate (F152).
        let mut refused: Option<String> = None;
        // And why a run that produced something is not a sample: it did not
        // produce what it was pinned to. The difference is divided by
        // sixteen because sixteen is what separates the two runs, and a
        // pair where that is not so is not divided (B-396).
        let mut fell_short: Option<String> = None;
        for _ in 0..REPEATS {
            let one =
                self.timed_generation(named, engine, depth, 1, picked.cloned(), started, waiting);
            let many = self.timed_generation(
                named,
                engine,
                depth,
                1 + SETTLED,
                picked.cloned(),
                started,
                waiting,
            );
            for (run, pinned) in [(&one, 1), (&many, 1 + SETTLED)] {
                match run {
                    Ok(timed) => {
                        ran_on = ran_on.take().or_else(|| timed.engine.clone());
                        peak_resident = timed.peak_resident.max(peak_resident);
                        window = timed.window.max(window);
                        if !timed.held_the_pin(pinned) {
                            fell_short = fell_short.take().or_else(|| Some(timed.short_of(pinned)));
                        }
                    }
                    Err(why) => refused = refused.take().or_else(|| Some(why.clone())),
                }
            }
            if let (Ok(short), Ok(long)) = (&one, &many)
                && short.held_the_pin(1)
                && long.held_the_pin(1 + SETTLED)
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
                    // As a whole number too, for the slope read between rungs.
                    (
                        "ns_per_token",
                        Value::Integer(i64::try_from(per_token).unwrap_or(i64::MAX)),
                    ),
                    (
                        "first_token_ms",
                        middle(&mut first_token)
                            .map_or(Value::Null, |ns| Value::text(as_milliseconds(ns))),
                    ),
                    // As a whole number too, for what is read between rungs.
                    (
                        "first_token_ns",
                        middle(&mut first_token).map_or(Value::Null, |ns| {
                            Value::Integer(i64::try_from(ns).unwrap_or(i64::MAX))
                        }),
                    ),
                    ("spread_ms", Value::text(as_milliseconds(spread))),
                    // As a whole number too, for the bracket the slope read
                    // between rungs sits in (B-427).
                    (
                        "spread_ns",
                        Value::Integer(i64::try_from(spread).unwrap_or(i64::MAX)),
                    ),
                    (
                        "samples",
                        Value::Integer(i64::try_from(samples.len()).unwrap_or(i64::MAX)),
                    ),
                    // The window the engine ran in and its peak resident
                    // memory across the repeats, for what is read between
                    // rungs (B-424).
                    (
                        "window",
                        window.map_or(Value::Null, |tokens| {
                            Value::Integer(i64::try_from(tokens).unwrap_or(i64::MAX))
                        }),
                    ),
                    (
                        "peak_resident_bytes",
                        peak_resident.map_or(Value::Null, |bytes| {
                            Value::Integer(i64::try_from(bytes).unwrap_or(i64::MAX))
                        }),
                    ),
                    ("measured", Value::Bool(true)),
                ])
            }
            None => not_measured(at_depth, refused, fell_short),
        };
        (reading, ran_on)
    }

    /// One generation, timed — or, where it produced nothing, why, in the
    /// words of the account's failure.
    ///
    /// The prompt is a run of identifiers rather than text: what is being
    /// measured is depth, and depth is a count of tokens. Sending text would
    /// make the reading depend on how the text happened to segment.
    #[allow(
        clippy::too_many_arguments,
        reason = "one timing's conditions, each named in what it writes"
    )]
    fn timed_generation(
        &self,
        named: &str,
        engine: Option<&str>,
        depth: u64,
        produce: u32,
        picked: Option<(crate::adapters::ProvisionedLlama, u32, u64)>,
        started: crate::declared::Started,
        waiting: crate::served::Waiting<'_>,
    ) -> std::result::Result<Timed, String> {
        let how_many =
            usize::try_from(depth).map_err(|_| "a depth this machine cannot count".to_owned())?;
        // Identifier 1 is inside every vocabulary MCF can address. What it
        // means does not matter; that there are `depth` of them does.
        let tokens: Vec<usize> = vec![1; how_many];
        // Pinned: the difference between the two runs is divided by the
        // tokens between them, so both have to have produced exactly what
        // they were asked for. The engine is told to run past its end of
        // text, and the count comes back with the account — a run that fell
        // short is refused below rather than divided by (B-396).
        let (produced, took) = self.drained_generation(
            named,
            engine,
            &tokens,
            usize::try_from(produce).unwrap_or(1),
            true,
            picked,
            started,
            waiting,
        )?;

        let conditions = produced.account.get("conditions");
        let condition = |key: &str| {
            conditions
                .and_then(|conditions| conditions.get(key))
                .and_then(Value::as_integer)
                .and_then(|held| u64::try_from(held).ok())
        };
        if produced.said.is_none() {
            return Err(failure_said(&produced.account));
        }
        Ok(Timed {
            ns: took.as_nanos(),
            engine: conditions
                .and_then(|conditions| conditions.get("engine"))
                .and_then(Value::as_text)
                .map(str::to_owned),
            peak_resident: condition("peak_resident_bytes"),
            window: condition("window"),
            produced: produced
                .account
                .get("tokens")
                .and_then(Value::as_integer)
                .and_then(|held| u64::try_from(held).ok()),
            stopped: produced
                .account
                .get("stopped")
                .and_then(Value::as_text)
                .map(str::to_owned),
        })
    }

    /// One generation nobody is listening to, and how long it took.
    ///
    /// A generation streams its tokens to whoever asked. Nothing is asking
    /// here — what is wanted is the account, or how long it took — so the far
    /// end of a socket pair is handed over and drained. This runs the *same*
    /// generation path a client's request runs, rather than a second one
    /// written to be measured, which is the difference between timing MCF and
    /// timing something that resembles it (A11, A12).
    #[allow(
        clippy::too_many_arguments,
        reason = "one generation's conditions, and who is waiting for it"
    )]
    #[allow(
        clippy::too_many_arguments,
        reason = "one generation's conditions, each named in its account"
    )]
    fn drained_generation(
        &self,
        named: &str,
        engine: Option<&str>,
        tokens: &[usize],
        produce: usize,
        pinned: bool,
        picked: Option<(crate::adapters::ProvisionedLlama, u32, u64)>,
        started: crate::declared::Started,
        waiting: crate::served::Waiting<'_>,
    ) -> std::result::Result<
        (
            crate::generation::Produced,
            mcf_core::time::Duration<Monotonic>,
        ),
        String,
    > {
        let mcf_home = self
            .places
            .models
            .parent()
            .map_or_else(|| self.places.models.clone(), Path::to_path_buf);
        let (mine, theirs) =
            UnixStream::pair().map_err(|error| format!("no socket pair to drain: {error}"))?;
        let drain = std::thread::spawn(move || {
            let mut end = &theirs;
            let _emptied = std::io::copy(&mut end, &mut std::io::sink());
        });

        let clock = SystemClock;
        let began = clock.now();
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
                Some(produce),
                crate::generation::Draw::greedy(0),
                Some(tokens),
                engine,
                picked,
                system_memory_free(),
                pinned,
                None,
                None,
                started,
                waiting,
                &mut writer,
            )
        };
        let took = clock.now().saturating_duration_since(began);
        drop(mine);
        let _joined = drain.join();
        Ok((produced, took))
    }

    /// Builds a component, saying each line the build prints as it prints it.
    ///
    /// **Unnamed, the engine this machine needs** — which is what the window
    /// asks for when a model is held and nothing here can run it. The choice
    /// is said in the first line, because a build the operator did not name
    /// is a choice MCF made (§3.15), and it is recorded by the builder with
    /// everything else about the build.
    ///
    /// **The connection is the progress bar.** A build is minutes of
    /// compiling, and a window that heard nothing for that long could not
    /// tell it from a hang (A2), so each line the container prints goes down
    /// the socket as it arrives, and the log on disk keeps all of them.
    ///
    /// **Then the engines are looked for again**, so that the build just
    /// finished is one the next request can use without a restart. Nothing
    /// else this daemon holds changes: a build is a new directory beside the
    /// ones it knew.
    fn provisioning(&self, component: Option<&str>, writer: &mut &UnixStream) {
        let say = |writer: &mut &UnixStream, answer: &Answer| {
            let _written = writeln!(writer, "{}", answer.to_line());
            let _flushed = writer.flush();
        };
        let (component, chosen) = match component {
            Some(name) => match mcf_core::component::COMPONENTS
                .iter()
                .find(|held| held.name == name)
            {
                Some(component) => (component, false),
                None => {
                    return say(
                        writer,
                        &Answer::refused(&crate::control::refused(
                            "a component MCF knows how to build",
                            name,
                        )),
                    );
                }
            },
            None => match needed_engine() {
                Some(component) => (component, true),
                None => {
                    return say(
                        writer,
                        &Answer::refused(&crate::control::refused(
                            "an engine in MCF's own component table",
                            "none — adding one is a change to MCF, not a setting",
                        )),
                    );
                }
            },
        };
        let mcf_home = self
            .places
            .models
            .parent()
            .map_or_else(|| self.places.models.clone(), Path::to_path_buf);
        let prefix =
            crate::provisioning::prefix_for(component, &crate::provisioning::root_under(&mcf_home));
        let progress = |doing: &str| {
            Value::map([
                ("provisioning", Value::text(component.name)),
                ("commit", Value::text(crate::provisioning::short(component))),
                ("chosen", Value::Bool(chosen)),
                ("doing", Value::text(doing.to_owned())),
                ("done", Value::Bool(false)),
            ])
        };
        say(
            writer,
            &Answer::served(progress(if chosen {
                "chosen: what a model on this machine would run on"
            } else {
                "starting"
            })),
        );
        let mut line_out = |line: &str| say(writer, &Answer::served(progress(line)));
        let built = crate::provisioning::provision(component, &prefix, &mut line_out);
        let answer = match built {
            Ok(crate::provisioning::Outcome::Already { prefix }) => Answer::served(Value::map([
                ("provisioning", Value::text(component.name)),
                ("already", Value::Bool(true)),
                ("prefix", Value::text(prefix.display().to_string())),
                ("done", Value::Bool(true)),
            ])),
            Ok(crate::provisioning::Outcome::Built(built)) => {
                self.rediscover();
                let reached = self
                    .engines_held()
                    .iter()
                    .any(|(engine, _)| engine.name == component.name);
                Answer::served(Value::map([
                    ("provisioning", Value::text(component.name)),
                    ("already", Value::Bool(false)),
                    ("prefix", Value::text(built.prefix.display().to_string())),
                    ("log", Value::text(built.log.display().to_string())),
                    ("toolchain", Value::text(built.toolchain)),
                    (
                        "recorded",
                        match built.recorded {
                            Ok(at) => Value::text(at.display().to_string()),
                            Err(failure) => mcf_record::encode::failure(&failure),
                        },
                    ),
                    // Whether the daemon now reaches it as an engine. A
                    // library is built and never an engine; an engine built
                    // and not reached is F31 and is said here, not hidden.
                    ("usable_engine", Value::Bool(reached)),
                    ("done", Value::Bool(true)),
                ]))
            }
            Err(failure) => Answer::refused(&failure),
        };
        say(writer, &answer);
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
            Ok((recommended, path)) => Answer::served(Value::map([
                ("model", Value::text(named.to_owned())),
                ("recommended", recommended.to_value()),
                ("settings", recommended.to_value()),
                // What the file declares that these settings do not start,
                // so that a person choosing them sees what the plain load
                // leaves in the file before they spend it (B-456).
                ("declares", crate::declared::Declared::of(&path).to_value()),
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
                // **And the rate, so a caller can price a window MCF did not
                // recommend.** `cache_bytes` answers *what does the window MCF
                // chose cost*; somebody deciding between windows is asking
                // about the ones it did not, and a round trip per candidate to
                // multiply by a constant is a poor way to answer it. The
                // window does this arithmetic already (B-423, A22).
                (
                    "cache_bytes_per_token",
                    self.cache_for(named, 1).map_or(Value::Null, |bytes| {
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
    /// What a model is made of, counted from its directory.
    ///
    /// The prefix that holds the header holds the tensor directory too, so
    /// this reads what `mcf explain` reads and loads nothing.
    fn anatomy_of(&self, named: &str) -> Answer {
        let path = crate::generation::resolved(&self.places.models, named);
        if !path.is_file() {
            return Answer::refused(&crate::control::refused(
                "a model this machine is not holding",
                named,
            ));
        }
        match header_of(&path) {
            Some(file) => Answer::served(crate::anatomy::encode(named, &file)),
            None => Answer::refused(&crate::control::refused(
                "a file whose header MCF could not read",
                named,
            )),
        }
    }

    fn cache_for(&self, named: &str, context: u64) -> Option<u64> {
        let path = crate::generation::resolved(&self.places.models, named);
        let file = header_of(&path)?;
        let per_token = crate::engines::cache_bytes_per_token(&file)?;
        Some(per_token.saturating_mul(context))
    }

    fn recommend(&self, named: &str) -> Result<(crate::hosting::Hosting, PathBuf)> {
        let path = crate::generation::resolved(&self.places.models, named);
        // **The whole set, not the part it is named by.** `metadata` here read
        // the length of the one file, and a split model's first part is a
        // fraction of it — 10.9 MB of 111 GB on one held here. Everything below
        // is memory arithmetic, so a model sized at a ten-thousandth of itself
        // was recommended a context the machine could not survive, and the
        // surface that lists models had the right figure all along (F138).
        let bytes = mcf_hub::store::bytes_of_the_whole(&path).map_err(|failure| {
            crate::control::refused("a model this machine is not holding", &failure.to_string())
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
        let choice = crate::engines::resolve(&self.engines_now(), bytes, cache, trained).map_err(
            |refused| {
                let failure = crate::control::refused(&refused.says(), named);
                // Named, not only described: a window that reads the name
                // can build it, where one that reads the sentence could only
                // print it (B-367).
                match (&refused, needed_engine()) {
                    (crate::engines::Refused::NoEngine, Some(component)) => {
                        failure.with_context("needs_component", component.name)
                    }
                    _ => failure,
                }
            },
        )?;
        let on_a_card = matches!(choice.device.kind, crate::engines::Kind::Gpu);
        // Whether the whole thing fits where it is going: the weights plus
        // the cache at the window MCF settled on. This is the figure the
        // layer recommendation turns on, and it is arithmetic rather than a
        // guess (A6).
        let wanted = bytes.saturating_add(cache.unwrap_or(0).saturating_mul(choice.context));
        let fits = choice.device.free.is_none_or(|free| wanted <= free);
        // The projector its publisher shipped beside it, where there is one:
        // the recommendation is the whole model, and a model hosted without
        // the file that lets it see is hosted at half its capability with
        // nothing saying so (§3.15).
        let projector = crate::projector::beside(&path);
        Ok((
            crate::hosting::Hosting::recommended(
                &choice.engine,
                &choice.device.name,
                on_a_card,
                choice.context,
                std::thread::available_parallelism().ok().map(Into::into),
                fits,
                projector.as_deref(),
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
        // What the file says it has, read before anything is started: a
        // switch it cannot honour is refused here rather than by an engine
        // that has already loaded the weights, and what it declares goes
        // into the answer whether it was asked for or not (B-456, A2).
        let declared = crate::declared::Declared::of(&path);
        if let Err(failure) = settings.started.against(&declared) {
            return Answer::refused(&failure);
        }

        // **The engine named in the settings, not whichever one is found.**
        // Looking one up by shape returned the processor build while the
        // settings said the CUDA one — so MCF would have resolved a model to
        // a card, said so, and started the build that cannot use it. That is
        // the same defect as the hardcoded layer count, one level up (F133).
        let Some((engine, _)) = self
            .engines_held()
            .into_iter()
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
                // What the engine will take, asked of the engine now that it
                // answers: which media reach the model through the port and
                // what its template does with tools and the thinking switch.
                // Read, not declared — a model hosted at half its capability
                // must say so, and so must one hosted at all of it (A21).
                let takes = crate::takes::Takes::asked_on(settings.port);
                let takes_value = takes
                    .as_ref()
                    .map_or(Value::Null, crate::takes::Takes::to_value);
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
                        ("takes", takes_value.clone()),
                        ("declares", declared.to_value()),
                    ]),
                );
                *holding = Some(Holding {
                    served,
                    model: path.clone(),
                    settings: settings.clone(),
                    recommended: recommended.clone(),
                    takes,
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
                    ("takes", takes_value),
                    // What the file declares, so that a model hosted without
                    // a feature its own file carries says so (B-456).
                    ("declares", declared.to_value()),
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
                (
                    "takes",
                    held.takes
                        .as_ref()
                        .map_or(Value::Null, crate::takes::Takes::to_value),
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
            // What is being carried right now, so that a daemon that is
            // busy says so rather than not answering (D48, B-460).
            (
                "running",
                Value::List(
                    self.running
                        .lock()
                        .unwrap_or_else(std::sync::PoisonError::into_inner)
                        .values()
                        .map(|running| running.to_value())
                        .collect(),
                ),
            ),
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
                            let short: String = component.commit.chars().take(12).collect();
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
                                        engines.iter().any(|engine| engine.name == component.name),
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
                                    ("parts", Value::Integer(i64::from(held.parts))),
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
