//! What a chat template will read from whoever addresses it, beyond the messages.
//!
//! A template is a program, and most of them take arguments: a switch that decides whether
//! the model thinks, a word that says how hard, a flag that keeps earlier reasoning in the
//! history. Every family spells its own, and MCF knew one name — `enable_thinking` — and
//! one other shape — a named effort level. Everything else a template offered was
//! invisible, so a control the model genuinely has could not be reached.
//!
//! What is read here is what the template asks for in the one way Jinja has of asking:
//! a name tested for having been given, or defaulted when it was not. That is the idiom
//! for "the caller may pass this", and it is what distinguishes a parameter from a local
//! variable the template made up for itself.
//!
//! Reading is a guess until an engine says otherwise. [`Parameter::probes`] gives the
//! values worth rendering the template with, and what comes back from `/apply-template`
//! settles whether the parameter does anything at all — see `takes`, which has done this
//! for the thinking switch since before this module existed.

use mcf_record::json::Value;

/// Names the chat protocol itself supplies. They are passed to every template by whatever
/// is talking to the model, so a template testing whether it was given one is asking about
/// the conversation rather than offering a setting.
const THE_CONVERSATION: [&str; 8] = [
    "messages",
    "tools",
    "add_generation_prompt",
    "bos_token",
    "eos_token",
    "date_string",
    "tools_in_user_message",
    "documents",
];

/// What kind of thing a parameter takes, as far as the template says.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Takes {
    /// On or off, with the template's own default where it declares one.
    Switch { on_unless_asked: Option<bool> },
    /// One of a set of words, where the template checks what it was given; otherwise any
    /// word, with the default it falls back to.
    Word {
        allowed: Vec<String>,
        falls_back_to: Option<String>,
    },
    /// A number.
    Count { falls_back_to: Option<String> },
}

/// One thing a template will read from the caller.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Parameter {
    pub name: String,
    pub takes: Takes,
}

impl Parameter {
    /// The values worth rendering the template with to find out whether this parameter
    /// does anything. Both states of a switch; each allowed word, or the two ends of the
    /// usual ladder where the template names none.
    #[must_use]
    pub fn probes(&self) -> Vec<Value> {
        match &self.takes {
            Takes::Switch { .. } => vec![Value::Bool(true), Value::Bool(false)],
            Takes::Word { allowed, .. } if !allowed.is_empty() => allowed
                .iter()
                .map(|held| Value::text(held.clone()))
                .collect(),
            Takes::Word { .. } => crate::thinking::CONVENTIONAL
                .iter()
                .map(|held| Value::text((*held).to_owned()))
                .collect(),
            Takes::Count { .. } => vec![Value::Integer(0), Value::Integer(1_024)],
        }
    }

    /// What the template would do if nobody said anything.
    #[must_use]
    pub fn default_said(&self) -> String {
        match &self.takes {
            Takes::Switch {
                on_unless_asked: Some(true),
            } => "on unless asked off".to_owned(),
            Takes::Switch {
                on_unless_asked: Some(false),
            } => "off unless asked on".to_owned(),
            Takes::Switch { .. } => "the template does not say".to_owned(),
            Takes::Word { falls_back_to, .. } | Takes::Count { falls_back_to } => falls_back_to
                .clone()
                .unwrap_or_else(|| "the template does not say".to_owned()),
        }
    }
}

/// Every parameter a template offers, in the order it first asks for them.
///
/// Two shapes count as asking, because they are the two Jinja has: testing whether a name
/// was given (`x is defined`), and standing in for it when it was not (`x | default(v)`).
/// A name the template only ever assigns to is its own working variable and is not a
/// parameter, however suggestive it looks.
#[must_use]
pub fn in_template(template: &str) -> Vec<Parameter> {
    let mut found: Vec<Parameter> = Vec::new();
    for name in asked_for(template) {
        if THE_CONVERSATION.contains(&name.as_str()) {
            continue;
        }
        if found.iter().any(|held| held.name == name) {
            continue;
        }
        if the_template_sets_it_itself(template, &name, first_asked(template, &name)) {
            continue;
        }
        let takes = what_it_takes(template, &name);
        found.push(Parameter { name, takes });
    }
    found
}

/// Whether the template assigns this name from something other than itself.
///
/// A template that writes `set system_message = messages[0]["content"]` is naming its own
/// working variable, and testing it later says nothing about what the caller may pass. A
/// template that writes `set enable_thinking = enable_thinking if enable_thinking is
/// defined else True` is doing the opposite: standing in for a parameter it was not given,
/// which is exactly the idiom this module is looking for. The difference is whether the
/// value it assigns mentions the name itself.
fn the_template_sets_it_itself(template: &str, name: &str, asked_at: usize) -> bool {
    let assigned = format!("set {name} =");
    let mut from = 0;
    while let Some(at) = template.get(from..).and_then(|rest| rest.find(&assigned)) {
        let opens = from.saturating_add(at);
        let here = opens.saturating_add(assigned.len());
        let rest = template.get(here..).unwrap_or_default();
        let end = rest.find(['%', '\n']).unwrap_or(rest.len());
        let value = rest.get(..end).unwrap_or_default();
        // Assigned from something else, before anybody asked whether it was given: the
        // template made this one up for itself. Assigned afterwards is the other idiom —
        // `if x is not defined, set x = "medium"` — which stands in for a parameter.
        if !mentions(value, name) && opens < asked_at {
            return true;
        }
        from = here;
    }
    false
}

/// Whether a value mentions a name as a name, rather than as part of a longer one or as
/// somebody else's property.
///
/// `message.content` is not a mention of `content`: it is one message's property, and a
/// template assigning from it is naming its own working variable. Matching on the letters
/// alone had that one read as a parameter every model could be asked for.
fn mentions(value: &str, name: &str) -> bool {
    let held: Vec<char> = value.chars().collect();
    let wanted: Vec<char> = name.chars().collect();
    for (at, _) in held.iter().enumerate() {
        if held.get(at..at.saturating_add(wanted.len())) != Some(wanted.as_slice()) {
            continue;
        }
        let before = at.checked_sub(1).and_then(|back| held.get(back)).copied();
        let after = held.get(at.saturating_add(wanted.len())).copied();
        let joined =
            |ch: Option<char>| ch.is_some_and(|ch| ch.is_alphanumeric() || ch == '_' || ch == '.');
        if !joined(before) && !joined(after) {
            return true;
        }
    }
    false
}

/// Where the template first asks whether this name was given.
fn first_asked(template: &str, name: &str) -> usize {
    [
        format!("{name} is defined"),
        format!("{name} is not defined"),
        format!("{name} | default("),
        format!("{name}|default("),
    ]
    .iter()
    .filter_map(|shape| asked_bare(template, shape))
    .min()
    .unwrap_or(usize::MAX)
}

/// Where a shape first occurs with the name standing on its own, rather than hanging off
/// something else. `message.content | default('', true)` is a question about one message,
/// and counting it as the moment somebody asked about `content` made the template's own
/// working variable look like a parameter that had been asked about before it was set.
fn asked_bare(template: &str, shape: &str) -> Option<usize> {
    let mut from = 0;
    while let Some(at) = template.get(from..).and_then(|rest| rest.find(shape)) {
        let here = from.saturating_add(at);
        let before = template
            .get(..here)
            .and_then(|held| held.chars().next_back());
        if !before.is_some_and(|ch| ch.is_alphanumeric() || ch == '_' || ch == '.') {
            return Some(here);
        }
        from = here.saturating_add(1);
    }
    None
}

/// The names a template tests for or defaults, in the order they appear.
fn asked_for(template: &str) -> Vec<String> {
    let mut names = Vec::new();
    for (marker, back) in [
        (" is defined", true),
        (" is not defined", true),
        ("| default(", true),
        ("|default(", true),
    ] {
        let mut from = 0;
        while let Some(at) = template.get(from..).and_then(|rest| rest.find(marker)) {
            let here = from.saturating_add(at);
            if back && let Some(name) = the_name_before(template, here) {
                names.push((here, name));
            }
            from = here.saturating_add(marker.len());
        }
    }
    names.sort_by_key(|(at, _)| *at);
    names.into_iter().map(|(_, name)| name).collect()
}

/// The identifier immediately to the left of a position, where there is one.
fn the_name_before(template: &str, at: usize) -> Option<String> {
    let before = template.get(..at)?;
    let mut name: Vec<char> = Vec::new();
    let mut skipped = 0_usize;
    for ch in before.chars().rev() {
        if ch.is_alphanumeric() || ch == '_' {
            name.push(ch);
            continue;
        }
        if ch == ' ' && name.is_empty() {
            skipped = skipped.saturating_add(1);
            continue;
        }
        break;
    }
    if name.is_empty() {
        return None;
    }
    // A name reached through a dot belongs to something else — `tool.function`,
    // `message.content` — and is a property of one message rather than a setting the
    // caller can pass. Without this the reader offered a dozen of them per template.
    let reached_through = before
        .get(
            ..before
                .len()
                .saturating_sub(name.len())
                .saturating_sub(skipped),
        )
        .and_then(|held| held.chars().next_back());
    if reached_through == Some('.') {
        return None;
    }
    let held: String = name.into_iter().rev().collect();
    // A number is a value, not a name, and a name cannot start with one.
    if held.starts_with(|ch: char| ch.is_numeric()) {
        return None;
    }
    Some(held)
}

/// What the template says this parameter takes, read from how it stands in for it.
///
/// The default is the tell: `false` or `True` is a switch, a quoted word is a word, a bare
/// number is a count. Where the template also checks the word it was given against a set,
/// that set is the answer to what it accepts — the same reading the thinking level has
/// always used.
fn what_it_takes(template: &str, name: &str) -> Takes {
    let allowed = crate::thinking::the_set_it_allows(template, name).unwrap_or_default();
    if !allowed.is_empty() {
        return Takes::Word {
            allowed,
            falls_back_to: stands_in_for(template, name),
        };
    }
    match stands_in_for(template, name) {
        Some(held) if held.eq_ignore_ascii_case("true") => Takes::Switch {
            on_unless_asked: Some(true),
        },
        Some(held) if held.eq_ignore_ascii_case("false") => Takes::Switch {
            on_unless_asked: Some(false),
        },
        Some(held) if held.parse::<i64>().is_ok() => Takes::Count {
            falls_back_to: Some(held),
        },
        Some(held) => Takes::Word {
            allowed: Vec::new(),
            falls_back_to: Some(held),
        },
        None => Takes::Switch {
            on_unless_asked: None,
        },
    }
}

/// What the template falls back to for this name: the value after `else` in the defined
/// test, or the one inside `default(...)`.
fn stands_in_for(template: &str, name: &str) -> Option<String> {
    let defaulted = format!("{name} | default(");
    let tight = format!("{name}|default(");
    for opening in [defaulted, tight] {
        if let Some(at) = template.find(&opening) {
            let from = at.saturating_add(opening.len());
            let rest = template.get(from..)?;
            let close = rest.find(')')?;
            return Some(unquoted(rest.get(..close)?));
        }
    }
    // `if x is not defined, set x = "medium"`: the value it stands in with is the value it
    // would have been given, so it is the default however the template spells the guard.
    let guarded = format!("set {name} = ");
    if let Some(at) = template.find(&guarded) {
        let from = at.saturating_add(guarded.len());
        let rest = template.get(from..)?;
        let end = rest.find(['%', '}', '\n']).unwrap_or(rest.len());
        let held = unquoted(rest.get(..end)?);
        if !held.is_empty() && !mentions(&held, name) {
            return Some(held);
        }
    }
    // `set x = x if x is defined else V`
    let tested = format!("{name} is defined else ");
    let at = template.find(&tested)?;
    let from = at.saturating_add(tested.len());
    let rest = template.get(from..)?;
    let end = rest
        .find(['%', '}', '\n'])
        .unwrap_or_else(|| rest.len().min(24));
    Some(unquoted(rest.get(..end)?))
}

fn unquoted(held: &str) -> String {
    held.trim()
        .trim_end_matches('-')
        .trim()
        .trim_matches(['\'', '"'])
        .to_owned()
}

#[cfg(test)]
mod tests;
