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
use mcf_standin::tokenizer::{Piece, Vocabulary};

/// Role words a template *assigns*, in the order it assigns them.
///
/// Lexical and deliberately narrow: the text after `set <name> =` up to the
/// closing quote, for a single- or double-quoted literal. It recognises the
/// one shape that matters — a template deciding what word to write — and
/// recognises nothing else, which is the honest extent of reading a program
/// without running it. Where it finds nothing the caller falls back to the
/// words the template mentions.
fn assigned_roles(template: &str) -> Vec<String> {
    let mut found: Vec<String> = Vec::new();
    for after in template.split("set ").skip(1) {
        let Some((name, rest)) = after.split_once('=') else {
            continue;
        };
        if !name.trim().eq_ignore_ascii_case("role") {
            continue;
        }
        let rest = rest.trim_start();
        let Some(quote) = rest
            .chars()
            .next()
            .filter(|mark| *mark == '"' || *mark == '\'')
        else {
            continue;
        };
        let Some((literal, _)) = rest[quote.len_utf8()..].split_once(quote) else {
            continue;
        };
        if !literal.is_empty() && !found.iter().any(|held| held == literal) {
            found.push(literal.to_owned());
        }
    }
    found
}

/// One way of putting a question to a model.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Addressing {
    /// What to call it in a result.
    pub name: String,
    /// What goes before the question.
    pub pieces_before: Vec<Piece>,
    /// What goes after it, up to where the model should begin.
    pub pieces_after: Vec<Piece>,
}

impl Addressing {
    /// The question, wrapped, as identifiers.
    ///
    /// `None` where a marker is not a token of this vocabulary — which cannot
    /// happen for a candidate that was built from it, and is checked anyway
    /// because the alternative is sending something else and calling it this
    /// (F37).
    #[must_use]
    pub fn wrap(&self, vocabulary: &Vocabulary, question: &str) -> Option<Vec<usize>> {
        let mut pieces = self.pieces_before.clone();
        pieces.push(Piece::Text(question.to_owned()));
        pieces.extend(self.pieces_after.iter().cloned());
        vocabulary.addressed(&pieces)
    }

    /// The turn as text, for a reader — never for sending.
    #[must_use]
    pub fn shown(&self, question: &str) -> String {
        let show = |pieces: &[Piece]| -> String {
            pieces
                .iter()
                .map(|piece| match piece {
                    Piece::Marker(marker) => marker.clone(),
                    Piece::Text(text) => text.clone(),
                })
                .collect()
        };
        format!(
            "{}{question}{}",
            show(&self.pieces_before),
            show(&self.pieces_after)
        )
    }
}

pub mod declined;
pub mod embedding;
pub mod language;
pub mod structured;
pub mod thinking;
pub mod tools;
pub mod vision;

/// The bracketed markers in a piece of text, in order of first appearance.
///
/// Used on a template, where they appear as string literals among the logic:
/// finding them is reading, and none of the logic around them is evaluated
/// (D46).
pub(crate) fn markers_in(text: &str) -> Vec<String> {
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

/// Every addressing this model can be given, likeliest first (D46).
///
/// **The candidates come from the model's own file.** Its template is text:
/// the marker strings it emits and the order it emits them in are readable
/// without evaluating one conditional, which is reading a declaration rather
/// than running a program (§3.18, §3.7). A built-in list of known shapes is a
/// *fallback* only — for a file with no template, or one nothing could be read
/// from — so a family nobody has seen yet needs no change here as long as it
/// ships a template naming its markers and a vocabulary holding them.
///
/// `raw` is always last and always present, because it is what MCF does today
/// and the probe has to be able to say that it is better.
#[must_use]
pub fn addressings(file: &gguf::Model, vocabulary: &Vocabulary) -> Vec<Addressing> {
    let mut found = from_template(file, vocabulary);
    if found.is_empty() {
        found = known_shapes(vocabulary);
    }
    found.push(Addressing {
        name: "raw".to_owned(),
        pieces_before: Vec::new(),
        pieces_after: Vec::new(),
    });
    found
}

/// What the file's own template says, read as data.
fn from_template(file: &gguf::Model, vocabulary: &Vocabulary) -> Vec<Addressing> {
    let Some(template) = file
        .get("tokenizer.chat_template")
        .and_then(gguf::Value::as_text)
    else {
        return Vec::new();
    };

    // Markers the template mentions that are really tokens of this model.
    // Both halves matter: a template naming a marker the vocabulary lacks is a
    // divergence (Llama-160M's names ChatML markers it does not have), and a
    // spelling that segments into several tokens cannot be sent as a marker.
    let mut markers: Vec<String> = Vec::new();
    for marker in markers_in(template) {
        if vocabulary.has_token(&marker) && !markers.contains(&marker) {
            markers.push(marker);
        }
    }
    // The closer is the model's *own* end-of-turn token, which the file states
    // outright — not "the second marker the template mentions", which picked
    // `<start_of_image>` out of gemma's template and would have addressed it
    // with a marker for pictures (F38).
    let Some(close) = vocabulary
        .ending
        .and_then(|ending| vocabulary.token(ending))
        .map(str::to_owned)
        .filter(|ending| markers.contains(ending))
    else {
        return Vec::new();
    };
    let Some(open) = markers.iter().find(|marker| **marker != close).cloned() else {
        return Vec::new();
    };
    let (open, close) = (&open, &close);

    // Which word names the answering side.
    //
    // Reading the template for the word it *emits*, not for the words it
    // mentions. Mentioning is not meaning: gemma's template names `assistant`
    // exactly once and does it to rename it —
    //
    //     {%- if (message['role'] == 'assistant') -%}
    //     {%- set role = "model" -%}
    //
    // — so a bag-of-words read produced a candidate the template explicitly
    // rejects, and then the probe could not tell the two apart because
    // *ending a turn* does not (F48, B-375). A word assigned to the role is
    // what gets written out; a word compared against is an input name being
    // translated away.
    //
    // This reads the template's shape and does not execute it: a template is
    // a program in somebody else's language, and running one is a door §3.7
    // keeps shut.
    let assigned = assigned_roles(template);
    let mut roles: Vec<String> = if assigned.is_empty() {
        // Nothing assigned: the template emits the role it was given, so the
        // ordinary names are the candidates and the model decides between them.
        ["assistant", "model"]
            .into_iter()
            .filter(|role| template.contains(role))
            .map(str::to_owned)
            .collect()
    } else {
        assigned
    };
    if roles.is_empty() {
        roles.push("assistant".to_owned());
    }

    roles
        .into_iter()
        .map(|role: String| Addressing {
            name: format!("{}…{} as {role}", trim(open), trim(close)),
            pieces_before: vec![
                Piece::Marker(open.clone()),
                Piece::Text("user\n".to_owned()),
            ],
            pieces_after: vec![
                Piece::Marker(close.clone()),
                Piece::Text("\n".to_owned()),
                Piece::Marker(open.clone()),
                Piece::Text(format!("{role}\n")),
            ],
        })
        .collect()
}

/// The shapes MCF knows without being told, for a file that says nothing.
fn known_shapes(vocabulary: &Vocabulary) -> Vec<Addressing> {
    let turn = |open: &str, close: &str, role: &str| Addressing {
        name: format!("{}…{} as {role}", trim(open), trim(close)),
        pieces_before: vec![
            Piece::Marker(open.to_owned()),
            Piece::Text("user\n".to_owned()),
        ],
        pieces_after: vec![
            Piece::Marker(close.to_owned()),
            Piece::Text("\n".to_owned()),
            Piece::Marker(open.to_owned()),
            Piece::Text(format!("{role}\n")),
        ],
    };
    let mut found = Vec::new();
    for (open, close, role) in [
        ("<|im_start|>", "<|im_end|>", "assistant"),
        ("<start_of_turn>", "<end_of_turn>", "model"),
    ] {
        if vocabulary.has_token(open) && vocabulary.has_token(close) {
            found.push(turn(open, close, role));
        }
    }
    if vocabulary.has_token("[INST]") && vocabulary.has_token("[/INST]") {
        found.push(Addressing {
            name: "[INST]…[/INST]".to_owned(),
            pieces_before: vec![
                Piece::Marker("[INST]".to_owned()),
                Piece::Text(" ".to_owned()),
            ],
            pieces_after: vec![
                Piece::Text(" ".to_owned()),
                Piece::Marker("[/INST]".to_owned()),
            ],
        });
    }
    found
}

/// A marker without its brackets, for a name a person reads.
fn trim(marker: &str) -> &str {
    marker
        .trim_start_matches(['<', '|', '['])
        .trim_end_matches(['>', '|', ']'])
}

/// What the chat-template probe observed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Addressed {
    /// The addressing under which the model most often answered and *then*
    /// ended its turn.
    pub best: String,
    /// Every candidate, and how many of its trials ended at the model's own
    /// end-of-turn token having first said something.
    pub stopped: Vec<(String, usize)>,
    /// Every candidate, and how many of its trials ended at that same token
    /// having said *nothing at all*.
    ///
    /// Kept rather than folded into the failures because it is a different
    /// fact: the model recognised the stop token and declined the turn. A1 —
    /// and the reason F38 was found at all.
    pub silent: Vec<(String, usize)>,
    /// Every candidate, and how long each of its finished turns ran.
    ///
    /// The lengths are already observed — a trial cannot tell a finished turn
    /// from a refusal without them (F38) — so throwing them away would be
    /// discarding a measurement MCF already paid for (A1). They are what a
    /// stop-condition question is asked of (B-056), and the first candidate
    /// for separating addressings that tie on *did the turn end* (B-375).
    pub lengths: Vec<(String, Vec<usize>)>,
    /// How many trials each candidate had.
    pub of: usize,
    /// What the file *declared*, for the divergence (B-058) — never used to
    /// decide, only to disagree with.
    pub declared_a_template: bool,
    /// The winning addressing itself, so that applying it needs no second
    /// search and cannot pick a different one than was reported (D43).
    pub best_addressing: Option<Addressing>,
}

/// The method, written where the result can carry it.
pub const CHAT_TEMPLATE: Method = Method {
    name: "chat-template",
    asks: "a set of short questions through each addressing the model's vocabulary can \
           express, and counts which ones the model answers under and then ends its turn at \
           its own end-of-turn token — an addressing it ends the turn under having said \
           nothing has refused to speak, not finished (F38)",
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
    generate: &mut dyn FnMut(&[usize], usize) -> Trial,
) -> Probed<Addressed> {
    let conditions = conditions(&CHAT_TEMPLATE, model, engine);
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
    // Every candidate is built from this vocabulary's own tokens, so each one
    // can be sent as itself. The check that it *can* is still made when the
    // turn is assembled, because the alternative is sending something else and
    // calling it this (F37).
    let candidates = addressings(&file, &vocabulary);
    let only_raw = candidates.len() == 1;
    if only_raw && declared_a_template {
        return Probed::inconclusive(
            CHAT_TEMPLATE,
            "the file declares a chat template, and no addressing could be built from it that \
             this model's vocabulary can express — so there is nothing to compare raw against, \
             and calling raw best would report the instrument's limit as the model's behaviour \
             (F25, F37)",
            0,
            0,
            conditions,
        );
    }

    let mut stopped: Vec<(String, usize)> = Vec::new();
    let mut silent: Vec<(String, usize)> = Vec::new();
    let mut lengths: Vec<(String, Vec<usize>)> = Vec::new();
    let mut spent = 0_usize;
    let mut ran = 0_usize;
    for addressing in &candidates {
        let mut ended = 0_usize;
        let mut said_nothing = 0_usize;
        let mut ran_for: Vec<usize> = Vec::new();
        for trial in 0..trials {
            // A different question each trial. MCF samples greedily from a
            // fixed seed, so five trials of one question are one trial
            // written down five times — the repetition looked like evidence
            // and was arithmetic (F38, A19). Five questions are five
            // observations of the same thing: does this addressing get an
            // answer out of this model.
            let question = QUESTIONS
                .get(trial % QUESTIONS.len())
                .copied()
                .unwrap_or(QUESTION);
            let Some(identifiers) = addressing.wrap(&vocabulary, question) else {
                return Probed::inconclusive(
                    CHAT_TEMPLATE,
                    format!(
                        "the {} addressing could not be assembled from this vocabulary's tokens",
                        addressing.name
                    ),
                    ran,
                    spent,
                    conditions,
                );
            };
            match generate(&identifiers, budget) {
                // Stopping counts only if the model spoke first. Ending a
                // turn having said nothing is a refusal to speak, and the
                // whole of F38 is that the two are opposite observations
                // wearing the same stop token.
                Trial::Stopped { after: 0 } => {
                    ran = ran.saturating_add(1);
                    spent = spent.saturating_add(budget);
                    said_nothing = said_nothing.saturating_add(1);
                }
                Trial::Stopped { after } => {
                    ran = ran.saturating_add(1);
                    spent = spent.saturating_add(budget);
                    ended = ended.saturating_add(1);
                    ran_for.push(after);
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
        stopped.push((addressing.name.clone(), ended));
        silent.push((addressing.name.clone(), said_nothing));
        ran_for.sort_unstable();
        lengths.push((addressing.name.clone(), ran_for));
    }

    // The best is the one that answered-then-stopped most, earliest first —
    // `max_by_key` answers with the *last* maximum, which handed every tie to
    // `raw`. `max_by_key` is not used here for that reason.
    // (the comment below is kept for the second half of the same lesson)
    // The best is the one that stopped most, earliest first — `max_by_key`
    // answers with the *last* maximum, which handed every tie to `raw`.
    let most = stopped.iter().map(|(_, ended)| *ended).max().unwrap_or(0);
    let best = stopped
        .iter()
        .find(|(_, ended)| *ended == most)
        .map(|(name, _)| name.clone());
    let all_zero = most == 0;

    // Every addressing doing equally well means this observation cannot tell
    // them apart — which is *could not decide*, not *raw is fine*. Preferring
    // a wrapping on a tie would be MCF choosing where it has no evidence, and
    // preferring raw would be reading its own default back as a finding
    // (D42, §3.15).
    let ties = stopped.iter().filter(|(_, ended)| *ended == most).count();
    if ties > 1 && !all_zero {
        // The turn lengths go with the tie. They are the first candidate for
        // the sharper question (B-375), and this branch is exactly where they
        // would otherwise be thrown away — an inconclusive result that
        // discards the measurement that might resolve it is the shape A1
        // forbids.
        let spans: Vec<String> = stopped
            .iter()
            .filter(|(_, ended)| *ended == most)
            .map(|(name, _)| {
                let ran_for = lengths
                    .iter()
                    .find(|(other, _)| other == name)
                    .map(|(_, ran)| ran.as_slice())
                    .unwrap_or_default();
                match (ran_for.first(), ran_for.last()) {
                    (Some(least), Some(longest)) if least == longest => {
                        format!("{name} ran {least}")
                    }
                    (Some(least), Some(longest)) => format!("{name} ran {least}-{longest}"),
                    _ => format!("{name} ran nothing recorded"),
                }
            })
            .collect();
        return Probed::inconclusive(
            CHAT_TEMPLATE,
            format!(
                "{ties} addressings ended the model's turn equally often ({most} of {trials}), \
                 so ending a turn does not tell them apart on this model — a sharper question \
                 than *did it stop* is needed to choose between them. The turns they ran for, \
                 in tokens, which is the first place to look for one (B-375): {}",
                spans.join("; ")
            ),
            ran,
            spent,
            conditions,
        );
    }
    let outcome = match (best, all_zero) {
        // Nothing stopped anywhere: the budget may simply be too small to
        // reach a turn's end. That is *could not tell*, not *no addressing
        // works* (D42, §3.18's third state).
        (_, true) => {
            return Probed::inconclusive(
                CHAT_TEMPLATE,
                {
                    // Which addressings went silent is the whole content of
                    // this negative, and dropping it would throw away the
                    // observation that found F38 (A1).
                    let refused: Vec<String> = silent
                        .iter()
                        .filter(|(_, times)| *times > 0)
                        .map(|(name, times)| format!("{name} {times} of {trials}"))
                        .collect();
                    if refused.is_empty() {
                        format!(
                            "no addressing ended at the model's own stop token within {budget} \
                             tokens; a larger budget may decide it"
                        )
                    } else {
                        format!(
                            "no addressing both answered and then ended its turn within {budget} \
                             tokens. Some ended the turn having said nothing at all ({}), which \
                             is the model declining to speak rather than finishing — a larger \
                             budget may let the ones that were still talking finish (F38)",
                            refused.join(", ")
                        )
                    }
                },
                ran,
                spent,
                conditions,
            );
        }
        (Some(best), false) => Outcome::Observed(Addressed {
            best_addressing: candidates
                .iter()
                .find(|candidate| candidate.name == best)
                .cloned(),
            best,
            stopped,
            silent,
            lengths,
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
pub const QUESTIONS: [&str; 5] = [
    "What is the capital of France?",
    "How many days are in a week?",
    "Name one colour of the rainbow.",
    "What is two plus two?",
    "Which planet do we live on?",
];

/// The first of them, for callers that need one question rather than a set.
pub const QUESTION: &str = QUESTIONS[0];

/// The conditions a probe result holds under (D42, §3.4).
/// The conditions a probe's result carries.
///
/// The method is a parameter and not `CHAT_TEMPLATE`. It was the constant once,
/// and the second probe's result then said it was the first probe's — a
/// condition naming the wrong experiment, which is the exact provenance
/// failure B-059 exists to prevent and would have been believed because it is
/// printed in the same place as the true ones (§3.4, A21).
fn conditions(method: &Method, model: &Path, engine: &str) -> Conditions {
    Conditions::new(
        mcf_core::build_identity::BuildIdentity::current(),
        Floor {
            mcf_configuration: mcf_core::attested::Attested::Known(ConditionValue::text(format!(
                "probe: {}, engine: {engine}, model: {}",
                method.name,
                model.display()
            ))),
            ..Floor::nothing_known()
        },
    )
}

/// What one trial did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Trial {
    /// It ended because the model emitted its own stop token, after saying
    /// this many tokens.
    ///
    /// The count is not decoration. A model addressed in a way it does not
    /// recognise can end its turn *immediately* — nothing said, then its stop
    /// token — and counting that as a turn scored the silent addressing best
    /// and the fluent one worst (F38).
    Stopped {
        /// How many tokens the model produced before its stop token.
        after: usize,
    },
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

/// One trial, and what the model actually said in it.
///
/// The chat-template probe needs only how a turn *ended*; a probe that reads
/// what came out — whether a tool call is well formed (B-053), whether a
/// structured answer parses (B-054) — needs the text as well. It was always
/// being read out of the account and thrown away.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Spoken {
    /// How the turn ended.
    pub trial: Trial,
    /// What the model said, as the engine reported it.
    ///
    /// Empty where the model said nothing, which both engines agree on even
    /// where they disagree about the token count (F39).
    pub text: String,
}

/// One trial through a running daemon: the wrapped question in, the answer
/// out, and what ended it.
///
/// Where the text matters, [`spoken`] returns it; this is that with the text
/// dropped, so the two cannot drift apart.
#[must_use]
pub fn trial(
    socket: &Path,
    model: &Path,
    prompt: &str,
    tokens: Option<&[usize]>,
    budget: usize,
    engine: Option<&str>,
) -> Trial {
    spoken(socket, model, prompt, tokens, budget, engine).trial
}

/// The same trial, keeping what the model said.
#[must_use]
pub fn spoken(
    socket: &Path,
    model: &Path,
    prompt: &str,
    tokens: Option<&[usize]>,
    budget: usize,
    engine: Option<&str>,
) -> Spoken {
    use std::io::{BufRead as _, BufReader, Write as _};

    let Ok(mut connection) = std::os::unix::net::UnixStream::connect(socket) else {
        return could_not_tell("nothing is listening on the control socket".to_owned());
    };
    let _deadline = connection.set_read_timeout(Some(std::time::Duration::from_secs(600)));
    let request = crate::control::Request::Generate {
        // A probe's question is MCF's own constant and the answer is to that
        // question, so this is fixture data rather than the operator's (§6.8,
        // B-146).
        whose: mcf_record::content::Whose::Fixture,
        model: model.display().to_string(),
        prompt: prompt.to_owned(),
        limit: Some(budget),
        seed: 0,
        tokens: tokens.map(<[usize]>::to_vec),
        engine: engine.map(str::to_owned),
    };
    if writeln!(connection, "{}", request.to_line())
        .and_then(|()| connection.flush())
        .is_err()
    {
        return could_not_tell("the request could not be sent".to_owned());
    }

    let reader = BufReader::new(&connection);
    for line in reader.lines() {
        let Ok(line) = line else {
            return could_not_tell("the stream ended before its account".to_owned());
        };
        match crate::control::Streamed::read(line.trim_end()) {
            // Deliberately not counted. MCF's own engine streams one line per
            // token and the provisioned server streams the whole answer as
            // one, so counting lines here measures the engine's chunking and
            // calls it the model's output — which made every addressing look
            // like a one-token turn through the server (F39). The count comes
            // from the account, which both engines fill in the same units.
            Ok(crate::control::Streamed::Token { .. }) => {}
            Ok(crate::control::Streamed::Done(account)) => {
                if let Some(failure) = account.get("failure") {
                    return could_not_tell(format!(
                        "the generation did not complete: {}",
                        failure.to_line()
                    ));
                }
                let counted = account
                    .get("tokens")
                    .and_then(mcf_record::json::Value::as_integer)
                    .and_then(|count| usize::try_from(count).ok());
                let Some(counted) = counted else {
                    return could_not_tell(
                        "the account did not say how many tokens were produced, so a finished \
                         turn cannot be told from a refusal to speak (F38)"
                            .to_owned(),
                    );
                };
                // The two engines disagree by one at exactly the boundary this
                // probe turns on: asked a question it does not recognise, a
                // model emits its end-of-turn token and nothing else, and MCF's
                // own engine calls that nought tokens while the provisioned
                // server calls it one — it counts the end-of-turn token itself
                // (F39). Neither is wrong, and a probe that took either
                // literally would report *said nothing* on one engine and
                // *said something* on the other for one behaviour.
                //
                // The text is the form both agree on: it is empty on both. So
                // the count is what was said, and having said nothing is nought
                // whatever the engine calls it.
                let wordless = account
                    .get("text")
                    .and_then(mcf_record::json::Value::as_text)
                    .is_none_or(|text| text.trim().is_empty());
                let said = if wordless { 0 } else { counted };
                let ended = match account
                    .get("stopped")
                    .and_then(mcf_record::json::Value::as_text)
                {
                    Some("stop_token") => Trial::Stopped { after: said },
                    Some("limit") => Trial::RanOut,
                    Some(other) => Trial::CouldNotTell(format!(
                        "this engine does not say why generation ended (it said {other:?}), so \
                         whether the model finished its turn cannot be observed through it"
                    )),
                    None => Trial::CouldNotTell("the account did not say how it ended".to_owned()),
                };
                return Spoken {
                    trial: ended,
                    text: account
                        .get("text")
                        .and_then(mcf_record::json::Value::as_text)
                        .unwrap_or_default()
                        .to_owned(),
                };
            }
            Err(_) => return could_not_tell("a line of the stream was unreadable".to_owned()),
        }
    }
    could_not_tell("the stream ended before its account".to_owned())
}

/// A trial that could not be told apart, with nothing said.
fn could_not_tell(because: impl Into<String>) -> Spoken {
    Spoken {
        trial: Trial::CouldNotTell(because.into()),
        text: String::new(),
    }
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
    let build = answer.body.get("build");
    let version = build
        .and_then(|build| build.get("version"))
        .and_then(mcf_record::json::Value::as_text);
    // The digest as well as the version, for the reason F93 established and
    // F104 found a second instance of: the version is the same string for
    // every build, so a probe recorded against *whatever the daemon chooses*
    // could not say which daemon, and a daemon is a different binary from the
    // one that asked. `Null` where the platform would not let it read itself,
    // which stays `unknown` rather than becoming a plausible digest (A7).
    let instrument = build
        .and_then(|build| build.get("instrument"))
        .and_then(mcf_record::json::Value::as_text)
        .map_or_else(
            || "unknown".to_owned(),
            |hex| hex.get(..12).unwrap_or(hex).to_owned(),
        );
    Some(version.map_or_else(
        || "a daemon that did not say".to_owned(),
        |version| format!("whatever the daemon at build {version}+{instrument} chooses"),
    ))
}

#[cfg(test)]
mod tests;

/// What the engine did with a prompt of a stated length.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Accepted {
    /// It read this many of the identifiers it was sent.
    ///
    /// Equal to what was sent is the ordinary case. *Fewer* is silent
    /// truncation, which is the failure this probe exists to catch: a prompt
    /// quietly shortened is a measurement of a different prompt (§3.8, D46).
    Read(usize),
    /// It refused, in its own words.
    Refused(String),
    /// Something else, and why.
    CouldNotTell(String),
}

/// The context length the file declares, against the longest prompt the engine
/// will actually take.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Context {
    /// What the file says.
    pub declared: usize,
    /// The longest prompt accepted whole, with one token left to generate.
    pub accepted: usize,
    /// What the engine said where it refused, kept because a refusal for an
    /// unrelated reason would otherwise be reported as a short context (A1).
    pub because: Option<String>,
}

/// The method.
pub const USABLE_CONTEXT: Method = Method {
    name: "usable-context",
    asks: "for a prompt of the length the file declares, and then — only if that is refused — \
           for the longest one the engine will take whole, by halving. What is compared is \
           integers: how many identifiers were sent against how many were read",
    decides: "how long a prompt MCF may give this model on this machine through this engine — \
              and nothing else: a probe writes the verified half of a capability and never a \
              default (D42)",
};

/// The usable context, by asking.
///
/// The declared length is asked for first, so the ordinary case — a file whose
/// claim holds — costs one trial rather than fifteen. Only a refusal starts
/// the search, and the search is a halving between the largest length known to
/// work and the smallest known to fail.
///
/// One token is left for the model to produce, because a context is the whole
/// budget and not the prompt's share of it: `llama.cpp` refuses a prompt of
/// exactly the declared length for that reason, and reporting *the declared
/// context is wrong by one* would be reporting arithmetic as a divergence.
#[allow(
    clippy::too_many_lines,
    reason = "one search, written as the search: ask the claim, then halve. Splitting it would \
              put the question in one function and the answer in another"
)]
#[must_use]
pub fn usable_context(
    model: &Path,
    declared: usize,
    engine: &str,
    ask: &mut dyn FnMut(usize) -> Accepted,
) -> Probed<Context> {
    let conditions = conditions(&USABLE_CONTEXT, model, engine);
    let inconclusive = |because: String, trials: usize, spent: usize| {
        Probed::inconclusive(USABLE_CONTEXT, because, trials, spent, conditions.clone())
    };
    if declared < 2 {
        return inconclusive(
            "the file declares no context length worth asking about".to_owned(),
            0,
            0,
        );
    }

    // Ask the *instrument* before asking the model, and ask it the cheapest
    // question there is. MCF's own engine does not report how many identifiers
    // it read, so it can never answer this probe — and finding that out by
    // sending it the whole declared context first cost eight thousand forward
    // passes to learn nothing (F44). One token learns the same thing.
    if let Accepted::CouldNotTell(said) = ask(1) {
        return inconclusive(said, 1, 1);
    }

    let mut trials = 1_usize;
    let mut spent = 1_usize;
    let mut because: Option<String> = None;
    // Whole is the length asked for; read is what came back. They differ only
    // under truncation, and that difference is the finding.
    let mut works = 0_usize;

    let mut attempt = |length: usize,
                       trials: &mut usize,
                       spent: &mut usize,
                       because: &mut Option<String>|
     -> Option<bool> {
        *trials = trials.saturating_add(1);
        *spent = spent.saturating_add(length);
        match ask(length) {
            Accepted::Read(read) if read == length => Some(true),
            Accepted::Read(read) => {
                *because = Some(format!(
                    "the engine read {read} of the {length} identifiers it was sent and said \
                     nothing about the difference — a prompt shortened in silence is a \
                     measurement of a different prompt (§3.8, A2)"
                ));
                Some(false)
            }
            Accepted::Refused(said) => {
                *because = Some(said);
                Some(false)
            }
            Accepted::CouldNotTell(said) => {
                *because = Some(said);
                None
            }
        }
    };

    // The claim itself, first.
    let full = declared.saturating_sub(1);
    match attempt(full, &mut trials, &mut spent, &mut because) {
        Some(true) => {
            return Probed {
                method: USABLE_CONTEXT,
                outcome: Outcome::Observed(Context {
                    declared,
                    accepted: full,
                    because: None,
                }),
                trials,
                tokens: spent,
                conditions,
            };
        }
        Some(false) => {}
        None => {
            return inconclusive(
                because.unwrap_or_else(|| "the engine did not answer".to_owned()),
                trials,
                spent,
            );
        }
    }

    // It refused, so find where it stops refusing.
    let mut fails = full;
    while fails.saturating_sub(works) > 1 {
        let middle = works.saturating_add(fails.saturating_sub(works).wrapping_div(2));
        match attempt(middle, &mut trials, &mut spent, &mut because) {
            Some(true) => works = middle,
            Some(false) => fails = middle,
            None => {
                return inconclusive(
                    because.unwrap_or_else(|| "the engine did not answer".to_owned()),
                    trials,
                    spent,
                );
            }
        }
    }

    if works == 0 {
        return inconclusive(
            format!(
                "the engine would not take a prompt of any length up to the {declared} this file \
                 declares, which is a fact about the engine or the machine rather than about the \
                 model. It said: {}",
                because.unwrap_or_else(|| "nothing".to_owned())
            ),
            trials,
            spent,
        );
    }
    Probed {
        method: USABLE_CONTEXT,
        outcome: Outcome::Observed(Context {
            declared,
            accepted: works,
            because,
        }),
        trials,
        tokens: spent,
        conditions,
    }
}

/// One length, asked of a running daemon.
///
/// The prompt is one identifier repeated. What is being asked is how many the
/// engine will take, and a filler that means something would invite the reply
/// that the answer depends on what was said — it does not, and the identifiers
/// are counted rather than read.
#[must_use]
pub fn accepts(
    socket: &Path,
    model: &Path,
    filler: usize,
    length: usize,
    engine: Option<&str>,
) -> Accepted {
    use std::io::{BufRead as _, BufReader, Write as _};

    let Ok(mut connection) = std::os::unix::net::UnixStream::connect(socket) else {
        return Accepted::CouldNotTell("nothing is listening on the control socket".to_owned());
    };
    let _deadline = connection.set_read_timeout(Some(std::time::Duration::from_mins(20)));
    let request = crate::control::Request::Generate {
        // A probe's question is MCF's own constant and the answer is to that
        // question, so this is fixture data rather than the operator's (§6.8,
        // B-146).
        whose: mcf_record::content::Whose::Fixture,
        model: model.display().to_string(),
        prompt: String::new(),
        limit: Some(1),
        seed: 0,
        tokens: Some(vec![filler; length]),
        engine: engine.map(str::to_owned),
    };
    if writeln!(connection, "{}", request.to_line())
        .and_then(|()| connection.flush())
        .is_err()
    {
        return Accepted::CouldNotTell("the request could not be sent".to_owned());
    }

    let reader = BufReader::new(&connection);
    for line in reader.lines() {
        let Ok(line) = line else {
            return Accepted::CouldNotTell("the stream ended before its account".to_owned());
        };
        match crate::control::Streamed::read(line.trim_end()) {
            Ok(crate::control::Streamed::Token { .. }) => {}
            Ok(crate::control::Streamed::Done(account)) => {
                if let Some(failure) = account.get("failure") {
                    // The engine's own sentence, not the whole classified
                    // record: a reader wants to know that the context was
                    // exceeded, and the record is on the journal either way.
                    return Accepted::Refused(
                        failure
                            .get("context")
                            .and_then(|context| context.get("engine_said"))
                            .and_then(mcf_record::json::Value::as_text)
                            .map_or_else(|| failure.to_line(), str::to_owned),
                    );
                }
                return match account
                    .get("conditions")
                    .and_then(|conditions| conditions.get("identifiers_read"))
                    .and_then(mcf_record::json::Value::as_integer)
                    .and_then(|read| usize::try_from(read).ok())
                {
                    Some(read) => Accepted::Read(read),
                    // MCF's own engine does not report this, and guessing that
                    // it read everything would be inventing the observation
                    // the probe is for (A7).
                    None => Accepted::CouldNotTell(
                        "this engine does not say how many identifiers it read, so a prompt \
                         taken whole cannot be told from one quietly shortened (B-376)"
                            .to_owned(),
                    ),
                };
            }
            Err(_) => {
                return Accepted::CouldNotTell("a line of the stream was unreadable".to_owned());
            }
        }
    }
    Accepted::CouldNotTell("the stream ended before its account".to_owned())
}

/// The model file, for a caller that has the bytes and needs the fields.
///
/// # Errors
///
/// Whatever reading the file reports.
pub fn gguf_of(bytes: &[u8]) -> Result<gguf::Model, mcf_core::Failure> {
    gguf::parse(bytes)
}

/// The context length the file declares, whatever family wrote it.
///
/// The key is prefixed by the architecture the file states, which is a field
/// GGUF exists to carry and not a family MCF recognises (DEC-053).
#[must_use]
pub fn declared_context(file: &gguf::Model) -> Option<usize> {
    let architecture = match file.get("general.architecture") {
        Some(gguf::Value::Text(named)) => named.clone(),
        _ => return None,
    };
    match file.get(&format!("{architecture}.context_length")) {
        Some(gguf::Value::Integer(found)) => usize::try_from(*found).ok(),
        _ => None,
    }
}

/// One identifier to repeat, for a question that is about length.
///
/// The lowest ordinary token in the vocabulary: not a marker, not a byte
/// fallback, and present in every file MCF reads. What it *means* is beside
/// the point — the engine is being asked how many identifiers it will take,
/// and it counts them.
#[must_use]
pub fn a_filler_token(file: &gguf::Model) -> Option<usize> {
    let vocabulary = Vocabulary::read(file).ok()?;
    (0..vocabulary.len()).find(|at| {
        vocabulary
            .token(*at)
            .is_some_and(|spelled| spelled.chars().count() > 1 && !spelled.starts_with('<'))
    })
}

/// How long this model's turns run, and whether it ends them at all.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Stopping {
    /// The longest turn that ended at the model's own stop token.
    pub longest: usize,
    /// How many trials ended that way.
    pub stopped: usize,
    /// How many were asked.
    pub of: usize,
    /// The largest budget any trial was given.
    pub ceiling: usize,
    /// MCF's default budget, for the divergence — what a caller gets if they
    /// say nothing.
    pub default_budget: usize,
}

/// The method.
pub const STOP_CONDITIONS: Method = Method {
    name: "stop-conditions",
    asks: "the same short questions, doubling the budget until the model ends its turn or a \
           ceiling is reached, and reports the longest turn it finished — so that *this model \
           does not stop* is told apart from *the budget was too small*, which look identical \
           from outside",
    decides: "how many tokens MCF should allow this model by default — and nothing else: a \
              probe writes the verified half of a capability and never a default (D42)",
};

/// The turn lengths a model actually needs.
///
/// **Doubling rather than one large budget.** A budget large enough for the
/// worst case is spent on every trial including the ones that end in ten
/// tokens, and tokens are what a probe costs (B49). Doubling pays for the
/// answer that was needed and one wasted step at most.
///
/// **A ceiling that is reported.** Reaching it is not *the model never stops* —
/// it is *not within this many tokens*, which is a different claim and the only
/// one the trials support (A7). The number travels so that a reader can decide
/// whether it was large enough.
#[must_use]
pub fn stop_conditions(
    model: &Path,
    trials: usize,
    from: usize,
    ceiling: usize,
    default_budget: usize,
    engine: &str,
    generate: &mut dyn FnMut(&str, usize) -> Trial,
) -> Probed<Stopping> {
    let conditions = conditions(&STOP_CONDITIONS, model, engine);
    let mut spent = 0_usize;
    let mut ran = 0_usize;
    let mut longest = 0_usize;
    let mut stopped = 0_usize;
    let mut reached = from;

    for trial in 0..trials {
        let question = QUESTIONS
            .get(trial % QUESTIONS.len())
            .copied()
            .unwrap_or(QUESTION);
        let mut budget = from;
        loop {
            ran = ran.saturating_add(1);
            spent = spent.saturating_add(budget);
            reached = reached.max(budget);
            match generate(question, budget) {
                Trial::Stopped { after } => {
                    stopped = stopped.saturating_add(1);
                    longest = longest.max(after);
                    break;
                }
                Trial::RanOut if budget >= ceiling => break,
                Trial::RanOut => budget = budget.saturating_mul(2).min(ceiling),
                Trial::CouldNotTell(because) => {
                    return Probed::inconclusive(
                        STOP_CONDITIONS,
                        format!("on the question {question:?}: {because}"),
                        ran,
                        spent,
                        conditions,
                    );
                }
            }
        }
    }

    if stopped == 0 {
        return Probed::inconclusive(
            STOP_CONDITIONS,
            format!(
                "no turn ended at this model's own stop token within {reached} tokens. That is \
                 not *this model never stops* — it is *not within {reached}*, and a larger \
                 ceiling or a different addressing may end it (A7). If this model is addressed \
                 wrongly it will not stop at any budget, which is what the chat-template probe \
                 is for"
            ),
            ran,
            spent,
            conditions,
        );
    }

    Probed {
        method: STOP_CONDITIONS,
        outcome: Outcome::Observed(Stopping {
            longest,
            stopped,
            of: trials,
            ceiling: reached,
            default_budget,
        }),
        trials: ran,
        tokens: spent,
        conditions,
    }
}
