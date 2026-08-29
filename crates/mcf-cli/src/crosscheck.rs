//! `mcf cross-check <model>` — MCF's engine against the one it provisioned.

use mcf_serve::crosscheck::{self, Agreement};

use crate::Response;
use crate::run::{ambiguous, resolve};

/// How much of a generation to compare.
///
/// Long enough to leave the region where two engines agree by construction —
/// F40 found them parting at step four — and short enough that MCF's own
/// engine, which pays a forward pass per position, answers in a minute rather
/// than an afternoon.
const POSITIONS: usize = 120;

/// What both engines are asked.
const PROMPT: &str = "The history of the city of Paris begins";

/// Compares the two engines on one model and says whether they agree.
pub(crate) fn run(model: &str) -> Response {
    let path = match resolve(model) {
        Ok(Some(path)) => path,
        Ok(None) => {
            return Response {
                text: format!(
                    "mcf: there is no model at {model}\n  `mcf list` says what this machine is \
                     holding; a path to a file works too"
                ),
                served: false,
            };
        }
        Err(found) => {
            return Response {
                text: ambiguous(model, &found),
                served: false,
            };
        }
    };
    let Some(socket) = crate::serve::socket_path() else {
        return Response {
            text: "mcf: there is nowhere a daemon could be listening".to_owned(),
            served: false,
        };
    };
    let Ok(bytes) = std::fs::read(&path) else {
        return Response {
            text: format!("mcf: {} could not be read", path.display()),
            served: false,
        };
    };

    // The other engine generates freely; MCF reads what it produced. Asking
    // both to generate and comparing texts is the thing that does not work
    // (F27, F40).
    let (prompt_tokens, produced) = match generated(&socket, &path, &bytes) {
        Ok(both) => both,
        Err(why) => {
            return Response {
                text: format!(
                    "mcf: the two engines could not be compared on {}\n  {why}",
                    path.display()
                ),
                served: false,
            };
        }
    };

    match crosscheck::against(&bytes, &prompt_tokens, &produced) {
        Err(failure) => Response {
            text: format!(
                "mcf: MCF's own engine could not read {}\n  {failure}",
                path.display()
            ),
            served: false,
        },
        Ok(agreement) => Response {
            text: reported(&path, &agreement),
            served: agreement.within_arithmetic(),
        },
    }
}

/// What the provisioned engine produced, and the prompt it was given.
fn generated(
    socket: &std::path::Path,
    path: &std::path::Path,
    bytes: &[u8],
) -> Result<(Vec<usize>, Vec<usize>), String> {
    use std::io::{BufRead as _, BufReader, Write as _};

    let file = mcf_standin::gguf::parse(bytes).map_err(|failure| failure.to_string())?;
    let vocabulary =
        mcf_standin::tokenizer::Vocabulary::read(&file).map_err(|failure| failure.to_string())?;
    let prompt_tokens = vocabulary
        .encode(PROMPT, true)
        .map_err(|failure| failure.to_string())?;

    let mut connection = std::os::unix::net::UnixStream::connect(socket)
        .map_err(|_| "nothing is listening; `mcf serve` starts a daemon".to_owned())?;
    let _deadline = connection.set_read_timeout(Some(std::time::Duration::from_mins(20)));
    let request = mcf_serve::control::Request::Generate {
        // A cross-check asks MCF's own question of two engines: fixture data
        // (§6.8, B-146).
        whose: mcf_record::content::Whose::Fixture,
        model: path.display().to_string(),
        prompt: String::new(),
        limit: Some(POSITIONS),
        seed: 0,
        tokens: Some(prompt_tokens.clone()),
        engine: Some("provisioned".to_owned()),
    };
    writeln!(connection, "{}", request.to_line())
        .and_then(|()| connection.flush())
        .map_err(|_| "the request could not be sent".to_owned())?;

    let reader = BufReader::new(&connection);
    for line in reader.lines() {
        let line = line.map_err(|_| "the stream ended before its account".to_owned())?;
        match mcf_serve::control::Streamed::read(line.trim_end()) {
            Ok(mcf_serve::control::Streamed::Token { .. }) => {}
            Ok(mcf_serve::control::Streamed::Done(account)) => {
                if let Some(failure) = account.get("failure") {
                    return Err(format!(
                        "the provisioned engine did not generate: {}",
                        failure.to_line()
                    ));
                }
                let produced = match account.get("produced_tokens") {
                    Some(mcf_record::json::Value::List(tokens)) => tokens
                        .iter()
                        .filter_map(mcf_record::json::Value::as_integer)
                        .filter_map(|token| usize::try_from(token).ok())
                        .collect::<Vec<usize>>(),
                    _ => Vec::new(),
                };
                if produced.is_empty() {
                    return Err(
                        "this engine does not hand back the identifiers it produced, \
                                so there is nothing to read with MCF's own (B-362, B-376)"
                            .to_owned(),
                    );
                }
                return Ok((prompt_tokens, produced));
            }
            Err(_) => return Err("a line of the stream was unreadable".to_owned()),
        }
    }
    Err("the stream ended before its account".to_owned())
}

/// The comparison, written so a reader can disagree with the rule as well as
/// the answer.
fn reported(path: &std::path::Path, agreement: &Agreement) -> String {
    let mut lines = vec![
        format!("cross-checked {}", path.display()),
        String::new(),
        format!(
            "  MCF's own engine read {} position(s) of what the provisioned engine produced,",
            agreement.positions
        ),
        format!(
            "  and would have chosen the same token at {}.",
            agreement.agreed
        ),
        String::new(),
    ];
    if agreement.set_aside > 0 {
        lines.push(format!(
            "  {} position(s) set aside: MCF would have ended the turn there, and the other \
             engine was generating freely — the two are answering different questions at those \
             positions rather than disagreeing (F40)",
            agreement.set_aside
        ));
        lines.push(String::new());
    }
    if agreement.within_arithmetic() {
        lines.push(format!(
            "  AGREE    where they differed, the other engine's token was never worse than MCF's \
             rank {} — the line is {}, and a swap of the top few is two implementations \
             summing in a different order rather than one of them being wrong (F27, F41)",
            agreement.furthest,
            crosscheck::FURTHEST_RANK
        ));
    } else {
        lines.push(format!(
            "  DIVERGE  at position {} MCF ranked the other engine's token {}, past the {} that \
             separates arithmetic from a defect. One of these two implementations is wrong and \
             this does not say which — what it says is that the difference is not summation \
             order (A19, F41)",
            agreement.furthest_at,
            agreement.furthest,
            crosscheck::FURTHEST_RANK
        ));
    }
    lines.push(String::new());
    lines.push(
        "  Neither engine is the authority here. What is compared is two readings of one file, \
         and a disagreement is a finding about one of them (§II, A12)."
            .to_owned(),
    );
    lines.push(String::new());
    lines.join("\n")
}
