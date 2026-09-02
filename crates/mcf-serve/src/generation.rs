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
    picked: Option<(crate::adapters::ProvisionedLlama, u32, u64)>,
    // What the machine says is free, read by the daemon. Handed in rather
    // than taken here for the same reason the engine is: B4 keeps hardware
    // sampling out of the serving path, so this is a number the caller
    // observed and not one this function goes and takes.
    free: Option<u64>,
    // Whether the limit is the length: a timing's request, which the engine
    // is told to run to and the account counts (B-396).
    pinned: bool,
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

    let (chosen, gpu_layers, context) = match (picked, engine) {
        // A caller that asked for MCF's own engine gets it, whatever was
        // resolved: naming the engine is the point of the argument (§3.15).
        (_, Some("stand-in")) => (Ok(Chosen::StandIn), 0, 0),
        (Some((llama, layers, window)), _) => (Ok(Chosen::Provisioned(llama)), layers, window),
        (None, asked) => (choose_engine(mcf_home, asked), 0, 0),
    };
    let produced = match chosen {
        // A turn of identifiers goes to the server, which can be given one;
        // a prompt goes to the completion tool, which cannot (B-376).
        Ok(Chosen::Provisioned(llama)) => match tokens {
            Some(tokens) => through_served(
                store, &llama, server, runtime, named, tokens, limit, seed, gpu_layers, context,
                pinned, writer,
            ),
            None => through_provisioned(store, &llama, named, prompt, limit, seed, pinned, writer),
        },
        Ok(Chosen::StandIn) => attempt(
            store, resident, named, prompt, tokens, limit, seed, free, pinned, writer,
        ),
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
    // **The header, not the model.** This read the whole file to reach the
    // vocabulary, and it sits in the serving path: every generation of a model
    // somebody has given an addressing paid it. A vocabulary is metadata and
    // arrives in the same bounded prefix everything else reads (F145, B-372).
    let file = crate::daemon::header_of(&path)?;
    let vocabulary = Vocabulary::read(&file).ok()?;
    let mut pieces = addressing.before.clone();
    pieces.push(mcf_standin::tokenizer::Piece::Text(prompt.to_owned()));
    pieces.extend(addressing.after.iter().cloned());
    vocabulary.addressed(&pieces)
}

/// A prompt as the model receives it, and where its own tokens begin.
///
/// The derived addressing around the text where somebody has put one on file,
/// the bare text otherwise — the same choice a generation makes, so that a
/// reading of the prompt is a reading of the prompt the answer was given
/// (§3.4). `after` is left off where the caller wants what *follows* the
/// prompt to be open, which is what ranking the prompt's own tokens needs.
pub(crate) struct Received {
    /// The identifiers.
    pub(crate) tokens: Vec<usize>,
    /// How many of them stand before the prompt's own: the beginning marker
    /// and the addressing's opening pieces.
    pub(crate) before: usize,
    /// What the prompt was addressed as, in words, for the report.
    pub(crate) under: String,
}

/// A prompt as the model receives it.
///
/// `None` where the vocabulary cannot be read or the text cannot be encoded.
pub(crate) fn received(
    store: &Path,
    mcf_home: &Path,
    named: &str,
    prompt: &str,
    with_after: bool,
) -> Option<Received> {
    let path = resolved(store, named);
    let file = crate::daemon::header_of(&path)?;
    let vocabulary = Vocabulary::read(&file).ok()?;
    let derived = crate::configured::read_derived(mcf_home, &path).addressing;
    let Some(addressing) = derived else {
        return Some(Received {
            tokens: vocabulary.encode(prompt, true).ok()?,
            before: 1,
            under: "the prompt alone: no addressing is on file for this model, so it went                     with no turn markers around it"
                .to_owned(),
        });
    };
    // Pieces encode independently (a marker is looked up, text is segmented),
    // so the opening on its own is a prefix of the whole turn.
    let opening = vocabulary.addressed(&addressing.before)?;
    let mut pieces = addressing.before.clone();
    pieces.push(mcf_standin::tokenizer::Piece::Text(prompt.to_owned()));
    if with_after {
        pieces.extend(addressing.after.iter().cloned());
    }
    let tokens = vocabulary.addressed(&pieces)?;
    if !tokens.starts_with(&opening) {
        return None;
    }
    Some(Received {
        tokens,
        before: opening.len(),
        under: addressing.provenance(),
    })
}

/// One generation through the provisioned engine driven as a *server*
/// (B-376), which is the shape that can be probed: the turn goes as
/// identifiers and the engine says why it stopped.
///
/// The smallest window a served request opens.
///
/// A window has to hold the turn and leave the engine room to work. Named here
/// because a floor chosen inside an expression is a decision nobody can find.
const SMALLEST_WINDOW: u64 = 4096;

/// Where the model put one token, and what it said the token was worth.
///
/// The engine's figure is the text it sent: the record has no floating-point
/// variant on purpose, and a log probability is exactly the sort of number
/// that grows a division somewhere else.
pub(crate) type Ranked = (Option<usize>, Option<String>);

/// Everything about *which* model, and where, that ranking a prompt needs.
///
/// Gathered into one because a function taking eight of them is a function
/// nobody can call correctly by position.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Where<'a> {
    /// The model store.
    pub store: &'a Path,
    /// The build to run.
    pub llama: &'a crate::adapters::ProvisionedLlama,
    /// Where a server may put its socket.
    pub runtime: &'a Path,
    /// The model, as it was named.
    pub named: &'a str,
    /// How many layers go on a card.
    pub gpu_layers: u32,
    /// The largest window MCF resolved for this model.
    pub context: u64,
}

/// How many positions of a prompt are ranked before MCF stops.
///
/// Each one is a request, so a long prompt is a long wait; a hundred and
/// twenty covers an ordinary instruction and the report says when it stopped.
pub(crate) const MOST_RANKED: usize = 120;

/// How deep the distribution is read at each position.
///
/// A token outside this is reported as outside it rather than given a rank it
/// does not have (A7). Sixty is deep enough that an ordinary word is found and
/// shallow enough that the answers stay small.
pub(crate) const HOW_DEEP: usize = 60;

/// Where the model ranked each token of a prompt, given the ones before it.
///
/// **A second reading of a prompt that does not depend on comparing answers.**
/// The ablation measures what changes when a sentence is removed, and under
/// greedy decoding removing anything shifts everything after it — which is why
/// it reports an ordering and not a measure. This asks a different question of
/// the same prompt: at each position, was the token one the model would have
/// written anyway? A token it ranked first carried no information from the
/// writer; one it ranked low, or did not list at all, is where the prompt said
/// something the model did not expect.
///
/// Returns one entry per position from `from` on, at most `most` of them —
/// `from` is never nought, since the first token has nothing before it to be
/// predicted from — each the rank counting from one and the engine's own
/// figure as the text it sent. The same reading serves two questions: the
/// prompt's own tokens after their addressing, and an answer's opening after
/// a prompt that is not the one it was written to (B-429).
///
/// # Errors
///
/// Whatever starting or asking the server reported.
pub(crate) fn ranks_over(
    where_it_lives: &Where<'_>,
    server: &std::sync::Mutex<Option<Served>>,
    tokens: &[usize],
    from: usize,
    most: usize,
) -> Result<Vec<Ranked>, Failure> {
    let Where {
        store,
        llama,
        runtime,
        named,
        gpu_layers,
        context,
    } = *where_it_lives;
    let path = resolved(store, named);
    let mut slot = server.lock().map_err(|_poisoned| {
        unavailable("the served engine's slot was left poisoned by an earlier failure")
    })?;
    let reused = slot.as_ref().is_some_and(|held| held.model == path);
    let asked_for = u64::try_from(tokens.len()).unwrap_or(SMALLEST_WINDOW);
    let needed = asked_for.saturating_mul(2).max(SMALLEST_WINDOW);
    let window = if context == 0 {
        needed
    } else {
        needed.min(context)
    };
    if !reused {
        *slot = None;
        *slot = Some(Served::start(llama, &path, runtime, gpu_layers, window)?);
    }
    let engine = slot
        .as_ref()
        .ok_or_else(|| unavailable("the served engine was started and then was not there"))?;

    let mut ranked = Vec::new();
    let from = from.max(1);
    for at in from..tokens.len().min(from.saturating_add(most)) {
        let (Some(prefix), Some(wanted)) = (tokens.get(..at), tokens.get(at).copied()) else {
            break;
        };
        ranked.push(engine.ranked_next(prefix, wanted, HOW_DEEP)?);
    }
    Ok(ranked)
}

/// **The window the REQUEST needs, not the largest one that fits.**
///
/// `context` arrives as what MCF resolved for this model on this machine,
/// which is the largest window it could hold — the right answer for a model
/// somebody is hosting and will send long prompts to, and the wrong one
/// here. A request of a few hundred tokens opened a 262,144-token window
/// because that is what fits: 57 GiB resident for a 17.6 GiB model, and the
/// allocation dominating the very timing being taken.
///
/// So the window is sized to this turn — what was sent plus what was asked
/// for, doubled for room to work — with a floor, and never more than the
/// machine was said to hold. A measurement then carries the window it
/// actually ran in (§3.4).
fn window_for(sent: usize, limit: usize, context: u64) -> u64 {
    let asked_for = u64::try_from(sent.saturating_add(limit)).unwrap_or(SMALLEST_WINDOW);
    let needed = asked_for.saturating_mul(2).max(SMALLEST_WINDOW);
    if context == 0 {
        needed
    } else {
        needed.min(context)
    }
}

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
    context: u64,
    pinned: bool,
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
        unavailable("the served engine's slot was left poisoned by an earlier failure")
    })?;
    // A server holding a different model is stopped rather than kept beside
    // this one: two resident models is a decision about memory nobody has
    // taken (D41, DEC-018), and taking it here silently would be the hidden
    // choice §3.15 forbids.
    let window = window_for(tokens.len(), limit, context);
    // **A server whose window is too small for this turn is not this
    // turn's server.** Reuse went by the model alone, so a ladder's first
    // rung opened a 4,096-token window and every rung past it was sent to
    // that server, which refuses a turn longer than its window — and the
    // refusal was read as a pair of runs that did not separate. No rung
    // deeper than 2,048 was ever measured, on any model, and the record
    // said *not measured* for the wrong reason (F152).
    let reused = slot
        .as_ref()
        .is_some_and(|held| held.model == path && held.window >= window);
    if !reused {
        *slot = None;
        *slot = Some(Served::start(llama, &path, runtime, gpu_layers, window)?);
    }
    let engine = slot
        .as_ref()
        .ok_or_else(|| unavailable("the served engine was started and then was not there"))?;

    let completed = engine.complete(tokens, limit, seed, pinned)?;
    // Read after the turn, while the mark includes it (B-424).
    let peak_resident = engine.peak_resident_bytes();
    let ran_in = engine.window;

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
            "length".to_owned(),
            Value::text(Length::of(pinned).as_str()),
        );
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
        fields.insert(
            "window".to_owned(),
            Value::Integer(i64::try_from(ran_in).unwrap_or(i64::MAX)),
        );
        // The engine's peak resident memory, or nothing where the kernel did
        // not say — never zero (A7).
        fields.insert(
            "peak_resident_bytes".to_owned(),
            peak_resident.map_or(Value::Null, |bytes| {
                Value::Integer(i64::try_from(bytes).unwrap_or(i64::MAX))
            }),
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
/// What the completion tool prints after the text when the model emitted its
/// own end-of-turn token.
///
/// **It is the tool talking, not the model.** llama.cpp's completion binary
/// writes this on its own standard output, in the same stream as the answer
/// and with nothing to separate them, and MCF passed the whole stream on as
/// what the model said. A model that ended `return result` had
/// `return result [end of text]` recorded against it — code that no longer
/// parses, in a laboratory whose whole business is running what a model wrote
/// (F142).
const ENDED_ITS_TURN: &str = "[end of text]";

/// How much of the end is held back before anything is sent.
///
/// Comfortably more than the marker and the blank lines the tool prints after
/// it. The first attempt kept back only a suffix that was a prefix of the
/// marker, which looks sufficient and is not: the tool writes `[end of text]`
/// and *then* two newlines, so by the end of the read the marker is no longer
/// at the end of the buffer and the test that was meant to catch it saw a
/// suffix of `\n\n` instead. Holding a fixed tail needs no reasoning about
/// what follows what.
const HELD_BACK: usize = 32;

/// The part of `held` that is certainly the model's, removed from it.
///
/// The marker arrives split across reads like any other output and with more
/// output after it, so the last stretch is never sent until the generation
/// ends. What that costs is the tail arriving in one piece at the end; what it
/// buys is that nothing the tool wrote about the model is shown as the model's
/// (F142).
fn ready_to_send(held: &mut String) -> String {
    let Some(mut at) = held.len().checked_sub(HELD_BACK) else {
        return String::new();
    };
    while at > 0 && !held.is_char_boundary(at) {
        at -= 1;
    }
    held.drain(..at).collect()
}

/// What the model actually said, and why it stopped.
///
/// A7 asks MCF not to claim what it does not know. It does not ask MCF to
/// throw away what it does: the marker is the engine saying *the model ended
/// its own turn*, and this path reported `unknown_the_engine_did_not_say`
/// while holding it (F142).
fn without_the_marker(said: &str) -> (String, &'static str) {
    let trimmed = said.trim_end();
    match trimmed.strip_suffix(ENDED_ITS_TURN) {
        Some(before) => (before.trim_end().to_owned(), "stop_token"),
        // No marker: the tool prints none when the budget ran out first, and
        // it is the budget MCF set — but this build does not say so on the
        // wire, and inferring it from a token count MCF also set would be a
        // guess wearing a measurement's clothes.
        None => (said.to_owned(), "unknown_the_engine_did_not_say"),
    }
}

#[allow(
    clippy::too_many_arguments,
    reason = "one request's conditions, each named in the account"
)]
fn through_provisioned(
    store: &Path,
    llama: &crate::adapters::ProvisionedLlama,
    named: &str,
    prompt: &str,
    limit: usize,
    seed: u64,
    pinned: bool,
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

    let mut command = llama.generate(&path, prompt, limit, seed, pinned);
    let mut at = 0_usize;
    let mut text = String::new();
    // Held back until it is known not to be the start of the marker, so that
    // what a caller watching the stream sees is what the model wrote.
    let mut waiting = String::new();
    let mut sent = 0_usize;
    let ended = crate::adapters::supervise(&mut command, &mut |chunk| {
        let piece = String::from_utf8_lossy(chunk).into_owned();
        text.push_str(&piece);
        waiting.push_str(&piece);
        let ready = ready_to_send(&mut waiting);
        if ready.is_empty() {
            return;
        }
        sent = sent.saturating_add(ready.len());
        let line = Streamed::Token { at, text: ready }.to_line();
        let _written = writeln!(writer, "{line}");
        let _flushed = writer.flush();
        at = at.saturating_add(1);
    });
    // Whatever was held back and turned out not to be the marker.
    let (text, why) = without_the_marker(&text);
    if let Some(rest) = text.get(sent..)
        && !rest.is_empty()
    {
        let line = Streamed::Token {
            at,
            text: rest.to_owned(),
        }
        .to_line();
        let _written = writeln!(writer, "{line}");
        let _flushed = writer.flush();
        at = at.saturating_add(1);
    }

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
        // The tool was told, and it does not count: `tokens` below is a count
        // of chunks it printed, so a pin through this path is asked and not
        // proven, and the account says which (A21).
        fields.insert(
            "length".to_owned(),
            Value::text(if pinned {
                Length::ExactlyButUncounted.as_str()
            } else {
                Length::AtMost.as_str()
            }),
        );
        fields.insert(
            "peak_resident_bytes".to_owned(),
            ended
                .as_ref()
                .ok()
                .and_then(|ended| ended.peak_resident)
                .map_or(Value::Null, |bytes| {
                    Value::Integer(i64::try_from(bytes).unwrap_or(i64::MAX))
                }),
        );
    }

    let mut account = vec![
        (
            "tokens",
            Value::Integer(i64::try_from(at).unwrap_or(i64::MAX)),
        ),
        // Its own end-of-turn token where the tool said so, and *unknown*
        // where it did not — which is A7 keeping what MCF knows and what it
        // does not apart, rather than discarding both (F142). And where the
        // engine died: what it produced was produced (A4), the failure is
        // the account, and the daemon is still here (§3.1).
        (
            "stopped",
            Value::text(if ended.is_ok() { why } else { "engine_died" }),
        ),
        (
            "text_bytes",
            Value::Integer(i64::try_from(text.len()).unwrap_or(i64::MAX)),
        ),
    ];
    if let Err(failure) = &ended {
        account.push(("failure", mcf_record::encode::failure(failure)));
    }
    account.push(("conditions", conditions));
    Ok(Produced {
        account: Value::map(account),
        said: Some(Said {
            text,
            tokens: Vec::new(),
        }),
    })
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
    free: Option<u64>,
    pinned: bool,
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
        // **The refusal the console already made, made here too.** `mcf run`
        // with no daemon weighs a model against free memory before
        // dequantizing it; through the daemon it did not, so the surface that
        // is meant to be able to do everything was the one that could not
        // refuse (A22, B-072). What this branch does next is read the whole
        // file and allocate several times its size, and a machine that cannot
        // afford that does not get told: the kernel ends some other process
        // instead, which is a failure MCF caused and did not report (A2,
        // F136).
        //
        // Weighed from the header, which is a few megabytes, because a check
        // that had to read the file first would already have spent what it is
        // trying to refuse. The same arithmetic the console uses, so that the
        // two surfaces agree about which models this machine can run.
        // Everything that can be decided from the header is decided from the
        // header, and decided *before* the resident model is let go: an
        // architecture MCF was never taught and a model this machine cannot
        // hold are both knowable from a few megabytes, and refusing on either
        // after evicting what was loaded would charge the operator a reload
        // for a question that was answerable without one.
        if let Some(header) = crate::daemon::header_of(&path) {
            llama::covers(&header)?;
            if let Some(available) = free {
                header.fits_dequantized(available)?;
            }
        }
        // **Released before the next is read, not after.** The line below has
        // always said the previous resident is released here, and it was not:
        // the new model was assigned at the end of the load, so the outgoing
        // one stayed resident for the whole of it and the peak was two models
        // and a file, on a path whose entire purpose is to hold one model at a
        // time. Dropping first costs a reload when what follows refuses, which
        // is much the cheaper of the two mistakes (F136).
        *held = None;
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
            // what the chat-template probe found first (F37). Unless the
            // length is pinned, in which case running to the budget is the
            // point (B-396).
            stop: if pinned {
                Vec::new()
            } else {
                vocabulary.ending.into_iter().collect()
            },
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
                conditions(named, Some((&path, held_bytes)), seed, limit)
                    .with_residency(loaded, &since, dequantized)
                    .with_length(pinned),
            ),
            ("degraded", Value::text(degradation)),
        ]),
        said: Some(Said {
            text,
            tokens: produced.tokens.clone(),
        }),
    })
}

/// The served engine is not there to ask, and this is why.
fn unavailable(why: &'static str) -> Failure {
    Failure::new(
        mcf_core::failure::Category::EngineUnavailable,
        mcf_core::failure::Attribution::Machine,
        mcf_core::failure::Disposition::Aborted,
        mcf_core::failure::Subsystem::new("mcf-serve::generation"),
        why,
    )
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

/// What the request's limit was: a ceiling, or the length.
///
/// Said in every account under `conditions.length`, so that a reader dividing
/// a duration by the count knows whether the count was the one asked for —
/// and, for a pin that went through a path that cannot count, that it was
/// asked and not proven (B-396, A21).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Length {
    /// The limit was a ceiling; the model's end of text ends the turn.
    AtMost,
    /// The limit was the length; the engine ran to it and counted.
    Exactly,
    /// The limit was the length and the engine was told so, but this path
    /// hands back text rather than a token count, so the pin is not proven.
    ExactlyButUncounted,
}

impl Length {
    /// The pin's word for a path that counts.
    pub(crate) const fn of(pinned: bool) -> Self {
        if pinned { Self::Exactly } else { Self::AtMost }
    }

    /// The word on the wire.
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::AtMost => "at_most",
            Self::Exactly => "exactly",
            Self::ExactlyButUncounted => "exactly_but_uncounted",
        }
    }
}

/// Residency, stated on every account (D41).
trait WithResidency {
    fn with_length(self, pinned: bool) -> Self;
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

    fn with_length(self, pinned: bool) -> Self {
        let Value::Map(mut fields) = self else {
            return self;
        };
        fields.insert(
            "length".to_owned(),
            Value::text(Length::of(pinned).as_str()),
        );
        Value::Map(fields)
    }
}

#[cfg(test)]
mod marker_tests {
    // A test says what went wrong by failing.
    #![allow(clippy::panic, clippy::expect_used)]

    use super::{ENDED_ITS_TURN, HELD_BACK, ready_to_send, without_the_marker};

    /// The defect exactly: code the tool made unparseable.
    #[test]
    fn the_tools_marker_is_not_the_models_words() {
        let said = "def merge(a, b):\n    return sorted(a + b) [end of text]\n\n\n";
        let (text, why) = without_the_marker(said);
        assert_eq!(text, "def merge(a, b):\n    return sorted(a + b)");
        assert_eq!(why, "stop_token", "the tool said why and MCF has it");
        assert!(
            !text.contains(ENDED_ITS_TURN),
            "the marker is still being attributed to the model"
        );
    }

    /// Without the marker, MCF does not know why it stopped and says so.
    ///
    /// A7 in both directions: the reason is kept where the tool gave one and
    /// not invented where it did not.
    #[test]
    fn no_marker_is_not_a_stop_token() {
        let (text, why) = without_the_marker("a partial answer that ran out of budget");
        assert_eq!(text, "a partial answer that ran out of budget");
        assert_eq!(why, "unknown_the_engine_did_not_say");
    }

    /// A model that writes the marker's text itself, mid-answer, keeps it.
    #[test]
    fn only_the_end_is_the_end() {
        let said = "print('[end of text]')\nreturn 1";
        let (text, why) = without_the_marker(said);
        assert_eq!(text, said, "a marker in the middle is the model's own text");
        assert_eq!(why, "unknown_the_engine_did_not_say");
    }

    /// Nothing is sent while it could still be part of the marker.
    ///
    /// The first attempt held back only a suffix that was a prefix of the
    /// marker, and the tool writes newlines *after* the marker — so by the end
    /// of the read the marker sat in the middle of the buffer and went
    /// straight out. This is that case.
    #[test]
    fn the_marker_is_never_streamed_even_with_output_after_it() {
        let mut held = String::new();
        let mut sent = String::new();
        // One read carrying the end of an answer, the marker, and the blank
        // lines that follow it — which is what the tool actually writes.
        held.push_str("    return result [end of text]\n\n\n");
        sent.push_str(&ready_to_send(&mut held));
        assert!(
            !sent.contains(ENDED_ITS_TURN),
            "the marker reached the caller: {sent:?}"
        );
    }

    /// A long answer still streams rather than arriving all at once.
    #[test]
    fn what_is_certainly_the_models_goes_out_as_it_arrives() {
        let mut held = "x".repeat(HELD_BACK * 4);
        let ready = ready_to_send(&mut held);
        assert_eq!(ready.len(), HELD_BACK * 3);
        assert_eq!(held.len(), HELD_BACK, "the tail is what is kept back");
    }

    /// The holdback never splits a character.
    #[test]
    fn a_character_is_not_cut_in_half() {
        let mut held = "é".repeat(HELD_BACK);
        let ready = ready_to_send(&mut held);
        // Valid UTF-8 on both sides is the property; where the cut lands is
        // arithmetic.
        assert!(ready.chars().all(|held| held == 'é'));
        assert!(held.chars().all(|c| c == 'é'));
        assert_eq!(ready.len() + held.len(), HELD_BACK * 2);
    }
}
