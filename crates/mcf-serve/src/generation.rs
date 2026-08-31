//! A generation served over the socket: the engine, the stream, the account
//! (B-034, B-032, PR9).
//!
//! **The engine is a recorded condition, not a code path** (B-032, §7.4). Today
//! there is one engine, MCF's own, and it runs in this process; the account
//! names it with its build, so that the day a provisioned engine serves the
//! same request the difference is a field in the record and nothing else.
//!
//! **Every refusal is also a terminating line.** A model that cannot be found,
//! read, or run is answered with `done` carrying a `failure`, so that a client
//! reading until `done` is never left waiting, and the record gets the same
//! account (A2, A26).

use std::io::Write as _;
use std::os::unix::net::UnixStream;
use std::path::Path;

use mcf_core::failure::Failure;
use mcf_record::json::Value;
use mcf_standin::gguf;
use mcf_standin::llama;
use mcf_standin::sample::Settings;
use mcf_standin::session::{self, Request, Stopped};
use mcf_standin::tokenizer::Vocabulary;

use crate::control::Streamed;
use crate::served::Served;

/// The model held between requests (D41, §7.18).
///
/// One at a time, identified by path and by the file's length and modification
/// time — a file replaced under the same name is another model, and is loaded
/// again rather than served from memory (A1).
#[derive(Debug)]
pub(crate) struct Resident {
    path: std::path::PathBuf,
    length: u64,
    modified: Option<std::time::SystemTime>,
    since: String,
    vocabulary: Vocabulary,
    model: llama::Loaded,
    dequantized_bytes: u64,
}

impl Resident {
    /// Path, size dequantized, and since when — what `mcf status` shows.
    pub(crate) fn describe(&self) -> Value {
        Value::map([
            ("path", Value::text(self.path.display().to_string())),
            (
                "bytes_dequantized",
                Value::Integer(i64::try_from(self.dequantized_bytes).unwrap_or(i64::MAX)),
            ),
            ("since", Value::text(self.since.clone())),
        ])
    }
}

/// What a generation produced, in two halves that cannot be confused (A25,
/// §6.8, F105).
///
/// **The record may hold the left half and never the right.** A25 is absolute:
/// what a person typed and what a model generated live in a store that is not
/// the record. It was violated for three months by the simplest possible route
/// — one `Value` served as both the line sent to the caller and the body of the
/// record entry, so the model's completion went into the journal as an ordinary
/// string and `mcf export` copied it out while printing *no prompt or
/// completion content, by construction*.
///
/// So there are two values and the join goes one way. [`Produced::account`] is
/// what the builders make and what the daemon records; [`Produced::on_the_wire`]
/// adds what the caller asked for. Nothing removes anything: a *filter* is
/// B9's named violation, and a filter that stopped being applied would be
/// silent.
pub(crate) struct Produced {
    /// Facts *about* the generation: counts, conditions, how it stopped, what
    /// was lost. This is the half the record may hold.
    pub(crate) account: Value,
    /// What the model said, and the identifiers it said it in.
    ///
    /// `None` for a generation that produced nothing, and for one that was
    /// refused before an engine was reached.
    pub(crate) said: Option<Said>,
}

/// The model's own output: content, in A25's sense.
pub(crate) struct Said {
    /// The text.
    pub(crate) text: String,
    /// The same thing as identifiers, which another engine can be asked about
    /// step by step where text cannot (B-362): past the first disagreement two
    /// engines are writing different sentences.
    pub(crate) tokens: Vec<usize>,
}

impl Produced {
    /// The line the caller receives: the account, plus what it asked for.
    ///
    /// One direction, one call site. A generation is a thing somebody asked
    /// for, and answering it is not publication — what A25 governs is where the
    /// answer is *kept*.
    fn on_the_wire(&self) -> Value {
        let Value::Map(fields) = &self.account else {
            return self.account.clone();
        };
        let mut fields = fields.clone();
        if let Some(said) = &self.said {
            fields.insert("text".to_owned(), Value::text(said.text.clone()));
            if !said.tokens.is_empty() {
                fields.insert(
                    "produced_tokens".to_owned(),
                    Value::List(
                        said.tokens
                            .iter()
                            .map(|token| Value::Integer(i64::try_from(*token).unwrap_or(i64::MAX)))
                            .collect(),
                    ),
                );
            }
        }
        Value::Map(fields)
    }
}

/// Serves one generation, writing the stream, and returns the account that
/// was sent as the terminating line.
#[allow(
    clippy::too_many_arguments,
    reason = "one request's worth of conditions, each of which the account names"
)]
pub(crate) fn serve_generation(
    store: &Path,
    mcf_home: &Path,
    resident: &std::sync::Mutex<Option<Resident>>,
    server: &std::sync::Mutex<Option<Served>>,
    runtime: &Path,
    named: &str,
    prompt: &str,
    limit: Option<usize>,
    seed: u64,
    tokens: Option<&[usize]>,
    engine: Option<&str>,
    // The engine the daemon resolved for this model, and how many of the
    // model's layers go on the card. Passed in rather than decided here:
    // which build to use and what device to use it on is one question, the
    // daemon is the only thing that knows what the machine has, and it has
    // answered. A generation that chose again for itself would be a second
    // answer to a question already settled — and when it did, it chose the
    // processor build every time, because it matched a name (§3.15, F133).
    picked: Option<(crate::adapters::ProvisionedLlama, u32)>,
    writer: &mut &UnixStream,
) -> Produced {
    // What somebody decided this model should be addressed as, if anybody
    // did (D43, B-059). A caller that sent identifiers has said exactly what
    // it wants and is not overridden; a caller that sent a prompt gets the
    // addressing that was applied, and the account says so — MCF doing
    // something other than the plain thing must never be invisible (§3.15).
    let derived = crate::configured::read_derived(mcf_home, &resolved(store, named));
    // The caller's word first, then what somebody derived for this model, then
    // MCF's stated default. A caller who said nothing is not a caller who said
    // the default (D43, §3.15).
    let limit = limit
        .or_else(|| derived.budget.as_ref().map(|budget| budget.tokens))
        .unwrap_or(crate::control::DEFAULT_LIMIT);
    let derived_budget = derived.budget.clone();
    let derived = derived.addressing;
    let wrapped = match (tokens, derived.as_ref()) {
        (None, Some(addressing)) => addressed_as(store, named, prompt, addressing),
        _ => None,
    };
    let tokens = wrapped.as_deref().or(tokens);

    let (chosen, gpu_layers) = match (picked, engine) {
        // A caller that asked for MCF's own engine gets it, whatever was
        // resolved: naming the engine is the point of the argument (§3.15).
        (_, Some("stand-in")) => (Ok(Chosen::StandIn), 0),
        (Some((llama, layers)), _) => (Ok(Chosen::Provisioned(llama)), layers),
        (None, asked) => (choose_engine(mcf_home, asked), 0),
    };
    let produced = match chosen {
        // A turn of identifiers goes to the server, which can be given one;
        // a prompt goes to the completion tool, which cannot (B-376).
        Ok(Chosen::Provisioned(llama)) => match tokens {
            Some(tokens) => through_served(
                store, &llama, server, runtime, named, tokens, limit, seed, gpu_layers, writer,
            ),
            None => through_provisioned(store, &llama, named, prompt, limit, seed, writer),
        },
        Ok(Chosen::StandIn) => attempt(store, resident, named, prompt, tokens, limit, seed, writer),
        Err(failure) => Err(failure),
    };
    // The provenance travels into the account, so that a measurement taken
    // through a derived configuration carries what set it — which is what
    // makes *are yesterday's number and today's comparable* answerable rather
    // than assumed (D43, §3.4).
    let produced = produced.map(|produced| Produced {
        account: match (produced.account, derived) {
            (Value::Map(mut fields), Some(addressing)) => {
                if let Some(Value::Map(conditions)) = fields.get_mut("conditions") {
                    conditions.insert(
                        "addressed_as".to_owned(),
                        Value::text(if wrapped.is_some() {
                            addressing.provenance()
                        } else {
                            format!(
                                "{} — not applied here: the caller sent its own identifiers",
                                addressing.provenance()
                            )
                        }),
                    );
                }
                Value::Map(fields)
            }
            (account, _) => account,
        },
        said: produced.said,
    });
    let produced = produced.map(|produced| Produced {
        account: match (produced.account, derived_budget) {
            (Value::Map(mut fields), Some(budget)) => {
                if let Some(Value::Map(conditions)) = fields.get_mut("conditions") {
                    conditions.insert("budget_from".to_owned(), Value::text(budget.provenance()));
                }
                Value::Map(fields)
            }
            (account, _) => account,
        },
        said: produced.said,
    });
    let produced = match produced {
        Ok(produced) => produced,
        Err(failure) => Produced {
            account: Value::map([
                ("tokens", Value::Integer(0)),
                ("stopped", Value::text("refused")),
                ("failure", mcf_record::encode::failure(&failure)),
                ("conditions", conditions(named, None, seed, limit)),
            ]),
            said: None,
        },
    };
    let _written = writeln!(
        writer,
        "{}",
        Streamed::Done(produced.on_the_wire()).to_line()
    );
    let _flushed = writer.flush();
    produced
}

/// Which engine serves a request (B-032, §3.15).
enum Chosen {
    StandIn,
    Provisioned(crate::adapters::ProvisionedLlama),
}

/// The stated rule: what the client asked for; else the provisioned engine
/// where there is exactly one; else MCF's own. Two provisioned pins is refused
/// rather than chosen between, because which of two builds served an answer is
/// a condition the operator has to have decided.
fn choose_engine(mcf_home: &Path, asked: Option<&str>) -> Result<Chosen, Failure> {
    use crate::adapters::provisioned_llama;
    match asked {
        Some("stand-in") => return Ok(Chosen::StandIn),
        Some("provisioned") | None => {}
        Some(other) => {
            return Err(crate::control::refused(
                "an engine MCF does not have: stand-in or provisioned",
                other,
            ));
        }
    }
    match crate::adapters::only_one(provisioned_llama(mcf_home))? {
        Some(llama) => Ok(Chosen::Provisioned(llama)),
        None => match asked {
            Some(_) => Err(Failure::new(
                mcf_core::failure::Category::EngineUnavailable,
                mcf_core::failure::Attribution::Machine,
                mcf_core::failure::Disposition::Refused,
                mcf_core::failure::Subsystem::new("mcf-serve::generation"),
                "no provisioned engine is here: `mcf provision llama.cpp` builds one (B-367)",
            )),
            None => Ok(Chosen::StandIn),
        },
    }
}

/// The model a request names, as a path.
///
/// Written once: three engines resolved it identically, and a fourth reader —
/// the derived configuration — would have made four.
pub(crate) fn resolved(store: &Path, named: &str) -> std::path::PathBuf {
    let given = Path::new(named);
    if given.is_file() {
        given.to_path_buf()
    } else {
        store.join(named.replace(':', "/"))
    }
}

/// A prompt wrapped the way somebody decided this model should be addressed.
///
/// `None` where the turn cannot be built — a vocabulary that will not read, a
/// marker the file no longer holds. That is not a silent fallback to raw: the
/// generation proceeds with the prompt as text, which is what would have
/// happened anyway, and the account still carries the configuration so a
/// reader can see it was on file. Wrapping *some* of a turn would be worse
/// than not wrapping it (F37).
fn addressed_as(
    store: &Path,
    named: &str,
    prompt: &str,
    addressing: &crate::configured::Addressing,
) -> Option<Vec<usize>> {
    let path = resolved(store, named);
    let bytes = std::fs::read(&path).ok()?;
    let file = mcf_standin::gguf::parse(&bytes).ok()?;
    let vocabulary = Vocabulary::read(&file).ok()?;
    let mut pieces = addressing.before.clone();
    pieces.push(mcf_standin::tokenizer::Piece::Text(prompt.to_owned()));
    pieces.extend(addressing.after.iter().cloned());
    vocabulary.addressed(&pieces)
}

/// One generation through the provisioned engine driven as a *server*
/// (B-376), which is the shape that can be probed: the turn goes as
/// identifiers and the engine says why it stopped.
///
/// The server holds the model between requests, which is the residency F36
/// left open. A request for a different model replaces the server, and
/// replacing it stops the old one — `Served` kills its child when it is
/// dropped, so the model does not stay in memory on a machine three other
/// projects share (A27).
#[allow(
    clippy::too_many_arguments,
    reason = "one request's conditions, each named in the account"
)]
fn through_served(
    store: &Path,
    llama: &crate::adapters::ProvisionedLlama,
    server: &std::sync::Mutex<Option<Served>>,
    runtime: &Path,
    named: &str,
    tokens: &[usize],
    limit: usize,
    seed: u64,
    gpu_layers: u32,
    writer: &mut &UnixStream,
) -> Result<Produced, Failure> {
    let given = Path::new(named);
    let path = if given.is_file() {
        given.to_path_buf()
    } else {
        store.join(named.replace(':', "/"))
    };
    let metadata = std::fs::metadata(&path).map_err(|error| missing(named, &path, &error))?;
    let held = metadata.len();

    let mut slot = server.lock().map_err(|_poisoned| {
        Failure::new(
            mcf_core::failure::Category::EngineUnavailable,
            mcf_core::failure::Attribution::Machine,
            mcf_core::failure::Disposition::Aborted,
            mcf_core::failure::Subsystem::new("mcf-serve::generation"),
            "the served engine's slot was left poisoned by an earlier failure",
        )
    })?;
    // A server holding a different model is stopped rather than kept beside
    // this one: two resident models is a decision about memory nobody has
    // taken (D41, DEC-018), and taking it here silently would be the hidden
    // choice §3.15 forbids.
    let reused = slot.as_ref().is_some_and(|held| held.model == path);
    if !reused {
        *slot = None;
        *slot = Some(Served::start(llama, &path, runtime, gpu_layers)?);
    }
    let engine = slot.as_ref().ok_or_else(|| {
        Failure::new(
            mcf_core::failure::Category::EngineUnavailable,
            mcf_core::failure::Attribution::Machine,
            mcf_core::failure::Disposition::Aborted,
            mcf_core::failure::Subsystem::new("mcf-serve::generation"),
            "the served engine was started and then was not there",
        )
    })?;

    let completed = engine.complete(tokens, limit, seed)?;

    // The answer arrives whole rather than token by token, so it is one chunk
    // of the stream. Calling it several would be inventing a shape the engine
    // did not have.
    let line = Streamed::Token {
        at: 0,
        text: completed.text.clone(),
    }
    .to_line();
    let _written = writeln!(writer, "{line}");
    let _flushed = writer.flush();

    let engine_name = format!(
        "provisioned {} server @{} from {}",
        llama.component,
        llama.commit.get(..12).unwrap_or(&llama.commit),
        llama.prefix.display()
    );
    let mut conditions = conditions(named, Some((&path, held)), seed, limit);
    if let Value::Map(fields) = &mut conditions {
        fields.insert("engine".to_owned(), Value::text(engine_name));
        fields.insert(
            "loaded".to_owned(),
            Value::text(if reused {
                "resident_in_server"
            } else {
                "loaded_for_this_request"
            }),
        );
        // What MCF sent against what the engine read. They agreeing is the
        // check that the turn arrived as itself (D46); them differing is a
        // finding, and either way it is recorded rather than assumed.
        fields.insert(
            "identifiers_sent".to_owned(),
            Value::Integer(i64::try_from(tokens.len()).unwrap_or(i64::MAX)),
        );
        fields.insert(
            "identifiers_read".to_owned(),
            Value::Integer(i64::try_from(completed.evaluated).unwrap_or(i64::MAX)),
        );
    }

    Ok(Produced {
        account: Value::map([
            (
                "tokens",
                Value::Integer(i64::try_from(completed.predicted).unwrap_or(i64::MAX)),
            ),
            ("stopped", Value::text(completed.stop.written())),
            (
                "text_bytes",
                Value::Integer(i64::try_from(completed.text.len()).unwrap_or(i64::MAX)),
            ),
            ("conditions", conditions),
        ]),
        said: Some(Said {
            text: completed.text,
            tokens: completed.produced,
        }),
    })
}

/// One generation through the provisioned engine, as a supervised subprocess
/// (B-032, B-033). Text arrives in chunks rather than tokens — the completion
/// tool prints text — and each chunk is one line of the stream.
fn through_provisioned(
    store: &Path,
    llama: &crate::adapters::ProvisionedLlama,
    named: &str,
    prompt: &str,
    limit: usize,
    seed: u64,
    writer: &mut &UnixStream,
) -> Result<Produced, Failure> {
    let given = Path::new(named);
    let path = if given.is_file() {
        given.to_path_buf()
    } else {
        store.join(named.replace(':', "/"))
    };
    let metadata = std::fs::metadata(&path).map_err(|error| missing(named, &path, &error))?;
    let held = metadata.len();

    let mut command = llama.generate(&path, prompt, limit, seed);
    let mut at = 0_usize;
    let mut text = String::new();
    let ended = crate::adapters::supervise(&mut command, &mut |chunk| {
        let piece = String::from_utf8_lossy(chunk).into_owned();
        text.push_str(&piece);
        let line = Streamed::Token { at, text: piece }.to_line();
        let _written = writeln!(writer, "{line}");
        let _flushed = writer.flush();
        at = at.saturating_add(1);
    });

    let engine_name = format!(
        "provisioned {} @{} from {}",
        llama.component,
        llama.commit.get(..12).unwrap_or(&llama.commit),
        llama.prefix.display()
    );
    let mut conditions = conditions(named, Some((&path, held)), seed, limit);
    if let Value::Map(fields) = &mut conditions {
        fields.insert("engine".to_owned(), Value::text(engine_name));
        fields.insert("loaded".to_owned(), Value::text("per_request_subprocess"));
    }

    let bytes = Value::Integer(i64::try_from(text.len()).unwrap_or(i64::MAX));
    let said = Some(Said {
        text,
        tokens: Vec::new(),
    });
    match ended {
        Ok(_) => Ok(Produced {
            account: Value::map([
                (
                    "tokens",
                    Value::Integer(i64::try_from(at).unwrap_or(i64::MAX)),
                ),
                // Not "the model stopped": this engine prints text and exits,
                // and why it ended — its own end-of-turn token, or the budget —
                // is not on the wire. A7: what MCF does not know it does not
                // say.
                ("stopped", Value::text("unknown_the_engine_did_not_say")),
                ("text_bytes", bytes),
                ("conditions", conditions),
            ]),
            said,
        }),
        // The engine died. What it produced was produced (A4); the failure is
        // the account, and the daemon is still here (§3.1).
        Err(failure) => Ok(Produced {
            account: Value::map([
                (
                    "tokens",
                    Value::Integer(i64::try_from(at).unwrap_or(i64::MAX)),
                ),
                ("stopped", Value::text("engine_died")),
                ("text_bytes", bytes),
                ("failure", mcf_record::encode::failure(&failure)),
                ("conditions", conditions),
            ]),
            said,
        }),
    }
}

/// The conditions every account carries, whether it succeeded or not.
fn conditions(named: &str, model: Option<(&Path, u64)>, seed: u64, limit: usize) -> Value {
    Value::map([
        ("model", Value::text(named)),
        (
            "path",
            match model {
                Some((path, _)) => Value::text(path.display().to_string()),
                None => Value::Null,
            },
        ),
        (
            "bytes",
            match model {
                Some((_, bytes)) => Value::Integer(i64::try_from(bytes).unwrap_or(i64::MAX)),
                None => Value::Null,
            },
        ),
        (
            "engine",
            Value::text(mcf_core::build_identity::stand_in_engine()),
        ),
        ("loaded", Value::text("not_loaded")),
        ("sampler", Value::text("greedy")),
        (
            "seed",
            Value::Integer(i64::try_from(seed).unwrap_or(i64::MAX)),
        ),
        (
            "limit",
            Value::Integer(i64::try_from(limit).unwrap_or(i64::MAX)),
        ),
    ])
}

#[allow(
    clippy::too_many_lines,
    reason = "one generation is one sequence — resolve, reuse or load, encode, stream, account \
              — and the residency decision sits in the middle of it; a function per half \
              would put the guard that holds the model in one place and what it guards in \
              another"
)]
#[allow(
    clippy::too_many_arguments,
    reason = "one request's conditions, each named in the account"
)]
fn attempt(
    store: &Path,
    resident: &std::sync::Mutex<Option<Resident>>,
    named: &str,
    prompt: &str,
    tokens: Option<&[usize]>,
    limit: usize,
    seed: u64,
    writer: &mut &UnixStream,
) -> Result<Produced, Failure> {
    // A path as given, or a name under the daemon's store — the two ways a
    // model is addressed, and no third.
    let given = Path::new(named);
    let path = if given.is_file() {
        given.to_path_buf()
    } else {
        store.join(named.replace(':', "/"))
    };
    // The file as it is now, so that a resident model whose file has changed
    // underneath is not served as if it were the file on disk.
    let metadata = std::fs::metadata(&path).map_err(|error| missing(named, &path, &error))?;
    let length = metadata.len();
    let modified = metadata.modified().ok();

    let mut held = resident
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let same = held.as_ref().is_some_and(|resident| {
        resident.path == path && resident.length == length && resident.modified == modified
    });
    let loaded = if same {
        "resident"
    } else {
        // Load, and hold: the previous resident, if any, is released here —
        // one model at a time, and which one is what was asked for last.
        let bytes = std::fs::read(&path).map_err(|error| missing(named, &path, &error))?;
        let file = gguf::parse(&bytes)?;
        llama::covers(&file)?;
        let dequantized_bytes = file.dequantized_bytes().unwrap_or(0);
        let vocabulary = Vocabulary::read(&file)?;
        // Every processor the machine reports, spent per product only as far as
        // that product's size earns it (B-366, F99). The daemon holds a model
        // across requests, so this is decided once at load.
        let model = llama::load(&file, &bytes)?
            .across(mcf_standin::threads::Threads::what_the_machine_reports());
        *held = Some(Resident {
            path: path.clone(),
            length,
            modified,
            since: mcf_core::time::Timestamp::now().to_string(),
            vocabulary,
            model,
            dequantized_bytes,
        });
        "loaded"
    };
    let Some(resident) = held.as_ref() else {
        return Err(missing(
            named,
            &path,
            &std::io::Error::other("nothing resident"),
        ));
    };
    let vocabulary = &resident.vocabulary;
    let model = &resident.model;
    let held_bytes = length;
    let since = resident.since.clone();
    let dequantized = resident.dequantized_bytes;

    // Identifiers the caller assembled take precedence over text: a chat turn
    // is built from the model's own markers and re-segmenting its text would
    // not give the same tokens back (D46).
    let prompt_tokens = match tokens {
        Some(tokens) => tokens.to_vec(),
        None => vocabulary.encode(prompt, true)?,
    };
    let build = mcf_core::build_identity::identifier();

    let mut at = 0_usize;
    let generated = session::generate_streaming(
        model,
        &build,
        &Request {
            prompt: prompt_tokens.clone(),
            limit,
            settings: Settings::Greedy,
            seed,
            // The model's own end of text, which the file states and MCF was
            // reading and never using: without it a generation always runs to
            // the budget, and *the model finished* is unobservable — which is
            // what the chat-template probe found first (F37).
            stop: vocabulary.ending.into_iter().collect(),
        },
        &mut |token| {
            // Each token goes out as it exists. A write that fails — the
            // client hung up — is not an error here: the generation finishes
            // and the account is recorded regardless (A26).
            let line = Streamed::Token {
                at,
                text: vocabulary.decode(&[token]),
            }
            .to_line();
            let _written = writeln!(writer, "{line}");
            let _flushed = writer.flush();
            at = at.saturating_add(1);
        },
    )?;

    let degradation = generated.degradation().to_string();
    let produced = generated.value().observed();
    let text = vocabulary.decode(&produced.tokens);
    Ok(Produced {
        account: Value::map([
            (
                "tokens",
                Value::Integer(i64::try_from(produced.tokens.len()).unwrap_or(i64::MAX)),
            ),
            (
                "prompt_tokens",
                Value::Integer(i64::try_from(produced.prompt_length).unwrap_or(i64::MAX)),
            ),
            (
                "stopped",
                Value::text(match produced.stopped {
                    Stopped::AtStopToken { .. } => "stop_token",
                    Stopped::AtLimit => "limit",
                    Stopped::NothingToRead => "nothing_to_read",
                }),
            ),
            (
                "text_bytes",
                Value::Integer(i64::try_from(text.len()).unwrap_or(i64::MAX)),
            ),
            (
                "conditions",
                conditions(named, Some((&path, held_bytes)), seed, limit).with_residency(
                    loaded,
                    &since,
                    dequantized,
                ),
            ),
            ("degraded", Value::text(degradation)),
        ]),
        said: Some(Said {
            text,
            tokens: produced.tokens.clone(),
        }),
    })
}

/// The refusal for a model that is not where it was said to be.
fn missing(named: &str, path: &Path, error: &std::io::Error) -> Failure {
    Failure::new(
        mcf_core::failure::Category::ArtifactMissing,
        mcf_core::failure::Attribution::User,
        mcf_core::failure::Disposition::Refused,
        mcf_core::failure::Subsystem::new("mcf-serve::generation"),
        "there is no model at that path or name",
    )
    .with_context("asked_for", named.to_owned())
    .with_context("looked_at", path.display().to_string())
    .with_context("os_error", error.to_string())
}

/// Residency, stated on every account (D41).
trait WithResidency {
    fn with_residency(self, loaded: &str, since: &str, dequantized: u64) -> Self;
}

impl WithResidency for Value {
    fn with_residency(self, loaded: &str, since: &str, dequantized: u64) -> Self {
        let Value::Map(mut fields) = self else {
            return self;
        };
        fields.insert("loaded".to_owned(), Value::text(loaded));
        fields.insert("resident_since".to_owned(), Value::text(since));
        fields.insert(
            "resident_bytes_dequantized".to_owned(),
            Value::Integer(i64::try_from(dequantized).unwrap_or(i64::MAX)),
        );
        Value::Map(fields)
    }
}
