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

/// Serves one generation, writing the stream, and returns the account that
/// was sent as the terminating line.
pub(crate) fn serve_generation(
    store: &Path,
    named: &str,
    prompt: &str,
    limit: usize,
    seed: u64,
    writer: &mut &UnixStream,
) -> Value {
    let account = match attempt(store, named, prompt, limit, seed, writer) {
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
        ("loaded", Value::text("per_request")),
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

fn attempt(
    store: &Path,
    named: &str,
    prompt: &str,
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
    let bytes = std::fs::read(&path).map_err(|error| {
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
    })?;
    let held = u64::try_from(bytes.len()).unwrap_or(u64::MAX);

    let file = gguf::parse(&bytes)?;
    llama::covers(&file)?;
    let vocabulary = Vocabulary::read(&file)?;
    let model = llama::load(&file, &bytes)?;
    let prompt_tokens = vocabulary.encode(prompt, true)?;
    let build = BuildIdentity::current().version.to_owned();

    let mut at = 0_usize;
    let generated = session::generate_streaming(
        &model,
        &build,
        &Request {
            prompt: prompt_tokens.clone(),
            limit,
            settings: Settings::Greedy,
            seed,
            stop: Vec::new(),
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
            conditions(named, Some((&path, held)), seed, limit),
        ),
        ("degraded", Value::text(degradation)),
    ]))
}
