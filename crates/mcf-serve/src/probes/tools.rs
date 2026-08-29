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
           suggests, and counts the trials that produce a call which appears where one was \
           asked for, parses as JSON, and names the tool offered",
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

    let best = well_formed
        .iter()
        .filter(|(_, count)| *count > 0)
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

    if let Some((open, close)) = paired(&declared.markers) {
        found.push(Offering {
            name: format!("the file's own markers, {open}…{close}"),
            text: format!(
                "You have one tool:\n{schema}\nTo use it, emit {open} then a JSON object with \
                 \"name\" and \"arguments\", then {close}.\n\n{}",
                OFFER.question
            ),
            between: Some((open, close)),
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
    });
    found
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
    let parsed = match mcf_record::json::parse(&candidate) {
        Ok(value) => value,
        Err(error) => {
            return Attempt::Malformed {
                because: format!("what came out did not parse as JSON ({error}): {candidate:?}"),
            };
        }
    };
    let Some(named) = parsed
        .get("name")
        .and_then(mcf_record::json::Value::as_text)
    else {
        return Attempt::Malformed {
            because: format!("the object has no \"name\": {candidate:?}"),
        };
    };
    if named != OFFER.name {
        return Attempt::Malformed {
            because: format!("it called {named:?}, which was not the tool offered"),
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
        // The markers were not there. A bare object still counts as something
        // the model did — reported as put elsewhere, never as absent (F101).
        return bare(said).map(Found::Elsewhere);
    }
    bare(said).map(Found::AsAsked)
}

/// The first balanced `{…}` in an answer, which is what a bare object looks
/// like in a reply that may carry prose around it.
fn bare(said: &str) -> Option<String> {
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
