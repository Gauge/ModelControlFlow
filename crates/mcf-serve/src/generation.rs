use std::fmt;
use std::io::Write as _;
use std::os::unix::net::UnixStream;
use std::path::Path;

use mcf_core::configuration::Thousandths;
use mcf_core::failure::Failure;
use mcf_record::json::Value;
use mcf_standin::gguf;
use mcf_standin::llama;
use mcf_standin::sample::Settings;
use mcf_standin::session::{self, Request, Stopped};
use mcf_standin::tokenizer::Vocabulary;

use crate::control::Streamed;
use crate::served::{Completed, Served};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Draw {
    pub seed: u64,
    pub temperature: Thousandths,
    pub truncation: Truncation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stated<T> {
    Declared(T),
    Off,
}

impl<T: fmt::Display> Stated<T> {
    fn value(self) -> Value {
        match self {
            Self::Declared(held) => Value::text(held.to_string()),
            Self::Off => Value::text("off"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Truncation {
    pub top_k: Stated<u32>,
    pub top_p: Stated<Thousandths>,
    pub min_p: Stated<Thousandths>,
    pub whose: Whose,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Whose {
    File,
    NoneDeclared,
    Moot,
}

impl Whose {
    #[must_use]
    pub const fn said(self) -> &'static str {
        match self {
            Self::File => "declared by the file",
            Self::NoneDeclared => "none declared by the file",
            Self::Moot => "moot under greedy",
        }
    }
}

impl Truncation {
    pub const OFF: Self = Self {
        top_k: Stated::Off,
        top_p: Stated::Off,
        min_p: Stated::Off,
        whose: Whose::Moot,
    };

    #[must_use]
    pub fn recommended(sampling: Option<&mcf_core::configuration::Sampling>) -> Self {
        use mcf_core::attested::Attested;
        let Some(sampling) = sampling else {
            return Self {
                whose: Whose::NoneDeclared,
                ..Self::OFF
            };
        };
        let fraction = |held: Attested<Thousandths>| match held {
            Attested::Known(value) => Stated::Declared(value),
            Attested::Unknown => Stated::Off,
        };
        let mut cut = Self {
            top_k: match sampling.top_k {
                Attested::Known(value) => Stated::Declared(value),
                Attested::Unknown => Stated::Off,
            },
            top_p: fraction(sampling.top_p),
            min_p: fraction(sampling.min_p),
            whose: Whose::File,
        };
        if !cut.any_stated() {
            cut.whose = Whose::NoneDeclared;
        }
        cut
    }

    #[must_use]
    pub const fn any_stated(self) -> bool {
        !matches!(
            (self.top_k, self.top_p, self.min_p),
            (Stated::Off, Stated::Off, Stated::Off)
        )
    }

    #[must_use]
    pub const fn is_declared(self) -> bool {
        matches!(self.whose, Whose::File)
    }

    #[must_use]
    pub const fn top_k_sent(self) -> u32 {
        match self.top_k {
            Stated::Declared(held) => held,
            Stated::Off => 0,
        }
    }

    #[must_use]
    pub const fn top_p_sent(self) -> Thousandths {
        match self.top_p {
            Stated::Declared(held) => held,
            Stated::Off => Thousandths(1_000),
        }
    }

    #[must_use]
    pub const fn min_p_sent(self) -> Thousandths {
        match self.min_p {
            Stated::Declared(held) => held,
            Stated::Off => Thousandths(0),
        }
    }

    #[must_use]
    pub fn entries(self) -> [(&'static str, Value); 4] {
        [
            ("top_k", self.top_k.value()),
            ("top_p", self.top_p.value()),
            ("min_p", self.min_p.value()),
            ("truncation", Value::text(self.whose.said())),
        ]
    }

    #[must_use]
    pub fn line(self) -> String {
        self.entries()
            .into_iter()
            .take(3)
            .map(|(name, value)| format!("{name} {}", value.as_text().unwrap_or("?")))
            .collect::<Vec<String>>()
            .join(" · ")
    }
}

impl Draw {
    #[must_use]
    pub const fn greedy(seed: u64) -> Self {
        Self {
            seed,
            temperature: Thousandths(0),
            truncation: Truncation::OFF,
        }
    }

    #[must_use]
    pub const fn is_greedy(self) -> bool {
        self.temperature.0 == 0
    }

    #[must_use]
    pub fn sampler(self) -> String {
        if self.is_greedy() {
            "greedy".to_owned()
        } else {
            format!("temperature {}", self.temperature)
        }
    }

    fn settings(self) -> Settings {
        Settings::at_thousandths(
            self.temperature.0,
            usize::try_from(self.truncation.top_k_sent()).unwrap_or(usize::MAX),
            self.truncation.top_p_sent().0,
            self.truncation.min_p_sent().0,
        )
    }
}

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

pub(crate) struct Produced {
    pub(crate) account: Value,
    pub(crate) said: Option<Said>,
}

pub(crate) struct Said {
    pub(crate) text: String,
    pub(crate) tokens: Vec<usize>,
    pub(crate) answer: Option<String>,
}

impl Produced {
    fn on_the_wire(&self) -> Value {
        let Value::Map(fields) = &self.account else {
            return self.account.clone();
        };
        let mut fields = fields.clone();
        if let Some(said) = &self.said {
            fields.insert("text".to_owned(), Value::text(said.text.clone()));
            if let (Some(answer), Some(Value::Map(before))) =
                (&said.answer, fields.get_mut("before_the_answer"))
            {
                before.insert("answer".to_owned(), Value::text(answer.clone()));
            }
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

#[derive(Debug, Clone, Copy)]
pub(crate) enum Given<'a> {
    Text,
    Identifiers(&'a [usize]),
    Pieces(&'a [mcf_standin::tokenizer::Piece]),
}

impl<'a> Given<'a> {
    pub(crate) fn from_request(
        tokens: Option<&'a [usize]>,
        pieces: Option<&'a [mcf_standin::tokenizer::Piece]>,
    ) -> Self {
        match (tokens, pieces) {
            (Some(tokens), _) => Self::Identifiers(tokens),
            (None, Some(pieces)) => Self::Pieces(pieces),
            (None, None) => Self::Text,
        }
    }

    pub(crate) fn identifiers(self) -> Option<&'a [usize]> {
        match self {
            Self::Identifiers(tokens) => Some(tokens),
            Self::Text | Self::Pieces(_) => None,
        }
    }

    pub(crate) fn is_own_turn(self) -> bool {
        !matches!(self, Self::Text)
    }

    pub(crate) fn trailing_markers(self) -> Option<String> {
        let Self::Pieces(pieces) = self else {
            return None;
        };
        trailing_markers_of(pieces)
    }
}

fn trailing_markers_of(pieces: &[mcf_standin::tokenizer::Piece]) -> Option<String> {
    let markers: Vec<&str> = pieces
        .iter()
        .rev()
        .map_while(|piece| match piece {
            mcf_standin::tokenizer::Piece::Marker(marker) => Some(marker.as_str()),
            mcf_standin::tokenizer::Piece::Text(_) => None,
        })
        .collect();
    (!markers.is_empty()).then(|| markers.into_iter().rev().collect())
}

fn tail_of<'a>(
    frame: Option<&'a crate::turn::Frame>,
    trailing: Option<&'a str>,
    given: Given<'_>,
    derived: Option<&'a String>,
) -> Option<&'a str> {
    frame
        .map(|frame| frame.after.as_str())
        .or(trailing)
        .or_else(|| matches!(given, Given::Text).then_some(derived?.as_str()))
}

fn derived_for(
    mcf_home: &Path,
    model: &Path,
    limit: Option<usize>,
) -> (
    Option<crate::configured::Addressing>,
    Option<crate::configured::Budget>,
    usize,
) {
    let derived = crate::configured::read_derived(mcf_home, model);
    let limit = limit
        .or_else(|| derived.budget.as_ref().map(|budget| budget.tokens))
        .unwrap_or(crate::control::DEFAULT_LIMIT);
    (derived.addressing, derived.budget, limit)
}

fn wrapped_turn(
    place: &Place<'_>,
    prompt: &str,
    given: Given<'_>,
    framed: Result<&Option<crate::turn::Frame>, &Failure>,
    derived: Option<&crate::configured::Addressing>,
    chosen: &Result<Chosen, Failure>,
) -> Result<Option<Vec<usize>>, Failure> {
    use Given::{Pieces, Text};
    let read = |read: Vec<Read>| Some(read.into_iter().map(|held| held.id).collect());
    match (given, framed, derived, chosen) {
        (_, Err(failure), _, _) => Err(failure.clone()),
        (Pieces(pieces), _, _, Ok(Chosen::Provisioned(llama))) => {
            place.engine(llama).addressed(pieces).map(read)
        }
        (Pieces(pieces), _, _, Ok(Chosen::StandIn)) => {
            Tokenizer::own(&resolved(place.store, place.named))
                .and_then(|tokenizer| tokenizer.addressed(pieces))
                .map(read)
        }
        (Text, Ok(Some(frame)), _, Ok(Chosen::Provisioned(llama))) => {
            framed_as(&place.engine(llama), prompt, frame).map(Some)
        }
        (Text, _, Some(addressing), Ok(Chosen::Provisioned(llama))) => {
            addressed_as(&place.engine(llama), prompt, addressing).map(Some)
        }
        (Text, _, Some(addressing), Ok(Chosen::StandIn)) => {
            Tokenizer::own(&resolved(place.store, place.named))
                .and_then(|tokenizer| addressed_as(&tokenizer, prompt, addressing))
                .map(Some)
        }
        (Text, _, None, Ok(Chosen::Provisioned(llama))) => {
            place.engine(llama).encode(prompt, true).map(read)
        }
        _ => Ok(None),
    }
}

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
    draw: Draw,
    given: Given<'_>,
    engine: Option<&str>,
    picked: Option<(crate::adapters::ProvisionedLlama, u32, u64)>,
    free: Option<u64>,
    pinned: bool,
    cached: bool,
    turn: Option<&crate::turn::Turn>,
    picture: Option<&Path>,
    started: crate::declared::Started,
    held: Option<&Served>,
    waiting: crate::served::Waiting<'_>,
    writer: &mut &UnixStream,
) -> Produced {
    let (derived, derived_budget, limit) = derived_for(mcf_home, &resolved(store, named), limit);

    let declared = crate::declared::Declared::of(&resolved(store, named));
    let (chosen, gpu_layers, context) = engine_for(mcf_home, picked, engine, started, &declared);
    let place = Place {
        store,
        runtime,
        named,
        gpu_layers,
        context,
        server,
        started,
        held,
    };
    let picture = picture.map(Picture::read).transpose();
    let shown_turn = crate::turn::Turn::default();
    let turn = match (turn, &picture) {
        (None, Ok(Some(_))) => Some(&shown_turn),
        (turn, _) => turn,
    };
    let framed = framed_turn(&place, given.is_own_turn(), turn, &chosen);
    let wrapped = wrapped_turn(
        &place,
        prompt,
        given,
        framed.as_ref(),
        derived.as_ref(),
        &chosen,
    );
    let frame = framed.ok().flatten();
    let shown = match (&picture, &frame) {
        (Ok(Some(picture)), Some(frame)) => Some(Shown {
            frame,
            prompt,
            picture,
        }),
        _ => None,
    };
    let addressed = frame.as_ref().map_or_else(
        || addressing_label(derived.as_ref(), given, wrapped.is_ok()),
        |frame| Some(framed_label(frame, shown.is_some())),
    );
    let charged = match &chosen {
        Ok(Chosen::Provisioned(llama)) => Some(served_engine_name(llama)),
        Ok(Chosen::StandIn) | Err(_) => None,
    };
    let trailing = given.trailing_markers();
    let applied = derived
        .as_ref()
        .and_then(|addressing| trailing_markers_of(&addressing.after));
    let tail = tail_of(frame.as_ref(), trailing.as_deref(), given, applied.as_ref());
    let produced = match (chosen, wrapped, &picture) {
        (Ok(Chosen::StandIn), _, Ok(Some(_))) => Err(text_only()),
        (_, Err(failure), _) | (Err(failure), _, _) => Err(failure),
        (_, _, Err(failure)) => Err(failure.clone()),
        (Ok(Chosen::Provisioned(llama)), Ok(wrapped), Ok(_)) => {
            match wrapped.as_deref().or(given.identifiers()) {
                Some(tokens) => {
                    let sent = Sent {
                        tokens,
                        limit,
                        pinned,
                        cached,
                        tail,
                        shown,
                    };
                    through_served(
                        store, &llama, server, held, runtime, named, &sent, draw, gpu_layers,
                        context, started, &declared, waiting, writer,
                    )
                }
                None => {
                    through_provisioned(store, &llama, named, prompt, limit, draw, pinned, writer)
                }
            }
        }
        (Ok(Chosen::StandIn), Ok(wrapped), Ok(None)) => attempt(
            store,
            resident,
            named,
            prompt,
            wrapped.as_deref().or(given.identifiers()),
            limit,
            draw,
            free,
            pinned,
            writer,
        ),
    };
    let produced = with_provenance(produced, addressed, derived_budget);
    let produced = match produced {
        Ok(produced) => produced,
        Err(failure) => refused_account(&failure, named, draw, limit, charged),
    };
    let _written = writeln!(
        writer,
        "{}",
        Streamed::Done(produced.on_the_wire()).to_line()
    );
    let _flushed = writer.flush();
    produced
}

fn text_only() -> Failure {
    Failure::new(
        mcf_core::failure::Category::ConfigInvalid,
        mcf_core::failure::Attribution::User,
        mcf_core::failure::Disposition::Refused,
        mcf_core::failure::Subsystem::new("mcf-serve::generation"),
        "MCF's own engine reads text only, so it cannot be shown a picture: the provisioned \
         engine can, where the model has a projector beside it",
    )
}

fn engine_for(
    mcf_home: &Path,
    picked: Option<(crate::adapters::ProvisionedLlama, u32, u64)>,
    engine: Option<&str>,
    started: crate::declared::Started,
    declared: &crate::declared::Declared,
) -> (Result<Chosen, Failure>, u32, u64) {
    let (chosen, gpu_layers, context) = chosen_engine(mcf_home, picked, engine);
    let chosen = chosen.and_then(|chosen| match chosen {
        Chosen::StandIn if started.asks_anything() => Err(no_extras()),
        chosen => started.against(declared).map(|()| chosen),
    });
    (chosen, gpu_layers, context)
}

fn no_extras() -> Failure {
    Failure::new(
        mcf_core::failure::Category::ConfigInvalid,
        mcf_core::failure::Attribution::User,
        mcf_core::failure::Disposition::Refused,
        mcf_core::failure::Subsystem::new("mcf-serve::generation"),
        "MCF's own engine runs the weights as the file lays them out: it has no draft head to \
         start and no scaling to set. The provisioned engine takes both",
    )
}

fn chosen_engine(
    mcf_home: &Path,
    picked: Option<(crate::adapters::ProvisionedLlama, u32, u64)>,
    engine: Option<&str>,
) -> (Result<Chosen, Failure>, u32, u64) {
    match (picked, engine) {
        (_, Some("stand-in")) => (Ok(Chosen::StandIn), 0, 0),
        (Some((llama, layers, window)), _) => (Ok(Chosen::Provisioned(llama)), layers, window),
        (None, asked) => (choose_engine(mcf_home, asked), 0, 0),
    }
}

fn shown_as(frame: &crate::turn::Frame, marker: &str, prompt: &str) -> String {
    format!("{}{marker}{prompt}{}", frame.before, frame.after)
}

fn framed_label(frame: &crate::turn::Frame, shown: bool) -> String {
    format!(
        "framed by the engine from the model's own template ({}){}",
        frame.asked,
        if shown {
            ", with a picture before the words"
        } else {
            ""
        }
    )
}

fn addressing_label(
    derived: Option<&crate::configured::Addressing>,
    given: Given<'_>,
    built: bool,
) -> Option<String> {
    match (derived, given) {
        (Some(addressing), Given::Identifiers(_)) => Some(format!(
            "{} — not applied here: the caller sent its own identifiers",
            addressing.provenance()
        )),
        (Some(addressing), Given::Pieces(_)) => Some(format!(
            "{} — not applied here: the caller sent its own markers and text",
            addressing.provenance()
        )),
        (Some(addressing), Given::Text) => Some(addressing.provenance()),
        (None, Given::Pieces(_)) if built => Some(OWN_PIECES.to_owned()),
        (None, Given::Text) if built => Some(BARE_PROMPT.to_owned()),
        (None, _) => None,
    }
}

pub(crate) const OWN_PIECES: &str = "the caller's own markers and text, each read by the \
                                     tokenizer of the engine that answered";

pub(crate) const BARE_PROMPT: &str = "the prompt alone: no addressing is on file for this \
                                      model, so it went with no turn markers around it";

fn with_provenance(
    produced: Result<Produced, Failure>,
    addressed: Option<String>,
    derived_budget: Option<crate::configured::Budget>,
) -> Result<Produced, Failure> {
    let produced = produced.map(|produced| Produced {
        account: match (produced.account, addressed) {
            (Value::Map(mut fields), Some(addressed)) => {
                if let Some(Value::Map(conditions)) = fields.get_mut("conditions") {
                    conditions.insert("addressed_as".to_owned(), Value::text(addressed));
                }
                Value::Map(fields)
            }
            (account, _) => account,
        },
        said: produced.said,
    });
    produced.map(|produced| Produced {
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
    })
}

struct Place<'a> {
    store: &'a Path,
    runtime: &'a Path,
    named: &'a str,
    gpu_layers: u32,
    context: u64,
    server: &'a std::sync::Mutex<Option<Served>>,
    started: crate::declared::Started,
    held: Option<&'a Served>,
}

impl<'a> Place<'a> {
    fn engine(&self, llama: &'a crate::adapters::ProvisionedLlama) -> Tokenizer<'a> {
        Tokenizer::Engine {
            where_it_lives: Where {
                store: self.store,
                llama,
                runtime: self.runtime,
                named: self.named,
                gpu_layers: self.gpu_layers,
                context: self.context,
                started: self.started,
                held: self.held,
            },
            server: self.server,
        }
    }
}

fn framed_turn(
    place: &Place<'_>,
    own_turn: bool,
    turn: Option<&crate::turn::Turn>,
    chosen: &Result<Chosen, Failure>,
) -> Result<Option<crate::turn::Frame>, Failure> {
    let refused = |why: &'static str| {
        Failure::new(
            mcf_core::failure::Category::ConfigInvalid,
            mcf_core::failure::Attribution::User,
            mcf_core::failure::Disposition::Refused,
            mcf_core::failure::Subsystem::new("mcf-serve::generation"),
            why,
        )
    };
    match (own_turn, turn, chosen) {
        (false, Some(turn), Ok(Chosen::Provisioned(llama))) => {
            place.engine(llama).framed(turn).map(Some)
        }
        (false, Some(_), Ok(Chosen::StandIn)) => Err(refused(
            "MCF's own engine runs no template, so it cannot frame a turn: the provisioned \
             engine can",
        )),
        (true, Some(_), _) => Err(refused(
            "a turn the caller built is already framed; the engine cannot frame it again",
        )),
        _ => Ok(None),
    }
}

enum Chosen {
    StandIn,
    Provisioned(crate::adapters::ProvisionedLlama),
}

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
                "no provisioned engine is here: `mcf provision llama.cpp` builds one",
            )),
            None => Ok(Chosen::StandIn),
        },
    }
}

pub(crate) fn resolved(store: &Path, named: &str) -> std::path::PathBuf {
    let given = Path::new(named);
    if given.is_file() {
        given.to_path_buf()
    } else {
        store.join(named.replace(':', "/"))
    }
}

pub(crate) enum Tokenizer<'a> {
    Own(Vocabulary),
    Engine {
        where_it_lives: Where<'a>,
        server: &'a std::sync::Mutex<Option<Served>>,
    },
}

pub(crate) type Read = crate::served::Token;

impl Tokenizer<'_> {
    pub(crate) fn own(path: &Path) -> Result<Self, Failure> {
        let file = crate::daemon::header_of(path).ok_or_else(|| {
            Failure::new(
                mcf_core::failure::Category::ArtifactMissing,
                mcf_core::failure::Attribution::Machine,
                mcf_core::failure::Disposition::Refused,
                mcf_core::failure::Subsystem::new("mcf-serve::generation"),
                "this model's header could not be read",
            )
            .with_context("path", path.display().to_string())
        })?;
        Vocabulary::read(&file).map(Self::Own)
    }

    pub(crate) fn named(&self) -> String {
        match self {
            Self::Own(_) => "MCF's own tokenizer, which its engine generates with".to_owned(),
            Self::Engine { .. } => format!("{}, which generated", self.who()),
        }
    }

    pub(crate) fn reader(&self) -> String {
        match self {
            Self::Own(_) => "MCF's own tokenizer, which its engine generates with".to_owned(),
            Self::Engine { .. } => format!("{}, which generates for this model", self.who()),
        }
    }

    fn who(&self) -> String {
        match self {
            Self::Own(_) => "MCF's own tokenizer".to_owned(),
            Self::Engine { where_it_lives, .. } => format!(
                "provisioned {} server @{}",
                where_it_lives.llama.component,
                where_it_lives
                    .llama
                    .commit
                    .get(..12)
                    .unwrap_or(&where_it_lives.llama.commit)
            ),
        }
    }

    pub(crate) fn encode(&self, text: &str, with_beginning: bool) -> Result<Vec<Read>, Failure> {
        match self {
            Self::Own(vocabulary) => {
                let identifiers = vocabulary.encode(text, with_beginning)?;
                Ok(spelled_by(vocabulary, &identifiers))
            }
            Self::Engine {
                where_it_lives,
                server,
            } => {
                let mut slot = Slot::for_request(server, where_it_lives.held)?;
                let (engine, _reused) = slot.engine(where_it_lives, SMALLEST_WINDOW)?;
                engine.tokenize(text, with_beginning, false)
            }
        }
    }

    pub(crate) fn framed(&self, turn: &crate::turn::Turn) -> Result<crate::turn::Frame, Failure> {
        match self {
            Self::Own(_) => Err(unavailable("MCF's own engine runs no template")),
            Self::Engine {
                where_it_lives,
                server,
            } => {
                let mut slot = Slot::for_request(server, where_it_lives.held)?;
                let (engine, _reused) = slot.engine(where_it_lives, SMALLEST_WINDOW)?;
                crate::turn::frame(engine, turn)
            }
        }
    }

    pub(crate) fn own_words(&self, text: &str, with_beginning: bool) -> Result<Vec<Read>, Failure> {
        match self {
            Self::Own(_) => Err(unavailable(
                "MCF's own engine reads markers one at a time and cannot read a rendered turn",
            )),
            Self::Engine {
                where_it_lives,
                server,
            } => {
                let mut slot = Slot::for_request(server, where_it_lives.held)?;
                let (engine, _reused) = slot.engine(where_it_lives, SMALLEST_WINDOW)?;
                engine.tokenize(text, with_beginning, true)
            }
        }
    }

    pub(crate) fn addressed(
        &self,
        pieces: &[mcf_standin::tokenizer::Piece],
    ) -> Result<Vec<Read>, Failure> {
        use mcf_standin::tokenizer::Piece;
        let not_a_token = |marker: &str| {
            Failure::new(
                mcf_core::failure::Category::ConfigInvalid,
                mcf_core::failure::Attribution::Machine,
                mcf_core::failure::Disposition::Refused,
                mcf_core::failure::Subsystem::new("mcf-serve::generation"),
                "a marker of this model's addressing is not one token of its vocabulary",
            )
            .with_context("marker", marker.to_owned())
        };
        match self {
            Self::Own(vocabulary) => {
                if let Some(marker) = pieces.iter().find_map(|piece| match piece {
                    Piece::Marker(marker) if !vocabulary.has_token(marker) => Some(marker.as_str()),
                    _ => None,
                }) {
                    return Err(not_a_token(marker));
                }
                let Some(identifiers) = vocabulary.addressed(pieces) else {
                    for piece in pieces {
                        if let Piece::Text(text) = piece {
                            let _represented = vocabulary.encode(text, false)?;
                        }
                    }
                    return Err(unavailable("the addressed turn could not be encoded"));
                };
                Ok(spelled_by(vocabulary, &identifiers))
            }
            Self::Engine { .. } => {
                let mut read = self.encode("", true)?;
                for piece in pieces {
                    match piece {
                        Piece::Marker(marker) => {
                            let held = self.own_words(marker, false)?;
                            if held.len() != 1 {
                                return Err(not_a_token(marker));
                            }
                            read.extend(held);
                        }
                        Piece::Text(text) => read.extend(self.encode(text, false)?),
                    }
                }
                Ok(read)
            }
        }
    }
}

fn spelled_by(vocabulary: &Vocabulary, identifiers: &[usize]) -> Vec<Read> {
    identifiers
        .iter()
        .map(|id| {
            vocabulary.bytes_of(*id).map_or_else(
                || Read {
                    id: *id,
                    piece: format!("<id {id}>"),
                },
                |bytes| Read::from_bytes(*id, &bytes),
            )
        })
        .collect()
}

fn addressed_as(
    tokenizer: &Tokenizer<'_>,
    prompt: &str,
    addressing: &crate::configured::Addressing,
) -> Result<Vec<usize>, Failure> {
    let mut pieces = addressing.before.clone();
    pieces.push(mcf_standin::tokenizer::Piece::Text(prompt.to_owned()));
    pieces.extend(addressing.after.iter().cloned());
    Ok(tokenizer
        .addressed(&pieces)?
        .into_iter()
        .map(|held| held.id)
        .collect())
}

pub(crate) fn framed_as(
    tokenizer: &Tokenizer<'_>,
    prompt: &str,
    frame: &crate::turn::Frame,
) -> Result<Vec<usize>, Failure> {
    let mut read = tokenizer.own_words(&frame.before, true)?;
    read.extend(tokenizer.encode(prompt, false)?);
    read.extend(tokenizer.own_words(&frame.after, false)?);
    Ok(read.into_iter().map(|held| held.id).collect())
}

pub(crate) struct Received {
    pub(crate) read: Vec<Read>,
    pub(crate) before: usize,
    pub(crate) under: String,
}

impl Received {
    pub(crate) fn tokens(&self) -> Vec<usize> {
        self.read.iter().map(|held| held.id).collect()
    }
}

pub(crate) fn received(
    tokenizer: &Tokenizer<'_>,
    mcf_home: &Path,
    path: &Path,
    prompt: &str,
    with_after: bool,
) -> Result<Received, Failure> {
    let derived = crate::configured::read_derived(mcf_home, path).addressing;
    let not_within = |what: &str| {
        Failure::new(
            mcf_core::failure::Category::EngineProtocolMalformed,
            mcf_core::failure::Attribution::Machine,
            mcf_core::failure::Disposition::Aborted,
            mcf_core::failure::Subsystem::new("mcf-serve::generation"),
            what,
        )
    };
    let Some(addressing) = derived else {
        let read = tokenizer.encode(prompt, true)?;
        let own = tokenizer.encode(prompt, false)?;
        let before = offset_of(&read, &own)
            .ok_or_else(|| not_within("the prompt's own tokens are not within its reading"))?;
        return Ok(Received {
            read,
            before,
            under: BARE_PROMPT.to_owned(),
        });
    };
    let opening = tokenizer.addressed(&addressing.before)?;
    let mut pieces = addressing.before.clone();
    pieces.push(mcf_standin::tokenizer::Piece::Text(prompt.to_owned()));
    if with_after {
        pieces.extend(addressing.after.iter().cloned());
    }
    let read = tokenizer.addressed(&pieces)?;
    if !read.starts_with(&opening) {
        return Err(not_within(
            "the addressing's opening is not a prefix of the addressed turn",
        ));
    }
    Ok(Received {
        read,
        before: opening.len(),
        under: addressing.provenance(),
    })
}

fn offset_of(read: &[Read], held: &[Read]) -> Option<usize> {
    if held.is_empty() {
        return Some(read.len());
    }
    let ids = |tokens: &[Read]| tokens.iter().map(|token| token.id).collect::<Vec<_>>();
    let within = ids(read);
    let wanted = ids(held);
    within.windows(wanted.len()).position(|run| run == wanted)
}

const SMALLEST_WINDOW: u64 = 4096;

pub(crate) type Ranked = (Option<usize>, Option<String>);

#[derive(Debug, Clone, Copy)]
pub(crate) struct Where<'a> {
    pub store: &'a Path,
    pub llama: &'a crate::adapters::ProvisionedLlama,
    pub runtime: &'a Path,
    pub named: &'a str,
    pub gpu_layers: u32,
    pub context: u64,
    pub started: crate::declared::Started,
    pub held: Option<&'a Served>,
}

enum Slot<'a> {
    Held(&'a Served),
    Mine(std::sync::MutexGuard<'a, Option<Served>>),
}

impl<'a> Slot<'a> {
    fn for_request(
        server: &'a std::sync::Mutex<Option<Served>>,
        held: Option<&'a Served>,
    ) -> Result<Self, Failure> {
        match held {
            Some(held) => Ok(Self::Held(held)),
            None => server.lock().map(Self::Mine).map_err(|_poisoned| {
                unavailable("the served engine's slot was left poisoned by an earlier failure")
            }),
        }
    }

    fn engine(
        &mut self,
        where_it_lives: &Where<'_>,
        window: u64,
    ) -> Result<(&Served, bool), Failure> {
        match self {
            Self::Held(held) => Ok((held, true)),
            Self::Mine(slot) => serving(slot, where_it_lives, window),
        }
    }
}

pub(crate) const MOST_RANKED: usize = 120;

pub(crate) const HOW_DEEP: usize = 60;

pub(crate) fn ranks_over(
    where_it_lives: &Where<'_>,
    server: &std::sync::Mutex<Option<Served>>,
    tokens: &[usize],
    from: usize,
    most: usize,
) -> Result<Vec<Ranked>, Failure> {
    let mut slot = Slot::for_request(server, where_it_lives.held)?;
    let (engine, _reused) = slot.engine(
        where_it_lives,
        window_for(tokens.len(), 0, where_it_lives.context),
    )?;

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

fn serving<'slot>(
    slot: &'slot mut Option<Served>,
    where_it_lives: &Where<'_>,
    window: u64,
) -> Result<(&'slot Served, bool), Failure> {
    let Where {
        store,
        llama,
        runtime,
        named,
        gpu_layers,
        context: _,
        started,
        held: _,
    } = *where_it_lives;
    let path = resolved(store, named);
    let reused = slot.as_ref().is_some_and(|held| {
        held.model == path
            && held.prefix == llama.prefix
            && held.gpu_layers == gpu_layers
            && window_suits(held.window, window)
            && held.started.same_switches(&started)
    });
    if !reused {
        *slot = None;
        let projector = crate::projector::beside(&path);
        *slot = Some(Served::start(
            llama,
            &path,
            runtime,
            gpu_layers,
            window,
            projector.as_deref(),
            started,
        )?);
    }
    let engine = slot
        .as_ref()
        .ok_or_else(|| unavailable("the served engine was started and then was not there"))?;
    Ok((engine, reused))
}

pub(crate) fn window_suits(held: u64, wanted: u64) -> bool {
    held >= wanted && held <= wanted.saturating_mul(TOO_WIDE)
}

const TOO_WIDE: u64 = 16;

fn window_for(sent: usize, limit: usize, context: u64) -> u64 {
    let asked_for = u64::try_from(sent.saturating_add(limit)).unwrap_or(SMALLEST_WINDOW);
    let needed = asked_for
        .saturating_mul(2)
        .max(SMALLEST_WINDOW)
        .checked_next_power_of_two()
        .unwrap_or(u64::MAX);
    if context == 0 {
        needed
    } else {
        needed.min(context)
    }
}

#[allow(
    clippy::too_many_arguments,
    clippy::too_many_lines,
    reason = "one request's conditions, each named in the account"
)]
fn through_served(
    store: &Path,
    llama: &crate::adapters::ProvisionedLlama,
    server: &std::sync::Mutex<Option<Served>>,
    held: Option<&Served>,
    runtime: &Path,
    named: &str,
    sent: &Sent<'_>,
    draw: Draw,
    gpu_layers: u32,
    context: u64,
    started: crate::declared::Started,
    declared: &crate::declared::Declared,
    waiting: crate::served::Waiting<'_>,
    writer: &mut &UnixStream,
) -> Result<Produced, Failure> {
    let Sent {
        tokens,
        limit,
        pinned,
        cached,
        tail: _,
        shown,
    } = *sent;
    let path = resolved(store, named);
    let metadata = std::fs::metadata(&path).map_err(|error| missing(named, &path, &error))?;
    let size = metadata.len();
    if shown.is_some() {
        projector_present(&path)?;
    }

    let mut slot = Slot::for_request(server, held)?;
    let where_it_lives = Where {
        store,
        llama,
        runtime,
        named,
        gpu_layers,
        context,
        started,
        held,
    };
    let room = if shown.is_some() { PICTURE_ROOM } else { 0 };
    let window = match started.window {
        Some(asked) if context == 0 => asked,
        Some(asked) => asked.min(context),
        None => window_for(tokens.len().saturating_add(room), limit, context),
    };
    let (engine, reused) = slot.engine(&where_it_lives, window)?;

    let text = shown.map(|shown| shown.placed_by(engine)).transpose()?;
    let prompt = match (shown, &text) {
        (Some(shown), Some(text)) => crate::served::Prompt::Shown {
            text,
            picture: &shown.picture.bytes,
        },
        _ => crate::served::Prompt::Identifiers(tokens),
    };
    let (completed, pieces) = as_it_arrives(
        engine,
        prompt,
        limit,
        draw,
        (pinned, cached),
        waiting,
        writer,
    )?;
    let peak_resident = engine.peak_resident_bytes();
    let ran_in = engine.window;

    let (before, text) = streamed(engine, &path, sent, &completed)?;
    whole_where_nothing_arrived(writer, pieces, &text);

    let mut conditions = conditions(named, Some((&path, size)), draw, limit);
    if let Value::Map(fields) = &mut conditions {
        fields.insert("engine".to_owned(), Value::text(served_engine_name(llama)));
        fields.insert(
            "length".to_owned(),
            Value::text(Length::of(pinned).as_str()),
        );
        fields.insert(
            "loaded".to_owned(),
            Value::text(if held.is_some() {
                "resident_in_hosted_server"
            } else if reused {
                "resident_in_server"
            } else {
                "loaded_for_this_request"
            }),
        );
        fields.insert(
            "identifiers_sent".to_owned(),
            if shown.is_some() {
                Value::Null
            } else {
                Value::Integer(i64::try_from(tokens.len()).unwrap_or(i64::MAX))
            },
        );
        fields.insert(
            "shown".to_owned(),
            shown.map_or(Value::Null, |shown| shown.to_value(engine)),
        );
        fields.insert("declares".to_owned(), declared.to_value());
        fields.insert("started_with".to_owned(), started.to_value());
        fields.insert(
            "identifiers_read".to_owned(),
            Value::Integer(i64::try_from(completed.evaluated).unwrap_or(i64::MAX)),
        );
        fields.insert(
            "window".to_owned(),
            Value::Integer(i64::try_from(ran_in).unwrap_or(i64::MAX)),
        );
        fields.insert(
            "peak_resident_bytes".to_owned(),
            peak_resident.map_or(Value::Null, |bytes| {
                Value::Integer(i64::try_from(bytes).unwrap_or(i64::MAX))
            }),
        );
    }
    let prompt_tokens = if shown.is_some() {
        completed.evaluated
    } else {
        tokens.len()
    };
    Ok(served_account(
        conditions,
        &completed,
        prompt_tokens,
        before.as_ref(),
        text,
    ))
}

fn projector_present(path: &Path) -> Result<(), Failure> {
    if crate::projector::beside(path).is_some() {
        return Ok(());
    }
    Err(Failure::new(
        mcf_core::failure::Category::ConfigUnsatisfiable,
        mcf_core::failure::Attribution::Artifact,
        mcf_core::failure::Disposition::Refused,
        mcf_core::failure::Subsystem::new("mcf-serve::generation"),
        "no projector sits beside this model, so a picture cannot reach it: the engine would \
         read the words alone. A publisher that shipped one puts it in the model's own \
         repository, and pulling that puts it beside the model",
    )
    .with_context("model", path.display().to_string()))
}

fn served_account(
    conditions: Value,
    completed: &Completed,
    prompt_tokens: usize,
    before: Option<&crate::turn::BeforeTheAnswer>,
    text: String,
) -> Produced {
    Produced {
        account: Value::map([
            (
                "tokens",
                Value::Integer(i64::try_from(completed.predicted).unwrap_or(i64::MAX)),
            ),
            (
                "prompt_tokens",
                Value::Integer(i64::try_from(prompt_tokens).unwrap_or(i64::MAX)),
            ),
            ("stopped", Value::text(completed.stop.written())),
            (
                "engine_timings",
                completed.timings.clone().unwrap_or(Value::Null),
            ),
            (
                "text_bytes",
                Value::Integer(i64::try_from(text.len()).unwrap_or(i64::MAX)),
            ),
            (
                "before_the_answer",
                before.map_or(Value::Null, crate::turn::BeforeTheAnswer::to_value),
            ),
            ("conditions", conditions),
        ]),
        said: Some(Said {
            text,
            tokens: completed.produced.clone(),
            answer: before
                .filter(|before| before.closed)
                .map(|before| before.answer.clone()),
        }),
    }
}

fn refused_account(
    failure: &Failure,
    named: &str,
    draw: Draw,
    limit: usize,
    charged: Option<String>,
) -> Produced {
    let mut conditions = conditions(named, None, draw, limit);
    if let (Value::Map(fields), Some(engine)) = (&mut conditions, charged) {
        fields.insert("engine".to_owned(), Value::text(engine));
        fields.insert("loaded".to_owned(), Value::Null);
    }
    Produced {
        account: Value::map([
            ("tokens", Value::Integer(0)),
            ("stopped", Value::text("refused")),
            ("failure", mcf_record::encode::failure(failure)),
            ("conditions", conditions),
        ]),
        said: None,
    }
}

fn served_engine_name(llama: &crate::adapters::ProvisionedLlama) -> String {
    format!(
        "provisioned {} server @{} from {}",
        llama.component,
        llama.commit.get(..12).unwrap_or(&llama.commit),
        llama.prefix.display()
    )
}

fn before_the_answer(
    engine: &Served,
    path: &Path,
    tail: Option<&str>,
    produced: &[usize],
) -> Result<Option<crate::turn::BeforeTheAnswer>, Failure> {
    let template = crate::daemon::header_of(path)
        .and_then(|file| {
            file.get("tokenizer.chat_template")
                .and_then(gguf::Value::as_text)
                .map(str::to_owned)
        })
        .unwrap_or_default();
    crate::turn::before_the_answer(engine, &template, tail, produced)
}

#[allow(
    clippy::too_many_arguments,
    reason = "one request's conditions, each named in the account"
)]
fn as_it_arrives(
    engine: &Served,
    prompt: crate::served::Prompt<'_>,
    limit: usize,
    draw: Draw,
    (pinned, cached): (bool, bool),
    waiting: crate::served::Waiting<'_>,
    writer: &mut &UnixStream,
) -> Result<(Completed, usize), Failure> {
    if pinned {
        return engine
            .complete_with(
                prompt,
                limit,
                draw,
                pinned,
                &crate::served::Extras {
                    cached,
                    ..crate::served::Extras::default()
                },
                waiting,
            )
            .map(|completed| (completed, 0));
    }
    let mut arriving = Arriving::new(writer);
    let completed =
        engine.complete_while(prompt, limit, draw, pinned, waiting, &mut |at, piece| {
            arriving.piece(at, piece);
        });
    match completed {
        Ok(completed) => {
            arriving.close(completed.stop == crate::served::Stop::Eos);
            Ok((completed, arriving.written))
        }
        Err(failure) => Err(arriving.left_off(failure)),
    }
}

fn streamed(
    engine: &Served,
    path: &Path,
    sent: &Sent<'_>,
    completed: &Completed,
) -> Result<(Option<crate::turn::BeforeTheAnswer>, String), Failure> {
    let words = completed.words();
    let before = if sent.pinned {
        None
    } else {
        before_the_answer(engine, path, sent.tail, words)?
    };
    Ok((before, engine.detokenize(words)?))
}

fn whole_where_nothing_arrived(writer: &mut &UnixStream, pieces: usize, text: &str) {
    if pieces > 0 {
        return;
    }
    let line = Streamed::Token {
        at: 0,
        text: text.to_owned(),
    }
    .to_line();
    let _written = writeln!(writer, "{line}");
    let _flushed = writer.flush();
}

struct Arriving<'a, 'w> {
    writer: &'a mut &'w UnixStream,
    held: Option<(usize, String)>,
    text: String,
    written: usize,
}

impl<'a, 'w> Arriving<'a, 'w> {
    fn new(writer: &'a mut &'w UnixStream) -> Self {
        Self {
            writer,
            held: None,
            text: String::new(),
            written: 0,
        }
    }

    fn piece(&mut self, at: usize, piece: &str) {
        let held = self.held.replace((at, piece.to_owned()));
        self.write(held);
    }

    fn close(&mut self, ended_itself: bool) {
        let held = self.held.take();
        if !ended_itself {
            self.write(held);
        }
        let _flushed = self.writer.flush();
    }

    fn left_off(&mut self, failure: Failure) -> Failure {
        let held = self.held.take();
        self.write(held);
        let _flushed = self.writer.flush();
        if self.text.is_empty() {
            return failure;
        }
        failure.with_context("produced_before_it_stopped", self.text.clone())
    }

    fn write(&mut self, piece: Option<(usize, String)>) {
        let Some((at, text)) = piece else {
            return;
        };
        self.text.push_str(&text);
        self.written = self.written.saturating_add(1);
        let line = Streamed::Token { at, text }.to_line();
        let _written = writeln!(self.writer, "{line}");
        let _flushed = self.writer.flush();
    }
}

#[derive(Clone, Copy)]
struct Sent<'a> {
    tokens: &'a [usize],
    limit: usize,
    pinned: bool,
    cached: bool,
    tail: Option<&'a str>,
    shown: Option<Shown<'a>>,
}

#[derive(Clone, Copy)]
struct Shown<'a> {
    frame: &'a crate::turn::Frame,
    prompt: &'a str,
    picture: &'a Picture,
}

impl Shown<'_> {
    fn placed_by(self, engine: &Served) -> Result<String, Failure> {
        let marker = engine.media_marker.as_deref().ok_or_else(|| {
            unavailable(
                "the served engine has no marker to stand a picture at, so a picture cannot be \
                 placed in its text",
            )
        })?;
        Ok(shown_as(self.frame, marker, self.prompt))
    }

    fn to_value(self, engine: &Served) -> Value {
        Value::map([
            ("picture", self.picture.to_value()),
            (
                "projector",
                engine.projector.as_ref().map_or(Value::Null, |projector| {
                    Value::text(projector.display().to_string())
                }),
            ),
            (
                "placed",
                Value::text("before the words, at the engine's marker"),
            ),
            (
                "read_as",
                Value::text(
                    "text: the frame and the words in one string, every marker in it read as \
                     one, since the engine gives a picture no other door",
                ),
            ),
        ])
    }
}

pub(crate) struct Picture {
    path: std::path::PathBuf,
    bytes: Vec<u8>,
}

impl Picture {
    pub(crate) fn read(path: &Path) -> Result<Self, Failure> {
        let refused = |why: &'static str, detail: String| {
            Failure::new(
                mcf_core::failure::Category::ArtifactUnreadable,
                mcf_core::failure::Attribution::User,
                mcf_core::failure::Disposition::Refused,
                mcf_core::failure::Subsystem::new("mcf-serve::generation"),
                why,
            )
            .with_context("path", path.display().to_string())
            .with_context("detail", detail)
        };
        let bytes = std::fs::read(path)
            .map_err(|error| refused("the picture could not be read", error.to_string()))?;
        if bytes.is_empty() {
            return Err(refused(
                "the picture is an empty file",
                "0 bytes".to_owned(),
            ));
        }
        Ok(Self {
            path: path.to_path_buf(),
            bytes,
        })
    }

    fn to_value(&self) -> Value {
        Value::map([
            ("path", Value::text(self.path.display().to_string())),
            (
                "bytes",
                Value::Integer(i64::try_from(self.bytes.len()).unwrap_or(i64::MAX)),
            ),
            (
                "sha256",
                Value::text(mcf_core::digest::sha256(&self.bytes).hex()),
            ),
        ])
    }
}

const PICTURE_ROOM: usize = 4096;

const ENDED_ITS_TURN: &str = "[end of text]";

const HELD_BACK: usize = 32;

fn ready_to_send(held: &mut String) -> String {
    let Some(mut at) = held.len().checked_sub(HELD_BACK) else {
        return String::new();
    };
    while at > 0 && !held.is_char_boundary(at) {
        at -= 1;
    }
    held.drain(..at).collect()
}

fn without_the_marker(said: &str) -> (String, &'static str) {
    let trimmed = said.trim_end();
    match trimmed.strip_suffix(ENDED_ITS_TURN) {
        Some(before) => (before.trim_end().to_owned(), "stop_token"),
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
    draw: Draw,
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

    let mut command = llama.generate(&path, prompt, limit, draw, pinned);
    let mut at = 0_usize;
    let mut text = String::new();
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
    let mut conditions = conditions(named, Some((&path, held)), draw, limit);
    if let Value::Map(fields) = &mut conditions {
        fields.insert("engine".to_owned(), Value::text(engine_name));
        fields.insert("loaded".to_owned(), Value::text("per_request_subprocess"));
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
            answer: None,
        }),
    })
}

fn conditions(named: &str, model: Option<(&Path, u64)>, draw: Draw, limit: usize) -> Value {
    Value::map(
        [
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
            ("sampler", Value::text(draw.sampler())),
            (
                "temperature_thousandths",
                Value::Integer(i64::from(draw.temperature.0)),
            ),
            (
                "seed",
                Value::Integer(i64::try_from(draw.seed).unwrap_or(i64::MAX)),
            ),
            (
                "limit",
                Value::Integer(i64::try_from(limit).unwrap_or(i64::MAX)),
            ),
        ]
        .into_iter()
        .chain(draw.truncation.entries())
        .collect::<Vec<(&str, Value)>>(),
    )
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
    draw: Draw,
    free: Option<u64>,
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
        if let Some(header) = crate::daemon::header_of(&path) {
            llama::covers(&header)?;
            if let Some(available) = free {
                header.fits_dequantized(available)?;
            }
        }
        *held = None;
        let bytes = std::fs::read(&path).map_err(|error| missing(named, &path, &error))?;
        let file = gguf::parse(&bytes)?;
        llama::covers(&file)?;
        let dequantized_bytes = file.dequantized_bytes().unwrap_or(0);
        let vocabulary = Vocabulary::read(&file)?;
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
            settings: draw.settings(),
            seed: draw.seed,
            stop: if pinned {
                Vec::new()
            } else {
                vocabulary.ending.into_iter().collect()
            },
        },
        &mut |token| {
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
                conditions(named, Some((&path, held_bytes)), draw, limit)
                    .with_residency(loaded, &since, dequantized)
                    .with_length(pinned),
            ),
            ("degraded", Value::text(degradation)),
        ]),
        said: Some(Said {
            text,
            tokens: produced.tokens.clone(),
            answer: None,
        }),
    })
}

fn unavailable(why: &'static str) -> Failure {
    Failure::new(
        mcf_core::failure::Category::EngineUnavailable,
        mcf_core::failure::Attribution::Machine,
        mcf_core::failure::Disposition::Aborted,
        mcf_core::failure::Subsystem::new("mcf-serve::generation"),
        why,
    )
}

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Length {
    AtMost,
    Exactly,
    ExactlyButUncounted,
}

impl Length {
    pub(crate) const fn of(pinned: bool) -> Self {
        if pinned { Self::Exactly } else { Self::AtMost }
    }

    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::AtMost => "at_most",
            Self::Exactly => "exactly",
            Self::ExactlyButUncounted => "exactly_but_uncounted",
        }
    }
}

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
mod truncation_tests {
    #![allow(clippy::panic, clippy::expect_used)]

    use mcf_core::attested::Attested;
    use mcf_core::configuration::{Sampling, Thousandths};

    use super::{Stated, Truncation, Whose};

    #[test]
    fn what_the_file_declares_is_taken_and_the_rest_is_off() {
        let sampling = Sampling {
            top_p: Attested::Known(Thousandths(950)),
            ..Sampling::nothing_set()
        };
        let cut = Truncation::recommended(Some(&sampling));
        assert_eq!(cut.top_p, Stated::Declared(Thousandths(950)));
        assert_eq!(cut.top_k, Stated::Off);
        assert_eq!(cut.min_p, Stated::Off);
        assert!(cut.is_declared());
        assert_eq!(cut.top_k_sent(), 0);
        assert_eq!(cut.top_p_sent(), Thousandths(950));
        assert_eq!(cut.min_p_sent(), Thousandths(0));
    }

    #[test]
    fn no_recommendation_is_off_and_off_leaves_the_distribution_whole() {
        let cut = Truncation::recommended(None);
        assert!(!cut.any_stated());
        assert!(!cut.is_declared());
        assert_eq!(cut.whose, Whose::NoneDeclared);
        assert_eq!(Truncation::OFF.whose, Whose::Moot);
        assert_eq!(cut.top_k_sent(), 0);
        assert_eq!(cut.top_p_sent(), Thousandths(1_000));
        assert_eq!(cut.min_p_sent(), Thousandths(0));
    }

    #[test]
    fn the_entries_name_the_three_and_whose_they_are() {
        let cut = Truncation {
            top_k: Stated::Declared(20),
            top_p: Stated::Off,
            min_p: Stated::Declared(Thousandths(50)),
            whose: Whose::File,
        };
        let entries: Vec<(&str, String)> = cut
            .entries()
            .into_iter()
            .map(|(name, value)| (name, value.as_text().unwrap_or("?").to_owned()))
            .collect();
        assert_eq!(
            entries,
            vec![
                ("top_k", "20".to_owned()),
                ("top_p", "off".to_owned()),
                ("min_p", "0.050".to_owned()),
                ("truncation", "declared by the file".to_owned()),
            ]
        );
        assert_eq!(cut.line(), "top_k 20 · top_p off · min_p 0.050");
        assert_eq!(
            Truncation::OFF
                .entries()
                .get(3)
                .and_then(|(_, value)| value.as_text()),
            Some("moot under greedy")
        );
        assert_eq!(
            Truncation::recommended(None)
                .entries()
                .get(3)
                .and_then(|(_, value)| value.as_text()),
            Some("none declared by the file")
        );
    }
}

#[cfg(test)]
mod marker_tests {
    #![allow(clippy::panic, clippy::expect_used)]

    use super::{ENDED_ITS_TURN, HELD_BACK, ready_to_send, without_the_marker};

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

    #[test]
    fn no_marker_is_not_a_stop_token() {
        let (text, why) = without_the_marker("a partial answer that ran out of budget");
        assert_eq!(text, "a partial answer that ran out of budget");
        assert_eq!(why, "unknown_the_engine_did_not_say");
    }

    #[test]
    fn only_the_end_is_the_end() {
        let said = "print('[end of text]')\nreturn 1";
        let (text, why) = without_the_marker(said);
        assert_eq!(text, said, "a marker in the middle is the model's own text");
        assert_eq!(why, "unknown_the_engine_did_not_say");
    }

    #[test]
    fn the_marker_is_never_streamed_even_with_output_after_it() {
        let mut held = String::new();
        let mut sent = String::new();
        held.push_str("    return result [end of text]\n\n\n");
        sent.push_str(&ready_to_send(&mut held));
        assert!(
            !sent.contains(ENDED_ITS_TURN),
            "the marker reached the caller: {sent:?}"
        );
    }

    #[test]
    fn what_is_certainly_the_models_goes_out_as_it_arrives() {
        let mut held = "x".repeat(HELD_BACK * 4);
        let ready = ready_to_send(&mut held);
        assert_eq!(ready.len(), HELD_BACK * 3);
        assert_eq!(held.len(), HELD_BACK, "the tail is what is kept back");
    }

    #[test]
    fn a_character_is_not_cut_in_half() {
        let mut held = "é".repeat(HELD_BACK);
        let ready = ready_to_send(&mut held);
        assert!(ready.chars().all(|held| held == 'é'));
        assert!(held.chars().all(|c| c == 'é'));
        assert_eq!(ready.len() + held.len(), HELD_BACK * 2);
    }
}

#[cfg(test)]
mod tokenizer_tests {
    #![allow(clippy::panic, clippy::expect_used, clippy::indexing_slicing)]

    use super::{Received, Tokenizer, addressed_as, received};
    use mcf_standin::tokenizer::Piece;
    use std::path::PathBuf;

    struct OnDisk {
        root: PathBuf,
        model: PathBuf,
    }

    impl OnDisk {
        fn chatml(name: &str) -> Self {
            let root = std::env::temp_dir().join(format!(
                "mcf-tokenizer-{name}-{}-{:?}",
                std::process::id(),
                std::thread::current().id()
            ));
            let _fresh = std::fs::remove_dir_all(&root);
            std::fs::create_dir_all(&root).expect("a scratch directory");
            let model = root.join("chatml.gguf");
            std::fs::write(&model, crate::probes::tests::chatml()).expect("a model file");
            Self { root, model }
        }
    }

    impl Drop for OnDisk {
        fn drop(&mut self) {
            let _removed = std::fs::remove_dir_all(&self.root);
        }
    }

    fn chatml_addressing() -> crate::configured::Addressing {
        crate::configured::Addressing {
            name: "im_start…im_end as assistant".to_owned(),
            before: vec![
                Piece::Marker("<|im_start|>".to_owned()),
                Piece::Text("user\n".to_owned()),
            ],
            after: vec![
                Piece::Marker("<|im_end|>".to_owned()),
                Piece::Text("\n".to_owned()),
                Piece::Marker("<|im_start|>".to_owned()),
                Piece::Text("assistant\n".to_owned()),
            ],
            probe: "addressing".to_owned(),
            at: "2026-09-02T00:00:00Z".to_owned(),
            build: "0.1.0 test".to_owned(),
            conditions: "a test".to_owned(),
        }
    }

    #[test]
    fn the_pieces_of_a_reading_spell_the_text() {
        let disk = OnDisk::chatml("pieces");
        let tokenizer = Tokenizer::own(&disk.model).expect("MCF's own tokenizer");
        let read = tokenizer.encode("a a", false).expect("a reading");
        let spelled: String = read.iter().map(|held| held.piece.as_str()).collect();
        assert_eq!(spelled, " a a", "{read:?}");
        assert!(read.iter().all(|held| !held.piece.is_empty()), "{read:?}");
    }

    #[test]
    fn a_byte_of_a_split_character_is_written_as_a_byte() {
        let disk = OnDisk::chatml("bytes");
        let tokenizer = Tokenizer::own(&disk.model).expect("MCF's own tokenizer");
        let read = tokenizer.encode("é", false).expect("a reading");
        let pieces: Vec<&str> = read.iter().map(|held| held.piece.as_str()).collect();
        assert!(pieces.contains(&"<0xC3>"), "{pieces:?}");
        assert!(pieces.contains(&"<0xA9>"), "{pieces:?}");
        assert!(
            !pieces
                .iter()
                .any(|piece| piece.is_empty() || piece.contains('\u{FFFD}')),
            "{pieces:?}"
        );
    }

    #[test]
    fn an_addressed_turn_holds_each_marker_as_one_token() {
        let disk = OnDisk::chatml("addressed");
        let tokenizer = Tokenizer::own(&disk.model).expect("MCF's own tokenizer");
        let read = tokenizer
            .addressed(&[
                Piece::Marker("<|im_start|>".to_owned()),
                Piece::Text("a".to_owned()),
                Piece::Marker("<|im_end|>".to_owned()),
            ])
            .expect("an addressed turn");
        let pieces: Vec<&str> = read.iter().map(|held| held.piece.as_str()).collect();
        assert!(
            pieces
                .iter()
                .filter(|piece| **piece == "<|im_start|>")
                .count()
                == 1,
            "{pieces:?}"
        );
        assert!(
            pieces
                .iter()
                .filter(|piece| **piece == "<|im_end|>")
                .count()
                == 1,
            "{pieces:?}"
        );
        assert!(pieces.contains(&" a"), "{pieces:?}");
    }

    #[test]
    fn a_marker_that_is_not_a_token_is_refused_by_name() {
        let disk = OnDisk::chatml("refused");
        let tokenizer = Tokenizer::own(&disk.model).expect("MCF's own tokenizer");
        let refused = tokenizer
            .addressed(&[
                Piece::Marker("<|start_header_id|>".to_owned()),
                Piece::Text("a".to_owned()),
            ])
            .expect_err("a spelling that is not a token cannot open anything");
        assert_eq!(
            refused.category(),
            mcf_core::failure::Category::ConfigInvalid,
            "{refused}"
        );
        assert_eq!(
            refused.context_value("marker"),
            Some("<|start_header_id|>"),
            "the refusal names the marker: {refused}"
        );
    }

    #[test]
    fn a_bare_prompt_has_nothing_before_it_but_the_beginning() {
        let disk = OnDisk::chatml("bare");
        let tokenizer = Tokenizer::own(&disk.model).expect("MCF's own tokenizer");
        let Received {
            read,
            before,
            under,
        } = received(&tokenizer, &disk.root, &disk.model, "a", false).expect("a reading");
        assert!(under.contains("no addressing is on file"), "{under}");
        assert_eq!(read.len() - before, 1, "{read:?} before {before}");
        let own: Vec<&str> = read[before..]
            .iter()
            .map(|held| held.piece.as_str())
            .collect();
        assert_eq!(own, vec![" a"]);
        assert_eq!(before, 0, "{read:?}");
    }

    #[test]
    fn an_addressed_prompt_begins_after_its_opening() {
        let disk = OnDisk::chatml("opening");
        let _wrote = crate::configured::write(&disk.root, &disk.model, &chatml_addressing())
            .expect("an addressing on file");
        let tokenizer = Tokenizer::own(&disk.model).expect("MCF's own tokenizer");
        let opened = received(&tokenizer, &disk.root, &disk.model, "a", false).expect("a reading");
        assert!(opened.under.contains("im_start"), "{}", opened.under);
        assert_eq!(opened.read[0].piece, "<|im_start|>", "{:?}", opened.read);
        let own: Vec<&str> = opened.read[opened.before..]
            .iter()
            .map(|held| held.piece.as_str())
            .collect();
        assert_eq!(
            own,
            vec![" a"],
            "{:?} before {}",
            opened.read,
            opened.before
        );

        let closed = received(&tokenizer, &disk.root, &disk.model, "a", true).expect("a reading");
        assert_eq!(closed.before, opened.before);
        assert!(closed.read.len() > opened.read.len());
        assert_eq!(
            closed.read.last().map(|held| held.piece.as_str()),
            Some("\n"),
            "{:?}",
            closed.read
        );
        assert_eq!(
            addressed_as(&tokenizer, "a", &chatml_addressing()).expect("the turn"),
            closed.tokens(),
            "the generation's turn is the reading's turn"
        );
    }

    #[test]
    fn the_account_says_what_the_prompt_went_as() {
        use super::{BARE_PROMPT, Given, OWN_PIECES, addressing_label};
        let on_file = chatml_addressing();
        let identifiers = [1_usize, 2, 3];
        let pieces = [mcf_standin::tokenizer::Piece::Text("a".to_owned())];
        let (own_identifiers, own_pieces) =
            (Given::Identifiers(&identifiers), Given::Pieces(&pieces));
        assert_eq!(
            addressing_label(Some(&on_file), Given::Text, true).as_deref(),
            Some(on_file.provenance().as_str())
        );
        assert!(
            addressing_label(Some(&on_file), own_identifiers, true)
                .is_some_and(|label| label.ends_with("the caller sent its own identifiers"))
        );
        assert!(
            addressing_label(Some(&on_file), own_pieces, true)
                .is_some_and(|label| label.ends_with("the caller sent its own markers and text"))
        );
        assert_eq!(
            addressing_label(None, Given::Text, true).as_deref(),
            Some(BARE_PROMPT)
        );
        assert_eq!(
            addressing_label(None, own_pieces, true).as_deref(),
            Some(OWN_PIECES)
        );
        assert_eq!(addressing_label(None, own_identifiers, true), None);
        assert_eq!(addressing_label(None, own_pieces, false), None);
        assert_eq!(addressing_label(None, Given::Text, false), None);
    }

    #[test]
    fn what_a_request_carried_is_read_in_order() {
        use super::Given;
        let identifiers = [4_usize, 5];
        let pieces = [mcf_standin::tokenizer::Piece::Marker("<s>".to_owned())];
        assert!(matches!(
            Given::from_request(Some(&identifiers), Some(&pieces)),
            Given::Identifiers(_)
        ));
        assert!(matches!(
            Given::from_request(None, Some(&pieces)),
            Given::Pieces(_)
        ));
        assert!(matches!(Given::from_request(None, None), Given::Text));
        assert_eq!(
            Given::from_request(Some(&identifiers), None).identifiers(),
            Some(&identifiers[..])
        );
        assert_eq!(Given::from_request(None, Some(&pieces)).identifiers(), None);
        assert!(Given::from_request(None, Some(&pieces)).is_own_turn());
        assert!(!Given::from_request(None, None).is_own_turn());
    }

    #[test]
    fn what_a_turn_of_pieces_leaves_the_model_inside_of() {
        use super::Given;
        use mcf_standin::tokenizer::Piece;
        let opened = [
            Piece::Marker("<|user|>".to_owned()),
            Piece::Text("q".to_owned()),
            Piece::Marker("<|assistant|>".to_owned()),
            Piece::Marker("<think>".to_owned()),
        ];
        assert_eq!(
            Given::Pieces(&opened).trailing_markers().as_deref(),
            Some("<|assistant|><think>")
        );
        let text_last = [
            Piece::Marker("<|im_start|>".to_owned()),
            Piece::Text("assistant\n".to_owned()),
        ];
        assert_eq!(Given::Pieces(&text_last).trailing_markers(), None);
        assert_eq!(Given::Pieces(&[]).trailing_markers(), None);
        assert_eq!(Given::Identifiers(&[1]).trailing_markers(), None);
        assert_eq!(Given::Text.trailing_markers(), None);
    }

    #[test]
    fn the_answer_past_a_closed_marker_goes_on_the_wire_apart() {
        use super::{Produced, Said};
        use mcf_record::json::Value;
        let account = Value::map([(
            "before_the_answer",
            Value::map([("closed", Value::Bool(true)), ("tokens", Value::Integer(3))]),
        )]);
        let produced = Produced {
            account: account.clone(),
            said: Some(Said {
                text: "<think>hm</think>4".to_owned(),
                tokens: vec![1, 2, 3, 4],
                answer: Some("4".to_owned()),
            }),
        };
        let wire = produced.on_the_wire();
        assert_eq!(
            wire.get("before_the_answer")
                .and_then(|before| before.get("answer"))
                .and_then(Value::as_text),
            Some("4")
        );
        assert_eq!(
            wire.get("text").and_then(Value::as_text),
            Some("<think>hm</think>4")
        );
        assert_eq!(
            account
                .get("before_the_answer")
                .and_then(|before| before.get("answer")),
            None
        );
        let unclosed = Produced {
            account: Value::map([("before_the_answer", Value::Null)]),
            said: Some(Said {
                text: "4".to_owned(),
                tokens: vec![4],
                answer: None,
            }),
        };
        assert_eq!(
            unclosed.on_the_wire().get("before_the_answer"),
            Some(&Value::Null)
        );
    }

    #[test]
    fn what_an_applied_addressing_leaves_the_model_inside_of() {
        use super::{Given, tail_of, trailing_markers_of};
        use mcf_standin::tokenizer::Piece;
        let applied = [
            Piece::Marker("<|assistant|>".to_owned()),
            Piece::Marker("<think>".to_owned()),
        ];
        let after = trailing_markers_of(&applied);
        assert_eq!(after.as_deref(), Some("<|assistant|><think>"));
        assert_eq!(
            tail_of(None, None, Given::Text, after.as_ref()),
            Some("<|assistant|><think>")
        );
        let frame = crate::turn::Frame {
            before: "<|user|>".to_owned(),
            after: "<|assistant|></think>".to_owned(),
            asked: "thinking off".to_owned(),
        };
        assert_eq!(
            tail_of(Some(&frame), None, Given::Text, after.as_ref()),
            Some("<|assistant|></think>")
        );
        assert_eq!(
            tail_of(
                None,
                Some("<|assistant|>"),
                Given::Pieces(&applied),
                after.as_ref()
            ),
            Some("<|assistant|>")
        );
        assert_eq!(
            tail_of(None, None, Given::Identifiers(&[1]), after.as_ref()),
            None
        );
        assert_eq!(tail_of(None, None, Given::Text, None), None);
        let text_last = [Piece::Text("assistant\n".to_owned())];
        assert_eq!(trailing_markers_of(&text_last), None);
    }
}

#[cfg(test)]
mod window_tests {
    use super::{TOO_WIDE, window_suits};

    #[test]
    fn a_window_far_wider_than_the_turn_is_not_reused() {
        assert!(window_suits(4096, 4096));
        assert!(window_suits(8192, 4096));
        assert!(window_suits(4096 * TOO_WIDE, 4096));
        assert!(!window_suits(4096 * TOO_WIDE + 1, 4096));
        assert!(!window_suits(262_144, 4096));
        assert!(!window_suits(2048, 4096));
        assert!(window_suits(33_000, 4096));
    }
}
