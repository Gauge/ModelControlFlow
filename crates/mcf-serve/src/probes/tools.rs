use std::path::Path;

use mcf_core::probe::{Method, Outcome, Probed};
use mcf_standin::gguf;
use mcf_standin::tokenizer::{Piece, Tokens};

use super::{Addressing, Trial};

#[derive(Debug)]
pub struct Offer {
    pub name: &'static str,
    pub argument: &'static str,
    pub question: &'static str,
}

pub const OFFER: Offer = Offer {
    name: "get_weather",
    argument: "city",
    question: "What is the weather in Paris right now? Use the tool.",
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Attempt {
    WellFormed,
    Malformed { because: String },
    NoCall,
    CouldNotTell { because: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Form {
    Json,
    Function,
}

impl Form {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Json => "a JSON object",
            Self::Function => "a nested function block",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Offering {
    pub name: String,
    pub text: String,
    pub between: Option<(String, String)>,
    pub form: Form,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Calling {
    pub well_formed: Vec<(String, usize)>,
    pub malformed: Vec<(String, usize)>,
    pub no_call: Vec<(String, usize)>,
    pub reasons: Vec<String>,
    pub of: usize,
    pub best: Option<String>,
    pub declared: Declared,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Declared {
    pub template_mentions_tools: bool,
    pub markers: Vec<String>,
    pub form: Form,
}

impl Declared {
    #[must_use]
    pub fn claims_support(&self) -> bool {
        self.template_mentions_tools || !self.markers.is_empty()
    }
}

pub type Ask<'a> = &'a mut dyn FnMut(&[Piece], usize) -> (Trial, String);

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
    let Ok(tokens) = Tokens::read(&file) else {
        return Probed::inconclusive(TOOL_CALLING, "the file lists no tokens", 0, 0, conditions);
    };
    let declared = declared(&file, &tokens);
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
            let wrapped = Addressing::wrapped(addressing, &offering.text);
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

fn declared(file: &gguf::Model, tokens: &Tokens) -> Declared {
    let template = file
        .get("tokenizer.chat_template")
        .and_then(gguf::Value::as_text)
        .unwrap_or_default()
        .to_owned();
    let mut markers: Vec<String> = Vec::new();
    for marker in super::markers_in(&template) {
        if !marker.to_lowercase().contains("tool") || !tokens.has_token(&marker) {
            continue;
        }
        if !markers.contains(&marker) {
            markers.push(marker);
        }
    }
    Declared {
        template_mentions_tools: template.to_lowercase().contains("tool"),
        markers,
        form: if template.contains("<function=") && template.contains("<parameter=") {
            Form::Function
        } else {
            Form::Json
        },
    }
}

fn offerings(declared: &Declared) -> Vec<Offering> {
    let schema = format!(
        "{{\"name\": \"{}\", \"description\": \"look up the current weather in a city\", \
         \"parameters\": {{\"{}\": \"string\"}}}}",
        OFFER.name, OFFER.argument
    );
    let mut found = Vec::new();

    let paired = paired(&declared.markers);
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

fn example(open: &str, close: &str) -> String {
    format!(
        "{open}\n<function={}>\n<parameter={}>\nParis\n</parameter>\n</function>\n{close}",
        OFFER.name, OFFER.argument
    )
}

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

#[derive(Debug, Clone, PartialEq, Eq)]
enum Found {
    AsAsked(String),
    Elsewhere(String),
}

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
        return call_shaped(said).map(Found::Elsewhere);
    }
    call_shaped(said).map(Found::AsAsked)
}

fn called(candidate: &str) -> Result<(String, Form), String> {
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

fn named_function(candidate: &str) -> Option<String> {
    let at = candidate
        .find("<function=")?
        .checked_add("<function=".len())?;
    let rest = candidate.get(at..)?;
    let end = rest.find('>')?;
    let named = rest.get(..end)?.trim();
    (!named.is_empty()).then(|| named.to_owned())
}

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
