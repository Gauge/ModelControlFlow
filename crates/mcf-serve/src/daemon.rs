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

const CLOSING: std::time::Duration = std::time::Duration::from_secs(10);

pub const PATIENCE: std::time::Duration = std::time::Duration::from_secs(2);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Places {
    pub socket: PathBuf,
    pub journal: PathBuf,
    pub models: PathBuf,
}

const PROMPT_REPORT_LIMIT: usize = 600;

struct RankedPrompt {
    rows: Vec<Value>,
    refused: Option<String>,
    under: Option<String>,
}

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

fn unit_chosen_by(report: &crate::prompt::Report) -> &'static str {
    if report.unit_chosen {
        "chosen"
    } else if report.unit == crate::prompt::Unit::Paragraph {
        "text: blank lines"
    } else {
        "text: no blank line"
    }
}

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
        ("thought", counted(clause.thought)),
    ])
}

type PickedOrWhy = core::result::Result<(crate::adapters::ProvisionedLlama, u32, u64), String>;

struct HeldFor {
    picked: (crate::adapters::ProvisionedLlama, u32, u64),
    started: crate::declared::Started,
    served: std::sync::Arc<crate::served::Served>,
}

#[derive(Default)]
struct ReportTally {
    asked: usize,
    engines: std::collections::BTreeSet<String>,
    addressed: std::collections::BTreeSet<String>,
    refused: Option<Value>,
    baseline_account: Option<(i64, String)>,
}

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

fn prompt_report_rows(served: &Value) -> Vec<mcf_record::readings::Reading> {
    use mcf_record::readings::Reading;
    let count = |held: usize| i64::try_from(held).unwrap_or(i64::MAX);
    let mut rows = Vec::new();
    for (at, clause) in served
        .get("clauses")
        .and_then(Value::as_list)
        .unwrap_or(&[])
        .iter()
        .enumerate()
    {
        let dims = [("clause", Value::Integer(count(at)))];
        if let Some(text) = clause.get("text").and_then(Value::as_text) {
            rows.push(Reading::new(
                &dims,
                "characters",
                count(text.chars().count()),
                "count",
            ));
        }
        if let Some(Value::Bool(changed)) = clause.get("changed") {
            rows.push(Reading::new(&dims, "changed", i64::from(*changed), "bool"));
        }
        if let Some(moved) = clause
            .get("moved_parts_per_million")
            .and_then(Value::as_integer)
        {
            rows.push(Reading::new(&dims, "moved_ppm", moved, "ppm"));
        }
        if let Some(Value::Bool(held)) = clause.get("held") {
            rows.push(Reading::new(&dims, "held", i64::from(*held), "bool"));
        }
    }
    let expected = served
        .get("expected")
        .and_then(Value::as_list)
        .unwrap_or(&[]);
    if !expected.is_empty() {
        let first_choice = expected
            .iter()
            .filter(|row| matches!(row.get("rank"), Some(Value::Integer(1))))
            .count();
        rows.push(Reading::new(
            &[],
            "positions_read",
            count(expected.len()),
            "count",
        ));
        rows.push(Reading::new(
            &[],
            "first_choice",
            count(first_choice),
            "count",
        ));
    }
    if let Some(tokens) = served.get("prompt_tokens").and_then(Value::as_integer) {
        rows.push(Reading::new(&[], "prompt_tokens", tokens, "tokens"));
    }
    rows
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
    let _shutdown = writer.shutdown(std::net::Shutdown::Read);
}

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
        ("baseline_thought", counted(report.baseline_thought)),
        (
            "floor_parts_per_million",
            Value::Integer(i64::try_from(report.floor).unwrap_or(i64::MAX)),
        ),
        ("floor_held", held_value(report.floor_held)),
        ("floor_thought", counted(report.floor_thought)),
        ("held_refused", not_held.map_or(Value::Null, Value::text)),
        ("floors", floors_value(report.floors.as_deref())),
        ("floor_spread", spread_value(report.floor_spread())),
        ("alone", readings_value(report.alone.as_deref())),
        ("alone_floor", reading_value(report.alone_floor.as_ref())),
        ("prefixes", readings_value(report.prefixes.as_deref())),
        ("swaps", readings_value(report.swaps.as_deref())),
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
        ("addressed_as", Value::Null),
        ("asked_as", Value::Null),
        ("settled", settled_value(report.settled.as_ref())),
        (
            "generations",
            Value::Integer(i64::try_from(generations).unwrap_or(i64::MAX)),
        ),
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

fn addressed_as(seen: &std::collections::BTreeSet<String>) -> String {
    if seen.is_empty() {
        return format!("{} · mcf probe sets one", crate::generation::BARE_PROMPT);
    }
    seen.iter().cloned().collect::<Vec<_>>().join(" · ")
}

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

fn needed_engine() -> Option<&'static mcf_core::component::Component> {
    crate::engines::required(crate::engines::backend_present())
}

fn system_memory_free() -> Option<u64> {
    mcf_core::hardware::memory_available_now()
}

fn failure_said(account: &Value) -> String {
    let failure = account.get("failure");
    let said = |key: &str| {
        failure
            .and_then(|failure| failure.get(key))
            .and_then(Value::as_text)
    };
    let words = failure
        .and_then(|failure| failure.get("context"))
        .and_then(|context| {
            context
                .get("engine_said")
                .or_else(|| context.get("reason"))
                .and_then(Value::as_text)
        })
        .map_or_else(String::new, |words| format!(": {words}"));
    match (said("detail"), said("category")) {
        (Some(detail), Some(category)) => format!("{detail}{words} ({category})"),
        (Some(detail), None) => format!("{detail}{words}"),
        (None, Some(category)) => category.to_owned(),
        (None, None) => "the account carries no failure".to_owned(),
    }
}

const ENOUGH_FOUND: usize = 5;

fn shorter_name(name: &str) -> Option<String> {
    let name = name.trim();
    if let Some(stem) = name.strip_suffix(".gguf") {
        return Some(stem.to_owned());
    }
    if let Some((before, of)) = name.rsplit_once("-of-")
        && of.chars().all(|c| c.is_ascii_digit())
        && let Some((prefix, at)) = before.rsplit_once('-')
        && at.chars().all(|c| c.is_ascii_digit())
        && !prefix.is_empty()
    {
        return Some(prefix.to_owned());
    }
    let (shorter, _) = name.rsplit_once('-')?;
    (!shorter.is_empty()).then(|| shorter.to_owned())
}

#[derive(Debug, Clone, Copy)]
struct Progress {
    part: usize,
    of: usize,
    whole: u64,
    before: u64,
}

impl Progress {
    fn said(self, path: &str, arrived: u64, total: u64, doing: &str) -> Value {
        let mut fields = vec![
            ("acquiring", Value::text(path.to_owned())),
            ("arrived", as_whole(arrived)),
            ("bytes", as_whole(total)),
            ("doing", Value::text(doing.to_owned())),
            ("done", Value::Bool(false)),
        ];
        if self.of > 1 {
            fields.extend([
                ("part", as_whole(self.part)),
                ("of", as_whole(self.of)),
                ("bytes_whole", as_whole(self.whole)),
                (
                    "arrived_whole",
                    as_whole(self.before.saturating_add(arrived.min(total))),
                ),
            ]);
        }
        Value::map(fields)
    }
}

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
    placed: Option<&(String, u32)>,
) -> Answer {
    let prompt_reading = crate::ladder::prompt_reading(&readings, as_milliseconds);
    let first_token = crate::ladder::first_token(&readings, as_milliseconds);
    let memory = crate::ladder::memory(&readings, planned);
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
                (
                    "device",
                    placed.map_or(Value::Null, |(device, _)| Value::text(device.clone())),
                ),
                (
                    "gpu_layers",
                    placed.map_or(Value::Null, |(_, layers)| {
                        Value::Integer(i64::from(*layers))
                    }),
                ),
                ("started_with", started.to_value()),
                ("repeats", Value::Integer(i64::from(REPEATS))),
                (
                    "tokens_per_reading",
                    Value::Integer(i64::from(SETTLED_AS_F32)),
                ),
                (
                    "method",
                    Value::text(
                        "three runs a depth: a warm run that pays the prefill, then one token and \
                         seventeen on the cached prefix; the difference over sixteen is generation",
                    ),
                ),
                ("loaded", Value::text("per_request")),
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

fn a_load_so_far(
    path: &Path,
    (resident, on_card): (u64, Option<u64>),
    of: Option<u64>,
    seconds: u64,
) -> Value {
    Value::map([
        ("hosting", Value::text(path.display().to_string())),
        (
            "loading",
            Value::map([
                ("resident_bytes", as_whole(resident)),
                ("card_bytes", on_card.map_or(Value::Null, as_whole)),
                ("of_bytes", of.map_or(Value::Null, as_whole)),
                ("seconds", as_whole(seconds)),
            ]),
        ),
        ("done", Value::Bool(false)),
    ])
}

fn kind_noun(what: &str) -> &str {
    match what {
        "examining" => "measurement",
        "probing" => "probe",
        "measurement" => "ladder",
        "hosting" => "hold",
        other => other,
    }
}

fn counters(metrics: &str) -> Vec<(&'static str, Value)> {
    let mut fields: Vec<(&'static str, Value)> = Vec::new();
    for line in metrics.lines() {
        let line = line.trim();
        if line.starts_with('#') {
            continue;
        }
        let Some((name, value)) = line.split_once(' ') else {
            continue;
        };
        let key = match name.trim_start_matches("llamacpp:") {
            "prompt_tokens_total" => "prompted_tokens",
            "tokens_predicted_total" => "generated_tokens",
            "prompt_tokens_seconds" => "engine_said_prompt_tokens_per_second",
            "predicted_tokens_seconds" => "engine_said_tokens_per_second",
            "kv_cache_usage_ratio" => "cache_used_ratio",
            "kv_cache_tokens" => "cache_tokens",
            "requests_processing" => "requests_processing",
            "requests_deferred" => "requests_queued",
            "n_decode_total" => "decodes",
            _ => continue,
        };
        fields.push((key, Value::text(value.trim().to_owned())));
    }
    fields
}

struct Counted {
    at: Instant<Monotonic>,
    generated: u64,
    prompted: u64,
}

fn where_it_answers(reach: &crate::served::Reach) -> String {
    match reach {
        crate::served::Reach::Socket(socket) => socket.display().to_string(),
        crate::served::Reach::Port { port, .. } => format!("port {port}"),
    }
}

static COUNTED: std::sync::Mutex<std::collections::BTreeMap<String, Counted>> =
    std::sync::Mutex::new(std::collections::BTreeMap::new());

const RATE_OVER_AT_MOST_NS: u64 = 30 * 1_000_000_000;

fn energy_of(reach: &crate::served::Reach) -> Option<(u64, u64)> {
    let _forgotten = COUNTED
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .remove(&where_it_answers(reach));
    let spent = energy_since_start(reach)?;
    (spent.covered_ns > 0).then_some((spent.microjoules, spent.covered_ns))
}

fn energy_since_start(reach: &crate::served::Reach) -> Option<crate::power::Spent> {
    let started = crate::served::live()
        .into_iter()
        .find(|live| live.reach == *reach)?
        .spent_at_start;
    Some(crate::power::spent().since(started))
}

fn use_figures(reach: &crate::served::Reach) -> Vec<(&'static str, Value)> {
    let mut fields: Vec<(&'static str, Value)> = Vec::new();
    if let Some(metrics) = crate::served::metrics_via(reach) {
        fields.extend(counters(&metrics));
    }
    let sensors = crate::engines::card_sensors();
    if let Some(power) = sensors.power_uw {
        fields.push((
            "card_power_watts",
            thousandths_of(u64::try_from(power.max(0)).unwrap_or(0), 1_000),
        ));
        fields.push(("power_named", Value::text(sensors.power_named())));
        fields.push(("power_is", Value::text(sensors.power_is())));
    }
    let whole = |key: &str| -> Option<u64> {
        fields
            .iter()
            .find(|(name, _)| *name == key)
            .and_then(|(_, value)| match value.as_text()?.trim().parse::<f64>() {
                #[expect(
                    clippy::cast_possible_truncation,
                    clippy::cast_sign_loss,
                    reason = "a count the engine wrote as a float"
                )]
                Ok(held) if held >= 0.0 => Some(held as u64),
                _ => None,
            })
    };
    let in_flight = crate::served::tokens_in_flight(reach);
    let generated =
        whole("generated_tokens").map(|finished| finished.saturating_add(in_flight.unwrap_or(0)));
    let prompted = whole("prompted_tokens");
    if let Some(generated) = generated {
        fields.push(("generated_tokens_live", as_whole(generated)));
    }
    let now = SystemClock.now();
    let mut counted = COUNTED
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let key = where_it_answers(reach);
    let before = counted.get(&key);
    let over = before.map_or(0, |before| {
        now.saturating_duration_since(before.at).as_nanos()
    });
    let usable = over > 0 && over <= RATE_OVER_AT_MOST_NS;
    let spent = energy_since_start(reach).unwrap_or_default();
    let before = counted.insert(
        key,
        Counted {
            at: now,
            generated: generated.unwrap_or(0),
            prompted: prompted.unwrap_or(0),
        },
    );
    counted
        .retain(|_, held| now.saturating_duration_since(held.at).as_nanos() < RATE_OVER_AT_MOST_NS);
    drop(counted);
    if spent.covered_ns > 0 {
        fields.push((
            "card_energy_joules",
            thousandths_of(spent.microjoules, 1_000),
        ));
        fields.push((
            "card_energy_over_seconds",
            thousandths_of(spent.covered_ns, 1_000_000),
        ));
    }
    let (Some(generated), Some(prompted)) = (generated, prompted) else {
        return fields;
    };
    let Some(before) = before else {
        return fields;
    };
    if !usable {
        return fields;
    }
    let rate = |now: u64, then: u64| {
        let tokens = now.saturating_sub(then);
        thousandths_of(tokens.saturating_mul(1_000_000_000_000), over)
    };
    fields.push((
        "generated_tokens_per_second",
        rate(generated, before.generated),
    ));
    fields.push(("prompt_tokens_per_second", rate(prompted, before.prompted)));
    fields.push(("rate_over_seconds", thousandths_of(over, 1_000_000)));
    fields
}

fn thousandths_of(held: u64, over: u64) -> Value {
    #[expect(
        clippy::integer_division,
        reason = "whole thousandths; what is discarded is under a thousandth"
    )]
    let thousandths = held / over.max(1);
    #[expect(
        clippy::integer_division,
        reason = "the whole number, and the thousandths after it"
    )]
    let (whole, part) = (thousandths / 1_000, thousandths % 1_000);
    Value::text(format!("{whole}.{part:03}"))
}

fn in_use(held: &Holding) -> Value {
    let mut fields: Vec<(&'static str, Value)> = use_figures(held.served.reach());
    fields.push((
        "resident_bytes",
        held.served.resident_bytes().map_or(Value::Null, as_whole),
    ));
    fields.push((
        "card_bytes",
        held.card_before
            .and_then(|before| {
                crate::engines::card_memory_used().map(|now| now.saturating_sub(before))
            })
            .map_or(Value::Null, as_whole),
    ));
    let up = Timestamp::now()
        .utc_nanos()
        .saturating_sub(held.since.utc_nanos())
        .max(0);
    #[expect(
        clippy::integer_division,
        reason = "nanoseconds to whole seconds; the remainder is under a second"
    )]
    let seconds = up / 1_000_000_000;
    fields.push((
        "uptime_seconds",
        Value::Integer(i64::try_from(seconds).unwrap_or(i64::MAX)),
    ));
    Value::map(fields)
}

struct Freed {
    model: String,
    resident: Option<u64>,
    card: Option<u64>,
}

fn asker_left(why: Option<&str>) -> bool {
    why.is_some_and(|why| why.contains(crate::served::CLIENT_LEFT))
}

fn estimate_line(named: &str, ladder: &[u64], guess: (u64, u64)) -> Answer {
    Answer::served(Value::map([
        ("measuring", Value::text(named.to_owned())),
        (
            "depths",
            Value::List(ladder.iter().map(|depth| as_whole(*depth)).collect()),
        ),
        ("estimate_low_seconds", as_whole(guess.0)),
        ("estimate_high_seconds", as_whole(guess.1)),
        ("done", Value::Bool(false)),
    ]))
}

fn as_whole<N: TryInto<i64>>(held: N) -> Value {
    Value::Integer(held.try_into().unwrap_or(i64::MAX))
}

fn announced(named: &str, key: &'static str, what: Value) -> Answer {
    Answer::served(Value::map([
        ("measuring", Value::text(named.to_owned())),
        (key, what),
        ("done", Value::Bool(false)),
    ]))
}

fn a_rung_starting(depth: u64, step: usize, of: usize) -> Value {
    let count = |held: usize| Value::Integer(i64::try_from(held).unwrap_or(i64::MAX));
    Value::map([
        (
            "depth",
            Value::Integer(i64::try_from(depth).unwrap_or(i64::MAX)),
        ),
        ("step", count(step.saturating_add(1))),
        ("of", count(of)),
    ])
}

fn a_run_starting(depth: u64, produce: u32, repeat: u32) -> Value {
    Value::map([
        (
            "depth",
            Value::Integer(i64::try_from(depth).unwrap_or(i64::MAX)),
        ),
        ("produce", Value::Integer(i64::from(produce))),
        (
            "repeat",
            Value::Integer(i64::from(repeat.saturating_add(1))),
        ),
        ("of_repeats", Value::Integer(i64::from(REPEATS))),
    ])
}

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
        bandwidth: crate::bandwidth::along_a_ladder(ladder, per_token),
    }
}

fn pair_rows(
    depth: u64,
    repeat: u32,
    which: &str,
    timed: &Timed,
) -> Vec<mcf_record::readings::Reading> {
    let dims = [
        (
            "depth",
            Value::Integer(i64::try_from(depth).unwrap_or(i64::MAX)),
        ),
        ("repeat", Value::Integer(i64::from(repeat))),
        ("run", Value::text(which)),
    ];
    let whole = |held: u64| i64::try_from(held).unwrap_or(i64::MAX);
    let mut rows = vec![
        mcf_record::readings::Reading::new(&dims, "ns", whole(timed.ns), "ns"),
        mcf_record::readings::Reading::new(
            &dims,
            "produced",
            whole(timed.produced.unwrap_or(0)),
            "tokens",
        ),
    ];
    if let Some(peak) = timed.peak_resident {
        rows.push(mcf_record::readings::Reading::new(
            &dims,
            "peak_resident_bytes",
            whole(peak),
            "bytes",
        ));
    }
    if let Some(Value::Map(timings)) = &timed.engine_timings {
        for (key, unit) in [
            ("prompt_n", "tokens"),
            ("predicted_n", "tokens"),
            ("tokens_cached", "tokens"),
            ("prompt_ms", "ms"),
            ("predicted_ms", "ms"),
        ] {
            let Some(value) = timings.get(key) else {
                continue;
            };
            let figure = match value {
                Value::Integer(held) => Some(*held),
                other => micros_of_milliseconds(&other.to_line()),
            };
            let Some(figure) = figure else {
                continue;
            };
            let (metric, unit) = if unit == "ms" {
                (format!("engine_{key}").replace("_ms", "_us"), "us")
            } else {
                (format!("engine_{key}"), unit)
            };
            if figure != i64::MIN {
                rows.push(mcf_record::readings::Reading::new(
                    &dims, &metric, figure, unit,
                ));
            }
        }
    }
    rows
}

fn micros_of_milliseconds(text: &str) -> Option<i64> {
    let (sign, digits) = match text.strip_prefix('-') {
        Some(rest) => (-1, rest),
        None => (1, text),
    };
    let (whole, fraction) = digits.split_once('.').unwrap_or((digits, ""));
    let whole: i64 = if whole.is_empty() {
        0
    } else {
        whole.parse().ok()?
    };
    let mut micros = 0_i64;
    for (digit, place) in fraction.chars().zip([100_i64, 10, 1]) {
        let digit = i64::from(digit.to_digit(10)?);
        micros = micros.checked_add(digit.checked_mul(place)?)?;
    }
    if fraction.chars().any(|held| !held.is_ascii_digit()) {
        return None;
    }
    whole
        .checked_mul(1000)?
        .checked_add(micros)?
        .checked_mul(sign)
}

fn position_rows(agreement: &crate::crosscheck::Agreement) -> Vec<mcf_record::readings::Reading> {
    agreement
        .ranks
        .iter()
        .enumerate()
        .flat_map(|(position, held)| {
            let dims = [(
                "position",
                Value::Integer(i64::try_from(position).unwrap_or(i64::MAX)),
            )];
            let rank = held.map_or(0, |rank| i64::try_from(rank).unwrap_or(i64::MAX));
            [
                mcf_record::readings::Reading::new(&dims, "rank", rank, "count"),
                mcf_record::readings::Reading::new(
                    &dims,
                    "set_aside",
                    i64::from(held.is_none()),
                    "bool",
                ),
            ]
        })
        .collect()
}

#[derive(Debug)]
struct Timed {
    ns: u64,
    engine: Option<String>,
    peak_resident: Option<u64>,
    window: Option<u64>,
    produced: Option<u64>,
    stopped: Option<String>,
    engine_timings: Option<Value>,
}

impl Timed {
    fn held_the_pin(&self, pinned: u32) -> bool {
        self.produced == Some(u64::from(pinned))
    }

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Pairs {
    of: u32,
    separated: usize,
    not_apart: usize,
    pin_missed: usize,
    refused: usize,
}

impl Pairs {
    const fn of(of: u32) -> Self {
        Self {
            of,
            separated: 0,
            not_apart: 0,
            pin_missed: 0,
            refused: 0,
        }
    }

    fn to_value(self) -> Value {
        Value::map([
            ("of", Value::Integer(i64::from(self.of))),
            ("separated", as_whole(self.separated)),
            ("did_not_separate", as_whole(self.not_apart)),
            ("missed_the_pin", as_whole(self.pin_missed)),
            ("refused", as_whole(self.refused)),
        ])
    }
}

fn rung_reading(
    depth: u64,
    samples: &mut [u64],
    first_token: &mut [u64],
    (window, peak_resident): (Option<u64>, Option<u64>),
    pairs: &Pairs,
    (refused, fell_short): (Option<String>, Option<String>),
) -> Value {
    let at_depth = Value::Integer(i64::try_from(depth).unwrap_or(i64::MAX));
    let Some(per_token) = middle(samples) else {
        let mut reading = not_measured(at_depth, refused, fell_short);
        if let Value::Map(fields) = &mut reading {
            fields.insert("pairs".to_owned(), pairs.to_value());
        }
        return reading;
    };
    let spread = (samples.len() >= 2).then(|| {
        samples
            .last()
            .copied()
            .unwrap_or(per_token)
            .saturating_sub(samples.first().copied().unwrap_or(per_token))
    });
    Value::map([
        ("depth", at_depth),
        ("pair", Value::text(PAIR_METHOD)),
        ("ms_per_token", Value::text(as_milliseconds(per_token))),
        ("ns_per_token", as_whole(per_token)),
        (
            "first_token_ms",
            middle(first_token).map_or(Value::Null, |ns| Value::text(as_milliseconds(ns))),
        ),
        (
            "first_token_ns",
            middle(first_token).map_or(Value::Null, as_whole),
        ),
        (
            "spread_ms",
            spread.map_or(Value::Null, |spread| Value::text(as_milliseconds(spread))),
        ),
        ("spread_ns", spread.map_or(Value::Null, as_whole)),
        ("samples", as_whole(samples.len())),
        ("pairs", pairs.to_value()),
        ("window", window.map_or(Value::Null, as_whole)),
        (
            "peak_resident_bytes",
            peak_resident.map_or(Value::Null, as_whole),
        ),
        ("measured", Value::Bool(true)),
    ])
}

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

fn is_the_stand_in(engine: &str) -> bool {
    engine == mcf_core::build_identity::stand_in_engine() || engine.contains("stand-in")
}

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

const REPEATS: u32 = 3;

const PAIR_METHOD: &str =
    "a warm run pays the prefill; one token and seventeen follow on the cached prefix";

const SETTLED: u32 = 16;
const SETTLED_AS_F32: u16 = 16;

const SHALLOWEST: u64 = 512;

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

#[expect(
    clippy::integer_division,
    reason = "an estimate in whole seconds, and the remainder of a second is \
              far inside the range the answer is given as"
)]
fn estimated_seconds(ladder: &[u64], bytes: Option<u64>) -> (u64, u64) {
    let per_load_ms = bytes.map_or(2_000, |bytes| bytes / 500_000);
    let prefill_ms: u64 = ladder.iter().map(|depth| depth / 4).sum();
    let runs = u64::from(REPEATS).saturating_mul(3);
    let middle_ms = (ladder.len() as u64)
        .saturating_mul(per_load_ms)
        .saturating_add(prefill_ms)
        .saturating_mul(runs);
    let low = (middle_ms.saturating_mul(58) / 100_000).max(1);
    let high = (middle_ms.saturating_mul(142) / 100_000).max(2);
    (low, high)
}

#[expect(clippy::integer_division, reason = "a bound on a duration")]
fn cross_check_seconds(bytes: Option<u64>, dequantized: Option<u64>, prompt: usize) -> (u64, u64) {
    let per_load_ms = bytes.map_or(2_000, |bytes| bytes / 1_000_000);
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

#[expect(
    clippy::integer_division,
    reason = "whole milliseconds; the rest is noise"
)]
fn milliseconds(took: mcf_core::time::Duration<Monotonic>) -> Value {
    Value::Integer(i64::try_from(took.as_nanos() / 1_000_000).unwrap_or(i64::MAX))
}

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

#[derive(Debug)]
struct Holding {
    served: std::sync::Arc<crate::served::Served>,
    model: PathBuf,
    settings: crate::hosting::Hosting,
    recommended: crate::hosting::Hosting,
    takes: Option<crate::takes::Takes>,
    since: Timestamp,
    card_before: Option<u64>,
}

pub(crate) fn header_describes_this_file(model: &mcf_standin::gguf::Model, published: u64) -> bool {
    let Some(extent) = model.data_bytes_required() else {
        return false;
    };
    if published == 0 || extent > published {
        return false;
    }
    extent.saturating_mul(2) >= published
}

fn plan_however_the_shape_can_be_found(
    hub: &mcf_hub::client::Hub,
    listing: &mcf_hub::source::Listing,
) -> (core::result::Result<mcf_hub::offer::Plan, String>, bool) {
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
            if prefix >= entry.size {
                break;
            }
        }
    }
    None
}

fn last_hold_in(journal: &Path) -> Option<Value> {
    if !journal.exists() {
        return None;
    }
    let index = mcf_record::journal::Index::over(
        journal,
        &mcf_record::journal::index::default_path(journal),
    )
    .ok()?;
    let mut last: Option<Value> = None;
    for located in index.entries() {
        match located.kind() {
            EntryKind::ModelHosted => {
                let Ok(entry) = index.read(located) else {
                    continue;
                };
                let text = |key: &str| {
                    entry
                        .body()
                        .get("settings")
                        .and_then(|settings| settings.get(key))
                        .and_then(Value::as_text)
                        .unwrap_or("?")
                        .to_owned()
                };
                let Some(model) = entry.body().get("model").and_then(Value::as_text) else {
                    continue;
                };
                last = Some(Value::map([
                    ("model", Value::text(model.to_owned())),
                    ("engine", Value::text(text("engine"))),
                    ("device", Value::text(text("device"))),
                    ("since", mcf_record::encode::timestamp(entry.recorded_at())),
                    ("until", Value::Null),
                ]));
            }
            EntryKind::ModelUnhosted => {
                if let (Some(Value::Map(fields)), Ok(entry)) = (last.as_mut(), index.read(located))
                {
                    fields.insert(
                        "until".to_owned(),
                        mcf_record::encode::timestamp(entry.recorded_at()),
                    );
                }
            }
            _ => {}
        }
    }
    last
}

fn with_ago(mut last: Value) -> Value {
    let ended = last
        .get("until")
        .filter(|held| !matches!(held, Value::Null))
        .or_else(|| last.get("since"))
        .and_then(|at| mcf_record::decode::timestamp(at).ok());
    if let (Some(ended), Value::Map(fields)) = (ended, &mut last) {
        #[expect(
            clippy::integer_division,
            reason = "nanoseconds to whole seconds; the remainder is under a second"
        )]
        let ago = Timestamp::now()
            .utc_nanos()
            .saturating_sub(ended.utc_nanos())
            .max(0)
            / 1_000_000_000;
        fields.insert(
            "ago_seconds".to_owned(),
            Value::Integer(i64::try_from(ago).unwrap_or(i64::MAX)),
        );
    }
    last
}

fn newest_of(journal: &Path, kind: EntryKind) -> std::collections::BTreeMap<PathBuf, Value> {
    let mut newest = std::collections::BTreeMap::new();
    if !journal.exists() {
        return newest;
    }
    let Ok(index) = mcf_record::journal::Index::over(
        journal,
        &mcf_record::journal::index::default_path(journal),
    ) else {
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
        let _replaced = newest.insert(
            PathBuf::from(model),
            dated(entry.body().clone(), entry.recorded_at()),
        );
    }
    newest
}

const REFUSAL_REPEAT: std::time::Duration = std::time::Duration::from_secs(60);

fn asked_in(line: &str) -> String {
    mcf_record::json::parse(line.trim_end())
        .ok()
        .and_then(|value| value.get("ask").and_then(Value::as_text).map(str::to_owned))
        .unwrap_or_else(|| line.trim_end().chars().take(60).collect())
}

fn dated(mut body: Value, at: Timestamp) -> Value {
    if let Value::Map(fields) = &mut body {
        let _added = fields.insert("at".to_owned(), Value::text(at.to_string()));
    }
    body
}

fn newest_hosted(journal: &Path) -> std::collections::BTreeMap<PathBuf, Value> {
    let mut newest = std::collections::BTreeMap::new();
    if !journal.exists() {
        return newest;
    }
    let Ok(index) = mcf_record::journal::Index::over(
        journal,
        &mcf_record::journal::index::default_path(journal),
    ) else {
        return newest;
    };
    for located in index.entries() {
        if located.kind() != EntryKind::ModelHosted {
            continue;
        }
        let Ok(entry) = index.read(located) else {
            continue;
        };
        let (Some(model), Some(settings)) = (
            entry.body().get("model").and_then(Value::as_text),
            entry.body().get("settings"),
        ) else {
            continue;
        };
        let _replaced = newest.insert(
            PathBuf::from(model),
            Value::map([
                ("settings", settings.clone()),
                ("since", mcf_record::encode::timestamp(entry.recorded_at())),
            ]),
        );
    }
    newest
}

fn all_readings(
    journal: &Path,
    only: Option<&Path>,
) -> std::collections::BTreeMap<PathBuf, Vec<Value>> {
    let mut found: std::collections::BTreeMap<PathBuf, Vec<Value>> =
        std::collections::BTreeMap::new();
    if !journal.exists() {
        return found;
    }
    let Ok(index) = mcf_record::journal::Index::over(
        journal,
        &mcf_record::journal::index::default_path(journal),
    ) else {
        return found;
    };
    for located in index.entries() {
        if located.kind() != EntryKind::Readings {
            continue;
        }
        let Ok(entry) = index.read(located) else {
            continue;
        };
        let Some(model) = entry.body().get("model").and_then(Value::as_text) else {
            continue;
        };
        if only.is_some_and(|only| only != Path::new(model)) {
            continue;
        }
        found
            .entry(PathBuf::from(model))
            .or_default()
            .push(dated(entry.body().clone(), entry.recorded_at()));
    }
    for runs in found.values_mut() {
        let parts = std::mem::take(runs);
        *runs = mcf_record::readings::merged(parts);
        runs.reverse();
    }
    found
}

fn newest_probes(
    journal: &Path,
    only: Option<&Path>,
) -> std::collections::BTreeMap<PathBuf, Vec<Value>> {
    let mut newest: std::collections::BTreeMap<PathBuf, std::collections::BTreeMap<String, Value>> =
        std::collections::BTreeMap::new();
    if !journal.exists() {
        return std::collections::BTreeMap::new();
    }
    let Ok(index) = mcf_record::journal::Index::over(
        journal,
        &mcf_record::journal::index::default_path(journal),
    ) else {
        return std::collections::BTreeMap::new();
    };
    for located in index.entries() {
        if located.kind() != EntryKind::ModelProbed {
            continue;
        }
        let Ok(entry) = index.read(located) else {
            continue;
        };
        let body = entry.body();
        let (Some(model), Some(method)) = (
            body.get("model").and_then(Value::as_text),
            body.get("method").and_then(Value::as_text),
        ) else {
            continue;
        };
        if only.is_some_and(|only| only != Path::new(model)) {
            continue;
        }
        let said = Value::map([
            ("method", Value::text(method.to_owned())),
            ("engine", body.get("engine").cloned().unwrap_or(Value::Null)),
            ("at", Value::text(entry.recorded_at().to_string())),
            ("said", Value::text(crate::probes::run::recorded_said(body))),
        ]);
        let _replaced = newest
            .entry(PathBuf::from(model))
            .or_default()
            .insert(method.to_owned(), said);
    }
    newest
        .into_iter()
        .map(|(model, by_method)| {
            let listed = crate::probes::run::RECORDED
                .iter()
                .chain(crate::examine::MEASURES.iter())
                .filter_map(|method| by_method.get(*method).cloned())
                .collect();
            (model, listed)
        })
        .collect()
}

fn asker_of(writer: &UnixStream) -> Option<UnixStream> {
    let listener = writer.try_clone().ok()?;
    let _deadline = listener.set_read_timeout(Some(std::time::Duration::from_millis(1)));
    Some(listener)
}

fn asker_has_gone(listener: Option<&UnixStream>) -> bool {
    use std::io::Read as _;
    let mut byte = [0_u8; 1];
    match listener.map(|listener| (&*listener).read(&mut byte)) {
        Some(Ok(0)) => true,
        Some(Err(error)) => !matches!(
            error.kind(),
            std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
        ),
        Some(Ok(_)) | None => false,
    }
}

const HUB_KEPT_FOR_NANOS: i128 = 86_400 * 1_000_000_000;

fn kept_answer(home: &Path, key: &str, fresh: bool, ask: impl FnOnce() -> Answer) -> Answer {
    let at = home.join("hub-cache").join(format!(
        "{}.json",
        mcf_core::digest::sha256(
            format!(
                "{key}\u{1f}{}",
                mcf_core::build_identity::BuildIdentity::current()
            )
            .as_bytes()
        )
        .hex()
    ));
    if !fresh
        && let Ok(text) = std::fs::read_to_string(&at)
        && let Ok(kept) = mcf_record::json::parse(&text)
        && let Some(read_at) = kept.get("read_at")
        && let Ok(then) = mcf_record::decode::timestamp(read_at)
        && Timestamp::now()
            .utc_nanos()
            .saturating_sub(then.utc_nanos())
            < HUB_KEPT_FOR_NANOS
        && let Some(Value::Map(body)) = kept.get("body").cloned()
    {
        let mut body = body;
        body.insert("read_at".to_owned(), read_at.clone());
        body.insert("kept".to_owned(), Value::Bool(true));
        body.insert("done".to_owned(), Value::Bool(true));
        return Answer::served(Value::Map(body));
    }
    let answer = ask();
    if !answer.served {
        return answer;
    }
    let now = mcf_record::encode::timestamp(Timestamp::now());
    let kept = Value::map([("read_at", now.clone()), ("body", answer.body.clone())]);
    if let Some(parent) = at.parent() {
        let _made = std::fs::create_dir_all(parent);
    }
    let _kept = std::fs::write(&at, kept.to_line());
    let Value::Map(mut body) = answer.body else {
        return Answer::served(answer.body);
    };
    body.insert("read_at".to_owned(), now);
    body.insert("kept".to_owned(), Value::Bool(false));
    body.insert("done".to_owned(), Value::Bool(true));
    Answer::served(Value::Map(body))
}

const DEFAULT_HUB: &str = "https://huggingface.co/";

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

#[derive(Debug)]
pub struct Daemon {
    timings: std::sync::Mutex<std::collections::BTreeMap<PathBuf, Value>>,
    last_hold: std::sync::Mutex<Option<Value>>,
    cross_checks: std::sync::Mutex<std::collections::BTreeMap<PathBuf, Value>>,
    prompt_reports: std::sync::Mutex<std::collections::BTreeMap<PathBuf, Value>>,
    last_settings: std::sync::Mutex<std::collections::BTreeMap<PathBuf, Value>>,
    probed: std::sync::Mutex<std::collections::BTreeMap<PathBuf, Vec<Value>>>,
    readings: std::sync::Mutex<std::collections::BTreeMap<PathBuf, Vec<Value>>>,
    holding: std::sync::Mutex<Option<Holding>>,
    places: Places,
    engines: std::sync::Mutex<Vec<(crate::engines::Engine, Vec<crate::engines::Device>)>>,
    listener: UnixListener,
    started: Timestamp,
    since: Instant<Monotonic>,
    recovered: Recovered,
    resident: std::sync::Mutex<Option<crate::generation::Resident>>,
    server: std::sync::Mutex<Option<crate::served::Served>>,
    stopping: std::sync::atomic::AtomicBool,
    running: std::sync::Mutex<std::collections::BTreeMap<u64, std::sync::Arc<Running>>>,
    arrivals: std::sync::atomic::AtomicU64,
    refusals: std::sync::Mutex<std::collections::BTreeMap<String, std::time::Instant>>,
}

#[derive(Debug)]
struct Running {
    what: &'static str,
    model: String,
    since: Timestamp,
    began: Instant<Monotonic>,
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Recovered {
    pub entries: usize,
    pub unreadable: Option<String>,
    pub held: usize,
    pub engines_stopped: Vec<crate::orphans::Orphan>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Stopped {
    Asked { reason: String },
    Broken { failure: Box<Failure> },
}

impl Daemon {
    pub fn start(places: Places) -> Result<Self> {
        if let Some(parent) = places.socket.parent() {
            std::fs::create_dir_all(parent).map_err(|error| {
                unusable("the directory the control socket lives in", parent, &error)
            })?;
        }

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

        let mut recovered = recover(&places)?;
        recovered.engines_stopped = crate::orphans::stop_all(
            places.models.parent().unwrap_or(&places.models),
            places.socket.parent().unwrap_or_else(|| Path::new("/tmp")),
        );
        let listener = UnixListener::bind(&places.socket)
            .map_err(|error| unusable("the control socket", &places.socket, &error))?;

        let started = Timestamp::now();
        let engines = discover_engines(&places);
        let daemon = Self {
            timings: std::sync::Mutex::new(newest_of(&places.journal, EntryKind::ModelTimed)),
            last_hold: std::sync::Mutex::new(last_hold_in(&places.journal)),
            cross_checks: std::sync::Mutex::new(newest_of(
                &places.journal,
                EntryKind::CrossChecked,
            )),
            prompt_reports: std::sync::Mutex::new(newest_of(
                &places.journal,
                EntryKind::PromptReported,
            )),
            probed: std::sync::Mutex::new(newest_probes(&places.journal, None)),
            readings: std::sync::Mutex::new(all_readings(&places.journal, None)),
            last_settings: std::sync::Mutex::new(newest_hosted(&places.journal)),
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
            refusals: std::sync::Mutex::new(std::collections::BTreeMap::new()),
        };
        daemon.note(
            EntryKind::DaemonStarted,
            started,
            daemon.recovered_as_value(),
        );
        Ok(daemon)
    }

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

    fn engines_held(&self) -> Vec<(crate::engines::Engine, Vec<crate::engines::Device>)> {
        self.engines
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
    }

    fn rediscover(&self) {
        let found = discover_engines(&self.places);
        *self
            .engines
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = found;
    }

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
                            ("device_free_bytes", shape(choice.device.free)),
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
            (
                "probed",
                Value::List(
                    self.probed
                        .lock()
                        .ok()
                        .and_then(|held| held.get(path).cloned())
                        .unwrap_or_default(),
                ),
            ),
            ("readings_at", self.readings_at(path)),
            ("trained_context", shape(trained)),
            ("cache_bytes_per_token", shape(cache)),
            ("resolved", resolved),
            ("configured", self.applied_to(path)),
        ])
    }

    fn readings_at(&self, path: &std::path::Path) -> Value {
        let mut latest: std::collections::BTreeMap<String, String> =
            std::collections::BTreeMap::new();
        if let Ok(readings) = self.readings.lock()
            && let Some(runs) = readings.get(path)
        {
            for run in runs {
                let (Some(method), Some(at)) = (
                    run.get("method").and_then(Value::as_text),
                    run.get("at").and_then(Value::as_text),
                ) else {
                    continue;
                };
                let entry = latest.entry(method.to_owned()).or_default();
                if at > entry.as_str() {
                    at.clone_into(entry);
                }
            }
        }
        Value::Map(
            latest
                .into_iter()
                .map(|(method, at)| (method, Value::text(at)))
                .collect(),
        )
    }

    fn applied_to(&self, path: &std::path::Path) -> Value {
        let Some(home) = self.places.models.parent() else {
            return Value::Null;
        };
        let derived = crate::configured::read_derived(home, path);
        if derived.is_empty() {
            return Value::Null;
        }
        Value::map([
            (
                "addressing",
                derived
                    .addressing
                    .as_ref()
                    .map_or(Value::Null, |held| Value::text(held.provenance())),
            ),
            (
                "budget",
                derived
                    .budget
                    .as_ref()
                    .map_or(Value::Null, |held| Value::text(held.provenance())),
            ),
        ])
    }

    fn cannot(&self) -> Value {
        if self.engines_held().is_empty() {
            return Value::List(vec![Value::text(
                "run a model: no engine is installed yet — MCF builds one when a model is held",
            )]);
        }
        Value::List(Vec::new())
    }

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
            (
                "engines_stopped",
                Value::List(
                    self.recovered
                        .engines_stopped
                        .iter()
                        .map(crate::orphans::Orphan::as_value)
                        .collect(),
                ),
            ),
        ])
    }

    #[must_use]
    pub const fn recovered(&self) -> &Recovered {
        &self.recovered
    }

    #[must_use]
    pub fn socket(&self) -> &Path {
        &self.places.socket
    }

    pub fn serve(&mut self) -> Stopped {
        let _watch = crate::signals::Watch::over(&self.places.socket);
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

    fn stopped(&self, stopped: &Stopped) {
        {
            {
                self.let_go("the daemon stopped");
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

    fn answer_one<'scope>(
        &'scope self,
        scope: &'scope std::thread::Scope<'scope, '_>,
        connection: &UnixStream,
    ) -> Option<Stopped> {
        let _deadline = connection.set_read_timeout(Some(PATIENCE));
        let _writing = connection.set_write_timeout(Some(PATIENCE));

        let mut line = String::new();
        let ceiling = u64::try_from(REQUEST_CEILING.saturating_add(1)).unwrap_or(u64::MAX);
        let read = BufReader::new(std::io::Read::take(connection, ceiling)).read_line(&mut line);
        let mut writer = connection;

        if line.len() >= REQUEST_CEILING && !line.ends_with('\n') {
            refuse_an_unending_request(&mut writer);
            return None;
        }

        let answer = match read {
            Err(_) | Ok(0) => return None,
            Ok(_) => match Request::read(line.trim_end()) {
                Ok(request) if Self::carried(&request).is_some() => {
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
                    self.tell(&asked_in(&line), writer, &answer);
                    return stop;
                }
                Err(failure) => Answer::refused(&failure),
            },
        };
        self.tell("a request MCF could not read", writer, &answer);
        None
    }

    fn tell(&self, asked: &str, writer: &UnixStream, answer: &Answer) {
        let mut writer = writer;
        let _written = writeln!(writer, "{}", answer.to_line());
        let _flushed = writer.flush();
        if !answer.served {
            self.record_refusal(asked, &answer.body);
        }
    }

    fn record_refusal(&self, asked: &str, body: &Value) {
        let said = |key: &str| body.get(key).and_then(Value::as_text).unwrap_or_default();
        let key = format!("{}\u{1f}{}\u{1f}{asked}", said("category"), said("detail"));
        {
            let mut lately = self
                .refusals
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            let now = std::time::Instant::now();
            if lately
                .get(&key)
                .is_some_and(|last| now.duration_since(*last) < REFUSAL_REPEAT)
            {
                return;
            }
            lately.retain(|_, last| now.duration_since(*last) < REFUSAL_REPEAT);
            let _was = lately.insert(key, now);
        }
        let mut recorded = body.clone();
        if let Value::Map(fields) = &mut recorded {
            let _added = fields.insert("asked".to_owned(), Value::text(asked));
        }
        let _noted = self.note(EntryKind::Failure, Timestamp::now(), recorded);
    }

    fn carried(request: &Request) -> Option<(&'static str, String)> {
        match request {
            Request::Generate { model, .. } => Some(("generation", model.clone())),
            Request::Measure { model, .. } => Some(("measurement", model.clone())),
            Request::CrossCheck { model } => Some(("cross-check", model.clone())),
            Request::Host { model, .. } => Some(("hosting", model.clone())),
            Request::PromptReport { model, .. } => Some(("prompt report", model.clone())),
            Request::Probe { model, .. } => Some(("probing", model.clone())),
            Request::Examine { model, .. } => Some(("examining", model.clone())),
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
            | Request::Failures { .. }
            | Request::Offered { .. }
            | Request::Search { .. }
            | Request::Settings { .. }
            | Request::Readings { .. }
            | Request::Anatomy { .. }
            | Request::Tokenize { .. }
            | Request::Hosted
            | Request::Unhost
            | Request::Stop { .. } => None,
        }
    }

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

    #[allow(
        clippy::too_many_lines,
        reason = "one arm per request the daemon carries, each a call; the match is total by design"
    )]
    fn carrying(
        &self,
        request: Request,
        waiting: crate::served::Waiting<'_>,
        writer: &mut &UnixStream,
    ) {
        if let Some((what, named)) = Self::carried(&request)
            && let Some(refusal) = self.one_model_rule(what, &named)
        {
            return self.write_refusal(
                Self::carried(&request).map_or("carry", |(what, _)| what),
                writer,
                &Answer::refused(&refusal),
            );
        }
        let restore = match &request {
            Request::Measure { model, .. } | Request::Examine { model, .. } => {
                self.release_for_a_run(model)
            }
            _ => None,
        };
        self.carrying_on(request, waiting, writer);
        if let Some((named, settings)) = restore {
            let _again = self.host(&named, &settings.to_request(), &mut |_| {});
        }
    }

    fn one_model_rule(&self, what: &str, named: &str) -> Option<Failure> {
        let noun = kind_noun;
        let path = crate::generation::resolved(&self.places.models, named);
        let refuse = |detail: String| {
            Failure::new(
                Category::ConfigInvalid,
                Attribution::User,
                Disposition::Refused,
                WHERE,
                &detail,
            )
        };
        let hosted = self
            .holding
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .as_ref()
            .map(|held| held.model.clone());
        let under_way: Vec<(String, String)> = self
            .running
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .values()
            .filter(|running| running.what != what || running.model != named)
            .map(|running| {
                (
                    running.what.to_owned(),
                    crate::generation::resolved(&self.places.models, &running.model)
                        .display()
                        .to_string(),
                )
            })
            .collect();
        let short = |path: &str| {
            Path::new(path).file_stem().map_or_else(
                || path.to_owned(),
                |stem| stem.to_string_lossy().into_owned(),
            )
        };
        if what == "hosting" {
            if let Some((kind, other)) = under_way
                .iter()
                .find(|(kind, other)| kind != "hosting" && Path::new(other) != path)
            {
                return Some(refuse(format!(
                    "a {} of {} is under way, and one model runs at a time: stop it, then host {}",
                    noun(kind),
                    short(other),
                    short(&path.display().to_string())
                )));
            }
            return None;
        }
        if let Some(hosted) = hosted
            && hosted != path
        {
            return Some(refuse(format!(
                "{} is hosted, and one model runs at a time: a {} runs on the hosted model. Let it \
                 go, or host {} first",
                short(&hosted.display().to_string()),
                noun(what),
                short(&path.display().to_string())
            )));
        }
        if let Some((kind, other)) = under_way.iter().find(|(_, other)| Path::new(other) != path) {
            return Some(refuse(format!(
                "a {} of {} is under way, and one model runs at a time: stop it, then run the {} \
                 on {}",
                noun(kind),
                short(other),
                noun(what),
                short(&path.display().to_string())
            )));
        }
        None
    }

    fn release_for_a_run(&self, named: &str) -> Option<(String, crate::hosting::Hosting)> {
        let path = crate::generation::resolved(&self.places.models, named);
        let settings = {
            let holding = self
                .holding
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            let held = holding.as_ref()?;
            if held.model != path {
                return None;
            }
            held.settings.clone()
        };
        let _released =
            self.let_go("let go for a measurement of the same model; hosted again after");
        Some((named.to_owned(), settings))
    }

    #[allow(
        clippy::too_many_lines,
        reason = "one arm a kind of request, each named"
    )]
    fn carrying_on(
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
                pieces,
                engine,
                pinned,
                turn,
                image,
                started,
            } => {
                let waiting = crate::served::Waiting {
                    told: waiting.client,
                    ..waiting
                };
                self.generate(
                    &model,
                    &prompt,
                    limit,
                    seed,
                    crate::generation::Given::from_request(tokens.as_deref(), pieces.as_deref()),
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
                on,
                deepest,
                started,
            } => self.measuring(
                &model,
                engine.as_deref(),
                on,
                deepest,
                started,
                waiting,
                writer,
            ),
            Request::CrossCheck { model } => self.cross_checking(&model, waiting, writer),
            Request::Host { model, settings } => self.hosting(&model, &settings, writer),
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
            Request::Probe {
                model,
                engine,
                apply,
                up_to,
                only,
            } => self.probing(&model, engine.as_deref(), (apply, up_to), &only, writer),
            Request::Examine {
                model,
                engine,
                only,
            } => self.examining(&model, engine.as_deref(), &only, waiting, writer),
            Request::Acquire {
                reference,
                file,
                from,
            } => self.acquiring(&reference, &file, from.as_deref(), writer),
            Request::Provision { component } => self.provisioning(component.as_deref(), writer),
            Request::Status
            | Request::Holding
            | Request::Components
            | Request::Failures { .. }
            | Request::Offered { .. }
            | Request::Search { .. }
            | Request::Settings { .. }
            | Request::Readings { .. }
            | Request::Anatomy { .. }
            | Request::Tokenize { .. }
            | Request::Hosted
            | Request::Unhost
            | Request::Stop { .. } => {}
        }
    }

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
        given: crate::generation::Given<'_>,
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
        let hosted = self.hosted_for(named, started, engine);
        let (picked, started) = hosted.as_ref().map_or_else(
            || (self.picked_engine(named), started),
            |hosted| (Some(hosted.picked.clone()), hosted.started),
        );
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
            given,
            engine,
            picked,
            system_memory_free(),
            pinned,
            false,
            turn,
            picture,
            started,
            hosted.as_ref().map(|hosted| &*hosted.served),
            waiting,
            writer,
        );
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

    fn ranked_prompt(
        &self,
        tokenizer: &Result<crate::generation::Tokenizer<'_>>,
        named: &str,
        prompt: &str,
        picked: core::result::Result<(crate::adapters::ProvisionedLlama, u32, u64), String>,
        held: Option<&crate::served::Served>,
    ) -> RankedPrompt {
        let refused = |why: &str| RankedPrompt {
            rows: Vec::new(),
            refused: Some(why.to_owned()),
            under: None,
        };
        let (llama, gpu_layers, context) = match picked {
            Ok(picked) => picked,
            Err(why) => return refused(&format!("no engine resolves this model: {why}")),
        };
        let path = crate::generation::resolved(&self.places.models, named);
        let tokenizer = match tokenizer {
            Ok(tokenizer) => tokenizer,
            Err(failure) => return refused(&failure.to_string()),
        };
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
            started: crate::declared::Started::default(),
            held,
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

    fn mcf_home(&self) -> std::path::PathBuf {
        self.places
            .models
            .parent()
            .map_or_else(|| self.places.models.clone(), Path::to_path_buf)
    }

    #[allow(
        clippy::too_many_arguments,
        reason = "one reading's conditions, and the server it goes to"
    )]
    fn forced(
        &self,
        tokenizer: &Result<crate::generation::Tokenizer<'_>>,
        named: &str,
        prompt: &str,
        opening: &[usize],
        picked: core::result::Result<(crate::adapters::ProvisionedLlama, u32, u64), String>,
        turn: Option<&crate::turn::Turn>,
        held: Option<&crate::served::Served>,
    ) -> core::result::Result<crate::prompt::Held, String> {
        let (llama, gpu_layers, context) =
            picked.map_err(|why| format!("no engine resolves this model: {why}"))?;
        let path = crate::generation::resolved(&self.places.models, named);
        let reads = tokenizer.as_ref().map_err(ToString::to_string)?;
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
            started: crate::declared::Started::default(),
            held,
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

    fn tokenizer_for<'a>(
        &'a self,
        named: &'a str,
        picked: Option<&'a (crate::adapters::ProvisionedLlama, u32, u64)>,
        held: Option<&'a crate::served::Served>,
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
                    held,
                },
                server: &self.server,
            }),
            None => crate::generation::Tokenizer::own(&crate::generation::resolved(
                &self.places.models,
                named,
            )),
        }
    }

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
        let hosted = self.hosted_for(named, crate::declared::Started::default(), None);
        let (picked, started) = hosted.as_ref().map_or_else(
            || (picked, crate::declared::Started::default()),
            |hosted| (Some(hosted.picked.clone()), hosted.started),
        );
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
                crate::generation::Given::Text,
                None,
                picked,
                system_memory_free(),
                false,
                false,
                turn,
                None,
                started,
                hosted.as_ref().map(|hosted| &*hosted.served),
                waiting,
                &mut into,
            )
        };
        drop(mine);
        let _joined = drain.join();
        Some(produced)
    }

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
        let (hosted, picked) = self.report_engine(named);
        let held = hosted.as_ref().map(|hosted| &*hosted.served);
        let settle = match self.settle_asked(named, settle) {
            Ok(settle) => settle,
            Err(failure) => {
                return self.write_refusal("prompt-report", writer, &Answer::refused(&failure));
            }
        };
        let mut tally = ReportTally::default();
        let left = std::cell::Cell::new(false);
        let mut say = |step: crate::prompt::Step| {
            let line = Answer::served(Value::map([
                ("reporting", Value::text(named.to_owned())),
                ("step", step.to_value()),
                ("done", Value::Bool(false)),
            ]));
            if writeln!(writer, "{}", line.to_line())
                .and_then(|()| writer.flush())
                .is_err()
            {
                left.set(true);
            }
        };
        let mut ask = |prompt: &str, draw: crate::prompt::Draw| {
            if left.get() {
                let _first = tally
                    .refused
                    .get_or_insert_with(|| Value::text(crate::served::CLIENT_LEFT.to_owned()));
                return crate::prompt::Answered::default();
            }
            self.report_generation(
                named,
                prompt,
                draw,
                picked.clone().ok(),
                turn,
                waiting,
                &mut tally,
            )
        };
        let tokenizer = self.tokenizer_for(named, picked.as_ref().ok(), held);
        let mut not_held: Option<String> = None;
        let mut force = |prompt: &str, opening: &[usize]| match self.forced(
            &tokenizer,
            named,
            prompt,
            opening,
            picked.clone(),
            turn,
            held,
        ) {
            Ok(held) => Some(held),
            Err(why) => {
                let _first = not_held.get_or_insert(why);
                None
            }
        };
        let report = crate::prompt::measure(taken, seed, settle, &mut ask, &mut force, &mut say);
        let ReportTally {
            asked,
            engines,
            addressed,
            refused,
            baseline_account,
        } = tally;
        if let Some(failure) = refused {
            return self.write_refusal("prompt-report", writer, &Answer::refused_as(failure));
        }
        let parts = taken.parts();
        let prompt = crate::prompt::joined(&parts);
        let prompt = prompt.as_str();
        let tokens = Self::tokens_in(&tokenizer, prompt);
        let ranked = self.ranked_prompt(&tokenizer, named, prompt, picked.clone(), held);
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
        let mut served = self.record_prompt_report(served, named, prompt, seed, &engines);
        if let Value::Map(fields) = &mut served {
            let _was = fields.insert("done".to_owned(), Value::Bool(true));
        }
        let answer = Answer::served(served);
        let _written = writeln!(writer, "{}", answer.to_line());
        let _flushed = writer.flush();
    }

    fn probing(
        &self,
        named: &str,
        engine: Option<&str>,
        (apply, up_to): (bool, Option<usize>),
        only: &[String],
        writer: &mut &UnixStream,
    ) {
        let asked = crate::probes::run::Asked {
            engine,
            apply,
            up_to,
            only,
        };
        let path = crate::generation::resolved(&self.places.models, named);
        if !path.is_file() {
            return self.write_refusal(
                "probe",
                writer,
                &Answer::refused(&crate::control::refused(
                    "there is no model at this name; `mcf list` says what this machine holds",
                    named,
                )),
            );
        }
        let resolved = self.picked_engine(named).map(|(llama, _, _)| {
            format!(
                "provisioned {} @{}",
                llama.component,
                llama.commit.get(..12).unwrap_or(&llama.commit)
            )
        });
        let listener = asker_of(writer);
        let gone = || asker_has_gone(listener.as_ref());
        let at = crate::probes::run::Places {
            socket: &self.places.socket,
            models: &self.places.models,
            resolved: resolved.as_deref(),
            gone: Some(&gone),
        };
        let model = path.display().to_string();
        let mut say = |step: &crate::probes::run::Step, lines: &[String]| -> bool {
            let line = Answer::served(Value::map([
                ("probing", Value::text(model.clone())),
                ("step", step.to_value()),
                (
                    "lines",
                    Value::List(lines.iter().cloned().map(Value::text).collect()),
                ),
                ("done", Value::Bool(false)),
            ]));
            writeln!(writer, "{}", line.to_line())
                .and_then(|()| writer.flush())
                .is_ok()
        };
        let ran = crate::probes::run::run(&at, &path, &asked, &mut say);
        self.refresh_readings(&path);
        let fresh = newest_probes(&self.places.journal, Some(&path));
        if let Ok(mut probed) = self.probed.lock() {
            match fresh.get(&path) {
                Some(found) => {
                    let _replaced = probed.insert(path.clone(), found.clone());
                }
                None => {
                    let _gone = probed.remove(&path);
                }
            }
        }
        let answer = match ran {
            Ok(closing) => Answer::served(Value::map([
                ("probing", Value::text(model)),
                (
                    "lines",
                    Value::List(closing.into_iter().map(Value::text).collect()),
                ),
                ("done", Value::Bool(true)),
            ])),
            Err(why) => Answer::refused(&crate::control::refused(&why, named)),
        };
        self.write_refusal("probe", writer, &answer);
    }

    fn readings_of(&self, named: &str, method: Option<&str>) -> Answer {
        let path = crate::generation::resolved(&self.places.models, named);
        self.refresh_readings(&path);
        let runs: Vec<Value> = self
            .readings
            .lock()
            .ok()
            .and_then(|held| held.get(&path).cloned())
            .unwrap_or_default()
            .into_iter()
            .filter(|run| {
                method
                    .is_none_or(|wanted| run.get("method").and_then(Value::as_text) == Some(wanted))
            })
            .collect();
        Answer::served(Value::map([
            ("model", Value::text(path.display().to_string())),
            ("runs", Value::List(runs)),
            ("done", Value::Bool(true)),
        ]))
    }

    fn record_rows(
        &self,
        path: &Path,
        method: &str,
        engine: &str,
        conditions: Vec<(&str, Value)>,
        rows: &[mcf_record::readings::Reading],
        at: Timestamp,
    ) {
        if rows.is_empty() {
            return;
        }
        let _rows = self.note(
            EntryKind::Readings,
            at,
            mcf_record::readings::run_body(
                &path.display().to_string(),
                method,
                engine,
                conditions,
                rows,
            ),
        );
        self.refresh_readings(path);
    }

    fn refresh_readings(&self, path: &Path) {
        let fresh = all_readings(&self.places.journal, Some(path));
        if let Ok(mut readings) = self.readings.lock() {
            match fresh.get(path) {
                Some(found) => {
                    let _replaced = readings.insert(path.to_path_buf(), found.clone());
                }
                None => {
                    let _gone = readings.remove(path);
                }
            }
        }
    }

    #[allow(
        clippy::too_many_lines,
        reason = "one run read straight through: the file resolved, the site built, each step announced and recorded"
    )]
    fn examining(
        &self,
        named: &str,
        engine: Option<&str>,
        only: &[String],
        waiting: crate::served::Waiting<'_>,
        writer: &mut &UnixStream,
    ) {
        let path = crate::generation::resolved(&self.places.models, named);
        if !path.is_file() {
            return self.write_refusal(
                "examine",
                writer,
                &Answer::refused(&crate::control::refused(
                    "there is no model at this name; `mcf list` says what this machine holds",
                    named,
                )),
            );
        }
        if engine.is_some_and(|engine| engine != "provisioned") {
            return self.write_refusal(
                "examine",
                writer,
                &Answer::refused(&crate::control::refused(
                    "a measurement starts the provisioned engine under the settings it varies; \
                     MCF's own engine takes none of them",
                    engine.unwrap_or_default(),
                )),
            );
        }
        let (llama, gpu_layers, context) = match self.picked_engine_or_why(named) {
            Ok(picked) => picked,
            Err(why) => {
                return self.write_refusal(
                    "examine",
                    writer,
                    &Answer::refused(&crate::control::refused(&why, named)),
                );
            }
        };
        let has_card = self.engines_held().iter().any(|(held, devices)| {
            held.name == llama.component
                && devices
                    .iter()
                    .any(|device| device.kind == crate::engines::Kind::Gpu)
        });
        if let Ok(mut slot) = self.server.lock() {
            *slot = None;
        }
        let listener = asker_of(writer);
        let gone = || asker_has_gone(listener.as_ref());
        let runtime = self
            .places
            .socket
            .parent()
            .map_or_else(std::env::temp_dir, Path::to_path_buf);
        let site = crate::examine::Site {
            llama: &llama,
            model: &path,
            runtime: &runtime,
            gpu_layers,
            context,
            projector: crate::projector::beside(&path),
            has_card,
            engine: format!(
                "provisioned {} @{}",
                llama.component,
                llama.commit.get(..12).unwrap_or(&llama.commit)
            ),
            gone: &gone,
            tell: &|_progress| {},
            waiting,
        };
        let model = path.display().to_string();
        let current: std::sync::Mutex<Value> = std::sync::Mutex::new(Value::Null);
        let stream: &UnixStream = writer;
        let tell = |progress: &crate::examine::Progress| {
            let step = current.lock().map_or(Value::Null, |held| held.clone());
            let line = Answer::served(Value::map([
                ("examining", Value::text(model.clone())),
                ("step", step),
                ("progress", progress.to_value()),
                ("done", Value::Bool(false)),
            ]));
            let mut out = stream;
            let _sent = writeln!(out, "{}", line.to_line()).and_then(|()| out.flush());
        };
        let site = crate::examine::Site {
            tell: &tell,
            ..site
        };
        let mut say = |step: &crate::probes::run::Step, lines: &[String]| -> bool {
            if let Ok(mut held) = current.lock() {
                *held = step.to_value();
            }
            let line = Answer::served(Value::map([
                ("examining", Value::text(model.clone())),
                ("step", step.to_value()),
                (
                    "lines",
                    Value::List(lines.iter().cloned().map(Value::text).collect()),
                ),
                ("done", Value::Bool(false)),
            ]));
            let mut out = stream;
            writeln!(out, "{}", line.to_line())
                .and_then(|()| out.flush())
                .is_ok()
        };
        let ran = crate::examine::run(&site, only, &mut say);
        self.refresh_readings(&path);
        let fresh = newest_probes(&self.places.journal, Some(&path));
        if let Ok(mut probed) = self.probed.lock() {
            match fresh.get(&path) {
                Some(found) => {
                    let _replaced = probed.insert(path.clone(), found.clone());
                }
                None => {
                    let _gone = probed.remove(&path);
                }
            }
        }
        let answer = match ran {
            Ok(closing) => Answer::served(Value::map([
                ("examining", Value::text(model)),
                (
                    "lines",
                    Value::List(closing.into_iter().map(Value::text).collect()),
                ),
                ("done", Value::Bool(true)),
            ])),
            Err(why) => Answer::refused(&crate::control::refused(&why, named)),
        };
        self.write_refusal("examine", writer, &answer);
    }

    fn report_engine(&self, named: &str) -> (Option<HeldFor>, PickedOrWhy) {
        let hosted = self.hosted_for(named, crate::declared::Started::default(), None);
        let picked = hosted.as_ref().map_or_else(
            || self.picked_engine_or_why(named),
            |hosted| Ok(hosted.picked.clone()),
        );
        (hosted, picked)
    }

    fn write_refusal(&self, asked: &str, writer: &mut &UnixStream, answer: &Answer) {
        self.tell(asked, writer, answer);
    }

    #[allow(
        clippy::too_many_arguments,
        reason = "the conditions of a generation and the tally it adds to"
    )]
    fn report_generation(
        &self,
        named: &str,
        prompt: &str,
        draw: crate::prompt::Draw,
        picked: Option<(crate::adapters::ProvisionedLlama, u32, u64)>,
        turn: Option<&crate::turn::Turn>,
        waiting: crate::served::Waiting<'_>,
        tally: &mut ReportTally,
    ) -> crate::prompt::Answered {
        tally.asked = tally.asked.saturating_add(1);
        let Some(produced) = self.generated_quietly(named, prompt, draw, picked, turn, waiting)
        else {
            return crate::prompt::Answered::default();
        };
        if tally.asked == 1 {
            tally.baseline_account = Some(Self::length_and_ending(&produced.account));
        }
        if produced.said.is_none()
            && let Some(failure) = produced.account.get("failure")
            && tally.refused.is_none()
        {
            tally.refused = Some(failure.clone());
        }
        if let Some(engine) = Self::condition_of(&produced.account, "engine") {
            let _seen = tally.engines.insert(engine);
        }
        if let Some(under) = Self::condition_of(&produced.account, "addressed_as") {
            let _seen = tally.addressed.insert(under);
        }
        let thought = Self::thought_in(&produced.account);
        produced
            .said
            .map(|held| crate::prompt::Answered {
                text: held.text,
                tokens: held.tokens,
                thought,
            })
            .unwrap_or_default()
    }

    fn condition_of(account: &Value, key: &str) -> Option<String> {
        account
            .get("conditions")
            .and_then(|conditions| conditions.get(key))
            .and_then(Value::as_text)
            .map(str::to_owned)
    }

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
        let at = Timestamp::now();
        let recorded = self.note(EntryKind::PromptReported, at, entry.clone());
        let engine = engines
            .iter()
            .cloned()
            .collect::<Vec<String>>()
            .join(" and ");
        self.record_rows(
            &path,
            "prompt-report",
            &engine,
            vec![(
                "seed",
                Value::Integer(i64::try_from(seed).unwrap_or(i64::MAX)),
            )],
            &prompt_report_rows(&served),
            at,
        );
        if let Ok(mut reports) = self.prompt_reports.lock() {
            let _replaced = reports.insert(path, dated(entry, at));
        }
        if let Value::Map(fields) = &mut served {
            let _added = fields.insert(
                "recorded".to_owned(),
                recorded.map_or(Value::Null, |id| Value::text(id.as_str().to_owned())),
            );
        }
        served
    }

    fn respond(&self, request: &Request) -> (Answer, Option<Stopped>) {
        match request {
            Request::Status => (Answer::served(self.status()), None),
            Request::Holding => (Answer::served(self.holding()), None),
            Request::Components => (Answer::served(self.components()), None),
            Request::Failures { last } => (Answer::served(self.failures(*last)), None),
            Request::Offered {
                reference,
                from,
                fresh,
            } => (
                kept_answer(
                    &self.mcf_home(),
                    &format!("files:{reference}@{}", from.as_deref().unwrap_or("")),
                    *fresh,
                    || Self::offered(reference, from.as_deref()),
                ),
                None,
            ),
            Request::Search { query, from, fresh } => (
                kept_answer(
                    &self.mcf_home(),
                    &format!("search:{}@{}", query.trim(), from.as_deref().unwrap_or("")),
                    *fresh,
                    || Self::searched(query, from.as_deref()),
                ),
                None,
            ),
            Request::Settings { model } => (self.settings_for(model), None),
            Request::Readings { model, method } => {
                (self.readings_of(model, method.as_deref()), None)
            }
            Request::Anatomy { model } => (self.anatomy_of(model), None),
            Request::Tokenize {
                model,
                text,
                engine,
                beginning,
            } => (
                self.tokenized(model, text, engine.as_deref(), *beginning),
                None,
            ),
            Request::Hosted => (Answer::served(self.hosted()), None),
            Request::Unhost => (Answer::served(self.unhost()), None),
            Request::Generate { .. }
            | Request::Acquire { .. }
            | Request::Measure { .. }
            | Request::CrossCheck { .. }
            | Request::Provision { .. }
            | Request::Host { .. }
            | Request::PromptReport { .. }
            | Request::Probe { .. }
            | Request::Examine { .. } => (
                Answer::refused(&crate::control::refused(
                    "a request that answers in many lines reached the one-answer path",
                    "generate, acquire, measure, cross-check, provision, host, prompt-report, \
                     probe or examine",
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

    #[allow(
        clippy::too_many_arguments,
        reason = "one measurement's conditions, each named in what it writes"
    )]
    #[allow(
        clippy::too_many_arguments,
        reason = "one run's conditions — what, through which engine, on which device, how deep, started with what — and who is waiting"
    )]
    #[allow(
        clippy::too_many_lines,
        reason = "one run carried straight through: refused early, announced, read, recorded, said"
    )]
    fn measuring(
        &self,
        named: &str,
        engine: Option<&str>,
        on: Option<crate::control::On>,
        deepest: u64,
        started: crate::declared::Started,
        waiting: crate::served::Waiting<'_>,
        writer: &mut &UnixStream,
    ) {
        let say = |writer: &mut &UnixStream, answer: &Answer| self.tell("measure", writer, answer);

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
        if let Err(failure) = started.against(&crate::declared::Declared::of(&path)) {
            return say(writer, &Answer::refused(&failure));
        }

        let picked = match self.picked_engine_on(named, on) {
            Ok((engine, layers, context, _)) => Some((engine, layers, context)),
            Err(why) if on.is_some() => {
                return say(
                    writer,
                    &Answer::refused(
                        &mcf_core::failure::Failure::new(
                            mcf_core::failure::Category::EngineSpawnNotFound,
                            mcf_core::failure::Attribution::Machine,
                            mcf_core::failure::Disposition::Refused,
                            WHERE,
                            "nothing here can measure where the caller asked",
                        )
                        .with_context("wanted", why)
                        .with_context("model", named.to_owned()),
                    ),
                );
            }
            Err(_) => None,
        };

        say(
            writer,
            &estimate_line(named, &ladder, estimated_seconds(&ladder, held)),
        );
        let planned = planned_memory(&path, held, &ladder);
        let mut readings: Vec<Value> = Vec::new();
        let mut rows: Vec<mcf_record::readings::Reading> = Vec::new();
        let mut ran_on: Option<String> = None;
        for (step, depth) in ladder.iter().enumerate() {
            let starting = a_rung_starting(*depth, step, ladder.len());
            say(writer, &announced(named, "starting", starting));
            let mut report = |running: Value| say(writer, &announced(named, "running", running));
            let (reading, engine, pairs) = self.one_depth(
                named,
                engine,
                *depth,
                picked.as_ref(),
                started,
                waiting,
                &mut report,
            );
            rows.extend(pairs);
            ran_on = ran_on.take().or(engine);
            if asker_left(reading.get("why").and_then(Value::as_text)) {
                return;
            }
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
            self.placed(named, on).as_ref(),
        );
        let at = Timestamp::now();
        let _recorded = self.note(EntryKind::ModelTimed, at, last.body.clone());
        self.record_rows(
            &path,
            "throughput",
            ran_on.as_deref().unwrap_or("MCF did not say"),
            vec![(
                "deepest",
                Value::Integer(i64::try_from(deepest).unwrap_or(i64::MAX)),
            )],
            &rows,
            at,
        );
        if let Ok(mut timings) = self.timings.lock() {
            let _replaced = timings.insert(path.clone(), dated(last.body.clone(), at));
        }
        say(writer, &last);
    }

    #[allow(
        clippy::too_many_lines,
        reason = "one run carried straight through: refused early, announced, read, recorded, said"
    )]
    fn cross_checking(
        &self,
        named: &str,
        waiting: crate::served::Waiting<'_>,
        writer: &mut &UnixStream,
    ) {
        let say =
            |writer: &mut &UnixStream, answer: &Answer| self.tell("cross-check", writer, answer);
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
        let at = Timestamp::now();
        let _recorded = self.note(EntryKind::CrossChecked, at, last.body.clone());
        self.record_rows(
            &path,
            "cross-check",
            "mcf-standin against the provisioned engine",
            vec![("positions_asked", count(crate::crosscheck::POSITIONS))],
            &position_rows(&agreement),
            at,
        );
        if let Ok(mut checks) = self.cross_checks.lock() {
            let _replaced = checks.insert(path, dated(last.body.clone(), at));
        }
        say(writer, &last);
    }

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
            (false, false),
            picked,
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

    fn picked_engine(&self, named: &str) -> Option<(crate::adapters::ProvisionedLlama, u32, u64)> {
        self.picked_engine_or_why(named).ok()
    }

    fn hosted_for(
        &self,
        named: &str,
        started: crate::declared::Started,
        engine: Option<&str>,
    ) -> Option<HeldFor> {
        if engine.is_some_and(|engine| engine != "provisioned") {
            return None;
        }
        let holding = self
            .holding
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let held = holding.as_ref()?;
        if held.model != crate::generation::resolved(&self.places.models, named) {
            return None;
        }
        if started.asks_anything() && started != held.served.started {
            return None;
        }
        Some(HeldFor {
            picked: (
                crate::adapters::ProvisionedLlama {
                    prefix: held.served.prefix.clone(),
                    commit: held.served.commit.clone(),
                    component: held.settings.engine.clone(),
                },
                held.settings.gpu_layers,
                held.settings.context,
            ),
            started: held.served.started,
            served: std::sync::Arc::clone(&held.served),
        })
    }

    fn picked_engine_or_why(
        &self,
        named: &str,
    ) -> core::result::Result<(crate::adapters::ProvisionedLlama, u32, u64), String> {
        let (recommended, _, _) = self.recommend(named).map_err(|failure| {
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
            recommended.context,
        ))
    }

    fn last_measurement(&self, model: &Path) -> Option<Value> {
        let held = self.timings.lock().ok()?;
        held.get(model).cloned()
    }

    #[allow(
        clippy::too_many_arguments,
        reason = "one repeat's conditions — what, through which engine, how deep, which repeat, started with what — and who is waiting"
    )]
    fn rung_runs(
        &self,
        named: &str,
        engine: Option<&str>,
        depth: u64,
        repeat: u32,
        picked: Option<&(crate::adapters::ProvisionedLlama, u32, u64)>,
        started: crate::declared::Started,
        waiting: crate::served::Waiting<'_>,
        report: &mut dyn FnMut(Value),
    ) -> (
        std::result::Result<Timed, String>,
        std::result::Result<Timed, String>,
        std::result::Result<Timed, String>,
    ) {
        let mut run = |produce: u32, cached: bool| {
            report(a_run_starting(depth, produce, repeat));
            self.timed_generation(
                named,
                engine,
                depth,
                produce,
                cached,
                picked.cloned(),
                started,
                waiting,
            )
        };
        let warm = run(1, false);
        let one = run(1, true);
        let many = run(1 + SETTLED, true);
        (warm, one, many)
    }

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
        report: &mut dyn FnMut(Value),
    ) -> (Value, Option<String>, Vec<mcf_record::readings::Reading>) {
        let mut samples: Vec<u64> = Vec::new();
        let mut first_token: Vec<u64> = Vec::new();
        let mut peak_resident: Option<u64> = None;
        let mut window: Option<u64> = None;
        let mut ran_on: Option<String> = None;
        let mut refused: Option<String> = None;
        let mut fell_short: Option<String> = None;
        let mut pairs = Pairs::of(REPEATS);
        let mut rows: Vec<mcf_record::readings::Reading> = Vec::new();
        for repeat in 0..REPEATS {
            let (warm, one, many) = self.rung_runs(
                named, engine, depth, repeat, picked, started, waiting, report,
            );
            for (run, pinned, which) in [
                (&warm, 1, "warm"),
                (&one, 1, "one"),
                (&many, 1 + SETTLED, "many"),
            ] {
                match run {
                    Ok(timed) => {
                        ran_on = ran_on.take().or_else(|| timed.engine.clone());
                        peak_resident = timed.peak_resident.max(peak_resident);
                        window = timed.window.max(window);
                        if !timed.held_the_pin(pinned) {
                            fell_short = fell_short.take().or_else(|| Some(timed.short_of(pinned)));
                        }
                        rows.extend(pair_rows(depth, repeat, which, timed));
                    }
                    Err(why) => refused = refused.take().or_else(|| Some(why.clone())),
                }
            }
            if asker_left(refused.as_deref()) {
                break;
            }
            match (&one, &many) {
                (Ok(short), Ok(long))
                    if short.held_the_pin(1)
                        && long.held_the_pin(1 + SETTLED)
                        && long.ns > short.ns =>
                {
                    #[expect(
                        clippy::integer_division,
                        reason = "a difference in nanoseconds over sixteen tokens; the \
                                  remainder is under a nanosecond a token"
                    )]
                    let per_token = (long.ns - short.ns) / u64::from(SETTLED);
                    samples.push(per_token);
                    if let Ok(warm) = &warm
                        && warm.held_the_pin(1)
                    {
                        first_token.push(warm.ns);
                    }
                }
                (Ok(short), Ok(long))
                    if short.held_the_pin(1) && long.held_the_pin(1 + SETTLED) =>
                {
                    pairs.not_apart = pairs.not_apart.saturating_add(1);
                }
                (Ok(_), Ok(_)) => pairs.pin_missed = pairs.pin_missed.saturating_add(1),
                _ => pairs.refused = pairs.refused.saturating_add(1),
            }
        }
        pairs.separated = samples.len();
        let reading = rung_reading(
            depth,
            &mut samples,
            &mut first_token,
            (window, peak_resident),
            &pairs,
            (refused, fell_short),
        );
        (reading, ran_on, rows)
    }

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
        cached: bool,
        picked: Option<(crate::adapters::ProvisionedLlama, u32, u64)>,
        started: crate::declared::Started,
        waiting: crate::served::Waiting<'_>,
    ) -> std::result::Result<Timed, String> {
        let how_many =
            usize::try_from(depth).map_err(|_| "a depth this machine cannot count".to_owned())?;
        let tokens: Vec<usize> = vec![1; how_many];
        let (produced, took) = self.drained_generation(
            named,
            engine,
            &tokens,
            usize::try_from(produce).unwrap_or(1),
            (true, cached),
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
            engine_timings: produced
                .account
                .get("engine_timings")
                .filter(|held| !matches!(held, Value::Null))
                .cloned(),
        })
    }

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
        (pinned, cached): (bool, bool),
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
                crate::generation::Given::Identifiers(tokens),
                engine,
                picked,
                system_memory_free(),
                pinned,
                cached,
                None,
                None,
                started,
                None,
                waiting,
                &mut writer,
            )
        };
        let took = clock.now().saturating_duration_since(began);
        drop(mine);
        let _joined = drain.join();
        Ok((produced, took))
    }

    fn provisioning(&self, component: Option<&str>, writer: &mut &UnixStream) {
        let say =
            |writer: &mut &UnixStream, answer: &Answer| self.tell("provision", writer, answer);
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
                    ("usable_engine", Value::Bool(reached)),
                    ("done", Value::Bool(true)),
                ]))
            }
            Err(failure) => Answer::refused(&failure),
        };
        say(writer, &answer);
    }

    fn acquiring(&self, reference: &str, file: &str, from: Option<&str>, writer: &mut &UnixStream) {
        let say = |writer: &mut &UnixStream, answer: &Answer| self.tell("acquire", writer, answer);

        let parsed = match mcf_hub::reference::parse(reference) {
            Ok(parsed) => parsed,
            Err(failure) => return say(writer, &Answer::refused(&failure)),
        };
        let base = match mcf_hub::http::Url::parse(from.unwrap_or(DEFAULT_HUB)) {
            Ok(base) => base,
            Err(failure) => return say(writer, &Answer::refused(&failure)),
        };
        let wire = match mcf_hub::wire::for_url(&base) {
            Ok(wire) => wire,
            Err(failure) => return say(writer, &Answer::refused(&failure)),
        };
        let hub = mcf_hub::client::Hub::at(base.clone(), wire);
        let listing = match hub.list(&parsed) {
            Ok(listing) => listing,
            Err(failure) => return say(writer, &Answer::refused(&failure)),
        };
        drop(hub);
        if listing.entry(file).is_none() {
            return say(
                writer,
                &Answer::refused(&crate::control::refused(
                    "a file this repository does not publish",
                    file,
                )),
            );
        }
        let parts: Vec<mcf_hub::source::Entry> = match listing.parts_of(file) {
            Some(set) if !set.is_whole() => {
                return say(
                    writer,
                    &Answer::refused(
                        &crate::control::refused(
                            "a model published in parts, not all of which this repository \
                             publishes",
                            file,
                        )
                        .with_context("parts_found", set.parts.len().to_string())
                        .with_context("parts_declared", set.of.to_string()),
                    ),
                );
            }
            Some(set) => set.parts.into_iter().cloned().collect(),
            None => listing.entry(file).into_iter().cloned().collect(),
        };
        let whole: u64 = parts
            .iter()
            .map(|part| part.size)
            .fold(0, u64::saturating_add);
        let count = parts.len();
        let mut before = 0_u64;
        let mut first: Option<(String, mcf_hub::acquisition::Done)> = None;
        for (index, part) in parts.iter().enumerate() {
            let place = Progress {
                part: index.saturating_add(1),
                of: count,
                whole,
                before,
            };
            match self.fetching_one(&base, &listing, part, place, writer) {
                Ok(done) => {
                    before = before.saturating_add(part.size);
                    if first.is_none() {
                        first = Some((part.path.clone(), done));
                    }
                }
                Err(answer) => return say(writer, &answer),
            }
        }
        let answer = match first {
            Some((path, done)) => {
                let mut answer = acquired(&path, &done);
                if let Value::Map(fields) = &mut answer.body {
                    let _p = fields.insert("parts".to_owned(), as_whole(count));
                    let _b = fields.insert("bytes_whole".to_owned(), as_whole(whole));
                }
                answer
            }
            None => Answer::refused(&crate::control::refused(
                "a file this repository does not publish",
                file,
            )),
        };
        say(writer, &answer);
    }

    fn fetching_one(
        &self,
        base: &mcf_hub::http::Url,
        listing: &mcf_hub::source::Listing,
        entry: &mcf_hub::source::Entry,
        place: Progress,
        writer: &mut &UnixStream,
    ) -> core::result::Result<mcf_hub::acquisition::Done, Answer> {
        let say = |writer: &mut &UnixStream, answer: &Answer| self.tell("acquire", writer, answer);
        let root = self.places.models.clone();
        let arriving = mcf_hub::acquisition::arriving_at(&root, listing, entry);
        let total = entry.size;
        say(
            writer,
            &Answer::served(place.said(&entry.path, 0, total, "fetching")),
        );

        let (listing_for_thread, entry_for_thread) = (listing.clone(), entry.clone());
        let where_from = base.clone();
        let handle = std::thread::spawn(move || {
            let wire = mcf_hub::wire::for_url(&where_from)?;
            let hub = mcf_hub::client::Hub::at(where_from, wire);
            mcf_hub::acquisition::one(&hub, &listing_for_thread, &entry_for_thread, &root)
        });

        let mut furthest = 0_u64;
        while !handle.is_finished() {
            std::thread::sleep(std::time::Duration::from_millis(700));
            if let Ok(about) = std::fs::metadata(&arriving) {
                furthest = furthest.max(about.len());
            }
            let checking = furthest >= total && total > 0;
            let doing = if checking { "checking" } else { "fetching" };
            say(
                writer,
                &Answer::served(place.said(&entry.path, furthest, total, doing)),
            );
        }

        match handle.join() {
            Ok(Ok(done)) => Ok(done),
            Ok(Err(failure)) => Err(Answer::refused(&failure)),
            Err(_) => Err(Answer::refused(&crate::control::refused(
                "the transfer stopped without saying why",
                &entry.path,
            ))),
        }
    }

    fn remember_hold(&self, path: &Path, settings: &crate::hosting::Hosting, at: Timestamp) {
        if let Ok(mut held) = self.last_settings.lock() {
            let _replaced = held.insert(
                path.to_path_buf(),
                Value::map([
                    ("settings", settings.to_value()),
                    ("since", mcf_record::encode::timestamp(at)),
                ]),
            );
        }
        if let Ok(mut last) = self.last_hold.lock() {
            *last = Some(Value::map([
                ("model", Value::text(path.display().to_string())),
                ("engine", Value::text(settings.engine.clone())),
                ("device", Value::text(settings.device.clone())),
                ("since", mcf_record::encode::timestamp(at)),
                ("until", Value::Null),
            ]));
        }
    }

    fn placed(&self, named: &str, on: Option<crate::control::On>) -> Option<(String, u32)> {
        let (_, layers, _, device) = self.picked_engine_on(named, on).ok()?;
        Some((device, layers))
    }

    fn picked_engine_on(
        &self,
        named: &str,
        on: Option<crate::control::On>,
    ) -> core::result::Result<(crate::adapters::ProvisionedLlama, u32, u64, String), String> {
        let (recommended, _, largest) = self.recommend(named).map_err(|failure| {
            failure
                .context_value("wanted")
                .map_or_else(|| failure.detail().to_owned(), str::to_owned)
        })?;
        let held = self.engines_held();
        let adapter = |engine: &crate::engines::Engine| crate::adapters::ProvisionedLlama {
            prefix: engine.prefix.clone(),
            commit: engine.commit.clone(),
            component: engine.name.clone(),
        };
        let resolved = || {
            held.iter()
                .find(|(engine, _)| engine.name == recommended.engine)
        };
        match on {
            None => {
                let (engine, _) = resolved().ok_or_else(|| {
                    format!(
                        "the engine this model resolves to, {}, is not provisioned",
                        recommended.engine
                    )
                })?;
                Ok((
                    adapter(engine),
                    recommended.gpu_layers,
                    largest,
                    recommended.device.clone(),
                ))
            }
            Some(crate::control::On::Processor) => {
                let (engine, _) = held
                    .iter()
                    .find(|(_, devices)| {
                        devices
                            .iter()
                            .all(|device| device.kind == crate::engines::Kind::Cpu)
                    })
                    .or_else(resolved)
                    .ok_or_else(|| "no engine is provisioned".to_owned())?;
                Ok((adapter(engine), 0, largest, "CPU".to_owned()))
            }
            Some(crate::control::On::Card) => {
                let (engine, card) = held
                    .iter()
                    .find_map(|(engine, devices)| {
                        devices
                            .iter()
                            .find(|device| device.kind == crate::engines::Kind::Gpu)
                            .map(|device| (engine, device.name.clone()))
                    })
                    .ok_or_else(|| {
                        crate::engines::wanted_for(crate::engines::backend_present()).map_or_else(
                            || "no card is here, and no engine drives one".to_owned(),
                            |component| {
                                format!(
                                    "no engine here drives the card; `mcf provision {}` builds one",
                                    component.name
                                )
                            },
                        )
                    })?;
                Ok((adapter(engine), 999, largest, card))
            }
        }
    }

    fn card_unused(&self) -> Option<Value> {
        let wanted = crate::engines::wanted_for(crate::engines::backend_present())?;
        if self
            .engines_held()
            .iter()
            .any(|(engine, _)| engine.name == wanted.name)
        {
            return None;
        }
        let card = crate::engines::card_driver_present().map_or_else(
            || "a card".to_owned(),
            |driver| format!("a card the kernel drives through {driver}"),
        );
        Some(Value::map([
            ("component", Value::text(wanted.name.to_owned())),
            ("card", Value::text(card.clone())),
            (
                "because",
                Value::text(format!(
                    "{card} is here, and no engine MCF has built drives it; {} would, and \
                     until it is built every figure here is the processor's",
                    wanted.name
                )),
            ),
        ]))
    }

    fn placements(&self, named: &str, recommended: &crate::hosting::Hosting) -> Value {
        let held = self.engines_held();
        let free_on = |device: &str| {
            held.iter()
                .flat_map(|(_, devices)| devices.iter())
                .find(|found| found.name == device)
                .and_then(|found| found.free)
        };
        let one = |on: &str, engine: &str, device: &str, layers: u32| {
            Value::map([
                ("on", Value::text(on.to_owned())),
                ("engine", Value::text(engine.to_owned())),
                ("device", Value::text(device.to_owned())),
                ("gpu_layers", Value::Integer(i64::from(layers))),
                ("free_bytes", free_on(device).map_or(Value::Null, as_whole)),
            ])
        };
        let mut placements = vec![one(
            "resolved",
            &recommended.engine,
            &recommended.device,
            recommended.gpu_layers,
        )];
        for on in [crate::control::On::Processor, crate::control::On::Card] {
            if let Ok((llama, layers, _, device)) = self.picked_engine_on(named, Some(on)) {
                placements.push(one(on.as_str(), &llama.component, &device, layers));
            }
        }
        Value::List(placements)
    }

    fn settings_for(&self, named: &str) -> Answer {
        match self.recommend(named) {
            Ok((recommended, path, _)) => Answer::served(Value::map([
                ("model", Value::text(named.to_owned())),
                ("recommended", recommended.to_value()),
                ("settings", recommended.to_value()),
                ("placements", self.placements(named, &recommended)),
                (
                    "last",
                    self.last_settings
                        .lock()
                        .ok()
                        .and_then(|held| held.get(&path).cloned())
                        .unwrap_or(Value::Null),
                ),
                ("declares", crate::declared::Declared::of(&path).to_value()),
                (
                    "cache_bytes",
                    self.cache_for(named, recommended.context)
                        .map_or(Value::Null, |bytes| {
                            Value::Integer(i64::try_from(bytes).unwrap_or(i64::MAX))
                        }),
                ),
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

    fn tokenized(&self, named: &str, text: &str, engine: Option<&str>, beginning: bool) -> Answer {
        let path = crate::generation::resolved(&self.places.models, named);
        if !path.is_file() {
            return Answer::refused(&crate::control::refused(
                "a model this machine is not holding",
                named,
            ));
        }
        let picked = match engine {
            Some("stand-in") => None,
            _ => self.picked_engine(named),
        };
        let tokenizer = match self.tokenizer_for(named, picked.as_ref(), None) {
            Ok(tokenizer) => tokenizer,
            Err(failure) => return Answer::refused(&failure),
        };
        match tokenizer.encode(text, beginning) {
            Ok(read) => Answer::served(Value::map([
                ("model", Value::text(named.to_owned())),
                (
                    "tokens",
                    Value::Integer(i64::try_from(read.len()).unwrap_or(i64::MAX)),
                ),
                ("read_by", Value::text(tokenizer.reader())),
                (
                    "read",
                    Value::List(
                        read.into_iter()
                            .map(|token| {
                                Value::map([
                                    (
                                        "id",
                                        Value::Integer(i64::try_from(token.id).unwrap_or(i64::MAX)),
                                    ),
                                    ("piece", Value::text(token.piece)),
                                ])
                            })
                            .collect(),
                    ),
                ),
            ])),
            Err(failure) => Answer::refused(&failure),
        }
    }

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

    fn recommend(&self, named: &str) -> Result<(crate::hosting::Hosting, PathBuf, u64)> {
        let path = crate::generation::resolved(&self.places.models, named);
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
                match (&refused, needed_engine()) {
                    (crate::engines::Refused::NoEngine, Some(component)) => {
                        failure.with_context("needs_component", component.name)
                    }
                    _ => failure,
                }
            },
        )?;
        let on_a_card = matches!(choice.device.kind, crate::engines::Kind::Gpu);
        let context = crate::engines::held_at(choice.context, bytes, cache.unwrap_or(0));
        let wanted = bytes.saturating_add(cache.unwrap_or(0).saturating_mul(context));
        let fits = choice.device.free.is_none_or(|free| wanted <= free);
        let projector = crate::projector::beside(&path);
        Ok((
            crate::hosting::Hosting::recommended(
                &choice.engine,
                &choice.device.name,
                on_a_card,
                context,
                std::thread::available_parallelism().ok().map(Into::into),
                fits,
                projector.as_deref(),
            ),
            path,
            choice.context,
        ))
    }

    fn engine_called(
        &self,
        name: &str,
    ) -> core::result::Result<crate::adapters::ProvisionedLlama, Answer> {
        let (engine, _) = self
            .engines_held()
            .into_iter()
            .find(|(engine, _)| engine.name == name)
            .ok_or_else(|| {
                Answer::refused(&crate::control::refused(
                    "no provisioned engine by that name: `mcf provision` builds one",
                    name,
                ))
            })?;
        Ok(crate::adapters::ProvisionedLlama {
            prefix: engine.prefix.clone(),
            commit: engine.commit.clone(),
            component: engine.name.clone(),
        })
    }

    fn hosting(&self, named: &str, asked: &Value, writer: &mut &UnixStream) {
        let say = |writer: &mut &UnixStream, answer: &Answer| self.tell("host", writer, answer);
        let answer = self.host(named, asked, &mut |loading| {
            say(writer, &Answer::served(loading));
        });
        say(writer, &answer);
    }

    #[allow(
        clippy::too_many_lines,
        reason = "one hold, each refusal named before it starts"
    )]
    fn host(&self, named: &str, asked: &Value, report: &mut dyn FnMut(Value)) -> Answer {
        let (recommended, path, _) = match self.recommend(named) {
            Ok(held) => held,
            Err(failure) => return Answer::refused(&failure),
        };
        let settings = crate::hosting::Hosting::from_value(asked, &recommended);
        let declared = crate::declared::Declared::of(&path);
        if let Err(failure) = settings.started.against(&declared) {
            return Answer::refused(&failure);
        }
        if settings.open && settings.api_key.is_none() {
            return Answer::refused(&Failure::new(
                Category::ConfigInvalid,
                Attribution::User,
                Disposition::Refused,
                WHERE,
                "a hold reachable from the network needs an API key: set one, or leave the hold \
                 on this computer alone",
            ));
        }

        let llama = match self.engine_called(&settings.engine) {
            Ok(llama) => llama,
            Err(refused) => return refused,
        };
        let _released = self.let_go("another model was hosted in its place");
        let mut holding = match self.holding.lock() {
            Ok(holding) => holding,
            Err(poisoned) => poisoned.into_inner(),
        };

        if !settings.port_is_free() {
            return Answer::refused(&crate::control::refused(
                "something is already listening on that port, so MCF did not start a second \
                 thing there — choose another port, or stop what is on it",
                &settings.port.to_string(),
            ));
        }

        let (of, began) = (
            std::fs::metadata(&path).map(|about| about.len()).ok(),
            std::time::Instant::now(),
        );
        let card_before = (settings.gpu_layers > 0)
            .then(crate::engines::card_memory_used)
            .flatten();
        let mut loading = |resident: u64| {
            let on_card = card_before.and_then(|before| {
                crate::engines::card_memory_used().map(|now| now.saturating_sub(before))
            });
            report(a_load_so_far(
                &path,
                (resident, on_card),
                of,
                began.elapsed().as_secs(),
            ));
        };
        match crate::served::Served::hosted(&llama, &path, &settings, &mut loading) {
            Ok(served) => {
                let at = Timestamp::now();
                let moved = settings.differs_from(&recommended);
                let takes = crate::takes::Takes::asked_on(settings.port);
                let takes_value = takes
                    .as_ref()
                    .map_or(Value::Null, crate::takes::Takes::to_value);
                let _recorded = self.note(
                    EntryKind::ModelHosted,
                    at,
                    Value::map([
                        ("model", Value::text(path.display().to_string())),
                        ("address", Value::text(settings.address())),
                        (
                            "network_address",
                            settings.network_address().map_or(Value::Null, Value::text),
                        ),
                        (
                            "network_address",
                            settings.network_address().map_or(Value::Null, Value::text),
                        ),
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
                    served: std::sync::Arc::new(served),
                    model: path.clone(),
                    settings: settings.clone(),
                    recommended: recommended.clone(),
                    takes,
                    since: at,
                    card_before,
                });
                self.remember_hold(&path, &settings, at);
                Answer::served(Value::map([
                    ("hosting", Value::text(path.display().to_string())),
                    ("address", Value::text(settings.address())),
                    (
                        "network_address",
                        settings.network_address().map_or(Value::Null, Value::text),
                    ),
                    ("settings", settings.to_value()),
                    ("recommended", recommended.to_value()),
                    (
                        "changed",
                        Value::List(moved.into_iter().map(Value::text).collect()),
                    ),
                    ("takes", takes_value),
                    ("declares", declared.to_value()),
                    ("since", Value::text(at.to_string())),
                    ("done", Value::Bool(true)),
                ]))
            }
            Err(failure) => Answer::refused(&failure),
        }
    }

    fn under_test() -> Value {
        let testing = crate::served::live()
            .into_iter()
            .rev()
            .find(|live| matches!(live.reach, crate::served::Reach::Socket(_)));
        let Some(testing) = testing else {
            return Value::Null;
        };
        let mut fields: Vec<(&'static str, Value)> = use_figures(&testing.reach);
        fields.push((
            "resident_bytes",
            crate::adapters::resident_of(testing.child).map_or(Value::Null, as_whole),
        ));
        Value::map([
            ("model", Value::text(testing.model.display().to_string())),
            (
                "engine",
                Value::text(format!("provisioned llama.cpp @{}", testing.commit)),
            ),
            ("window", as_whole(testing.window)),
            ("use", Value::map(fields)),
        ])
    }

    fn hosted(&self) -> Value {
        let holding = match self.holding.lock() {
            Ok(holding) => holding,
            Err(poisoned) => poisoned.into_inner(),
        };
        match holding.as_ref() {
            None => Value::map([
                ("hosting", Value::Null),
                ("under_test", Self::under_test()),
                (
                    "last",
                    self.last_hold
                        .lock()
                        .ok()
                        .and_then(|last| last.clone())
                        .map_or(Value::Null, with_ago),
                ),
            ]),
            Some(held) => Value::map([
                ("hosting", Value::text(held.model.display().to_string())),
                ("address", Value::text(held.settings.address())),
                (
                    "network_address",
                    held.settings
                        .network_address()
                        .map_or(Value::Null, Value::text),
                ),
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
                ("use", in_use(held)),
                ("under_test", Self::under_test()),
            ]),
        }
    }

    fn let_go(&self, why: &str) -> Option<Freed> {
        let mut holding = match self.holding.lock() {
            Ok(holding) => holding,
            Err(poisoned) => poisoned.into_inner(),
        };
        let held = holding.take()?;
        for _ in 0..240 {
            if std::sync::Arc::strong_count(&held.served) == 1 {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(250));
        }
        let model = held.model.display().to_string();
        let resident = held.served.resident_bytes();
        let energy = energy_of(held.served.reach());
        let card_before = (held.settings.gpu_layers > 0)
            .then(crate::engines::card_memory_used)
            .flatten();
        drop(held);
        let card = card_before.and_then(|before| {
            let mut freed = 0;
            for _ in 0..8 {
                std::thread::sleep(std::time::Duration::from_millis(250));
                let now = crate::engines::card_memory_used()?;
                freed = freed.max(before.saturating_sub(now));
                if freed > 0 {
                    break;
                }
            }
            Some(freed)
        });
        let freed = Freed {
            model,
            resident,
            card,
        };
        let spent = energy;
        let at = Timestamp::now();
        let _recorded = self.note(
            EntryKind::ModelUnhosted,
            at,
            Value::map([
                ("model", Value::text(freed.model.clone())),
                ("reason", Value::text(why.to_owned())),
                ("freed_bytes", freed.resident.map_or(Value::Null, as_whole)),
                ("freed_card_bytes", freed.card.map_or(Value::Null, as_whole)),
                (
                    "card_energy_joules",
                    spent.map_or(Value::Null, |(microjoules, _)| {
                        thousandths_of(microjoules, 1_000)
                    }),
                ),
                (
                    "card_energy_over_seconds",
                    spent.map_or(Value::Null, |(_, covered_ns)| {
                        thousandths_of(covered_ns, 1_000_000)
                    }),
                ),
            ]),
        );
        if let Ok(mut last) = self.last_hold.lock()
            && let Some(Value::Map(fields)) = last.as_mut()
        {
            fields.insert("until".to_owned(), mcf_record::encode::timestamp(at));
        }
        Some(freed)
    }

    fn unhost(&self) -> Value {
        let was = self.let_go("asked");
        Value::map([
            ("stopped", Value::Bool(was.is_some())),
            (
                "was",
                was.as_ref()
                    .map_or(Value::Null, |freed| Value::text(freed.model.clone())),
            ),
            (
                "freed_bytes",
                was.as_ref()
                    .and_then(|freed| freed.resident)
                    .map_or(Value::Null, as_whole),
            ),
            (
                "freed_card_bytes",
                was.and_then(|freed| freed.card)
                    .map_or(Value::Null, as_whole),
            ),
        ])
    }

    fn searched(query: &str, from: Option<&str>) -> Answer {
        if query.trim().is_empty() {
            return Answer::refused(&crate::control::refused("a word to search for", query));
        }
        let base = match mcf_hub::http::Url::parse(from.unwrap_or(DEFAULT_HUB)) {
            Ok(base) => base,
            Err(failure) => return Answer::refused(&failure),
        };
        let wire = match mcf_hub::wire::for_url(&base) {
            Ok(wire) => wire,
            Err(failure) => return Answer::refused(&failure),
        };
        let hub = mcf_hub::client::Hub::at(base, wire);
        let mut searched = query.trim().to_owned();
        let mut gathered: Vec<mcf_hub::client::Found> = Vec::new();
        let found = loop {
            match hub.search(&searched) {
                Ok(found) => {
                    for one in found {
                        if !gathered.iter().any(|held| held.id == one.id) {
                            gathered.push(one);
                        }
                    }
                    if gathered.len() >= ENOUGH_FOUND {
                        break Ok(());
                    }
                    match shorter_name(&searched) {
                        Some(shorter) => searched = shorter,
                        None => break Ok(()),
                    }
                }
                Err(failure) => break Err(failure),
            }
        };
        gathered.sort_by_key(|found| std::cmp::Reverse(found.downloads));
        match found {
            Ok(()) => Answer::served(Value::map([
                ("query", Value::text(query.to_owned())),
                ("searched_as", Value::text(searched)),
                (
                    "repositories",
                    Value::List(
                        gathered
                            .iter()
                            .map(mcf_hub::client::Found::to_value)
                            .collect(),
                    ),
                ),
            ])),
            Err(failure) => Answer::refused(&failure),
        }
    }

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
        let (planned, from_the_header) = plan_however_the_shape_can_be_found(&hub, &listing);

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
            .variants()
            .into_iter()
            .map(|variant| {
                let verdict = verdicts.get(&variant.first);
                Value::map([
                    ("file", Value::text(variant.first.clone())),
                    ("name", Value::text(variant.name.clone())),
                    ("parts", Value::Integer(i64::from(variant.parts))),
                    ("whole", Value::Bool(variant.whole)),
                    (
                        "bytes",
                        Value::Integer(i64::try_from(variant.bytes).unwrap_or(i64::MAX)),
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

    fn status(&self) -> Value {
        let identity = BuildIdentity::current();
        Value::map([
            ("protocol", Value::Integer(VERSION)),
            ("build", mcf_record::encode::build_identity(identity)),
            ("resident", self.resident_value()),
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
                    (
                        "engines_stopped",
                        Value::List(
                            self.recovered
                                .engines_stopped
                                .iter()
                                .map(crate::orphans::Orphan::as_value)
                                .collect(),
                        ),
                    ),
                ]),
            ),
            ("engines", self.engines_as_value()),
            ("cannot", self.cannot()),
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

    fn failures(&self, last: usize) -> Value {
        let journal = &self.places.journal;
        let index = if journal.exists() {
            mcf_record::journal::Index::over(
                journal,
                &mcf_record::journal::index::default_path(journal),
            )
            .ok()
        } else {
            None
        };
        let Some(index) = index else {
            return Value::map([
                ("failures", Value::List(Vec::new())),
                ("in_record", Value::Integer(0)),
            ]);
        };
        let mut failures: Vec<Value> = index
            .latest(Some(EntryKind::Failure), last)
            .iter()
            .filter_map(|located| index.read(located).ok())
            .map(|entry| entry.to_value())
            .collect();
        failures.reverse();
        Value::map([
            ("failures", Value::List(failures)),
            (
                "in_record",
                Value::Integer(i64::try_from(index.count(EntryKind::Failure)).unwrap_or(i64::MAX)),
            ),
        ])
    }

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
        if let Ok(mut readings) = self.readings.lock() {
            *readings = all_readings(&self.places.journal, None);
        }
        match mcf_hub::store::held(&self.places.models) {
            Err(failure) => Value::map([
                ("readable", Value::Bool(false)),
                ("why", mcf_record::encode::failure(&failure)),
            ]),
            Ok(holding) => Value::map([
                ("readable", Value::Bool(true)),
                ("card_unused", self.card_unused().unwrap_or(Value::Null)),
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
    fn drop(&mut self) {
        let _removed = std::fs::remove_file(&self.places.socket);
    }
}

fn recover(places: &Places) -> Result<Recovered> {
    let (entries, unreadable) = if places.journal.exists() {
        let index = mcf_record::journal::Index::over(
            &places.journal,
            &mcf_record::journal::index::default_path(&places.journal),
        )?;
        (index.entries().len(), index.loss().map(ToString::to_string))
    } else {
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
        engines_stopped: Vec::new(),
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
