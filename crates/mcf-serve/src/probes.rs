//! Asking a model to do the thing (B-051, B-052, B-056, D42, §3.18).
//!
//! **The one probe here, and why it is first.** Every instruct model in the
//! corpus carries `tokenizer.chat_template`, and MCF sends raw text to all of
//! them. A model trained to see `<|im_start|>user` and given a bare sentence
//! completes it instead of answering it — which is §3.8's misconfiguration, in
//! the place §X says to look for it.
//!
//! **The signal needs no judgement.** A model addressed the way it was trained
//! emits its own end-of-turn token and stops; addressed raw it runs to the
//! budget. That is mechanically checkable, it is the same observation B-056
//! wants for stop conditions, and it lets the probe be an experiment rather
//! than somebody reading output and forming an impression (A19, F25).
//!
//! **The template is read as a declaration and never executed.** A21: the file
//! says what wrapping it wants, and MCF does not believe it. What MCF does is
//! try each *addressing* its vocabulary can express — the control tokens are
//! in the vocabulary, and which ones are there is a fact — and observe which
//! makes the model stop. The declaration is used only to order the candidates,
//! so that the likeliest is tried first; the answer comes from the model.

use std::path::Path;

use mcf_core::measurement::{ConditionValue, Conditions, Floor};
use mcf_core::probe::{Method, Outcome, Probed};
use mcf_standin::gguf;
use mcf_standin::tokenizer::Vocabulary;

/// One way of putting a question to a model.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Addressing {
    /// What to call it in a result.
    pub name: &'static str,
    /// What goes before the question.
    pub before: String,
    /// What goes after it, up to where the model should begin.
    pub after: String,
}

impl Addressing {
    /// The question, wrapped.
    #[must_use]
    pub fn wrap(&self, question: &str) -> String {
        format!("{}{question}{}", self.before, self.after)
    }
}

/// Whether an addressing's markers survive as the tokens they are.
///
/// **The check exists because the probe was wrong without it.** MCF refuses to
/// parse control tokens out of prompt text — a prompt should not be able to
/// produce a chat marker by spelling one (F26) — so `<|im_start|>` written into
/// a prompt reaches the model as eight ordinary tokens rather than as the
/// marker. A probe that sent that and read the result would be reporting on an
/// addressing it never actually applied, which is the shape of mistake F25
/// warns about: an answer that is an artifact of the instrument.
///
/// So each marker is tokenized and must come back as *one* token spelling
/// itself. Where it does not, the addressing is untestable **as text**, and
/// the probe says so instead of scoring it.
fn markers_survive(vocabulary: &Vocabulary, addressing: &Addressing) -> bool {
    for text in [addressing.before.as_str(), addressing.after.as_str()] {
        for marker in markers_in(text) {
            let Ok(identifiers) = vocabulary.encode(&marker, false) else {
                return false;
            };
            if identifiers.len() != 1 {
                return false;
            }
        }
    }
    true
}

/// The bracketed markers in a piece of addressing, and nothing else.
///
/// `<|im_start|>assistant\n` holds one marker and a word: the word is meant to
/// be ordinary text and the marker is not, so they are checked apart.
fn markers_in(text: &str) -> Vec<String> {
    let mut found = Vec::new();
    let characters: Vec<char> = text.chars().collect();
    let mut at = 0;
    while at < characters.len() {
        let opener = characters.get(at).copied();
        let closer = match opener {
            Some('<') => '>',
            Some('[') => ']',
            _ => {
                at = at.saturating_add(1);
                continue;
            }
        };
        let mut end = at.saturating_add(1);
        while end < characters.len() && characters.get(end).copied() != Some(closer) {
            end = end.saturating_add(1);
        }
        if end < characters.len() {
            found.push(
                characters
                    .get(at..=end)
                    .unwrap_or_default()
                    .iter()
                    .collect(),
            );
            at = end.saturating_add(1);
        } else {
            at = at.saturating_add(1);
        }
    }
    found
}

/// Every addressing this model's own vocabulary can express, likeliest first.
///
/// Read from the vocabulary rather than from a family name: a model that has
/// `<|im_start|>` as a token is a model that can be addressed that way,
/// whoever published it and whatever its architecture says (§3.18, DEC-053).
/// `raw` is always last and always present, because it is what MCF does today
/// and the probe has to be able to say that it is worse.
#[must_use]
pub fn addressings(file: &gguf::Model) -> Vec<Addressing> {
    let has = |token: &str| {
        file.get("tokenizer.ggml.tokens")
            .and_then(gguf::Value::as_list)
            .is_some_and(|tokens| {
                tokens
                    .iter()
                    .filter_map(gguf::Value::as_text)
                    .any(|candidate| candidate == token)
            })
    };
    let mut found = Vec::new();
    if has("<|im_start|>") && has("<|im_end|>") {
        found.push(Addressing {
            name: "chatml",
            before: "<|im_start|>user\n".to_owned(),
            after: "<|im_end|>\n<|im_start|>assistant\n".to_owned(),
        });
    }
    if has("<start_of_turn>") && has("<end_of_turn>") {
        found.push(Addressing {
            name: "turns",
            before: "<start_of_turn>user\n".to_owned(),
            after: "<end_of_turn>\n<start_of_turn>model\n".to_owned(),
        });
    }
    if has("[INST]") && has("[/INST]") {
        found.push(Addressing {
            name: "instruction-tags",
            before: "[INST] ".to_owned(),
            after: " [/INST]".to_owned(),
        });
    }
    found.push(Addressing {
        name: "raw",
        before: String::new(),
        after: String::new(),
    });
    found
}

/// What the chat-template probe observed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Addressed {
    /// The addressing that made the model stop most often.
    pub best: String,
    /// Every candidate, and how many of its trials ended at the model's own
    /// end-of-turn token rather than at the budget.
    pub stopped: Vec<(String, usize)>,
    /// How many trials each candidate had.
    pub of: usize,
    /// What the file *declared*, for the divergence (B-058) — never used to
    /// decide, only to disagree with.
    pub declared_a_template: bool,
}

/// The method, written where the result can carry it.
pub const CHAT_TEMPLATE: Method = Method {
    name: "chat-template",
    asks: "the same question through each addressing the model's vocabulary can express, \
           and counts which ones end at the model's own end-of-turn token rather than at \
           the token budget",
    decides: "how MCF should address this model — and nothing else: a probe writes the \
              verified half of a capability and never a default (D42)",
};

/// Runs the chat-template probe.
///
/// `generate` is how a trial is run: it takes the wrapped question and a token
/// budget, and answers with the identifiers produced and whether generation
/// ended because the model emitted a stop token. Passing it in is what keeps
/// this crate's probe independent of *which* engine ran it — the engine is a
/// condition, and the caller states it.
///
/// # Errors
///
/// Never: a probe that cannot decide reports [`Outcome::Inconclusive`] with the
/// reason, which is D42's third state and not a failure.
#[allow(
    clippy::too_many_lines,
    reason = "one experiment is one sequence — read the model, decide which addressings can be \
              applied at all, run the trials, decide — and the guards that make it honest sit \
              between those steps; splitting it would separate a guard from what it guards"
)]
pub fn chat_template(
    model: &Path,
    bytes: &[u8],
    trials: usize,
    budget: usize,
    engine: &str,
    generate: &mut dyn FnMut(&str, usize) -> Trial,
) -> Probed<Addressed> {
    let conditions = conditions(model, engine);
    let Ok(file) = gguf::parse(bytes) else {
        return Probed::inconclusive(
            CHAT_TEMPLATE,
            "the file could not be read as a model",
            0,
            0,
            conditions,
        );
    };
    let declared_a_template = file.get("tokenizer.chat_template").is_some();
    let Ok(vocabulary) = Vocabulary::read(&file) else {
        return Probed::inconclusive(
            CHAT_TEMPLATE,
            "the vocabulary could not be read",
            0,
            0,
            conditions,
        );
    };
    let all_candidates = addressings(&file);
    let untestable: Vec<&str> = all_candidates
        .iter()
        .filter(|addressing| !markers_survive(&vocabulary, addressing))
        .map(|addressing| addressing.name)
        .collect();
    // A comparison that can only run one side is not a comparison. If the file
    // declares a template and the addressing that would honour it cannot be
    // expressed, saying "raw is best" would be reporting the instrument's
    // limit as the model's behaviour (F25, F37).
    if !untestable.is_empty() {
        return Probed::inconclusive(
            CHAT_TEMPLATE,
            format!(
                "{} cannot be applied: its markers are control tokens, and MCF does not parse \
                 control tokens out of prompt text (F26) — so they would reach the model spelled \
                 out rather than as themselves. Addressing has to be built from token \
                 identifiers before this probe can compare anything (B-374)",
                untestable.join(" and ")
            ),
            0,
            0,
            conditions,
        );
    }
    let candidates = all_candidates;

    let mut stopped: Vec<(String, usize)> = Vec::new();
    let mut spent = 0_usize;
    let mut ran = 0_usize;
    for addressing in &candidates {
        let mut ended = 0_usize;
        for _ in 0..trials {
            match generate(&addressing.wrap(QUESTION), budget) {
                Trial::Stopped => {
                    ran = ran.saturating_add(1);
                    spent = spent.saturating_add(budget);
                    ended = ended.saturating_add(1);
                }
                Trial::RanOut => {
                    ran = ran.saturating_add(1);
                    spent = spent.saturating_add(budget);
                }
                // The reason travels: "could not tell" that says why is the
                // difference between a probe somebody can act on and one that
                // only says no (D42, A2).
                Trial::CouldNotTell(because) => {
                    return Probed::inconclusive(
                        CHAT_TEMPLATE,
                        format!("on the {} addressing: {because}", addressing.name),
                        ran,
                        spent,
                        conditions,
                    );
                }
            }
        }
        stopped.push((addressing.name.to_owned(), ended));
    }

    // The best is the one that stopped most; a tie goes to the earlier
    // candidate, which is the likelier one, and `raw` is last — so a model
    // that stops equally often either way is *not* reported as needing a
    // wrapping it does not need.
    let best = stopped
        .iter()
        .max_by_key(|(_, ended)| *ended)
        .map(|(name, _)| name.clone());
    let all_zero = stopped.iter().all(|(_, ended)| *ended == 0);
    let outcome = match (best, all_zero) {
        // Nothing stopped anywhere: the budget may simply be too small to
        // reach a turn's end. That is *could not tell*, not *no addressing
        // works* (D42, §3.18's third state).
        (_, true) => {
            return Probed::inconclusive(
                CHAT_TEMPLATE,
                format!(
                    "no addressing ended at the model's own stop token within {budget} tokens; \
                     a larger budget may decide it"
                ),
                ran,
                spent,
                conditions,
            );
        }
        (Some(best), false) => Outcome::Observed(Addressed {
            best,
            stopped,
            of: trials,
            declared_a_template,
        }),
        (None, false) => {
            return Probed::inconclusive(
                CHAT_TEMPLATE,
                "no addressing could be built for this vocabulary",
                ran,
                spent,
                conditions,
            );
        }
    };

    Probed {
        method: CHAT_TEMPLATE,
        outcome,
        trials: ran,
        tokens: spent,
        conditions,
    }
}

/// The question every trial asks.
///
/// Short, ordinary, and answerable in a sentence: what is being observed is
/// whether the model *finishes a turn*, so a question that invites an essay
/// would make every addressing run to the budget and tell nothing.
pub const QUESTION: &str = "What is the capital of France?";

/// The conditions a probe result holds under (D42, §3.4).
fn conditions(model: &Path, engine: &str) -> Conditions {
    Conditions::new(
        mcf_core::build_identity::BuildIdentity::current(),
        Floor {
            mcf_configuration: mcf_core::attested::Attested::Known(ConditionValue::text(format!(
                "probe: {}, engine: {engine}, model: {}",
                CHAT_TEMPLATE.name,
                model.display()
            ))),
            ..Floor::nothing_known()
        },
    )
}

/// What one trial did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Trial {
    /// It ended because the model emitted its own stop token.
    Stopped,
    /// It ended because the budget ran out.
    RanOut,
    /// It could not be told apart, and why.
    ///
    /// The third case is not a failure of the model: an engine that prints
    /// text and exits does not say *why* it stopped, and a probe that read
    /// that silence as "it did not stop" would be inventing an observation
    /// (A7, D42).
    CouldNotTell(String),
}

/// One trial through a running daemon: the wrapped question in, the answer
/// out, and what ended it.
#[must_use]
pub fn trial(
    socket: &Path,
    model: &Path,
    prompt: &str,
    budget: usize,
    engine: Option<&str>,
) -> Trial {
    use std::io::{BufRead as _, BufReader, Write as _};

    let Ok(mut connection) = std::os::unix::net::UnixStream::connect(socket) else {
        return Trial::CouldNotTell("nothing is listening on the control socket".to_owned());
    };
    let _deadline = connection.set_read_timeout(Some(std::time::Duration::from_secs(600)));
    let request = crate::control::Request::Generate {
        model: model.display().to_string(),
        prompt: prompt.to_owned(),
        limit: budget,
        seed: 0,
        engine: engine.map(str::to_owned),
    };
    if writeln!(connection, "{}", request.to_line())
        .and_then(|()| connection.flush())
        .is_err()
    {
        return Trial::CouldNotTell("the request could not be sent".to_owned());
    }

    let reader = BufReader::new(&connection);
    for line in reader.lines() {
        let Ok(line) = line else {
            return Trial::CouldNotTell("the stream ended before its account".to_owned());
        };
        match crate::control::Streamed::read(line.trim_end()) {
            Ok(crate::control::Streamed::Token { .. }) => {}
            Ok(crate::control::Streamed::Done(account)) => {
                if let Some(failure) = account.get("failure") {
                    return Trial::CouldNotTell(format!(
                        "the generation did not complete: {}",
                        failure.to_line()
                    ));
                }
                return match account
                    .get("stopped")
                    .and_then(mcf_record::json::Value::as_text)
                {
                    Some("stop_token") => Trial::Stopped,
                    Some("limit") => Trial::RanOut,
                    Some(other) => Trial::CouldNotTell(format!(
                        "this engine does not say why generation ended (it said {other:?}), so \
                         whether the model finished its turn cannot be observed through it"
                    )),
                    None => Trial::CouldNotTell("the account did not say how it ended".to_owned()),
                };
            }
            Err(_) => return Trial::CouldNotTell("a line of the stream was unreadable".to_owned()),
        }
    }
    Trial::CouldNotTell("the stream ended before its account".to_owned())
}

/// Which engine a running daemon would use, for the conditions (D42).
#[must_use]
pub fn describe_engine(socket: &Path) -> Option<String> {
    use std::io::{BufRead as _, BufReader, Write as _};

    let mut connection = std::os::unix::net::UnixStream::connect(socket).ok()?;
    let _deadline = connection.set_read_timeout(Some(std::time::Duration::from_secs(10)));
    writeln!(connection, "{}", crate::control::Request::Status.to_line()).ok()?;
    connection.flush().ok()?;
    let mut line = String::new();
    BufReader::new(&connection).read_line(&mut line).ok()?;
    let answer = crate::control::Answer::read(line.trim_end()).ok()?;
    Some(
        answer
            .body
            .get("build")
            .and_then(|build| build.get("version"))
            .and_then(mcf_record::json::Value::as_text)
            .map_or_else(
                || "a daemon that did not say".to_owned(),
                |version| format!("whatever the daemon at build {version} chooses"),
            ),
    )
}

#[cfg(test)]
mod tests;
