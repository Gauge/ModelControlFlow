use std::path::Path;

const SILENCE: std::time::Duration = std::time::Duration::from_secs(3600);

use mcf_core::measurement::{ConditionValue, Conditions, Floor};
use mcf_core::probe::{Method, Outcome, Probed};
use mcf_standin::gguf;
use mcf_standin::tokenizer::{Piece, Tokens};

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Addressing {
    pub name: String,
    pub pieces_before: Vec<Piece>,
    pub pieces_after: Vec<Piece>,
}

impl Addressing {
    #[must_use]
    pub fn wrap(&self, question: &str) -> Vec<Piece> {
        let mut pieces = self.pieces_before.clone();
        pieces.push(Piece::Text(question.to_owned()));
        pieces.extend(self.pieces_after.iter().cloned());
        pieces
    }

    #[must_use]
    pub fn wrapped(addressing: Option<&Self>, question: &str) -> Vec<Piece> {
        addressing.map_or_else(
            || vec![Piece::Text(question.to_owned())],
            |addressing| addressing.wrap(question),
        )
    }

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
pub mod run;
pub mod structured;
pub mod thinking;
pub mod tools;
pub mod vision;

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

#[must_use]
pub fn addressings(file: &gguf::Model, tokens: &Tokens) -> Vec<Addressing> {
    let mut found = from_template(file, tokens);
    if found.is_empty() {
        found = known_shapes(tokens);
    }
    found.push(Addressing {
        name: "raw".to_owned(),
        pieces_before: Vec::new(),
        pieces_after: Vec::new(),
    });
    found
}

fn from_template(file: &gguf::Model, tokens: &Tokens) -> Vec<Addressing> {
    let Some(template) = file
        .get("tokenizer.chat_template")
        .and_then(gguf::Value::as_text)
    else {
        return Vec::new();
    };

    let mut markers: Vec<String> = Vec::new();
    for marker in markers_in(template) {
        if tokens.has_token(&marker) && !markers.contains(&marker) {
            markers.push(marker);
        }
    }
    let Some(close) = tokens
        .ending
        .and_then(|ending| tokens.token(ending))
        .map(str::to_owned)
        .filter(|ending| markers.contains(ending))
    else {
        return role_named(template, &markers, tokens);
    };
    let Some(open) = markers
        .iter()
        .filter(|marker| **marker != close)
        .find(|marker| opens_a_role(template, marker))
        .or_else(|| markers.iter().find(|marker| **marker != close))
        .cloned()
    else {
        return Vec::new();
    };
    let (open, close) = (&open, &close);

    let assigned = assigned_roles(template);
    let mut roles: Vec<String> = if assigned.is_empty() {
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

fn role_named(template: &str, markers: &[String], tokens: &Tokens) -> Vec<Addressing> {
    const THINKING_OPEN: &str = "<think>";
    const THINKING_CLOSED: &str = "</think>";
    let spelled = |role: &str| markers.iter().find(|marker| trim(marker) == role).cloned();
    let (Some(user), Some(assistant)) = (spelled("user"), spelled("assistant")) else {
        return Vec::new();
    };
    let base = format!("{}…{}", trim(&user), trim(&assistant));
    let writes = |marker: &str| template.contains(marker) && tokens.has_token(marker);
    if !writes(THINKING_OPEN) && !writes(THINKING_CLOSED) {
        return vec![Addressing {
            name: base,
            pieces_before: vec![Piece::Marker(user)],
            pieces_after: vec![Piece::Marker(assistant)],
        }];
    }
    let (marker, how) = if writes(THINKING_OPEN) {
        (THINKING_OPEN, "open")
    } else {
        (THINKING_CLOSED, "closed")
    };
    vec![Addressing {
        name: format!("{base}, thinking {how}"),
        pieces_before: vec![Piece::Marker(user)],
        pieces_after: vec![Piece::Marker(assistant), Piece::Marker(marker.to_owned())],
    }]
}

fn opens_a_role(template: &str, marker: &str) -> bool {
    const ROLE_WORDS: [&str; 4] = ["role", "user", "system", "assistant"];
    const ACCESSORS: [&str; 4] = ["", "message", "messages", "m"];
    template.match_indices(marker).any(|(at, _)| {
        let after = template
            .get(at.saturating_add(marker.len())..)
            .unwrap_or_default()
            .chars()
            .take(24)
            .collect::<String>();
        ROLE_WORDS.iter().any(|word| {
            after.find(word).is_some_and(|found| {
                let between: String = after
                    .get(..found)
                    .unwrap_or_default()
                    .chars()
                    .filter(char::is_ascii_alphabetic)
                    .collect();
                ACCESSORS.contains(&between.as_str())
            })
        })
    })
}

fn known_shapes(tokens: &Tokens) -> Vec<Addressing> {
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
        if tokens.has_token(open) && tokens.has_token(close) {
            found.push(turn(open, close, role));
        }
    }
    if tokens.has_token("[INST]") && tokens.has_token("[/INST]") {
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

pub(crate) fn trim(marker: &str) -> &str {
    marker
        .trim_start_matches(['<', '|', '['])
        .trim_end_matches(['>', '|', ']'])
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Addressed {
    pub best: String,
    pub stopped: Vec<(String, usize)>,
    pub silent: Vec<(String, usize)>,
    pub lengths: Vec<(String, Vec<usize>)>,
    pub of: usize,
    pub declared_a_template: bool,
    pub best_addressing: Option<Addressing>,
}

pub const CHAT_TEMPLATE: Method = Method {
    name: "chat-template",
    asks: "a set of short questions through each addressing the model's vocabulary can \
           express, and counts which ones the model answers under and then ends its turn at \
           its own end-of-turn token — an addressing it ends the turn under having said \
           nothing has refused to speak, not finished (F38)",
    decides: "how MCF should address this model — and nothing else: a probe writes the \
              verified half of a capability and never a default (D42)",
};

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
    generate: &mut dyn FnMut(&[Piece], usize) -> Trial,
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
    let Ok(tokens) = Tokens::read(&file) else {
        return Probed::inconclusive(CHAT_TEMPLATE, "the file lists no tokens", 0, 0, conditions);
    };
    let candidates = addressings(&file, &tokens);
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
            let question = QUESTIONS
                .get(trial % QUESTIONS.len())
                .copied()
                .unwrap_or(QUESTION);
            match generate(&addressing.wrap(question), budget) {
                Trial::Stopped { after: 0, .. } => {
                    ran = ran.saturating_add(1);
                    spent = spent.saturating_add(budget);
                    said_nothing = said_nothing.saturating_add(1);
                }
                Trial::Stopped { after, .. } => {
                    ran = ran.saturating_add(1);
                    spent = spent.saturating_add(budget);
                    ended = ended.saturating_add(1);
                    ran_for.push(after);
                }
                Trial::RanOut => {
                    ran = ran.saturating_add(1);
                    spent = spent.saturating_add(budget);
                }
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

    let most = stopped.iter().map(|(_, ended)| *ended).max().unwrap_or(0);
    let best = stopped
        .iter()
        .find(|(_, ended)| *ended == most)
        .map(|(name, _)| name.clone());
    let all_zero = most == 0;

    let ties = stopped.iter().filter(|(_, ended)| *ended == most).count();
    if ties > 1 && !all_zero {
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
        (_, true) => {
            return Probed::inconclusive(
                CHAT_TEMPLATE,
                {
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

pub const QUESTIONS: [&str; 5] = [
    "What is the capital of France?",
    "How many days are in a week?",
    "Name one colour of the rainbow.",
    "What is two plus two?",
    "Which planet do we live on?",
];

pub const QUESTION: &str = QUESTIONS[0];

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Trial {
    Stopped { after: usize, before: Option<usize> },
    RanOut,
    CouldNotTell(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Spoken {
    pub trial: Trial,
    pub text: String,
    pub answer: Option<String>,
    pub engine_ran: Option<String>,
    pub window_ran: Option<u64>,
}

impl Spoken {
    #[must_use]
    pub fn answered(&self) -> &str {
        self.answer.as_deref().unwrap_or(&self.text)
    }
}

#[must_use]
pub fn trial(
    socket: &Path,
    model: &Path,
    prompt: &str,
    pieces: Option<&[Piece]>,
    budget: usize,
    engine: Option<&str>,
) -> Trial {
    spoken(socket, model, prompt, pieces, budget, engine).trial
}

#[must_use]
#[allow(
    clippy::too_many_lines,
    reason = "one account read straight through: how it ended, what it said, what ran it"
)]
pub fn spoken(
    socket: &Path,
    model: &Path,
    prompt: &str,
    pieces: Option<&[Piece]>,
    budget: usize,
    engine: Option<&str>,
) -> Spoken {
    spoken_as(
        socket,
        model,
        prompt,
        pieces,
        budget,
        engine,
        crate::declared::Started::default(),
    )
}

#[allow(
    clippy::too_many_arguments,
    clippy::too_many_lines,
    reason = "one ask's conditions, each named in the account"
)]
pub fn spoken_as(
    socket: &Path,
    model: &Path,
    prompt: &str,
    pieces: Option<&[Piece]>,
    budget: usize,
    engine: Option<&str>,
    started: crate::declared::Started,
) -> Spoken {
    use std::io::{BufRead as _, BufReader, Write as _};

    let Ok(mut connection) = std::os::unix::net::UnixStream::connect(socket) else {
        return could_not_tell("nothing is listening on the control socket".to_owned());
    };
    let _deadline = connection.set_read_timeout(Some(SILENCE));
    let request = crate::control::Request::Generate {
        whose: mcf_record::content::Whose::Fixture,
        model: model.display().to_string(),
        prompt: prompt.to_owned(),
        limit: Some(budget),
        seed: 0,
        tokens: None,
        pieces: pieces.map(<[Piece]>::to_vec),
        engine: engine.map(str::to_owned),
        pinned: false,
        turn: None,
        image: None,
        started,
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
            Ok(
                crate::control::Streamed::Token { .. } | crate::control::Streamed::Progress { .. },
            ) => {}
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
                let wordless = account
                    .get("text")
                    .and_then(mcf_record::json::Value::as_text)
                    .is_none_or(|text| text.trim().is_empty());
                let said = if wordless { 0 } else { counted };
                let before = account.get("before_the_answer");
                let closed = before
                    .and_then(|before| before.get("closed"))
                    .and_then(mcf_record::json::Value::as_bool)
                    .unwrap_or(false);
                let inside = before
                    .filter(|_| closed)
                    .and_then(|before| before.get("tokens"))
                    .and_then(mcf_record::json::Value::as_integer)
                    .and_then(|found| usize::try_from(found).ok());
                let ended = match account
                    .get("stopped")
                    .and_then(mcf_record::json::Value::as_text)
                {
                    Some("stop_token") => Trial::Stopped {
                        after: said,
                        before: inside,
                    },
                    Some("limit") => Trial::RanOut,
                    Some(other) => Trial::CouldNotTell(format!(
                        "this engine does not say why generation ended (it said {other:?}), so \
                         whether the model finished its turn cannot be observed through it"
                    )),
                    None => Trial::CouldNotTell("the account did not say how it ended".to_owned()),
                };
                return Spoken {
                    trial: ended,
                    engine_ran: account
                        .get("conditions")
                        .and_then(|conditions| conditions.get("engine"))
                        .and_then(mcf_record::json::Value::as_text)
                        .map(str::to_owned),
                    window_ran: account
                        .get("conditions")
                        .and_then(|conditions| conditions.get("window"))
                        .and_then(mcf_record::json::Value::as_integer)
                        .and_then(|window| u64::try_from(window).ok()),
                    text: account
                        .get("text")
                        .and_then(mcf_record::json::Value::as_text)
                        .unwrap_or_default()
                        .to_owned(),
                    answer: before
                        .filter(|_| closed)
                        .and_then(|before| before.get("answer"))
                        .and_then(mcf_record::json::Value::as_text)
                        .map(str::to_owned),
                };
            }
            Err(_) => return could_not_tell("a line of the stream was unreadable".to_owned()),
        }
    }
    could_not_tell("the stream ended before its account".to_owned())
}

fn could_not_tell(because: impl Into<String>) -> Spoken {
    Spoken {
        trial: Trial::CouldNotTell(because.into()),
        text: String::new(),
        answer: None,
        engine_ran: None,
        window_ran: None,
    }
}

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

pub fn counted(
    socket: &Path,
    model: &Path,
    text: &str,
    engine: Option<&str>,
) -> Result<Counted, String> {
    use std::io::{BufRead as _, BufReader, Write as _};

    let mut connection = std::os::unix::net::UnixStream::connect(socket)
        .map_err(|_| "nothing is listening on the control socket".to_owned())?;
    let _deadline = connection.set_read_timeout(Some(SILENCE));
    let request = crate::control::Request::Tokenize {
        model: model.display().to_string(),
        text: text.to_owned(),
        engine: engine.map(str::to_owned),
        beginning: false,
    };
    writeln!(connection, "{}", request.to_line())
        .and_then(|()| connection.flush())
        .map_err(|_| "the request could not be sent".to_owned())?;
    let mut line = String::new();
    BufReader::new(&connection)
        .read_line(&mut line)
        .map_err(|_| "the daemon closed the connection before answering".to_owned())?;
    let answer = crate::control::Answer::read(line.trim_end())
        .map_err(|failure| format!("the daemon's answer could not be read: {failure}"))?;
    if !answer.served {
        return Err(
            mcf_record::decode::failure_said(&answer.body).unwrap_or_else(|| {
                format!(
                    "the daemon refused with something that is not a failure: {}",
                    answer.body.to_line()
                )
            }),
        );
    }
    let tokens = answer
        .body
        .get("tokens")
        .and_then(mcf_record::json::Value::as_integer)
        .and_then(|held| usize::try_from(held).ok())
        .ok_or_else(|| "the daemon's answer carried no count".to_owned())?;
    let by = answer
        .body
        .get("read_by")
        .and_then(mcf_record::json::Value::as_text)
        .map_or_else(
            || "a tokenizer the daemon did not name".to_owned(),
            str::to_owned,
        );
    Ok(Counted { tokens, by })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Counted {
    pub tokens: usize,
    pub by: String,
}

#[cfg(test)]
pub(crate) mod tests;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Accepted {
    Read(usize),
    Refused(String),
    CouldNotTell(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Context {
    pub declared: usize,
    pub ceiling: usize,
    pub accepted: usize,
    pub because: Option<String>,
}

pub const USABLE_CONTEXT: Method = Method {
    name: "usable-context",
    asks: "for a prompt of the length the file declares, and then — only if that is refused — \
           for the longest one the engine will take whole, by halving. What is compared is \
           integers: how many identifiers were sent against how many were read",
    decides: "how long a prompt MCF may give this model on this machine through this engine — \
              and nothing else: a probe writes the verified half of a capability and never a \
              default (D42)",
};

#[allow(
    clippy::too_many_lines,
    reason = "one search, written as the search: ask the claim, then halve. Splitting it would \
              put the question in one function and the answer in another"
)]
#[must_use]
pub fn usable_context(
    model: &Path,
    declared: usize,
    up_to: Option<usize>,
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

    if let Accepted::CouldNotTell(said) = ask(1) {
        return inconclusive(said, 1, 1);
    }

    let mut trials = 1_usize;
    let mut spent = 1_usize;
    let mut because: Option<String> = None;
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

    let full = ceiling_of(declared, up_to);
    match attempt(full, &mut trials, &mut spent, &mut because) {
        Some(true) => {
            return Probed {
                method: USABLE_CONTEXT,
                outcome: Outcome::Observed(Context {
                    declared,
                    ceiling: full,
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
            ceiling: full,
            accepted: works,
            because,
        }),
        trials,
        tokens: spent,
        conditions,
    }
}

#[must_use]
pub fn ceiling_of(declared: usize, up_to: Option<usize>) -> usize {
    let full = declared.saturating_sub(1);
    up_to.map_or(full, |asked| asked.min(full))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Projection {
    pub sample: usize,
    pub nanos: u64,
    pub target: usize,
}

impl Projection {
    #[must_use]
    pub fn nanos_at_the_rate(&self) -> Option<u64> {
        let sample = u128::try_from(self.sample).ok()?;
        if sample == 0 {
            return None;
        }
        let target = u128::try_from(self.target).ok()?;
        let nanos = u128::from(self.nanos);
        target
            .checked_mul(nanos)?
            .checked_div(sample)
            .and_then(|projected| u64::try_from(projected).ok())
    }

    #[must_use]
    pub fn sentence(&self) -> String {
        let sample_seconds = spelled_seconds(self.nanos);
        match self.nanos_at_the_rate() {
            Some(projected) => format!(
                "projected: {} identifiers took {} to read, so the trial of {} is at least {} at \
                 that rate — a projection from the short prompt, not a measurement, and reading \
                 slows as the prompt deepens",
                self.sample,
                sample_seconds,
                self.target,
                spelled_seconds(projected)
            ),
            None => format!(
                "projected: {} identifiers took {} to read; the trial of {} is too long to put a \
                 figure on from that",
                self.sample, sample_seconds, self.target
            ),
        }
    }
}

fn spelled_seconds(nanos: u64) -> String {
    const SECOND: u64 = 1_000_000_000;
    let seconds = nanos.checked_div(SECOND).unwrap_or(0);
    let minutes = seconds.checked_div(60).unwrap_or(0);
    let hours = minutes.checked_div(60).unwrap_or(0);
    if hours > 0 {
        format!("{hours} h {} min", minutes.checked_rem(60).unwrap_or(0))
    } else if minutes > 0 {
        format!("{minutes} min {} s", seconds.checked_rem(60).unwrap_or(0))
    } else {
        format!("{seconds} s")
    }
}

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
    let _deadline = connection.set_read_timeout(Some(SILENCE));
    let request = crate::control::Request::Generate {
        whose: mcf_record::content::Whose::Fixture,
        model: model.display().to_string(),
        prompt: String::new(),
        limit: Some(1),
        seed: 0,
        tokens: Some(vec![filler; length]),
        pieces: None,
        engine: engine.map(str::to_owned),
        pinned: false,
        turn: None,
        image: None,
        started: crate::declared::Started::default(),
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
            Ok(
                crate::control::Streamed::Token { .. } | crate::control::Streamed::Progress { .. },
            ) => {}
            Ok(crate::control::Streamed::Done(account)) => {
                if let Some(failure) = account.get("failure") {
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

pub fn gguf_of(bytes: &[u8]) -> Result<gguf::Model, mcf_core::Failure> {
    gguf::parse(bytes)
}

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

#[must_use]
pub fn a_filler_token(file: &gguf::Model) -> Option<usize> {
    let tokens = Tokens::read(file).ok()?;
    (0..tokens.len()).find(|at| {
        tokens
            .token(*at)
            .is_some_and(|spelled| spelled.chars().count() > 1 && !spelled.starts_with('<'))
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Stopping {
    pub longest: usize,
    pub before: Option<usize>,
    pub stopped: usize,
    pub of: usize,
    pub ceiling: usize,
    pub default_budget: usize,
}

pub const STOP_CONDITIONS: Method = Method {
    name: "stop-conditions",
    asks: "the same short questions, doubling the budget until the model ends its turn or a \
           ceiling is reached, and reports the longest turn it finished — so that *this model \
           does not stop* is told apart from *the budget was too small*, which look identical \
           from outside",
    decides: "how many tokens MCF should allow this model by default — and nothing else: a \
              probe writes the verified half of a capability and never a default (D42)",
};

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
    let mut before: Option<usize> = None;
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
                Trial::Stopped {
                    after,
                    before: inside,
                } => {
                    stopped = stopped.saturating_add(1);
                    longest = longest.max(after);
                    before = before.max(inside);
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
            before,
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
