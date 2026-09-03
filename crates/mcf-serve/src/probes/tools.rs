//! Whether a model *emits* a tool call, or only claims it can (B-053, D42,
//! §3.18, §X, A21).
//!
//! **The distinction this exists to make.** A file can declare tool support in
//! three readable ways — its template renders a tool section, its vocabulary
//! carries call markers, its card says so — and none of them is an observation.
//! §3.18 admits three states and A21 forbids a fourth: *declared* is what the
//! artifact says, *verified* is what MCF saw, and a probe writes only the
//! second. The done-when is exactly that separation: a model that emits a
//! well-formed call is told apart from one whose metadata merely claims
//! support.
//!
//! **What "well formed" means here, and it needs no judgement.** Three
//! mechanical questions, each answerable by a parser (A19):
//!
//! 1. Did a call appear where one was asked for — between the markers the
//!    model's own file names, or as the bare object a model without markers
//!    would emit?
//! 2. Does the text between them parse as JSON?
//! 3. Does it name the tool MCF offered, and only that tool?
//!
//! Nothing here reads the *arguments* for sense. Whether `Paris` is a good
//! city to ask the weather of is a judgement, and a probe that formed one would
//! be scoring the model rather than observing it.
//!
//! **The offering is a condition, not a constant.** MCF does not execute chat
//! templates (D46), so how a tool is described to the model is something MCF
//! *chooses* — and a single chosen framing deciding the answer would make the
//! probe a measurement of the framing. So several offerings are tried, the way
//! [`super::addressings`] tries several addressings, and every one is reported
//! with its own count. A model that calls under one and not another has told
//! MCF something true about itself.
//!
//! **A model that answers in prose is not a failure.** It is
//! [`Attempt::NoCall`], and it is the observation that most often matters:
//! a file that declares tools and never emits one is B-058's divergence, which
//! is frequently the most useful thing MCF can say about it.

use std::path::Path;

use mcf_core::probe::{Method, Outcome, Probed};
use mcf_standin::gguf;
use mcf_standin::tokenizer::Vocabulary;

use super::{Addressing, Trial};

/// The tool MCF offers, and the question that needs it.
///
/// One tool with one required argument. More would test whether the model can
/// choose between tools, which is a different question — this one is *does a
/// call come out at all, and is it well formed* — and a probe that asked two
/// questions at once could not say which it had answered.
#[derive(Debug)]
pub struct Offer {
    /// What the tool is called. What a well-formed call must name.
    pub name: &'static str,
    /// The one argument it takes.
    pub argument: &'static str,
    /// A question that cannot be answered without calling it.
    pub question: &'static str,
}

/// The offer every run of this probe uses.
///
/// A weather lookup, because it is the example every tool-calling model was
/// trained on and the probe is asking whether the machinery works rather than
/// whether the model is clever. A question the model *could* answer from
/// memory would confound *did not call* with *did not need to*.
pub const OFFER: Offer = Offer {
    name: "get_weather",
    argument: "city",
    question: "What is the weather in Paris right now? Use the tool.",
};

/// What one trial produced.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Attempt {
    /// A call appeared, parsed, and named the offered tool.
    WellFormed,
    /// Something call-shaped appeared and was not a well-formed call.
    ///
    /// Kept apart from [`Self::NoCall`] because they are different facts about
    /// a model: one tried and got the form wrong, the other did not try. A1 —
    /// folding them together would discard the more interesting of the two.
    Malformed {
        /// Which of the three questions it failed, in a sentence.
        because: String,
    },
    /// No call appeared. The model answered, or said nothing.
    NoCall,
    /// The trial itself could not be run or read (D42's third state).
    CouldNotTell {
        /// Why.
        because: String,
    },
}

/// The shape a call comes in.
///
/// **Read off the model's own template, never chosen by MCF.** A family that
/// writes its calls as a JSON object and one that writes them as a nested
/// function block are both calling correctly; a reader that knew only the
/// first would report the second as making no call, which is a statement
/// about the reader (B-453, A21).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Form {
    /// An object with `name` and `arguments`.
    Json,
    /// `<function=name>` with a `<parameter=key>` for each argument, which is
    /// what several templates write inside their call markers.
    Function,
}

impl Form {
    /// What it is called where a person reads it.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Json => "a JSON object",
            Self::Function => "a nested function block",
        }
    }
}

/// One way of telling a model about a tool.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Offering {
    /// What to call it in a result.
    pub name: String,
    /// The text put to the model, tool description and question together.
    pub text: String,
    /// The markers a call is looked for between, if this offering names any.
    ///
    /// Empty means *a bare JSON object counts*, which is what a model with no
    /// call markers in its vocabulary can produce.
    pub between: Option<(String, String)>,
    /// The shape this offering asked for. A call that came in the other one
    /// is still a call, and is reported as one (B-453).
    pub form: Form,
}

/// What the tool-calling probe observed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Calling {
    /// Every offering, and how many of its trials produced a well-formed call.
    pub well_formed: Vec<(String, usize)>,
    /// Every offering, and how many produced something call-shaped that was
    /// not well formed.
    pub malformed: Vec<(String, usize)>,
    /// Every offering, and how many produced no call at all.
    pub no_call: Vec<(String, usize)>,
    /// The reasons the malformed ones gave, in order, so a reader can see what
    /// the model actually emitted rather than a count of failures (A1).
    pub reasons: Vec<String>,
    /// How many trials each offering had.
    pub of: usize,
    /// The offering that produced the most well-formed calls, if any did.
    pub best: Option<String>,
    /// What the file *declared*, for the divergence (B-058, A21) — never used
    /// to decide.
    pub declared: Declared,
}

/// What the artifact says about tools, read and never believed (A21).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Declared {
    /// Its chat template mentions tools.
    pub template_mentions_tools: bool,
    /// Call markers its template names that its vocabulary also carries.
    pub markers: Vec<String>,
    /// The shape its template writes a call in, read from the template's own
    /// text (B-453).
    pub form: Form,
}

impl Declared {
    /// Whether the file claims tool support by any readable route.
    #[must_use]
    pub fn claims_support(&self) -> bool {
        self.template_mentions_tools || !self.markers.is_empty()
    }
}

/// How a caller runs one trial: the wrapped turn and a budget in, what the
/// model said and how the turn ended out.
///
/// Named because it is the seam that keeps this probe independent of which
/// engine ran it — the engine is a condition and the caller states it.
pub type Ask<'a> = &'a mut dyn FnMut(&[usize], usize) -> (Trial, String);

/// The method, written where the result carries it.
pub const TOOL_CALLING: Method = Method {
    name: "tool-calling",
    asks: "offers one tool with one argument and asks a question that cannot be answered \
           without it, through each way of describing a tool that this model's own file \
           suggests — including the form its own template writes a call in — and counts the \
           trials that produce a call which appears where one was asked for, parses as that \
           form, and names the tool offered",
    decides: "whether MCF may present this model with tools — and nothing else: what the \
              arguments say is not read, because whether they are sensible is a judgement \
              and a probe makes none (D42)",
};

/// Runs the tool-calling probe.
///
/// `generate` takes the wrapped question and a budget and answers with what the
/// model said and how the turn ended. Passing it in keeps this independent of
/// which engine ran it — the engine is a condition and the caller states it.
///
/// # Errors
///
/// Never: a probe that cannot decide reports `Inconclusive` with its reason,
/// which is D42's third state rather than a failure.
pub fn tool_calling(
    model: &Path,
    bytes: &[u8],
    addressing: Option<&Addressing>,
    trials: usize,
    budget: usize,
    engine: &str,
    generate: Ask<'_>,
) -> Probed<Calling> {
    let conditions = super::conditions(&TOOL_CALLING, model, engine);
    let Ok(file) = gguf::parse(bytes) else {
        return Probed::inconclusive(
            TOOL_CALLING,
            "the file could not be read as a model",
            0,
            0,
            conditions,
        );
    };
    let Ok(vocabulary) = Vocabulary::read(&file) else {
        return Probed::inconclusive(
            TOOL_CALLING,
            "the vocabulary could not be read",
            0,
            0,
            conditions,
        );
    };
    let declared = declared(&file, &vocabulary);
    let offerings = offerings(&declared);

    let mut well_formed = Vec::new();
    let mut malformed = Vec::new();
    let mut no_call = Vec::new();
    let mut reasons = Vec::new();
    let mut spent = 0_usize;
    let mut undecidable = Vec::new();

    for offering in &offerings {
        let mut good = 0_usize;
        let mut bad = 0_usize;
        let mut none = 0_usize;
        for _ in 0..trials {
            let Some(wrapped) = wrap(addressing, &vocabulary, &offering.text) else {
                undecidable.push(format!(
                    "{}: the question could not be put through this model's vocabulary",
                    offering.name
                ));
                break;
            };
            let (trial, said) = generate(&wrapped, budget);
            spent = spent.saturating_add(budget);
            match read(&said, offering, &trial) {
                Attempt::WellFormed => good = good.saturating_add(1),
                Attempt::Malformed { because } => {
                    bad = bad.saturating_add(1);
                    reasons.push(format!("{}: {because}", offering.name));
                }
                Attempt::NoCall => none = none.saturating_add(1),
                Attempt::CouldNotTell { because } => {
                    undecidable.push(format!("{}: {because}", offering.name));
                }
            }
        }
        well_formed.push((offering.name.clone(), good));
        malformed.push((offering.name.clone(), bad));
        no_call.push((offering.name.clone(), none));
    }

    // Nothing decidable at all is the third state, not a negative: *the model
    // did not call* and *MCF could not tell* are different facts (D42).
    let decided: usize = well_formed
        .iter()
        .chain(malformed.iter())
        .chain(no_call.iter())
        .map(|(_, count)| *count)
        .sum();
    if decided == 0 {
        let because = undecidable
            .first()
            .cloned()
            .unwrap_or_else(|| "no offering could be put to this model".to_owned());
        return Probed::inconclusive(TOOL_CALLING, &because, trials, spent, conditions);
    }

    // The offerings are built likeliest-first — the model's own form, then
    // its markers, then a plain description — so a tie goes to the earlier
    // one. `max_by_key` keeps the *last* maximum, which named a plain
    // description as best for a model that called just as well in the form
    // its own template writes (B-453).
    let best = well_formed
        .iter()
        .filter(|(_, count)| *count > 0)
        .rev()
        .max_by_key(|(_, count)| *count)
        .map(|(name, _)| name.clone());

    Probed {
        method: TOOL_CALLING,
        outcome: Outcome::Observed(Calling {
            well_formed,
            malformed,
            no_call,
            reasons,
            of: trials,
            best,
            declared,
        }),
        trials,
        tokens: spent,
        conditions,
    }
}

/// The question, wrapped the way the chat-template probe found this model wants
/// to be addressed — or bare, where nothing was found.
fn wrap(
    addressing: Option<&Addressing>,
    vocabulary: &Vocabulary,
    question: &str,
) -> Option<Vec<usize>> {
    match addressing {
        Some(addressing) => addressing.wrap(vocabulary, question),
        None => vocabulary.encode(question, true).ok(),
    }
}

/// What the file says about tools. Read, never believed (A21).
fn declared(file: &gguf::Model, vocabulary: &Vocabulary) -> Declared {
    let template = file
        .get("tokenizer.chat_template")
        .and_then(gguf::Value::as_text)
        .unwrap_or_default()
        .to_owned();
    // The markers a template names *and* the vocabulary carries. A marker the
    // template writes that the vocabulary cannot express is not a token the
    // model was trained on — it is ordinary text — and counting it as a
    // declaration would be reading the template as a program (D46, F37).
    // Each marker once. `markers_in` reports every occurrence, and a template
    // that writes `<tool_call>` four times is not four declarations — the first
    // run of this probe printed the same two markers five times over.
    let mut markers: Vec<String> = Vec::new();
    for marker in super::markers_in(&template) {
        if !marker.to_lowercase().contains("tool") || !vocabulary.has_token(&marker) {
            continue;
        }
        if !markers.contains(&marker) {
            markers.push(marker);
        }
    }
    Declared {
        template_mentions_tools: template.to_lowercase().contains("tool"),
        markers,
        // **The form is read, not assumed.** A template that writes
        // `<function=…>` and `<parameter=…>` writes its calls that way, in
        // its instructions to the model and in how it renders a call back —
        // and the model was trained on what its template writes. This is the
        // template's text read as text, which is what the markers above are
        // read from; running it is what D46 forbids and nothing here does.
        form: if template.contains("<function=") && template.contains("<parameter=") {
            Form::Function
        } else {
            Form::Json
        },
    }
}

/// The ways this model might be told about a tool, likeliest first.
///
/// The first candidate uses the model's own call markers where its file names
/// any, because a model trained to emit them was shown them. The plain one is
/// always present and always last, because it is what a caller with no
/// knowledge of the family would do and the probe has to be able to say
/// whether that works.
fn offerings(declared: &Declared) -> Vec<Offering> {
    let schema = format!(
        "{{\"name\": \"{}\", \"description\": \"look up the current weather in a city\", \
         \"parameters\": {{\"{}\": \"string\"}}}}",
        OFFER.name, OFFER.argument
    );
    let mut found = Vec::new();

    let paired = paired(&declared.markers);
    // **The model's own form, where its template writes one.** A family
    // trained to answer with a function block will answer with one however it
    // is asked, so an offering that asks for a JSON object measures the
    // asking rather than the model (B-453, F164 for the same shape of
    // mistake in another place).
    if declared.form == Form::Function {
        let (open, close) = paired
            .clone()
            .unwrap_or_else(|| ("<tool_call>".to_owned(), "</tool_call>".to_owned()));
        found.push(Offering {
            name: format!("the template's own form, {open}…{close}"),
            text: format!(
                "You have one tool:\n{schema}\nTo use it, reply in this form and nothing \
                 else:\n{}\n\n{}",
                example(&open, &close),
                OFFER.question
            ),
            between: Some((open, close)),
            form: Form::Function,
        });
    }

    if let Some((open, close)) = paired {
        found.push(Offering {
            name: format!("the file's own markers, {open}…{close}"),
            text: format!(
                "You have one tool:\n{schema}\nTo use it, emit {open} then a JSON object with \
                 \"name\" and \"arguments\", then {close}.\n\n{}",
                OFFER.question
            ),
            between: Some((open, close)),
            form: Form::Json,
        });
    }

    found.push(Offering {
        name: "a plain description, a bare object expected".to_owned(),
        text: format!(
            "You have one tool:\n{schema}\nTo use it, reply with only a JSON object having \
             \"name\" and \"arguments\" and nothing else.\n\n{}",
            OFFER.question
        ),
        between: None,
        form: Form::Json,
    });
    found
}

/// A call in the function form, written out for the offered tool.
///
/// The shape the templates that use it write: the call markers around a
/// function block, one parameter block inside it, each on its own line.
fn example(open: &str, close: &str) -> String {
    format!(
        "{open}\n<function={}>\n<parameter={}>\nParis\n</parameter>\n</function>\n{close}",
        OFFER.name, OFFER.argument
    )
}

/// An opening and closing marker from the ones the file names.
///
/// A closing marker is the opening one with a `/` after its bracket, which is
/// how every family that has them writes them. Where no such pair is found
/// there is nothing to pair, and the plain offering is the only candidate —
/// which is a fact about the file rather than a failure.
fn paired(markers: &[String]) -> Option<(String, String)> {
    for marker in markers {
        if marker.contains('/') {
            continue;
        }
        let closing = marker.replacen(['<', '['], "", 1);
        for candidate in markers {
            let stripped = candidate.replacen(['<', '['], "", 1);
            if candidate.contains('/') && stripped.trim_start_matches('/') == closing {
                return Some((marker.clone(), candidate.clone()));
            }
        }
    }
    None
}

/// Reads one answer: did a well-formed call come out?
fn read(said: &str, offering: &Offering, trial: &Trial) -> Attempt {
    if let Trial::CouldNotTell(because) = trial {
        return Attempt::CouldNotTell {
            because: because.clone(),
        };
    }
    let Some(found) = between(said, offering) else {
        return Attempt::NoCall;
    };
    let (candidate, as_asked) = match &found {
        Found::AsAsked(text) => (text.clone(), true),
        Found::Elsewhere(text) => (text.clone(), false),
    };
    // **Either form is a call.** A model that wrote a function block where a
    // JSON object was asked for has called the tool; which form it used is a
    // fact about the model, and the offering that asked for the other one is
    // what says so (B-453).
    let (named, form) = match called(&candidate) {
        Ok(called) => called,
        Err(because) => return Attempt::Malformed { because },
    };
    if named != OFFER.name {
        return Attempt::Malformed {
            because: format!("it called {named:?}, which was not the tool offered"),
        };
    }
    if form != offering.form {
        // The tool was called, in the shape the model's own template writes
        // rather than the shape this offering asked for. Not well formed
        // under this offering, and emphatically a call (B-453).
        return Attempt::Malformed {
            because: format!(
                "a call to {} written as {}, where this offering asked for {}: {candidate}",
                OFFER.name,
                form.as_str(),
                offering.form.as_str()
            ),
        };
    }
    if !as_asked {
        // Everything about the call is right except where it was put. Malformed
        // rather than well formed, because the offering asked for a wrapper and
        // did not get one — and emphatically not `NoCall`, because the model
        // did call.
        return Attempt::Malformed {
            because: format!(
                "a well-formed call to {}, emitted without the markers this offering asked \
                 for: {candidate}",
                OFFER.name
            ),
        };
    }
    Attempt::WellFormed
}

/// Where a candidate call was found, which decides what its failure means.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Found {
    /// Where the offering asked for it.
    AsAsked(String),
    /// A bare object, in an offering that asked for markers.
    ///
    /// **This is not "no call".** The model attempted one and put it somewhere
    /// else, which is a fact about the model and a different fact from having
    /// answered in prose. The first real run of this probe reported *no call*
    /// five times out of five under a model's own markers while the same model
    /// called perfectly under a plain description — and without this
    /// distinction there was no way to tell *it ignored the tool* from *it
    /// called without the wrapper it was asked for* (A1, F101).
    Elsewhere(String),
}

/// The candidate call inside an answer, or nothing where none is call-shaped.
///
/// Between the offering's markers where it has them — and where it has them and
/// they are absent, a bare object is still looked for, because a model that
/// emitted a call without the wrapper has done something the probe must not
/// record as having done nothing.
fn between(said: &str, offering: &Offering) -> Option<Found> {
    let call_shaped = |said: &str| {
        first_object(said).or_else(|| {
            let at = said.find("<function=")?;
            let rest = said.get(at..)?;
            let end = rest
                .find("</function>")
                .and_then(|end| end.checked_add("</function>".len()))
                .unwrap_or(rest.len());
            rest.get(..end).map(str::to_owned)
        })
    };
    if let Some((open, close)) = &offering.between {
        let inside = said
            .find(open.as_str())
            .and_then(|at| at.checked_add(open.len()))
            .and_then(|start| said.get(start..))
            .and_then(|rest| {
                let end = rest.find(close.as_str()).unwrap_or(rest.len());
                rest.get(..end)
            });
        if let Some(inner) = inside {
            return Some(Found::AsAsked(inner.trim().to_owned()));
        }
        // The markers were not there. A bare call still counts as something
        // the model did — reported as put elsewhere, never as absent (F101).
        return call_shaped(said).map(Found::Elsewhere);
    }
    call_shaped(said).map(Found::AsAsked)
}

/// What a candidate call names, and in which form, or nothing where it is
/// neither.
///
/// Mechanical, both ways: an object is parsed and its `name` read; a function
/// block's name is what stands between `<function=` and the `>` that closes
/// it. Nothing here reads the arguments for sense, for the reason the module
/// gives.
fn called(candidate: &str) -> Result<(String, Form), String> {
    // A function block first, because a template that writes one may write a
    // JSON object inside a parameter and the block is the outer fact.
    if let Some(named) = named_function(candidate) {
        return Ok((named, Form::Function));
    }
    if !candidate.trim_start().starts_with('{') {
        return Err(format!(
            "what came out is neither a JSON object nor a function block: {candidate:?}"
        ));
    }
    let value = mcf_record::json::parse(candidate)
        .map_err(|error| format!("what came out did not parse as JSON ({error}): {candidate:?}"))?;
    let named = value
        .get("name")
        .and_then(mcf_record::json::Value::as_text)
        .ok_or_else(|| format!("the object has no \"name\": {candidate:?}"))?;
    Ok((named.to_owned(), Form::Json))
}

/// The name in a function block, where the text carries one.
fn named_function(candidate: &str) -> Option<String> {
    let at = candidate
        .find("<function=")?
        .checked_add("<function=".len())?;
    let rest = candidate.get(at..)?;
    let end = rest.find('>')?;
    let named = rest.get(..end)?.trim();
    (!named.is_empty()).then(|| named.to_owned())
}

/// The first balanced `{…}` in an answer, which is what a bare object looks
/// like in a reply that may carry prose around it.
///
/// Public because [`super::structured`] asks the same question of an answer,
/// and *what an object looks like in a model's output* is one question: two
/// readers of it would eventually disagree, and the probe that disagreed would
/// be the one nobody was looking at (F79).
pub fn first_object(said: &str) -> Option<String> {
    let start = said.find('{')?;
    let mut depth = 0_usize;
    for (offset, character) in said.get(start..)?.char_indices() {
        match character {
            '{' => depth = depth.saturating_add(1),
            '}' => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    let end = start.checked_add(offset)?.checked_add(1)?;
                    return said.get(start..end).map(str::to_owned);
                }
            }
            _ => {}
        }
    }
    None
}

#[cfg(test)]
mod tests;
