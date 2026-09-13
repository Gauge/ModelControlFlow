use mcf_standin::gguf::{Model, Value};

pub const CONVENTIONAL: [&str; 3] = ["low", "medium", "high"];

const RANKED: [&str; 5] = ["none", "low", "medium", "high", "xhigh"];

const TAGS: [&str; 6] = [
    "<think>",
    "<thinking>",
    "<seed:think>",
    "◁think▷",
    "<|channel|>analysis",
    "<reasoning>",
];

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
        let section = TAGS.iter().any(|tag| template.contains(tag));
        let switch = template.contains("enable_thinking");
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
            None => "this model's template reads no thinking level, so setting one changes \
                     nothing"
                .to_owned(),
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

fn the_set_it_allows(template: &str, named: &str) -> Option<Vec<String>> {
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
