use mcf_standin::gguf::{Model, Value};

pub const CONVENTIONAL: [&str; 3] = ["low", "medium", "high"];

/// What MCF calls no thinking at all. It is MCF's word and not any template's: a template
/// that validates its own vocabulary raises on it, and one that does not falls back to its
/// default. Whatever asks for it has to turn it into something an engine acts on before it
/// reaches one.
pub const OFF: &str = "off";

const RANKED: [&str; 5] = ["none", "low", "medium", "high", "xhigh"];

/// Tags that open a thinking section but do not say so in their own name, so no pattern
/// would find them. Everything else is recognised by [`marks_a_section`].
const NAMED_TAGS: [&str; 3] = ["◁think▷", "<|channel|>analysis", "<|channel>thought"];

/// What a tag has to speak of for MCF to read it as opening a thinking section.
const ABOUT_THINKING: [&str; 3] = ["think", "reason", "analysis"];

/// The longest a tag can be and still be a tag. A run of angle brackets far apart is prose
/// about tags, or markup that has nothing to do with thinking.
const TAG_CEILING: usize = 40;

/// Names a template reads to be told whether to think at all.
///
/// `enable_thinking` is what llama.cpp writes when it is asked for `--reasoning on` or
/// `off`, and what every family MCF has read uses. The others are here because a template
/// that spells it differently still has a switch, and MCF would otherwise report a model
/// that can stop thinking as one that cannot.
const SWITCHES: [&str; 3] = ["enable_thinking", "enable_reasoning", "thinking_enabled"];

/// Whether a template marks its thinking off from its answer.
///
/// A fixed list of tags could only ever cover the families somebody had already read: this
/// machine holds a Gemma whose tag is `<|think|>`, which no list of MCF's had, and the
/// model was reported as unable to think at all. A thinking model has to delimit its
/// thinking somehow — an engine cannot strip what is not marked — so what MCF looks for is
/// a tag that says what it is for, whatever it is spelled like.
#[must_use]
pub fn marks_a_section(template: &str) -> bool {
    if NAMED_TAGS.iter().any(|tag| template.contains(tag)) {
        return true;
    }
    let held: Vec<char> = template.chars().collect();
    for (at, ch) in held.iter().enumerate() {
        if *ch != '<' {
            continue;
        }
        let mut name = String::new();
        for next in held.iter().skip(at.saturating_add(1)).take(TAG_CEILING) {
            if *next == '>' {
                let name = name.to_ascii_lowercase();
                if ABOUT_THINKING.iter().any(|about| name.contains(about)) {
                    return true;
                }
                break;
            }
            if *next == '<' {
                break;
            }
            name.push(*next);
        }
    }
    false
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Thinking {
    pub variable: Option<String>,
    pub levels: Vec<String>,
    pub closed: bool,
    pub section: bool,
    pub switch: bool,
}

impl Thinking {
    #[must_use]
    pub fn of(file: &Model) -> Self {
        let Some(template) = file.get("tokenizer.chat_template").and_then(Value::as_text) else {
            return Self::default();
        };
        Self::in_template(template)
    }

    #[must_use]
    pub fn in_template(template: &str) -> Self {
        let section = marks_a_section(template);
        let switch = SWITCHES.iter().any(|named| template.contains(named));
        let variable = if template.contains("reasoning_effort") {
            Some("reasoning_effort".to_owned())
        } else if template.contains("reasoning_strength") {
            Some("reasoning_strength".to_owned())
        } else {
            None
        };
        let Some(named) = variable.clone() else {
            return Self {
                variable: None,
                levels: Vec::new(),
                closed: false,
                section,
                switch,
            };
        };
        let (levels, closed) = match the_set_it_allows(template, &named) {
            Some(held) => (held, true),
            None => (
                CONVENTIONAL.iter().map(|held| (*held).to_owned()).collect(),
                false,
            ),
        };
        let mut levels = ranked(levels);
        if switch && !levels.iter().any(|held| held == "none") {
            levels.insert(0, "none".to_owned());
        }
        Self {
            variable: Some(named),
            levels,
            closed,
            section,
            switch,
        }
    }

    #[must_use]
    pub fn to_value(&self) -> mcf_record::json::Value {
        use mcf_record::json::Value as Said;
        Said::map([
            (
                "variable",
                self.variable
                    .as_ref()
                    .map_or(Said::Null, |held| Said::text(held.clone())),
            ),
            (
                "levels",
                Said::List(
                    self.levels
                        .iter()
                        .map(|held| Said::text(held.clone()))
                        .collect(),
                ),
            ),
            ("closed", Said::Bool(self.closed)),
            ("section", Said::Bool(self.section)),
            ("switch", Said::Bool(self.switch)),
        ])
    }

    #[must_use]
    pub fn from_value(value: Option<&mcf_record::json::Value>) -> Self {
        use mcf_record::json::Value as Said;
        let Some(value) = value else {
            return Self::default();
        };
        let yes = |key: &str| matches!(value.get(key), Some(Said::Bool(true)));
        Self {
            variable: value
                .get("variable")
                .and_then(Said::as_text)
                .map(str::to_owned),
            levels: match value.get("levels") {
                Some(Said::List(held)) => held
                    .iter()
                    .filter_map(Said::as_text)
                    .map(str::to_owned)
                    .collect(),
                _ => Vec::new(),
            },
            closed: yes("closed"),
            section: yes("section"),
            switch: yes("switch"),
        }
    }

    #[must_use]
    pub fn reads_a_level(&self) -> bool {
        self.variable.is_some() && !self.levels.is_empty()
    }

    #[must_use]
    pub fn allows(&self, level: &str) -> bool {
        self.levels.iter().any(|held| held == level)
    }

    #[must_use]
    pub fn said(&self) -> String {
        match &self.variable {
            // Naming no level is not the same as not thinking, and saying so as though it
            // were had most of the families on a real machine reported as unable to think.
            None if self.switch => "this model's template names no thinking level, but reads \
                                    a switch, so thinking can be turned off"
                .to_owned(),
            None if self.section => "this model's template names no thinking level, but marks \
                                     a thinking section the engine can cut short"
                .to_owned(),
            None => "this model's template says nothing about thinking".to_owned(),
            Some(named) => format!(
                "the template reads {named} and {} — {}",
                if self.closed {
                    "accepts only these"
                } else {
                    "passes on whatever it is given, so these are the usual ones"
                },
                self.levels.join(", ")
            ),
        }
    }
}

fn ranked(mut levels: Vec<String>) -> Vec<String> {
    levels.sort_by_key(|held| {
        (
            RANKED
                .iter()
                .position(|known| known == held)
                .unwrap_or(RANKED.len()),
            held.clone(),
        )
    });
    levels.dedup();
    levels
}

pub(crate) fn the_set_it_allows(template: &str, named: &str) -> Option<Vec<String>> {
    let mut from = 0;
    while let Some(at) = template.get(from..)?.find(named) {
        let here = from.saturating_add(at).saturating_add(named.len());
        let rest = template.get(here..)?;
        let ahead = rest.get(..rest.len().min(120)).unwrap_or(rest);
        if let Some(open) = after_not_in(ahead) {
            let inside = ahead.get(open..)?;
            let close = inside.find([')', ']'])?;
            let held = quoted(inside.get(..close)?);
            if !held.is_empty() {
                return Some(held);
            }
        }
        from = here;
    }
    None
}

fn after_not_in(ahead: &str) -> Option<usize> {
    let at = ahead.find("not in")?;
    let rest = ahead.get(at..)?;
    let open = rest.find(['(', '['])?;
    Some(at.saturating_add(open).saturating_add(1))
}

fn quoted(inside: &str) -> Vec<String> {
    let mut held = Vec::new();
    let mut rest = inside;
    while let Some(open) = rest.find(['\'', '"']) {
        let mark = rest.get(open..).and_then(|held| held.chars().next());
        let Some(mark) = mark else { break };
        let after = rest.get(open.saturating_add(1)..).unwrap_or("");
        let Some(close) = after.find(mark) else { break };
        let word = after.get(..close).unwrap_or("").trim().to_owned();
        if !word.is_empty() {
            held.push(word);
        }
        rest = after.get(close.saturating_add(1)..).unwrap_or("");
    }
    held
}

#[cfg(test)]
mod tests;
