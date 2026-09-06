//! What a client may ask a running MCF, and what it may not (B-030, B-036).
//!
//! **A protocol before a daemon, for the reason the wire came before TLS.**
//! This module turns a request into bytes and bytes into a request and touches
//! no socket, so what a client and a daemon agree on can be exercised without
//! either. Everything hostile about a control plane — a request that is not
//! one, a request larger than anything MCF will read, a version this build does
//! not know — is decided here (§3.7).
//!
//! **It is deliberately tiny, and the reason is A19 rather than taste.** Three
//! requests: *what are you*, *what are you holding*, and *stop*. Serving a
//! model is not among them because MCF cannot yet run one, and a control plane
//! that advertised what it could not do would be the fabricated report C7 is
//! written against. B-040 adds to this list when there is an engine to add for.
//!
//! **One line per message.** The record is line-delimited JSON (D20) and so is
//! this, for the same reason: a torn message is a torn *line*, which a reader
//! can identify and report rather than being unable to find the boundary at
//! all. It also means a person can drive the control plane with `socat` and
//! read what came back, which A22 asks for — a surface only a client can reach
//! is a surface the laboratory cannot test.

use mcf_core::failure::{Attribution, Category, Disposition, Failure, Result, Subsystem};
use mcf_record::json::{self, Value};

const WHERE: Subsystem = Subsystem::new("mcf-serve::control");

/// The protocol this build speaks.
///
/// Sent in every request and every answer. §7.30's habit applied to a live
/// connection rather than to a record: a client from another version is told so
/// rather than being half-understood, and *half-understood* is the failure mode
/// a version number exists to prevent (C5).
pub const VERSION: i64 = 1;

/// How long a request may be.
///
/// Four mebibytes. It was sixty-four kibibytes, on the reasoning that *a
/// control request is a verb and a name* — true when it was written and false
/// since B-374, which made a request able to carry a turn of **token
/// identifiers**. A context of a hundred and thirty thousand of them, written
/// as decimal numbers with commas, is around nine hundred kilobytes, and the
/// old bound stopped a legitimate request at four thousand tokens.
///
/// It is still a stated number and not *whatever arrives*, which is what §3.7
/// asks for. What changed is the largest thing a request can honestly be, not
/// the principle that there is a largest.
pub const REQUEST_CEILING: usize = 4 * 1024 * 1024;

/// Where a measurement puts the model, where the caller chooses.
///
/// Two words rather than a layer count: what a person wants to know is
/// whether the figure is the card's or the processor's, and *the whole model
/// on the card* or *none of it* are the two conditions that answer that. A
/// partial offload is a real configuration and not one this chooses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum On {
    /// The processor, with nothing on a card.
    Processor,
    /// A card, with the whole model on it.
    Card,
}

impl On {
    /// The word on the wire and at the command line.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Processor => "processor",
            Self::Card => "card",
        }
    }

    /// The word, read; `cpu` and `gpu` are taken too, being what people type.
    #[must_use]
    pub fn parse(word: &str) -> Option<Self> {
        match word.trim().to_ascii_lowercase().as_str() {
            "processor" | "cpu" => Some(Self::Processor),
            "card" | "gpu" => Some(Self::Card),
            _ => None,
        }
    }
}

/// What a client is asking for.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Request {
    /// What this daemon is: its build, how long it has been up, what it will
    /// and will not do.
    Status,
    /// What models this machine is holding.
    Holding,
    /// What a prompt does to a model: which of its sentences reach the answer,
    /// and whether the answer settles.
    ///
    /// Served here rather than computed by a caller so that every surface — the
    /// command line, the window, the console — asks one thing and gets one
    /// answer (A22, B-072). It is expensive by construction: one generation per
    /// sentence and one per seed, which is why it is asked for and never done
    /// on the way past.
    PromptReport {
        /// A path, or a name under the daemon's store.
        model: String,
        /// The prompt to take apart, whole: a persona, an instruction sheet,
        /// or a question on its own (B-430).
        prompt: String,
        /// What to take it apart into; `None` lets the text decide.
        by: Option<crate::prompt::Unit>,
        /// How many parts to remove at most; `None` is the default cap.
        most: Option<usize>,
        /// The further readings asked for, each costing generations
        /// (B-434, B-435).
        extras: crate::prompt::Extras,
        /// How the turn is framed, where the caller asked the engine to
        /// frame it from the model's own template (D47): a persona read as
        /// a system turn is read where it will live (B-455).
        turn: Option<crate::turn::Turn>,
        /// The temperature the settledness seeds are drawn at, in
        /// thousandths; `None` spends no generation on the question (B-431).
        temperature: Option<mcf_core::configuration::Thousandths>,
        /// The seed, held still across every ablation so that what differs
        /// between a baseline and a clause left out is the prompt (D19).
        seed: u64,
    },
    /// What components MCF can build, and which of them are here.
    ///
    /// Read-only: it says what the catalogue holds and what is on the disk
    /// beside it. Building one is [`Request::Provision`].
    Components,
    /// Provision: build a component, streamed a line at a time as the build
    /// prints them, so that a surface can show minutes of compiling as
    /// something happening rather than something hung (B-367).
    ///
    /// Unnamed, the engine this machine needs — which the window asks for when
    /// a model is held and nothing here can run it. The daemon says what it
    /// chose in the first line, because a build the operator did not name is
    /// a choice MCF made and §3.15 wants it visible.
    Provision {
        /// The component's name in MCF's table, or `None` for the engine this
        /// machine needs.
        component: Option<String>,
    },
    /// Stop: refuse new work, finish what is in hand, and exit.
    Stop {
        /// Why, which is recorded. A26 makes stopping an act with an account
        /// rather than a signal that leaves no trace.
        reason: String,
    },
    /// Generate: a model answers a prompt, one line per token and a
    /// terminating line carrying the conditions (B-034, PR9).
    Generate {
        /// A path, or a name under the daemon's store.
        model: String,
        /// What to ask.
        prompt: String,
        /// How many tokens, if the caller said.
        ///
        /// `None` is not the same as the default. A caller that said nothing
        /// may be given a budget somebody derived for this model, and a caller
        /// that said `32` meant `32` — MCF never changes a value under
        /// somebody who set it (D43). The daemon substitutes, so the two stay
        /// distinguishable until the last moment.
        limit: Option<usize>,
        /// The seed, which is a condition of the answer (D19).
        seed: u64,
        /// The prompt already segmented, where the caller built it.
        ///
        /// A chat turn is a sequence of identifiers, and MCF assembles it from
        /// the model's own markers (D46) — so a caller that has done that
        /// sends the identifiers rather than text nobody could re-segment the
        /// same way. `prompt` stays beside it for the record and for a client
        /// that has only text.
        tokens: Option<Vec<usize>>,
        /// The turn as markers and text, where the caller built it and
        /// left the segmenting to the engine that answers (B-442).
        ///
        /// A caller that segmented for itself sends `tokens`; a caller that
        /// knows which markers go around its question but not how this
        /// model's vocabulary spells them as identifiers sends these, and
        /// the daemon reads them through the tokenizer of whichever engine
        /// answers — MCF's own where its engine does, the provisioned
        /// server's where that does. So a model whose vocabulary MCF's own
        /// tokenizer refuses can still be asked an addressed turn, by the
        /// engine that can read it (F158).
        pieces: Option<Vec<mcf_standin::tokenizer::Piece>>,
        /// Which engine, where the client says: `stand-in` for MCF's own,
        /// `provisioned` for the one MCF built. Absent means the daemon's
        /// stated rule: the provisioned engine where there is exactly one,
        /// MCF's own otherwise — and the account says which (§3.15).
        engine: Option<String>,
        /// Whose text this turn is: a person's, or MCF's own (B-146, §6.8).
        ///
        /// **It travels rather than being inferred.** The daemon cannot tell a
        /// probe's constant question from a person's `mcf run` by looking at
        /// it — they arrive on the same socket in the same shape — and §6.8
        /// requires suite data and user traffic be separated *structurally*.
        /// So the caller says, at the point it makes the request, and the
        /// answer decides which store the text is filed in and what the record
        /// says the generation was.
        ///
        /// A request that does not say is the operator's, which is the safe
        /// direction: MCF's own traffic filed under a person's retention is a
        /// tidiness problem, and a person's filed under MCF's is the privacy
        /// failure §6.8 exists to prevent.
        whose: mcf_record::content::Whose,
        /// Whether `limit` is the length rather than a ceiling on it.
        ///
        /// A timing divides a duration by a count of tokens, so the count
        /// has to be the one that was asked for: a pinned generation tells
        /// the engine to ignore the model's end of text and run to the limit,
        /// and the account's `tokens` is what proves it did (B-396, A21).
        /// The pin travels with the request rather than being assumed by
        /// whoever reads the account, because a declaration is not an
        /// observation — the ladder and `mcf bench` read the count back and
        /// keep no run that fell short of it.
        pinned: bool,
        /// How the turn around the prompt is framed, where the caller asked
        /// for the engine to frame it: thinking on or off, a reasoning
        /// effort, a system turn, each in the model's own template (D47).
        ///
        /// Absent, the prompt goes as it always did — under the addressing
        /// on file for the model, or bare. Present, the frame is the
        /// engine's rendering of the template with these switches, and the
        /// account says so.
        turn: Option<crate::turn::Turn>,
        /// A picture to show the model, as a path the daemon reads (B-452).
        ///
        /// A path and not the bytes: the client and the daemon share a
        /// machine and a user, and the record names the file rather than
        /// keeping a picture inside a line of it. A picture goes in a turn
        /// the engine frames from the model's own template, since that is
        /// the only turn with a place for one.
        image: Option<String>,
        /// What the engine is started with beyond the plain load, where the
        /// caller asked for either: the model's own draft head, a scaling
        /// for the positions. Nothing is on unless it was asked for, and a
        /// request that says nothing is the plain load (B-456, D43).
        started: crate::declared::Started,
    },
    /// What a hub publishes under a reference, and which of it will run here.
    ///
    /// **Reading, never fetching.** This is what a person is asking when they
    /// name a repository and no file: not *what exists* but *which of these is
    /// for my machine* (PR3, B-213). It acquires nothing, so it is safe to
    /// send while somebody is still typing.
    Offered {
        /// The reference, as a person wrote it.
        reference: String,
        /// A hub other than the default, where the caller says.
        from: Option<String>,
        /// Whether to ask the hub again rather than answer from what was
        /// kept of its last answer within the day (B-488).
        fresh: bool,
    },
    /// The probes, as one run the daemon carries: each announced as it
    /// starts and its finding as it lands (B-478, D50).
    Probe {
        /// A path, or a name under the daemon's store.
        model: String,
        /// Which engine to ask through, where the caller named one.
        engine: Option<String>,
        /// Whether to apply what was observed, which is an act (D43).
        apply: bool,
        /// The longest prompt the usable-context probe may ask for (B-461).
        up_to: Option<usize>,
        /// Which probes, by name; empty is all of them.
        only: Vec<String>,
    },
    /// The measurements, as one run the daemon carries: each announced as
    /// it starts and its finding as it lands, the same stream a probe run
    /// has (D52).
    Examine {
        /// A path, or a name under the daemon's store.
        model: String,
        /// Which engine to measure through, where the caller named one.
        engine: Option<String>,
        /// Which measurements, by name; empty is all of them.
        only: Vec<String>,
    },
    /// Which repositories a hub lists for a word, most downloaded first.
    Search {
        /// The word, as a person typed it.
        query: String,
        /// A hub other than the default, where the caller says.
        from: Option<String>,
        /// Whether to ask the hub again rather than answer from what was
        /// kept of its last answer within the day (B-488).
        fresh: bool,
    },
    /// Fetch one published file into this machine's store.
    ///
    /// The file is named rather than chosen by MCF: [`Self::Offered`] said
    /// which would run and what each costs, and choosing between them is the
    /// operator's (§3.15). What comes back is one line per step of progress
    /// and a last line carrying the provenance that was written down.
    Acquire {
        /// The reference, as a person wrote it.
        reference: String,
        /// Which published file.
        file: String,
        /// A hub other than the default.
        from: Option<String>,
    },
    /// Time a model on this machine, at doubling depths.
    ///
    /// One line per depth as it is measured, so that a window can show a bar
    /// moving and an estimate narrowing, and a last line carrying the readings
    /// and the conditions they were taken under (A6).
    Measure {
        /// A path, or a name under the daemon's store.
        model: String,
        /// Which engine, where the caller says. Absent means the daemon's
        /// stated rule.
        engine: Option<String>,
        /// Where the model is put, where the caller says: the processor or
        /// the card. Absent means where MCF resolves it to, which a surface
        /// shows before the run and the conditions name after it.
        on: Option<On>,
        /// The deepest context to sample, which implies every power of two
        /// below it — the shallow points are what the deep one is read
        /// against, so a run that skipped them would be a run whose deepest
        /// number meant nothing.
        deepest: u64,
        /// What the engine is started with beyond the plain load, where the
        /// caller asked for either. A timing under one of them is a timing
        /// of that condition and not of the plain load, so it travels with
        /// the request and is named in what comes back (B-463, B-456).
        started: crate::declared::Started,
    },
    /// Read what the provisioned engine produces from one model with MCF's
    /// own engine, and say whether the two agree (B-362, B-424).
    ///
    /// Many lines: what is about to run and what it is expected to cost, what
    /// the provisioned engine produced, and a last line carrying the
    /// agreement in figures and in sentences — the same sentences on every
    /// surface, because they are composed once, here, and never at a surface
    /// (B-072). The prompt and the length are the daemon's
    /// ([`crate::crosscheck::PROMPT`], [`crate::crosscheck::POSITIONS`]) so
    /// that every cross-check is the same question.
    CrossCheck {
        /// A path, or a name under the daemon's store.
        model: String,
    },
    /// What MCF would run this model under, and what it recommends.
    ///
    /// Reading, never starting: a surface asks this to fill in a form, and a
    /// form that started a server to be drawn would be a form nobody could
    /// open twice.
    Settings {
        /// A path, or a name under the daemon's store.
        model: String,
    },
    /// Hold a model and answer on a port under these settings.
    Host {
        /// A path, or a name under the daemon's store.
        model: String,
        /// What to run it under. Anything absent keeps what MCF recommends —
        /// a caller who said nothing has not asked for a setting's lowest
        /// value (A7, D43).
        settings: Value,
    },
    /// What is being hosted, if anything.
    Hosted,
    /// Stop holding it.
    Unhost,
    /// What a model is made of: its directory counted, its header set against
    /// the directory, and what one token costs in arithmetic.
    ///
    /// Reading, never running: the header and the tensor directory are read
    /// off the file's prefix, and nothing is loaded. Served here so that the
    /// window shows what `mcf explain` prints, from one counting (A22, B-072).
    Anatomy {
        /// A path, or a name under the daemon's store.
        model: String,
    },
    /// How many identifiers a text costs this model, read by the tokenizer
    /// of the engine that would answer for it (B-442).
    ///
    /// The count a generation would be charged, from the tokenizer that
    /// would charge it: the provisioned server's where the model resolves
    /// to one, MCF's own where its engine answers. A count taken through
    /// any other tokenizer is a count of a different reading (F158).
    Tokenize {
        /// A path, or a name under the daemon's store.
        model: String,
        /// The text to count, read as a person's text: a marker spelled in
        /// it stays spelled.
        text: String,
        /// Which engine, as a generation names one; absent for the daemon's
        /// stated rule.
        engine: Option<String>,
        /// Whether to read it as the start of a turn, with the beginning
        /// marker the model's own convention asks for — what a prompt costs
        /// — or as text alone, which is what a sentence costs.
        beginning: bool,
    },
}

/// An optional string, as the protocol carries one.
fn maybe(held: Option<&str>) -> Value {
    held.map_or(Value::Null, Value::text)
}

/// A generation, as it goes onto the wire.
///
/// Its own function for the same reason the prompt report's is: `to_line` is
/// one arm per request under a line cap, and the longest arm crowds every
/// other one.
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

/// A prompt report, as it goes onto the wire.
///
/// Its own function because `to_line` is one match arm per request and the
/// whole of it has a line cap: a request added inside it is a request that
/// makes every other one harder to read.
/// The line a measurement goes as.
/// Whether what a person typed names a repository — `owner/name`, with a
/// file or a revision, or a hub URL — rather than being a word to search
/// for. Answered here so that a surface can decide which to ask without
/// reaching past the wire to the hub itself.
#[must_use]
pub fn is_reference(typed: &str) -> bool {
    mcf_hub::reference::parse(typed).is_ok()
}

/// The search's line: the word, and the hub where one was named.
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

fn measure_line(request: &Request) -> Value {
    let Request::Measure {
        model,
        engine,
        on,
        deepest,
        started,
    } = request
    else {
        return Value::Null;
    };
    Value::map([
        ("ask", Value::text("measure")),
        ("model", Value::text(model.clone())),
        ("engine", maybe(engine.as_deref())),
        ("on", maybe(on.map(On::as_str))),
        (
            "deepest",
            Value::Integer(i64::try_from(*deepest).unwrap_or(i64::MAX)),
        ),
        ("started_with", started.to_value()),
    ])
}

fn probe_line(request: &Request) -> Value {
    let Request::Probe {
        model,
        engine,
        apply,
        up_to,
        only,
    } = request
    else {
        return Value::Null;
    };
    let mut fields = vec![
        ("ask", Value::text("probe")),
        ("model", Value::text(model.clone())),
        ("apply", Value::Bool(*apply)),
        (
            "only",
            Value::List(only.iter().cloned().map(Value::text).collect()),
        ),
    ];
    if let Some(engine) = engine {
        fields.push(("engine", Value::text(engine.clone())));
    }
    if let Some(up_to) = up_to {
        fields.push((
            "up_to",
            Value::Integer(i64::try_from(*up_to).unwrap_or(i64::MAX)),
        ));
    }
    Value::map(fields)
}

fn examine_line(model: &str, engine: Option<&str>, only: &[String]) -> Value {
    let mut fields = vec![
        ("ask", Value::text("examine")),
        ("model", Value::text(model.to_owned())),
        (
            "only",
            Value::List(only.iter().cloned().map(Value::text).collect()),
        ),
    ];
    if let Some(engine) = engine {
        fields.push(("engine", Value::text(engine.to_owned())));
    }
    Value::map(fields)
}

fn prompt_report_line(request: &Request) -> Value {
    let Request::PromptReport {
        model,
        prompt,
        by,
        most,
        extras,
        turn,
        temperature,
        seed,
    } = request
    else {
        return Value::Null;
    };
    let mut fields = vec![
        ("ask", Value::text("prompt-report")),
        ("model", Value::text(model.clone())),
        ("prompt", Value::text(prompt.clone())),
        ("seed", Value::Integer(i64::try_from(*seed).unwrap_or(0))),
    ];
    if let Some(by) = by {
        fields.push(("by", Value::text(by.name().to_owned())));
    }
    if let Some(most) = most {
        fields.push(("most", Value::Integer(i64::try_from(*most).unwrap_or(0))));
    }
    let asked: Vec<Value> = extras
        .asked()
        .map(|extra| Value::text(extra.name()))
        .collect();
    if !asked.is_empty() {
        fields.push(("extras", Value::List(asked)));
    }
    if let Some(turn) = turn {
        fields.push(("turn", turn.to_value()));
    }
    if let Some(temperature) = temperature {
        fields.push((
            "temperature_thousandths",
            Value::Integer(i64::from(temperature.0)),
        ));
    }
    Value::map(fields)
}

impl Request {
    /// The line a client sends.
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
            Self::Provision { component } => {
                let mut fields = vec![("ask", Value::text("provision"))];
                if let Some(component) = component {
                    fields.push(("component", Value::text(component.clone())));
                }
                Value::map(fields)
            }
            Self::PromptReport { .. } => prompt_report_line(self),
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
            Self::Probe { .. } => probe_line(self),
            Self::Examine {
                model,
                engine,
                only,
            } => examine_line(model, engine.as_deref(), only),
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
            Self::Tokenize {
                model,
                text,
                engine,
                beginning,
            } => Value::map([
                ("ask", Value::text("tokenize")),
                ("model", Value::text(model.clone())),
                ("text", Value::text(text.clone())),
                ("engine", maybe(engine.as_deref())),
                ("beginning", Value::Bool(*beginning)),
            ]),
            Self::Host { model, settings } => Value::map([
                ("ask", Value::text("host")),
                ("model", Value::text(model.clone())),
                ("settings", settings.clone()),
            ]),
            Self::Hosted => Value::map([("ask", Value::text("hosted"))]),
            Self::Unhost => Value::map([("ask", Value::text("unhost"))]),
            Self::CrossCheck { model } => Value::map([
                ("ask", Value::text("cross_check")),
                ("model", Value::text(model.clone())),
            ]),
            Self::Measure { .. } => measure_line(self),
        };
        let Value::Map(mut fields) = body else {
            return String::new();
        };
        fields.insert("protocol".to_owned(), Value::Integer(VERSION));
        Value::Map(fields).to_line()
    }

    /// Reads what a client sent.
    ///
    /// # Errors
    ///
    /// `config.invalid` for a line that is not a request, naming what was seen
    /// but bounded — a client is untrusted, and a refusal that quoted a
    /// megabyte back would be a way of writing to MCF's own output (§3.7, A1).
    /// `exchange.schema.unreadable` for a protocol version this build does not
    /// speak, which is a different thing from a request it does not have.
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
            Some("probe") => Ok(Self::Probe {
                model: value
                    .get("model")
                    .and_then(Value::as_text)
                    .ok_or_else(|| refused("a probe naming no model", line))?
                    .to_owned(),
                engine: optional("engine"),
                apply: matches!(value.get("apply"), Some(Value::Bool(true))),
                up_to: value
                    .get("up_to")
                    .and_then(Value::as_integer)
                    .and_then(|held| usize::try_from(held).ok()),
                only: value
                    .get("only")
                    .and_then(Value::as_list)
                    .unwrap_or(&[])
                    .iter()
                    .filter_map(Value::as_text)
                    .map(str::to_owned)
                    .collect(),
            }),
            Some("examine") => Ok(Self::Examine {
                model: value
                    .get("model")
                    .and_then(Value::as_text)
                    .ok_or_else(|| refused("an examination naming no model", line))?
                    .to_owned(),
                engine: optional("engine"),
                only: value
                    .get("only")
                    .and_then(Value::as_list)
                    .unwrap_or(&[])
                    .iter()
                    .filter_map(Value::as_text)
                    .map(str::to_owned)
                    .collect(),
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
            Some("tokenize") => Ok(Self::Tokenize {
                model: value
                    .get("model")
                    .and_then(Value::as_text)
                    .ok_or_else(|| refused("a tokenize request naming no model", line))?
                    .to_owned(),
                text: value
                    .get("text")
                    .and_then(Value::as_text)
                    .ok_or_else(|| refused("a tokenize request with no text to count", line))?
                    .to_owned(),
                engine: value
                    .get("engine")
                    .and_then(Value::as_text)
                    .map(str::to_owned),
                beginning: value
                    .get("beginning")
                    .and_then(Value::as_bool)
                    .unwrap_or(false),
            }),
            Some("host") => Ok(Self::Host {
                model: value
                    .get("model")
                    .and_then(Value::as_text)
                    .ok_or_else(|| refused("a hosting request naming no model", line))?
                    .to_owned(),
                settings: value.get("settings").cloned().unwrap_or(Value::Null),
            }),
            Some("cross_check") => Ok(Self::CrossCheck {
                model: value
                    .get("model")
                    .and_then(Value::as_text)
                    .ok_or_else(|| refused("a cross-check naming no model", line))?
                    .to_owned(),
            }),
            Some("measure") => Ok(Self::Measure {
                model: value
                    .get("model")
                    .and_then(Value::as_text)
                    .ok_or_else(|| refused("a measurement naming no model", line))?
                    .to_owned(),
                engine: optional("engine"),
                on: match value.get("on").and_then(Value::as_text) {
                    None => None,
                    Some(word) => Some(On::parse(word).ok_or_else(|| {
                        refused("a measurement naming a device MCF does not place on", word)
                    })?),
                },
                // A measurement with no ceiling would be one that runs until
                // the machine runs out, which is not a diagnostic but an
                // accident. The caller says how deep, always.
                deepest: value
                    .get("deepest")
                    .and_then(Value::as_integer)
                    .and_then(|deepest| u64::try_from(deepest).ok())
                    .ok_or_else(|| refused("a measurement naming no depth", line))?,
                // Absent is the plain load, which is what a client that
                // predates the field asked for.
                started: value
                    .get("started_with")
                    .map(crate::declared::Started::from_value)
                    .unwrap_or_default(),
            }),
            Some("holding") => Ok(Self::Holding),
            Some("components") => Ok(Self::Components),
            Some("provision") => Ok(Self::Provision {
                component: optional("component"),
            }),
            Some("prompt-report") => Ok(Self::PromptReport {
                model: value
                    .get("model")
                    .and_then(Value::as_text)
                    .ok_or_else(|| refused("a prompt report naming no model", line))?
                    .to_owned(),
                prompt: value
                    .get("prompt")
                    .and_then(Value::as_text)
                    .ok_or_else(|| refused("a prompt report with no prompt", line))?
                    .to_owned(),
                by: match value.get("by").and_then(Value::as_text) {
                    None => None,
                    Some(word) => Some(crate::prompt::Unit::named(word).ok_or_else(|| {
                        refused(
                            "a prompt report taking the text apart by something that is \
                             none of word, phrase, sentence or paragraph",
                            line,
                        )
                    })?),
                },
                most: match value.get("most") {
                    None => None,
                    Some(most) => Some(
                        most.as_integer()
                            .and_then(|most| usize::try_from(most).ok())
                            .filter(|most| *most > 0)
                            .ok_or_else(|| {
                                refused("a prompt report removing no parts at most", line)
                            })?,
                    ),
                },
                // Absent means the prompt goes as it always did: a client
                // that predates the field asked for no frame.
                turn: value.get("turn").and_then(crate::turn::Turn::from_value),
                extras: crate::prompt::Extras::named(
                    value
                        .get("extras")
                        .and_then(Value::as_list)
                        .unwrap_or(&[])
                        .iter()
                        .filter_map(Value::as_text),
                ),
                // In thousandths, so that no fraction crosses the wire (A19).
                // Nought is not a temperature to settle at: it is what every
                // other generation draws at, and asking for it would spend
                // three generations on a question the sampler cannot answer.
                temperature: match value.get("temperature_thousandths") {
                    None => None,
                    Some(held) => Some(
                        held.as_integer()
                            .and_then(|held| u32::try_from(held).ok())
                            .filter(|held| *held > 0)
                            .map(mcf_core::configuration::Thousandths)
                            .ok_or_else(|| {
                                refused(
                                    "a prompt report settling at a temperature that is not \
                                     above nought",
                                    line,
                                )
                            })?,
                    ),
                },
                seed: value
                    .get("seed")
                    .and_then(Value::as_integer)
                    .and_then(|held| u64::try_from(held).ok())
                    .unwrap_or(0),
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
                    // A request that names no budget takes the stated default;
                    // a request that names no seed takes zero, and both are
                    // said back in the terminating line (§3.15).
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
                    // Absent means no turn was built: a client that predates
                    // the field sent text or identifiers.
                    pieces: value
                        .get("pieces")
                        .and_then(crate::configured::pieces_from_value),
                    engine: value
                        .get("engine")
                        .and_then(Value::as_text)
                        .map(str::to_owned),
                    // Absent means the operator's, which is the safe
                    // direction: a request from a client that predates this
                    // field is a person's until something says otherwise, and
                    // a name this build does not know is treated the same way
                    // rather than guessed at (A7, B-146).
                    whose: value
                        .get("whose")
                        .and_then(Value::as_text)
                        .and_then(mcf_record::content::Whose::parse)
                        .unwrap_or(mcf_record::content::Whose::User),
                    // Absent means a ceiling, which is what every request
                    // was before there was a pin: a client that predates the
                    // field asked for at most `limit`, and is given that.
                    pinned: value
                        .get("pinned")
                        .and_then(Value::as_bool)
                        .unwrap_or(false),
                    // Absent means the prompt goes as it always did: a
                    // client that predates the field asked for no frame.
                    turn: value.get("turn").and_then(crate::turn::Turn::from_value),
                    // Absent means nothing shown: a client that predates the
                    // field asked with words alone.
                    image: value
                        .get("image")
                        .and_then(Value::as_text)
                        .map(str::to_owned),
                    // Absent means the plain load: a client that predates
                    // the field asked for neither switch, which is what
                    // every client that does not say is asking for.
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

/// How many tokens a generation produces when the request does not say.
///
/// The same number `mcf run` uses when nobody says, for the same reason: a
/// budget in tokens rather than seconds (B49), and one number in one place.
pub const DEFAULT_LIMIT: usize = 32;

/// One line of a streamed generation (PR9).
///
/// A token line carries the token's text and its index; the terminating line
/// carries everything else — count, why it stopped, the conditions, and the
/// mark. A client that reads lines until it sees `done` has the whole answer,
/// and a client that hangs up early has the tokens it was sent (A4).
#[derive(Debug, Clone, PartialEq)]
pub enum Streamed {
    /// One token, as text, at its index.
    Token {
        /// Its index in the generation.
        at: usize,
        /// The text it decodes to.
        text: String,
    },
    /// How far the engine has got, while the client waits (D48, B-458).
    ///
    /// Sent every ten seconds while a turn is being read or answered, so a
    /// client waiting an hour can see the hour being spent; never sent for a
    /// turn that is done inside ten seconds.
    Progress {
        /// Identifiers of the turn the engine has read so far.
        read: u64,
        /// Identifiers the turn holds.
        of: u64,
        /// Identifiers produced so far.
        produced: u64,
        /// Seconds since the request reached the engine.
        seconds: u64,
    },
    /// The end, with the account.
    Done(Value),
}

impl Streamed {
    /// The line the daemon writes.
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

    /// Reads one line of a stream.
    ///
    /// # Errors
    ///
    /// `config.invalid` for a line that is neither a token nor the end.
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

/// What the daemon says back.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Answer {
    /// Whether MCF could serve the request.
    pub served: bool,
    /// The answer itself, or what went wrong.
    pub body: Value,
}

impl Answer {
    /// An answer MCF could give.
    #[must_use]
    pub fn served(body: Value) -> Self {
        Self { served: true, body }
    }

    /// A classified refusal, in the record's own shape so that a client reads
    /// the same structure a record holds (A2, C1).
    #[must_use]
    pub fn refused(failure: &Failure) -> Self {
        Self {
            served: false,
            body: mcf_record::encode::failure(failure),
        }
    }

    /// A refusal already in the record's shape: a failure one generation
    /// reported, passed on whole rather than described again (A2).
    #[must_use]
    pub fn refused_as(failure: Value) -> Self {
        Self {
            served: false,
            body: failure,
        }
    }

    /// The line the daemon sends.
    #[must_use]
    pub fn to_line(&self) -> String {
        Value::map([
            ("protocol", Value::Integer(VERSION)),
            ("served", Value::Bool(self.served)),
            ("answer", self.body.clone()),
        ])
        .to_line()
    }

    /// Reads what a daemon sent.
    ///
    /// # Errors
    ///
    /// `config.invalid` for a line that is not an answer.
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

/// What a client said, kept but bounded.
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
