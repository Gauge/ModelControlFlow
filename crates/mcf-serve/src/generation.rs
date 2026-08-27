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

use mcf_core::build_identity::BuildIdentity;
use mcf_core::failure::Failure;
use mcf_record::json::Value;
use mcf_standin::gguf;
use mcf_standin::llama;
use mcf_standin::sample::Settings;
use mcf_standin::session::{self, Request, Stopped};
use mcf_standin::tokenizer::Vocabulary;

use crate::control::Streamed;

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
    named: &str,
    prompt: &str,
    limit: usize,
    seed: u64,
    tokens: Option<&[usize]>,
    engine: Option<&str>,
    writer: &mut &UnixStream,
) -> Value {
    let chosen = choose_engine(mcf_home, engine);
    let account = match chosen {
        Ok(Chosen::Provisioned(llama)) => {
            through_provisioned(store, &llama, named, prompt, limit, seed, writer)
        }
        Ok(Chosen::StandIn) => attempt(store, resident, named, prompt, tokens, limit, seed, writer),
        Err(failure) => Err(failure),
    };
    let account = match account {
        Ok(account) => account,
        Err(failure) => Value::map([
            ("tokens", Value::Integer(0)),
            ("stopped", Value::text("refused")),
            ("failure", mcf_record::encode::failure(&failure)),
            ("conditions", conditions(named, None, seed, limit)),
        ]),
    };
    let _written = writeln!(writer, "{}", Streamed::Done(account.clone()).to_line());
    let _flushed = writer.flush();
    account
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
) -> Result<Value, Failure> {
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
        "provisioned llama.cpp @{} from {}",
        llama.commit.get(..12).unwrap_or(&llama.commit),
        llama.prefix.display()
    );
    let mut conditions = conditions(named, Some((&path, held)), seed, limit);
    if let Value::Map(fields) = &mut conditions {
        fields.insert("engine".to_owned(), Value::text(engine_name));
        fields.insert("loaded".to_owned(), Value::text("per_request_subprocess"));
    }

    match ended {
        Ok(_) => Ok(Value::map([
            (
                "tokens",
                Value::Integer(i64::try_from(at).unwrap_or(i64::MAX)),
            ),
            // Not "the model stopped": this engine prints text and exits, and
            // why it ended — its own end-of-turn token, or the budget — is not
            // on the wire. A7: what MCF does not know it does not say.
            ("stopped", Value::text("unknown_the_engine_did_not_say")),
            ("text", Value::text(text)),
            ("conditions", conditions),
        ])),
        // The engine died. What it produced was produced (A4); the failure is
        // the account, and the daemon is still here (§3.1).
        Err(failure) => Ok(Value::map([
            (
                "tokens",
                Value::Integer(i64::try_from(at).unwrap_or(i64::MAX)),
            ),
            ("stopped", Value::text("engine_died")),
            ("text", Value::text(text)),
            ("failure", mcf_record::encode::failure(&failure)),
            ("conditions", conditions),
        ])),
    }
}

/// The conditions every account carries, whether it succeeded or not.
fn conditions(named: &str, model: Option<(&Path, u64)>, seed: u64, limit: usize) -> Value {
    let identity = BuildIdentity::current();
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
            Value::text(format!("MCF's own stand-in, build {}", identity.version)),
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
) -> Result<Value, Failure> {
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
        let model = llama::load(&file, &bytes)?;
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
    let build = BuildIdentity::current().version.to_owned();

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
    Ok(Value::map([
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
        ("text", Value::text(vocabulary.decode(&produced.tokens))),
        (
            "conditions",
            conditions(named, Some((&path, held_bytes)), seed, limit).with_residency(
                loaded,
                &since,
                dequantized,
            ),
        ),
        ("degraded", Value::text(degradation)),
    ]))
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
