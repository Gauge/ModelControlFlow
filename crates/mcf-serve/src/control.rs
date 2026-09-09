use mcf_core::failure::{Attribution, Category, Disposition, Failure, Result, Subsystem};
use mcf_record::json::{self, Value};

const WHERE: Subsystem = Subsystem::new("mcf-serve::control");

pub const VERSION: i64 = 1;

pub const REQUEST_CEILING: usize = 4 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum On {
    Processor,
    Card,
}

impl On {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Processor => "processor",
            Self::Card => "card",
        }
    }

    #[must_use]
    pub fn parse(word: &str) -> Option<Self> {
        match word.trim().to_ascii_lowercase().as_str() {
            "processor" | "cpu" => Some(Self::Processor),
            "card" | "gpu" => Some(Self::Card),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Request {
    Status,
    Holding,
    Components,
    Failures {
        last: usize,
    },
    Provision {
        component: Option<String>,
    },
    Stop {
        reason: String,
    },
    Generate {
        model: String,
        prompt: String,
        limit: Option<usize>,
        seed: u64,
        tokens: Option<Vec<usize>>,
        pieces: Option<Vec<mcf_standin::tokenizer::Piece>>,
        engine: Option<String>,
        whose: mcf_record::content::Whose,
        pinned: bool,
        turn: Option<crate::turn::Turn>,
        image: Option<String>,
        started: crate::declared::Started,
    },
    Offered {
        reference: String,
        from: Option<String>,
        fresh: bool,
    },
    Search {
        query: String,
        from: Option<String>,
        fresh: bool,
    },
    Acquire {
        reference: String,
        file: String,
        from: Option<String>,
    },
    Settings {
        model: String,
    },
    Host {
        model: String,
        settings: Value,
    },
    Hosted,
    Unhost,
    Anatomy {
        model: String,
    },
}

fn maybe(held: Option<&str>) -> Value {
    held.map_or(Value::Null, Value::text)
}

#[allow(
    clippy::too_many_arguments,
    reason = "one request's fields, each of which the wire names"
)]
fn generate_line(
    model: &str,
    prompt: &str,
    limit: Option<usize>,
    seed: u64,
    tokens: Option<&[usize]>,
    pieces: Option<&[mcf_standin::tokenizer::Piece]>,
    engine: Option<&str>,
    whose: mcf_record::content::Whose,
    pinned: bool,
    turn: Option<&crate::turn::Turn>,
    image: Option<&str>,
    started: crate::declared::Started,
) -> Value {
    Value::map([
        ("ask", Value::text("generate")),
        ("model", Value::text(model.to_owned())),
        ("prompt", Value::text(prompt.to_owned())),
        (
            "limit",
            match limit {
                Some(limit) => Value::Integer(i64::try_from(limit).unwrap_or(i64::MAX)),
                None => Value::Null,
            },
        ),
        (
            "seed",
            Value::Integer(i64::try_from(seed).unwrap_or(i64::MAX)),
        ),
        (
            "tokens",
            match tokens {
                Some(tokens) => Value::List(
                    tokens
                        .iter()
                        .map(|token| Value::Integer(i64::try_from(*token).unwrap_or(i64::MAX)))
                        .collect(),
                ),
                None => Value::Null,
            },
        ),
        (
            "pieces",
            pieces.map_or(Value::Null, crate::configured::pieces_to_value),
        ),
        (
            "engine",
            match engine {
                Some(engine) => Value::text(engine.to_owned()),
                None => Value::Null,
            },
        ),
        ("whose", Value::text(whose.as_str())),
        ("pinned", Value::Bool(pinned)),
        (
            "turn",
            turn.map_or(Value::Null, crate::turn::Turn::to_value),
        ),
        (
            "image",
            image.map_or(Value::Null, |path| Value::text(path.to_owned())),
        ),
        ("started_with", started.to_value()),
    ])
}

#[must_use]
pub fn is_reference(typed: &str) -> bool {
    mcf_hub::reference::parse(typed).is_ok()
}

fn offered_line(reference: &str, from: Option<&str>, fresh: bool) -> Value {
    Value::map([
        ("ask", Value::text("offered")),
        ("reference", Value::text(reference.to_owned())),
        (
            "from",
            from.map_or(Value::Null, |hub| Value::text(hub.to_owned())),
        ),
        ("fresh", Value::Bool(fresh)),
    ])
}

fn search_line(query: &str, from: Option<&str>, fresh: bool) -> Value {
    Value::map([
        ("ask", Value::text("search")),
        ("query", Value::text(query.to_owned())),
        (
            "from",
            from.map_or(Value::Null, |hub| Value::text(hub.to_owned())),
        ),
        ("fresh", Value::Bool(fresh)),
    ])
}

impl Request {
    #[must_use]
    #[allow(
        clippy::too_many_lines,
        reason = "one arm per request, each a call or a short map; the match is total by design"
    )]
    pub fn to_line(&self) -> String {
        let body = match self {
            Self::Status => Value::map([("ask", Value::text("status"))]),
            Self::Holding => Value::map([("ask", Value::text("holding"))]),
            Self::Components => Value::map([("ask", Value::text("components"))]),
            Self::Failures { last } => Value::map([
                ("ask", Value::text("failures")),
                (
                    "last",
                    Value::Integer(i64::try_from(*last).unwrap_or(i64::MAX)),
                ),
            ]),
            Self::Provision { component } => {
                let mut fields = vec![("ask", Value::text("provision"))];
                if let Some(component) = component {
                    fields.push(("component", Value::text(component.clone())));
                }
                Value::map(fields)
            }
            Self::Stop { reason } => Value::map([
                ("ask", Value::text("stop")),
                ("reason", Value::text(reason.clone())),
            ]),
            Self::Generate {
                model,
                prompt,
                limit,
                seed,
                tokens,
                pieces,
                engine,
                whose,
                pinned,
                turn,
                image,
                started,
            } => generate_line(
                model,
                prompt,
                *limit,
                *seed,
                tokens.as_deref(),
                pieces.as_deref(),
                engine.as_deref(),
                *whose,
                *pinned,
                turn.as_ref(),
                image.as_deref(),
                *started,
            ),
            Self::Offered {
                reference,
                from,
                fresh,
            } => offered_line(reference, from.as_deref(), *fresh),
            Self::Search { query, from, fresh } => search_line(query, from.as_deref(), *fresh),
            Self::Acquire {
                reference,
                file,
                from,
            } => Value::map([
                ("ask", Value::text("acquire")),
                ("reference", Value::text(reference.clone())),
                ("file", Value::text(file.clone())),
                ("from", maybe(from.as_deref())),
            ]),
            Self::Settings { model } => Value::map([
                ("ask", Value::text("settings")),
                ("model", Value::text(model.clone())),
            ]),
            Self::Anatomy { model } => Value::map([
                ("ask", Value::text("anatomy")),
                ("model", Value::text(model.clone())),
            ]),
            Self::Host { model, settings } => Value::map([
                ("ask", Value::text("host")),
                ("model", Value::text(model.clone())),
                ("settings", settings.clone()),
            ]),
            Self::Hosted => Value::map([("ask", Value::text("hosted"))]),
            Self::Unhost => Value::map([("ask", Value::text("unhost"))]),
        };
        let Value::Map(mut fields) = body else {
            return String::new();
        };
        fields.insert("protocol".to_owned(), Value::Integer(VERSION));
        Value::Map(fields).to_line()
    }

    #[expect(
        clippy::too_many_lines,
        reason = "one arm a request, and splitting it would put the wire format \
                  of a request somewhere other than beside the wire format of \
                  every other request"
    )]
    pub fn read(line: &str) -> Result<Self> {
        if line.len() > REQUEST_CEILING {
            return Err(refused(
                "a request larger than MCF will read",
                &format!("{} bytes", line.len()),
            ));
        }
        let value = json::parse(line)
            .map_err(|error| refused("a request that is not a request", &error.to_string()))?;

        match value.get("protocol").and_then(Value::as_integer) {
            Some(VERSION) => {}
            Some(other) => {
                return Err(Failure::new(
                    Category::ExchangeSchemaUnreadable,
                    Attribution::User,
                    Disposition::Refused,
                    WHERE,
                    "a client speaking a protocol version this build does not",
                )
                .with_context("client_speaks", other.to_string())
                .with_context("this_build_speaks", VERSION.to_string()));
            }
            None => return Err(refused("a request naming no protocol version", line)),
        }

        let optional = |key: &str| value.get(key).and_then(Value::as_text).map(str::to_owned);
        match value.get("ask").and_then(Value::as_text) {
            Some("status") => Ok(Self::Status),
            Some("offered") => Ok(Self::Offered {
                reference: value
                    .get("reference")
                    .and_then(Value::as_text)
                    .ok_or_else(|| refused("a listing naming no reference", line))?
                    .to_owned(),
                from: optional("from"),
                fresh: matches!(value.get("fresh"), Some(Value::Bool(true))),
            }),
            Some("search") => Ok(Self::Search {
                query: value
                    .get("query")
                    .and_then(Value::as_text)
                    .ok_or_else(|| refused("a search naming no word", line))?
                    .to_owned(),
                from: optional("from"),
                fresh: matches!(value.get("fresh"), Some(Value::Bool(true))),
            }),
            Some("acquire") => Ok(Self::Acquire {
                reference: value
                    .get("reference")
                    .and_then(Value::as_text)
                    .ok_or_else(|| refused("an acquisition naming no reference", line))?
                    .to_owned(),
                file: value
                    .get("file")
                    .and_then(Value::as_text)
                    .ok_or_else(|| refused("an acquisition naming no file", line))?
                    .to_owned(),
                from: optional("from"),
            }),
            Some("hosted") => Ok(Self::Hosted),
            Some("unhost") => Ok(Self::Unhost),
            Some("settings") => Ok(Self::Settings {
                model: value
                    .get("model")
                    .and_then(Value::as_text)
                    .ok_or_else(|| refused("a settings request naming no model", line))?
                    .to_owned(),
            }),
            Some("anatomy") => Ok(Self::Anatomy {
                model: value
                    .get("model")
                    .and_then(Value::as_text)
                    .ok_or_else(|| refused("an anatomy request naming no model", line))?
                    .to_owned(),
            }),
            Some("host") => Ok(Self::Host {
                model: value
                    .get("model")
                    .and_then(Value::as_text)
                    .ok_or_else(|| refused("a hosting request naming no model", line))?
                    .to_owned(),
                settings: value.get("settings").cloned().unwrap_or(Value::Null),
            }),
            Some("holding") => Ok(Self::Holding),
            Some("components") => Ok(Self::Components),
            Some("failures") => Ok(Self::Failures {
                last: value
                    .get("last")
                    .and_then(Value::as_integer)
                    .and_then(|held| usize::try_from(held).ok())
                    .unwrap_or(20),
            }),
            Some("provision") => Ok(Self::Provision {
                component: optional("component"),
            }),
            Some("stop") => Ok(Self::Stop {
                reason: value
                    .get("reason")
                    .and_then(Value::as_text)
                    .unwrap_or_default()
                    .to_owned(),
            }),
            Some("generate") => {
                let text = |key: &str| -> Result<String> {
                    value
                        .get(key)
                        .and_then(Value::as_text)
                        .map(str::to_owned)
                        .ok_or_else(|| refused("a generation naming no model or no prompt", key))
                };
                Ok(Self::Generate {
                    model: text("model")?,
                    prompt: text("prompt")?,
                    limit: value
                        .get("limit")
                        .and_then(Value::as_integer)
                        .and_then(|limit| usize::try_from(limit).ok()),
                    seed: value
                        .get("seed")
                        .and_then(Value::as_integer)
                        .and_then(|seed| u64::try_from(seed).ok())
                        .unwrap_or(0),
                    tokens: value.get("tokens").and_then(Value::as_list).map(|tokens| {
                        tokens
                            .iter()
                            .filter_map(Value::as_integer)
                            .filter_map(|token| usize::try_from(token).ok())
                            .collect()
                    }),
                    pieces: value
                        .get("pieces")
                        .and_then(crate::configured::pieces_from_value),
                    engine: value
                        .get("engine")
                        .and_then(Value::as_text)
                        .map(str::to_owned),
                    whose: value
                        .get("whose")
                        .and_then(Value::as_text)
                        .and_then(mcf_record::content::Whose::parse)
                        .unwrap_or(mcf_record::content::Whose::User),
                    pinned: value
                        .get("pinned")
                        .and_then(Value::as_bool)
                        .unwrap_or(false),
                    turn: value.get("turn").and_then(crate::turn::Turn::from_value),
                    image: value
                        .get("image")
                        .and_then(Value::as_text)
                        .map(str::to_owned),
                    started: value
                        .get("started_with")
                        .map(crate::declared::Started::from_value)
                        .unwrap_or_default(),
                })
            }
            Some(other) => Err(refused("a request MCF does not have", other)),
            None => Err(refused("a request asking for nothing", line)),
        }
    }
}

pub const DEFAULT_LIMIT: usize = 32;

#[derive(Debug, Clone, PartialEq)]
pub enum Streamed {
    Token {
        at: usize,
        text: String,
    },
    Progress {
        read: u64,
        of: u64,
        produced: u64,
        seconds: u64,
    },
    Done(Value),
}

impl Streamed {
    #[must_use]
    pub fn to_line(&self) -> String {
        let body = match self {
            Self::Token { at, text } => Value::map([
                ("protocol", Value::Integer(VERSION)),
                ("token", Value::text(text.clone())),
                ("at", Value::Integer(i64::try_from(*at).unwrap_or(i64::MAX))),
            ]),
            Self::Progress {
                read,
                of,
                produced,
                seconds,
            } => {
                let figure = |held: u64| Value::Integer(i64::try_from(held).unwrap_or(i64::MAX));
                Value::map([
                    ("protocol", Value::Integer(VERSION)),
                    (
                        "progress",
                        Value::map([
                            ("read", figure(*read)),
                            ("of", figure(*of)),
                            ("produced", figure(*produced)),
                            ("seconds", figure(*seconds)),
                        ]),
                    ),
                ])
            }
            Self::Done(account) => Value::map([
                ("protocol", Value::Integer(VERSION)),
                ("done", account.clone()),
            ]),
        };
        body.to_line()
    }

    pub fn read(line: &str) -> Result<Self> {
        let value = json::parse(line)
            .map_err(|error| refused("a streamed line that is not one", &error.to_string()))?;
        if let Some(done) = value.get("done") {
            return Ok(Self::Done(done.clone()));
        }
        if let Some(progress) = value.get("progress") {
            let figure = |key: &str| {
                progress
                    .get(key)
                    .and_then(Value::as_integer)
                    .and_then(|figure| u64::try_from(figure).ok())
                    .unwrap_or(0)
            };
            return Ok(Self::Progress {
                read: figure("read"),
                of: figure("of"),
                produced: figure("produced"),
                seconds: figure("seconds"),
            });
        }
        match (
            value.get("token").and_then(Value::as_text),
            value.get("at").and_then(Value::as_integer),
        ) {
            (Some(text), Some(at)) => Ok(Self::Token {
                at: usize::try_from(at).unwrap_or(usize::MAX),
                text: text.to_owned(),
            }),
            _ => Err(refused(
                "a streamed line naming neither a token nor the end",
                line,
            )),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Answer {
    pub served: bool,
    pub body: Value,
}

impl Answer {
    #[must_use]
    pub fn served(body: Value) -> Self {
        Self { served: true, body }
    }

    #[must_use]
    pub fn refused(failure: &Failure) -> Self {
        Self {
            served: false,
            body: mcf_record::encode::failure(failure),
        }
    }

    #[must_use]
    pub fn refused_as(failure: Value) -> Self {
        Self {
            served: false,
            body: failure,
        }
    }

    #[must_use]
    pub fn to_line(&self) -> String {
        Value::map([
            ("protocol", Value::Integer(VERSION)),
            ("served", Value::Bool(self.served)),
            ("answer", self.body.clone()),
        ])
        .to_line()
    }

    pub fn read(line: &str) -> Result<Self> {
        let value = json::parse(line)
            .map_err(|error| refused("an answer that is not one", &error.to_string()))?;
        let served = match value.get("served") {
            Some(Value::Bool(served)) => *served,
            _ => {
                return Err(refused(
                    "an answer that does not say whether it is one",
                    line,
                ));
            }
        };
        let body = value
            .get("answer")
            .cloned()
            .ok_or_else(|| refused("an answer with nothing in it", line))?;
        Ok(Self { served, body })
    }
}

pub(crate) fn does_not_fit(why: &str, model: &str, on_a_card: bool) -> Failure {
    let kept: String = model.chars().take(200).collect();
    Failure::new(
        if on_a_card {
            Category::AccelMemoryExhausted
        } else {
            Category::ResourceMemoryExhausted
        },
        Attribution::Machine,
        Disposition::Refused,
        WHERE,
        why,
    )
    .with_context("model", kept)
}

pub(crate) fn refused(wanted: &str, found: &str) -> Failure {
    let kept: String = found.chars().take(200).collect();
    Failure::new(
        Category::ConfigInvalid,
        Attribution::User,
        Disposition::Refused,
        WHERE,
        "a client sent something MCF cannot read",
    )
    .with_context("wanted", wanted.to_owned())
    .with_context("found", kept)
}

#[cfg(test)]
mod tests;
