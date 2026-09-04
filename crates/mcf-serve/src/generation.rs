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

/// How one generation's sampler was told to draw: a seed, a temperature, and
/// how the distribution is cut before the draw.
///
/// **Greedy is MCF's own choice, and it is named as such.** Every measurement
/// draws at temperature 0 unless the caller states otherwise, because a
/// greedy generation is the one two machines can compare to the token
/// (§3.12). A temperature above nought is never a house default — B60 forbids
/// one — so it reaches here only as something a caller stated, and it travels
/// with the seed because the two together are what makes a sample
/// reproducible (D19).
///
/// **The truncation is stated on every request** (B-440). An engine that is
/// sent a temperature and nothing else fills in `top_k`, `top_p` and `min_p`
/// for itself — from the file where the file recommends, and from its own
/// house values where it does not — and a seeded draw then runs under a
/// condition nobody stated (F157). So the three go with every request, each
/// either what the file declared or *off*, and the account names them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Draw {
    /// The seed, which is a condition of the result (D19).
    pub seed: u64,
    /// The temperature, in thousandths. Nought is greedy — the limit of the
    /// distribution rather than a special case.
    pub temperature: Thousandths,
    /// How the distribution is cut before the draw.
    pub truncation: Truncation,
}

/// One truncation parameter: what the file declared, or off.
///
/// *Off* is a stated value and not an absence (A7): it is sent to the engine
/// as the number that leaves the distribution whole, and recorded as the word.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stated<T> {
    /// The file recommends this, and it is sent as read.
    Declared(T),
    /// The file recommends nothing, and the engine is told to cut nothing.
    Off,
}

impl<T: fmt::Display> Stated<T> {
    /// The value as a client reads it: the number, or `off`.
    fn value(self) -> Value {
        match self {
            Self::Declared(held) => Value::text(held.to_string()),
            Self::Off => Value::text("off"),
        }
    }
}

/// How the distribution is cut before a draw above nought: the three cuts the
/// provisioned engine applies, each as the file declared it or off.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Truncation {
    /// How many of the likeliest tokens are kept.
    pub top_k: Stated<u32>,
    /// The probability mass the kept set must reach.
    pub top_p: Stated<Thousandths>,
    /// The least probability kept, as a fraction of the likeliest token's.
    pub min_p: Stated<Thousandths>,
    /// Whose the cut is, or why there is none.
    pub whose: Whose,
}

/// Where a cut came from — or, where there is none, why not: a draw with
/// none because the file declares none is a decision, and a greedy draw
/// with none because no cut could change it is not, and the account says
/// which (A7).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Whose {
    /// The file's own recommendation, adopted as stated.
    File,
    /// The file recommends none, so none: never the engine's own.
    NoneDeclared,
    /// Greedy takes the likeliest token, which is the likeliest under any cut.
    Moot,
}

impl Whose {
    /// The words the account carries.
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
    /// No cut, because a greedy draw could not be changed by one.
    pub const OFF: Self = Self {
        top_k: Stated::Off,
        top_p: Stated::Off,
        min_p: Stated::Off,
        whose: Whose::Moot,
    };

    /// What the file recommends, where it recommends anything, and off where
    /// it does not — parameter by parameter, because a file that states a
    /// `top_p` and no `top_k` has stated exactly that (A7).
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

    /// Whether any of the three is on.
    #[must_use]
    pub const fn any_stated(self) -> bool {
        !matches!(
            (self.top_k, self.top_p, self.min_p),
            (Stated::Off, Stated::Off, Stated::Off)
        )
    }

    /// Whether the cut is the file's.
    #[must_use]
    pub const fn is_declared(self) -> bool {
        matches!(self.whose, Whose::File)
    }

    /// `top_k` as the engine takes it: nought keeps every token.
    #[must_use]
    pub const fn top_k_sent(self) -> u32 {
        match self.top_k {
            Stated::Declared(held) => held,
            Stated::Off => 0,
        }
    }

    /// `top_p` as the engine takes it: one is the whole mass.
    #[must_use]
    pub const fn top_p_sent(self) -> Thousandths {
        match self.top_p {
            Stated::Declared(held) => held,
            Stated::Off => Thousandths(1_000),
        }
    }

    /// `min_p` as the engine takes it: nought drops nothing.
    #[must_use]
    pub const fn min_p_sent(self) -> Thousandths {
        match self.min_p {
            Stated::Declared(held) => held,
            Stated::Off => Thousandths(0),
        }
    }

    /// The three, as every account and the served report carry them, with
    /// whose they are.
    #[must_use]
    pub fn entries(self) -> [(&'static str, Value); 4] {
        [
            ("top_k", self.top_k.value()),
            ("top_p", self.top_p.value()),
            ("min_p", self.min_p.value()),
            ("truncation", Value::text(self.whose.said())),
        ]
    }

    /// One line of it, for a page: `top_k 20 · top_p 0.950 · min_p off`.
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
    /// The likeliest token every time, under this seed. Nothing is cut,
    /// because nothing needs to be: the likeliest token is the likeliest
    /// token under any cut.
    #[must_use]
    pub const fn greedy(seed: u64) -> Self {
        Self {
            seed,
            temperature: Thousandths(0),
            truncation: Truncation::OFF,
        }
    }

    /// Whether the seed can change anything.
    #[must_use]
    pub const fn is_greedy(self) -> bool {
        self.temperature.0 == 0
    }

    /// The sampler, as the account names it.
    #[must_use]
    pub fn sampler(self) -> String {
        if self.is_greedy() {
            "greedy".to_owned()
        } else {
            format!("temperature {}", self.temperature)
        }
    }

    /// What the stand-in's sampler is told; the thousandths become a float
    /// only inside the stand-in, where floats are allowed to live.
    fn settings(self) -> Settings {
        Settings::at_thousandths(
            self.temperature.0,
            usize::try_from(self.truncation.top_k_sent()).unwrap_or(usize::MAX),
            self.truncation.top_p_sent().0,
            self.truncation.min_p_sent().0,
        )
    }
}

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

/// What the caller sent beside the prompt's text, if anything.
///
/// Three callers, three shapes. A prompt alone is addressed and framed here;
/// identifiers are the caller's own segmentation and go as sent; pieces are
/// the caller's own turn — its markers and its text — left for the tokenizer
/// of the engine that answers to read, which is the only tokenizer that can
/// read every vocabulary that engine generates from (B-442, F158).
#[derive(Debug, Clone, Copy)]
pub(crate) enum Given<'a> {
    /// The prompt's text and nothing else.
    Text,
    /// The turn, segmented by the caller.
    Identifiers(&'a [usize]),
    /// The turn as markers and text, segmented by whichever engine answers.
    Pieces(&'a [mcf_standin::tokenizer::Piece]),
}

impl<'a> Given<'a> {
    /// What a request carried, in order of how much of the work the caller
    /// did: identifiers first, since they are already read.
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

    /// The caller's own segmentation, where they sent one.
    pub(crate) fn identifiers(self) -> Option<&'a [usize]> {
        match self {
            Self::Identifiers(tokens) => Some(tokens),
            Self::Text | Self::Pieces(_) => None,
        }
    }

    /// Whether the caller built the turn, either way — so it is not
    /// addressed or framed again.
    pub(crate) fn is_own_turn(self) -> bool {
        !matches!(self, Self::Text)
    }
}

/// The identifiers the engine is sent, or `None` where a prompt goes as text.
///
/// The caller's identifiers are never touched; the caller's pieces are read
/// by the tokenizer of the engine that answers and nothing is put around
/// them; a prompt is framed by the engine from the model's template where a
/// turn was asked for, addressed as somebody derived where they did, and
/// otherwise read plainly by the engine that will answer it (B-441, B-442,
/// F160).
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
        // **A prompt with no addressing on file goes to the server too, read
        // by its own tokenizer.** Before this it went to the completion tool
        // as text (B-376's first cut, from before the engine could read for
        // MCF), which loaded the model again for every generation — ten
        // times in one prompt report — and handed back no identifiers, so the
        // report's held reading was never taken and said *needs the served
        // engine* of an engine that was serving. The bare prompt is read the
        // way the ranking reads it, with the beginning marker and nothing
        // else, so the answer and the reading are of one prompt (§3.4, F160).
        (Text, _, None, Ok(Chosen::Provisioned(llama))) => {
            place.engine(llama).encode(prompt, true).map(read)
        }
        _ => Ok(None),
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
    draw: Draw,
    given: Given<'_>,
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
    // How the caller asked the turn framed, where they asked the engine to
    // frame it from the model's own template (D47).
    turn: Option<&crate::turn::Turn>,
    // A picture to show the model, where the caller sent one (B-452). It
    // goes inside a turn the engine frames, the only turn with a place for
    // it, so a caller that named no switch is framed as if they had named
    // none — the template's own defaults.
    picture: Option<&Path>,
    // What the engine is started with beyond the plain load, where the
    // caller asked for either: the model's own draft head, a rope scaling.
    // Nothing here is inferred from the file — a feature a file declares is
    // reported and left alone until somebody asks for it (B-456, D43).
    started: crate::declared::Started,
    // Who is waiting, so that the engine's request is closed when they
    // leave and its progress reaches them while they stay (D48).
    waiting: crate::served::Waiting<'_>,
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

    // What the file says it has, read before anything is loaded: it decides
    // whether a switch can be honoured at all, and it is what the account
    // sets the run against.
    let declared = crate::declared::Declared::of(&resolved(store, named));
    let (chosen, gpu_layers, context) = engine_for(mcf_home, picked, engine, started, &declared);
    // **The addressed turn is tokenized by the engine that will answer it**
    // (B-441): the server's own tokenizer where the server answers, MCF's
    // where the stand-in does. A turn that cannot be built is the
    // generation's failure, not a bare prompt sent instead (F158).
    let place = Place {
        store,
        runtime,
        named,
        gpu_layers,
        context,
        server,
        started,
    };
    // The picture is read before anything is started: a file that is not
    // there is the caller's to fix, and loading a model to find that out
    // would be a minute spent on nothing.
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
    // Which engine a failure is charged to. An account of a served engine's
    // refusal that named the stand-in — which is what every failure said
    // until D48's cancellations made the misstatement common — was an
    // account of a run that did not happen (A7).
    let charged = match &chosen {
        Ok(Chosen::Provisioned(llama)) => Some(served_engine_name(llama)),
        Ok(Chosen::StandIn) | Err(_) => None,
    };
    let produced = match (chosen, wrapped, &picture) {
        // MCF's own engine reads text and nothing else; a picture sent to
        // it is refused rather than dropped on the way (A2), and refused
        // for that reason, before the frame the picture forced is missed.
        (Ok(Chosen::StandIn), _, Ok(Some(_))) => Err(text_only()),
        (_, Err(failure), _) | (Err(failure), _, _) => Err(failure),
        (_, _, Err(failure)) => Err(failure.clone()),
        // A turn of identifiers goes to the server, which can be given one;
        // a prompt goes to the completion tool, which cannot (B-376).
        (Ok(Chosen::Provisioned(llama)), Ok(wrapped), Ok(_)) => {
            match wrapped.as_deref().or(given.identifiers()) {
                Some(tokens) => {
                    let sent = Sent {
                        tokens,
                        limit,
                        pinned,
                        tail: frame.as_ref().map(|frame| frame.after.as_str()),
                        shown,
                    };
                    through_served(
                        store, &llama, server, runtime, named, &sent, draw, gpu_layers, context,
                        started, &declared, waiting, writer,
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

/// The refusal MCF's own engine gives a picture.
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

/// Which engine answers, on which device, in which window — and whether it
/// can be asked for what was asked of it.
///
/// The switches beyond the plain load are settled here because this is
/// before anything is started: an engine loaded and then refused would be a
/// minute spent to say no (A2, B-456).
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

/// The refusal MCF's own engine gives a switch meant for the provisioned one.
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

/// Which engine answers, on which device, in which window.
fn chosen_engine(
    mcf_home: &Path,
    picked: Option<(crate::adapters::ProvisionedLlama, u32, u64)>,
    engine: Option<&str>,
) -> (Result<Chosen, Failure>, u32, u64) {
    match (picked, engine) {
        // A caller that asked for MCF's own engine gets it, whatever was
        // resolved: naming the engine is the point of the argument (§3.15).
        (_, Some("stand-in")) => (Ok(Chosen::StandIn), 0, 0),
        (Some((llama, layers, window)), _) => (Ok(Chosen::Provisioned(llama)), layers, window),
        (None, asked) => (choose_engine(mcf_home, asked), 0, 0),
    }
}

/// The turn as text with the picture's place in it: the frame's two halves
/// around the engine's marker and then the words, which is where the chat
/// path of the same engine puts a picture sent before its text (B-452).
fn shown_as(frame: &crate::turn::Frame, marker: &str, prompt: &str) -> String {
    format!("{}{marker}{prompt}{}", frame.before, frame.after)
}

/// The account's word for a turn the engine framed.
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

/// What the prompt went to the model as, for the account: the derived
/// addressing's provenance where one is on file (and that it was not
/// applied, where the caller sent its own turn), the caller's own pieces as
/// the engine read them, the bare prompt where nothing is on file; nothing
/// where the turn was never built.
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

/// What a turn the caller sent as markers and text went as, in words
/// (§3.15, B-442).
pub(crate) const OWN_PIECES: &str = "the caller's own markers and text, each read by the \
                                     tokenizer of the engine that answered";

/// What a prompt with no addressing on file went as, in words (§3.15).
pub(crate) const BARE_PROMPT: &str = "the prompt alone: no addressing is on file for this \
                                      model, so it went with no turn markers around it";

/// The provenance travels into the account, so that a measurement taken
/// through a derived configuration carries what set it — which is what
/// makes *are yesterday's number and today's comparable* answerable rather
/// than assumed (D43, §3.4).
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

/// Where a turn is built: everything the engine's tokenizer needs but the
/// engine, which is chosen after.
struct Place<'a> {
    store: &'a Path,
    runtime: &'a Path,
    named: &'a str,
    gpu_layers: u32,
    context: u64,
    server: &'a std::sync::Mutex<Option<Served>>,
    /// What the engine is started with beyond the plain load, so that the
    /// engine that frames a turn is the engine that answers it.
    started: crate::declared::Started,
}

impl<'a> Place<'a> {
    /// The engine's tokenizer, for the build that was chosen.
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
            },
            server: self.server,
        }
    }
}

/// **The engine frames the turn where the caller asked it to** (D47). A
/// caller that named a switch — thinking off, an effort, a system turn — is
/// answered from the model's own template as its engine renders it, whatever
/// addressing is on file: they said what they wanted. MCF's own engine runs
/// no template, so on it the request is refused rather than sent under the
/// addressing on file as if the switch had been thrown (A2, §3.15).
///
/// `Ok(None)` where no frame was asked for.
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

/// Which tokenizer reads a prompt: the one that will generate from it
/// (B-441).
///
/// **The tokenizer that generates is the tokenizer that reads.** A prompt
/// report counted and ranked the prompt through MCF's own segmentation while
/// the answers it compared came through the provisioned server, so one
/// report carried two readings of one prompt — and for a vocabulary MCF's
/// tokenizer does not segment, no reading at all, on a model the server had
/// just answered for (F158). The choice is made once, by whichever engine
/// the request resolved to, and every count, rank and addressed turn goes
/// through that one.
pub(crate) enum Tokenizer<'a> {
    /// MCF's own, which is what the stand-in generates with.
    Own(Vocabulary),
    /// The provisioned server, which is what it generates with.
    Engine {
        /// Which model, and where.
        where_it_lives: Where<'a>,
        /// The slot the daemon holds a server in.
        server: &'a std::sync::Mutex<Option<Served>>,
    },
}

/// One token as a tokenizer read it: the identifier and what it spells.
pub(crate) type Read = crate::served::Token;

impl Tokenizer<'_> {
    /// MCF's own tokenizer for this file, or why not.
    ///
    /// # Errors
    ///
    /// The header or the vocabulary could not be read, in the vocabulary's
    /// own words.
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

    /// Who read the prompt, in words, for the account of a generation.
    pub(crate) fn named(&self) -> String {
        match self {
            Self::Own(_) => "MCF's own tokenizer, which its engine generates with".to_owned(),
            Self::Engine { .. } => format!("{}, which generated", self.who()),
        }
    }

    /// Who read, in words, where nothing was generated: a count is a reading
    /// by the tokenizer that *would* generate, and saying it generated would
    /// be a claim about a run that did not happen (A7, B-442).
    pub(crate) fn reader(&self) -> String {
        match self {
            Self::Own(_) => "MCF's own tokenizer, which its engine generates with".to_owned(),
            Self::Engine { .. } => format!("{}, which generates for this model", self.who()),
        }
    }

    /// The tokenizer as a noun: MCF's own, or the provisioned server's by
    /// component and commit.
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

    /// Text as the model receives it, with the model's own beginning
    /// convention where `with_beginning` asks for it.
    ///
    /// A person's text: a marker spelled in it stays spelled, on either
    /// engine (F26, F161). The model's own — a marker, a rendered turn — is
    /// read by [`Self::own_words`].
    ///
    /// # Errors
    ///
    /// Text the vocabulary cannot represent, or a server that did not answer.
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
                let mut slot = server.lock().map_err(|_poisoned| {
                    unavailable("the served engine's slot was left poisoned by an earlier failure")
                })?;
                let (engine, _reused) = serving(&mut slot, where_it_lives, SMALLEST_WINDOW)?;
                engine.tokenize(text, with_beginning, false)
            }
        }
    }

    /// The turn as this engine renders it from the model's own template
    /// (D47).
    ///
    /// # Errors
    ///
    /// MCF's own engine, which runs no template; a server that did not
    /// answer; or a template that refused the switches, in its own words.
    pub(crate) fn framed(&self, turn: &crate::turn::Turn) -> Result<crate::turn::Frame, Failure> {
        match self {
            Self::Own(_) => Err(unavailable("MCF's own engine runs no template")),
            Self::Engine {
                where_it_lives,
                server,
            } => {
                let mut slot = server.lock().map_err(|_poisoned| {
                    unavailable("the served engine's slot was left poisoned by an earlier failure")
                })?;
                let (engine, _reused) = serving(&mut slot, where_it_lives, SMALLEST_WINDOW)?;
                crate::turn::frame(engine, turn)
            }
        }
    }

    /// The model's own text as it receives it: a marker from its template,
    /// or a turn its template rendered, with every control token in it taken
    /// as itself.
    ///
    /// Only the engine reads this way. MCF's own tokenizer looks a marker up
    /// as one token and never parses one out of text, so on it the model's
    /// own words are the markers of [`Self::addressed`] and nothing else.
    ///
    /// # Errors
    ///
    /// A server that did not answer, or MCF's own engine, which cannot read
    /// a rendered turn.
    pub(crate) fn own_words(&self, text: &str, with_beginning: bool) -> Result<Vec<Read>, Failure> {
        match self {
            Self::Own(_) => Err(unavailable(
                "MCF's own engine reads markers one at a time and cannot read a rendered turn",
            )),
            Self::Engine {
                where_it_lives,
                server,
            } => {
                let mut slot = server.lock().map_err(|_poisoned| {
                    unavailable("the served engine's slot was left poisoned by an earlier failure")
                })?;
                let (engine, _reused) = serving(&mut slot, where_it_lives, SMALLEST_WINDOW)?;
                engine.tokenize(text, with_beginning, true)
            }
        }
    }

    /// An addressed turn: the beginning convention, then each piece — a
    /// marker looked up, which must be one token of the vocabulary and not a
    /// spelling that segments into several (F37); text segmented the ordinary
    /// way.
    ///
    /// # Errors
    ///
    /// A marker that is not one token, text the vocabulary cannot represent,
    /// or a server that did not answer.
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
                    // Every marker is a token, so what refused was text; the
                    // vocabulary says which, in its own words.
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

/// Identifiers with what each spells, by MCF's own vocabulary.
///
/// Each token's piece is the bytes it contributes, not a decoding of it on
/// its own — one byte of a character the vocabulary splits is not text by
/// itself, and is written as a byte (F19). An identifier the vocabulary does
/// not have is said rather than dropped (A1).
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

/// A prompt wrapped the way somebody decided this model should be addressed,
/// by the tokenizer of the engine that will answer it.
///
/// # Errors
///
/// A turn that cannot be built — a marker the vocabulary does not hold as one
/// token, text it cannot represent, a server that did not answer — is a
/// failure and not a fallback to the bare prompt: wrapping *none* of a turn
/// somebody put on file, silently, would be the hidden choice §3.15 forbids
/// (F37, F158).
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

/// A prompt inside the frame the engine rendered, read by the engine: the
/// frame's two halves as the model's own words, the person's text as a
/// person's (F26, F161).
///
/// # Errors
///
/// A server that did not answer, or text it cannot represent.
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

/// A prompt as the model receives it, and where its own tokens begin.
///
/// The derived addressing around the text where somebody has put one on file,
/// the bare text otherwise — the same choice a generation makes, so that a
/// reading of the prompt is a reading of the prompt the answer was given
/// (§3.4). `after` is left off where the caller wants what *follows* the
/// prompt to be open, which is what ranking the prompt's own tokens needs.
pub(crate) struct Received {
    /// The identifiers, each with what it spells.
    pub(crate) read: Vec<Read>,
    /// How many of them stand before the prompt's own: the beginning marker
    /// and the addressing's opening pieces.
    pub(crate) before: usize,
    /// What the prompt was addressed as, in words, for the report.
    pub(crate) under: String,
}

impl Received {
    /// The identifiers alone.
    pub(crate) fn tokens(&self) -> Vec<usize> {
        self.read.iter().map(|held| held.id).collect()
    }
}

/// A prompt as the model receives it, read by the tokenizer of the engine
/// that answers it.
///
/// # Errors
///
/// Whatever the tokenizer refused.
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
        // What the beginning convention adds is whatever the prompt's own
        // reading is surrounded by once it is asked for — not a reading of
        // empty text, which a unigram vocabulary encodes as the space it
        // prefixes everything with.
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
    // Pieces encode independently (a marker is looked up, text is segmented),
    // so the opening on its own is a prefix of the whole turn.
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

/// Where `held` sits within `read`, as a run of the same identifiers.
///
/// An empty `held` sits at the end: a prompt that reads as nothing has
/// nothing of its own after whatever surrounds it.
fn offset_of(read: &[Read], held: &[Read]) -> Option<usize> {
    if held.is_empty() {
        return Some(read.len());
    }
    let ids = |tokens: &[Read]| tokens.iter().map(|token| token.id).collect::<Vec<_>>();
    let within = ids(read);
    let wanted = ids(held);
    within.windows(wanted.len()).position(|run| run == wanted)
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
    /// What the engine is started with beyond the plain load. An engine
    /// started under one set of these is not the engine for another.
    pub started: crate::declared::Started,
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
    let mut slot = server.lock().map_err(|_poisoned| {
        unavailable("the served engine's slot was left poisoned by an earlier failure")
    })?;
    let (engine, _reused) = serving(
        &mut slot,
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

/// The server for this model: the one already holding it where its window
/// is wide enough, started otherwise.
///
/// A server holding a different model is stopped rather than kept beside
/// this one: two resident models is a decision about memory nobody has
/// taken (D41, DEC-018), and taking it here silently would be the hidden
/// choice §3.15 forbids. **A server whose window is too small for this turn
/// is not this turn's server.** Reuse went by the model alone, so a ladder's
/// first rung opened a 4,096-token window and every rung past it was sent
/// to that server, which refuses a turn longer than its window — and the
/// refusal was read as a pair of runs that did not separate (F152). Written
/// once for the three things that ask — a generation, a ranking, a
/// tokenization — so that they cannot answer differently (B-072).
///
/// Returns the server and whether it was already holding the model.
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
    } = *where_it_lives;
    let path = resolved(store, named);
    // **And started with the same switches.** An engine holding this model
    // with its draft head running is not the engine for a request that asked
    // for the plain load: the tokens come from a different arrangement of
    // the same weights, and answering one from the other would put a
    // condition in the account that was not the condition (A6, B-456).
    let reused = slot
        .as_ref()
        .is_some_and(|held| held.model == path && held.window >= window && held.started == started);
    if !reused {
        *slot = None;
        // The projector its publisher shipped beside it, where one sits
        // there: hosted whole, so that a picture through the socket reaches
        // the same model a caller on the port sees (B-449, B-452).
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
        tail: _,
        shown,
    } = *sent;
    let path = resolved(store, named);
    let metadata = std::fs::metadata(&path).map_err(|error| missing(named, &path, &error))?;
    let held = metadata.len();
    if shown.is_some() {
        projector_present(&path)?;
    }

    let mut slot = server.lock().map_err(|_poisoned| {
        unavailable("the served engine's slot was left poisoned by an earlier failure")
    })?;
    let where_it_lives = Where {
        store,
        llama,
        runtime,
        named,
        gpu_layers,
        context,
        started,
    };
    let room = if shown.is_some() { PICTURE_ROOM } else { 0 };
    let window = window_for(tokens.len().saturating_add(room), limit, context);
    let (engine, reused) = serving(&mut slot, &where_it_lives, window)?;

    let text = shown.map(|shown| shown.placed_by(engine)).transpose()?;
    let prompt = match (shown, &text) {
        (Some(shown), Some(text)) => crate::served::Prompt::Shown {
            text,
            picture: &shown.picture.bytes,
        },
        _ => crate::served::Prompt::Identifiers(tokens),
    };
    let (completed, pieces) = as_it_arrives(engine, prompt, limit, draw, pinned, waiting, writer)?;
    // Read after the turn, while the mark includes it (B-424).
    let peak_resident = engine.peak_resident_bytes();
    let ran_in = engine.window;

    let (before, text) = streamed(engine, &path, sent, &completed)?;
    whole_where_nothing_arrived(writer, pieces, &text);

    let mut conditions = conditions(named, Some((&path, held)), draw, limit);
    if let Value::Map(fields) = &mut conditions {
        fields.insert("engine".to_owned(), Value::text(served_engine_name(llama)));
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
        // finding, and either way it is recorded rather than assumed. A turn
        // with a picture went as text, so nothing was sent as identifiers
        // and the field says so rather than counting the words (A7).
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
        // What the file says it has, and what the engine was started with.
        // Both, always: a run of a model whose file declares a draft head
        // nobody started is a run that left something on the disk, and the
        // only place a person can learn that is here (B-456, A7).
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
        // The engine's peak resident memory, or nothing where the kernel did
        // not say — never zero (A7).
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

/// A picture needs the projector, and whether the model has one is known
/// before anything is loaded: a refusal here costs nobody a minute.
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

/// The account of a served turn, from its conditions and what came back.
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
            // The turn as sent, which the answer was given (§3.4): an account
            // that said *?* here was withholding a count MCF held (A7). With
            // a picture the engine's own count, since the picture's tokens
            // are in it and only the engine made them.
            (
                "prompt_tokens",
                Value::Integer(i64::try_from(prompt_tokens).unwrap_or(i64::MAX)),
            ),
            ("stopped", Value::text(completed.stop.written())),
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
        }),
    }
}

/// The account of a generation that did not happen: the failure, and the
/// conditions it was asked under, charged to the engine that was chosen.
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
        // Whether the server had the model loaded when it refused was not
        // observed here, and is not guessed.
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

/// How a served engine is named in an account.
fn served_engine_name(llama: &crate::adapters::ProvisionedLlama) -> String {
    format!(
        "provisioned {} server @{} from {}",
        llama.component,
        llama.commit.get(..12).unwrap_or(&llama.commit),
        llama.prefix.display()
    )
}

/// What the model spent before its answer, where it opened a marker of its
/// own or was put inside one by the turn (B-451). Not for a pinned run: a
/// timing reads its count and never its text.
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

/// The turn, with every piece of it written to whoever asked as the engine
/// writes it (B-454).
///
/// **What has arrived stays arrived.** An engine that dies in the ninth
/// minute of a ten-minute thought leaves what it produced on the page, with
/// the failure after it, which is the most useful thing it can leave (A2,
/// A4). The last piece is held back until the turn's end says whether it was
/// the model ending its turn, which the account leaves out (F142).
#[allow(
    clippy::too_many_arguments,
    reason = "one request's conditions, each named in the account"
)]
fn as_it_arrives(
    engine: &Served,
    prompt: crate::served::Prompt<'_>,
    limit: usize,
    draw: Draw,
    pinned: bool,
    waiting: crate::served::Waiting<'_>,
    writer: &mut &UnixStream,
) -> Result<(Completed, usize), Failure> {
    // **A timing does not stream.** A pinned length is a request whose
    // duration is the measurement, and asking the engine to frame and write
    // every token as it goes is work it would not otherwise do — a condition
    // MCF would have added to the thing it was measuring (A6, B-396). Nobody
    // is watching those anyway: they are spent by a bench or a probe.
    if pinned {
        return engine
            .complete(prompt, limit, draw, pinned, waiting)
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

/// What the turn came to, once its pieces have all been written.
///
/// The text is the model's words spelled by the engine (A4), not the
/// server's `content` as a whole-answer request returns it, which leaves
/// every marker out: a `</think>` the model wrote is where its answer began.
/// The pieces reached the person as they arrived; this is the same text
/// again, for the account, and what the model spent inside a marker before
/// it began answering.
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

/// An engine that answered whole wrote no pieces, so the answer goes as
/// one — which is what this path did before it could be watched (B-454).
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

/// The answer on its way to whoever asked, a piece at a time (B-454).
///
/// **One piece is always held back.** A turn that ends at the model's own
/// end-of-turn token ends with a token the account leaves out — the model
/// ending its turn is not the model saying something (F142) — and the only
/// way to leave it out of what a person watches is to hold each piece until
/// the next one proves it was not the last.
struct Arriving<'a, 'w> {
    /// Where the pieces go.
    writer: &'a mut &'w UnixStream,
    /// The piece that has arrived and may be the last.
    held: Option<(usize, String)>,
    /// Everything handed on so far, for a failure to carry.
    text: String,
    /// How many pieces have gone. Nought at the end is an engine that
    /// answered whole rather than one that said nothing.
    written: usize,
}

impl<'a, 'w> Arriving<'a, 'w> {
    /// Nothing written yet.
    fn new(writer: &'a mut &'w UnixStream) -> Self {
        Self {
            writer,
            held: None,
            text: String::new(),
            written: 0,
        }
    }

    /// One piece from the engine: the one before it is now known not to be
    /// the last, so it goes.
    fn piece(&mut self, at: usize, piece: &str) {
        let held = self.held.replace((at, piece.to_owned()));
        self.write(held);
    }

    /// The turn is over: the held piece goes unless it was the model
    /// ending its turn.
    fn close(&mut self, ended_itself: bool) {
        let held = self.held.take();
        if !ended_itself {
            self.write(held);
        }
        let _flushed = self.writer.flush();
    }

    /// The engine stopped: what had arrived goes, and is named on the
    /// failure so that an account of the failure says what was produced
    /// before it (A4).
    fn left_off(&mut self, failure: Failure) -> Failure {
        let held = self.held.take();
        self.write(held);
        let _flushed = self.writer.flush();
        if self.text.is_empty() {
            return failure;
        }
        failure.with_context("produced_before_it_stopped", self.text.clone())
    }

    /// One piece onto the wire.
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

/// One turn as it goes to the served engine: the identifiers, how many
/// tokens, whether that is the length, and where the turn left the model.
#[derive(Clone, Copy)]
struct Sent<'a> {
    /// The turn.
    tokens: &'a [usize],
    /// How many tokens to produce.
    limit: usize,
    /// Whether `limit` is the length rather than a ceiling (B-396).
    pinned: bool,
    /// The rendered frame's text after the person's words, where the engine
    /// framed the turn: what the model was left inside of.
    tail: Option<&'a str>,
    /// The picture and the turn as text around it, where one was shown
    /// (B-452). The identifiers above are then the words alone, counted
    /// for the window and not sent: the turn goes as text.
    shown: Option<Shown<'a>>,
}

/// A picture inside its turn, as the engine is given it.
///
/// The frame and the words apart, not the text they make: where the
/// picture goes is a marker only the engine that answers knows, and that
/// engine is started after this is built.
#[derive(Clone, Copy)]
struct Shown<'a> {
    /// The turn the engine rendered around the words.
    frame: &'a crate::turn::Frame,
    /// The person's words.
    prompt: &'a str,
    /// The picture.
    picture: &'a Picture,
}

impl Shown<'_> {
    /// The text this turn goes as, with the picture at the marker only the
    /// engine that answers knows.
    fn placed_by(self, engine: &Served) -> Result<String, Failure> {
        let marker = engine.media_marker.as_deref().ok_or_else(|| {
            unavailable(
                "the served engine has no marker to stand a picture at, so a picture cannot be \
                 placed in its text",
            )
        })?;
        Ok(shown_as(self.frame, marker, self.prompt))
    }

    /// The record's account: the picture, the projector that read it, and
    /// how the turn was read — as text, with every marker in it a marker,
    /// which is the engine's condition for a picture (F26, F161).
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

/// A picture a caller sent, read from where they said it was.
pub(crate) struct Picture {
    /// Where it was.
    path: std::path::PathBuf,
    /// What it held.
    bytes: Vec<u8>,
}

impl Picture {
    /// Reads the file the caller named.
    ///
    /// # Errors
    ///
    /// A file that could not be read, or one that held nothing — an empty
    /// picture is a mistake and not a picture.
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

    /// The record's account of it: where it was and what it was, so a
    /// reader can tell whether the picture they hold is the one shown.
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

/// How many tokens of window are kept for a picture beyond the words.
///
/// **A stated allowance, because the count is the engine's to make.** A
/// picture becomes as many tokens as the projector's geometry makes of it,
/// and the engine says how many only once it has read the turn. The window
/// is sized before that, so the words are counted and this much is added
/// for the picture; a picture that needs more is refused by the engine in
/// its own words, and the account carries the window it ran in (§3.4).
const PICTURE_ROOM: usize = 4096;

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
    let mut conditions = conditions(named, Some((&path, held)), draw, limit);
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
            settings: draw.settings(),
            seed: draw.seed,
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
                conditions(named, Some((&path, held_bytes)), draw, limit)
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
mod truncation_tests {
    // A test says what went wrong by failing.
    #![allow(clippy::panic, clippy::expect_used)]

    use mcf_core::attested::Attested;
    use mcf_core::configuration::{Sampling, Thousandths};

    use super::{Stated, Truncation, Whose};

    /// **Parameter by parameter** (A7): a file that states a `top_p` and no
    /// `top_k` has stated exactly that, and the cut sent is that `top_p` with
    /// the other two off — not the engine's `top_k` filled in beside it.
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

    /// No recommendation is off on all three, said to be the file's
    /// silence rather than greedy's indifference, and off is what leaves
    /// the distribution whole on the wire.
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

    /// The account and the page carry the three by name, with whose they
    /// are; *off* is the word and not a number, since the number is the
    /// engine's spelling of it.
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

#[cfg(test)]
mod tokenizer_tests {
    // A test says what went wrong by failing.
    #![allow(clippy::panic, clippy::expect_used, clippy::indexing_slicing)]

    use super::{Received, Tokenizer, addressed_as, received};
    use mcf_standin::tokenizer::Piece;
    use std::path::PathBuf;

    /// A model file on disk and a home of its own, gone when the test is.
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

    /// The `ChatML` addressing the fixture vocabulary can carry.
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

    /// What a tokenizer reads, put back together, is what it was given —
    /// the pieces are each token's own contribution and not a decoding of
    /// each alone (F19). The vocabulary's own conventions show: a unigram
    /// vocabulary puts a space in front of the first word, so that is what
    /// the first token spells.
    #[test]
    fn the_pieces_of_a_reading_spell_the_text() {
        let disk = OnDisk::chatml("pieces");
        let tokenizer = Tokenizer::own(&disk.model).expect("MCF's own tokenizer");
        let read = tokenizer.encode("a a", false).expect("a reading");
        let spelled: String = read.iter().map(|held| held.piece.as_str()).collect();
        assert_eq!(spelled, " a a", "{read:?}");
        assert!(read.iter().all(|held| !held.piece.is_empty()), "{read:?}");
    }

    /// One byte of a character the vocabulary splits is written as a byte,
    /// not as a replacement mark and not as nothing (F19, A1).
    #[test]
    fn a_byte_of_a_split_character_is_written_as_a_byte() {
        let disk = OnDisk::chatml("bytes");
        let tokenizer = Tokenizer::own(&disk.model).expect("MCF's own tokenizer");
        // `é` is two bytes and the fixture has no token for it, so it goes
        // as byte tokens.
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

    /// A marker is looked up as one token; the addressing's text is
    /// segmented around it.
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

    /// A spelling the vocabulary does not hold as one token is refused,
    /// named — not segmented into text and sent as if it opened a turn (F37).
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

    /// With nothing on file, the prompt is received bare and every token of
    /// it is the prompt's own.
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
        // The fixture declares no beginning token, and none is invented.
        assert_eq!(before, 0, "{read:?}");
    }

    /// With an addressing on file, the prompt is received inside it, and the
    /// count of what stands before the prompt's own tokens is the opening —
    /// what a rank over the prompt's tokens needs to skip (§3.4).
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

        // The closing pieces, asked for, follow the prompt and change nothing
        // before it.
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

    /// What the account says the prompt went as: the addressing on file,
    /// that it was not applied over a caller's own identifiers or own
    /// pieces, the caller's pieces as the engine read them, or the bare
    /// prompt — and nothing where the turn was never built (§3.15, F160,
    /// B-442).
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

    /// What a request carried, read in the order of how much of the work the
    /// caller did: identifiers are already read and win; pieces are a turn
    /// the engine reads; text alone is neither (B-442).
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
}
