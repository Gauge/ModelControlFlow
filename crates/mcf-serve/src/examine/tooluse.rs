//! Tool use: a fixed set of tasks, each offering one to three tools and
//! asking one thing, the call checked by exact match; then the call's
//! result fed back and the answer checked for it (B-517, D54, D52).
//!
//! **What the tool probe asks, and what this asks.** The probe asks
//! whether a well-formed call comes out at all, under several ways of
//! telling the model about one tool. This declares the tools the way a
//! caller of the hosted server declares them — through the model's own
//! template, which renders them in its own place and form — and asks
//! what a person choosing a model for tool use wants counted: did it
//! call when it should, the right tool, with the arguments the request
//! stated, in the types the tool declared; did it hold back when no tool
//! fit; and once the tool answered, did the answer carry what the tool
//! said. Every one of those is a parser's question (A19), and every
//! trial is a row.
//!
//! **Exact match, stated.** The arguments the request states are compared
//! after trimming, case aside for text and with spaces removed where the
//! tool takes an expression; a number is a number, and a number written
//! as text is a mismatch of type, counted as such. A key beyond the ones
//! asked is counted, not condemned.

use std::collections::BTreeMap;

use mcf_record::json::Value;

use super::{Found, Reading, Site, as_integer, timed};
use crate::generation::{Draw, Truncation};
use crate::served::{Prompt, Served, Startup};
use mcf_core::configuration::Thousandths;

/// The measurement's name.
pub const NAME: &str = "tool-use";

/// How many trials each task has: one greedy, then two drawn.
pub const TRIALS: usize = 3;

/// The temperature the drawn trials use, in thousandths.
pub(crate) const TEMPERATURE: u32 = 700;

/// How many tokens a call may take.
const CALL_BUDGET: usize = 200;

/// How many tokens the answer after the tool's result may take.
const ANSWER_BUDGET: usize = 160;

/// One tool as it is declared to the model.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Tool {
    pub(crate) name: &'static str,
    pub(crate) description: &'static str,
    /// Each parameter: its name, its JSON type, and whether it is required.
    pub(crate) parameters: &'static [(&'static str, &'static str, bool)],
}

/// What a task expects of the model.
#[derive(Debug, Clone, Copy)]
enum Expect {
    /// A call to this tool with these arguments, and, once the tool has
    /// answered with `result`, an answer carrying `carries`.
    Call {
        tool: &'static str,
        arguments: &'static [(&'static str, Arg)],
        result: &'static str,
        carries: &'static str,
    },
    /// No call: nothing offered fits.
    NoCall,
}

/// An expected argument.
#[derive(Debug, Clone, Copy)]
pub(crate) enum Arg {
    /// Text, compared trimmed and case aside.
    Text(&'static str),
    /// Text with every space removed before comparing: an expression.
    Expression(&'static str),
    /// A whole number, compared as one.
    Integer(i64),
}

/// One task.
#[derive(Debug, Clone, Copy)]
struct Task {
    name: &'static str,
    tools: &'static [Tool],
    asks: &'static str,
    expects: Expect,
}

pub(crate) const WEATHER: Tool = Tool {
    name: "get_weather",
    description: "Look up the current weather in a city.",
    parameters: &[("city", "string", true)],
};
const TIME: Tool = Tool {
    name: "get_time",
    description: "Look up the current local time in a city.",
    parameters: &[("city", "string", true)],
};
const CALCULATE: Tool = Tool {
    name: "calculate",
    description: "Evaluate an arithmetic expression exactly.",
    parameters: &[("expression", "string", true)],
};
const ALARM: Tool = Tool {
    name: "set_alarm",
    description: "Set an alarm for a time of day, in 24-hour form.",
    parameters: &[("hour", "integer", true), ("minute", "integer", true)],
};
const CONVERT: Tool = Tool {
    name: "convert_temperature",
    description: "Convert a temperature between celsius and fahrenheit.",
    parameters: &[
        ("value", "integer", true),
        ("from", "string", true),
        ("to", "string", true),
    ],
};
const EVENT: Tool = Tool {
    name: "create_event",
    description: "Add an event to the calendar on a date written as YYYY-MM-DD.",
    parameters: &[("title", "string", true), ("date", "string", true)],
};

/// The tasks, in order.
const TASKS: [Task; 8] = [
    Task {
        name: "one-tool",
        tools: &[WEATHER],
        asks: "What is the weather in Paris right now?",
        expects: Expect::Call {
            tool: "get_weather",
            arguments: &[("city", Arg::Text("Paris"))],
            result: "{\"temperature_c\": 17, \"sky\": \"overcast\"}",
            carries: "17",
        },
    },
    Task {
        name: "choose-the-tool",
        tools: &[WEATHER, TIME],
        asks: "What time is it in Tokyo?",
        expects: Expect::Call {
            tool: "get_time",
            arguments: &[("city", Arg::Text("Tokyo"))],
            result: "{\"time\": \"14:32\"}",
            carries: "14:32",
        },
    },
    Task {
        name: "an-expression",
        tools: &[CALCULATE],
        asks: "Use the calculator to work out 1234 * 5678.",
        expects: Expect::Call {
            tool: "calculate",
            arguments: &[("expression", Arg::Expression("1234*5678"))],
            result: "{\"value\": 7006652}",
            carries: "7006652",
        },
    },
    Task {
        name: "no-tool-fits",
        tools: &[WEATHER, TIME],
        asks: "Write one sentence about autumn leaves.",
        expects: Expect::NoCall,
    },
    Task {
        name: "whole-numbers",
        tools: &[ALARM],
        asks: "Set an alarm for 6:45 in the morning.",
        expects: Expect::Call {
            tool: "set_alarm",
            arguments: &[("hour", Arg::Integer(6)), ("minute", Arg::Integer(45))],
            result: "{\"alarm_id\": \"A-8841\"}",
            carries: "A-8841",
        },
    },
    Task {
        name: "three-arguments",
        tools: &[CONVERT],
        asks: "Convert 30 degrees celsius to fahrenheit with the tool.",
        expects: Expect::Call {
            tool: "convert_temperature",
            arguments: &[
                ("value", Arg::Integer(30)),
                ("from", Arg::Text("celsius")),
                ("to", Arg::Text("fahrenheit")),
            ],
            result: "{\"value\": 86}",
            carries: "86",
        },
    },
    Task {
        name: "two-from-the-request",
        tools: &[EVENT, TIME],
        asks: "Add \"Dentist\" to my calendar on 2026-10-03.",
        expects: Expect::Call {
            tool: "create_event",
            arguments: &[
                ("title", Arg::Text("Dentist")),
                ("date", Arg::Text("2026-10-03")),
            ],
            result: "{\"event_id\": \"E-2207\"}",
            carries: "E-2207",
        },
    },
    Task {
        name: "three-tools-one-fits",
        tools: &[WEATHER, CALCULATE, EVENT],
        asks: "What is the weather in Lisbon?",
        expects: Expect::Call {
            tool: "get_weather",
            arguments: &[("city", Arg::Text("Lisbon"))],
            result: "{\"temperature_c\": 24, \"sky\": \"clear\"}",
            carries: "24",
        },
    },
];

/// A tool as the engine's template takes it.
pub(crate) fn tool_value(tool: &Tool) -> Value {
    let properties: Vec<(String, Value)> = tool
        .parameters
        .iter()
        .map(|(name, kind, _)| {
            (
                (*name).to_owned(),
                Value::map([("type", Value::text((*kind).to_owned()))]),
            )
        })
        .collect();
    let required: Vec<Value> = tool
        .parameters
        .iter()
        .filter(|(_, _, required)| *required)
        .map(|(name, _, _)| Value::text((*name).to_owned()))
        .collect();
    Value::map([
        ("type", Value::text("function")),
        (
            "function",
            Value::map([
                ("name", Value::text(tool.name)),
                ("description", Value::text(tool.description)),
                (
                    "parameters",
                    Value::map([
                        ("type", Value::text("object")),
                        ("properties", Value::Map(properties.into_iter().collect())),
                        ("required", Value::List(required)),
                    ]),
                ),
            ]),
        ),
    ])
}

/// A call as read from what the model said.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Call {
    /// What it named.
    pub(crate) name: String,
    /// Its arguments, as the model wrote them.
    pub(crate) arguments: BTreeMap<String, Value>,
}

/// Every call in what the model said, in order, in either form a
/// template writes: the calls a turn made at once (B-520).
pub(crate) fn calls_in(said: &str) -> Vec<Call> {
    let mut found = Vec::new();
    let mut rest = said
        .rfind("</think>")
        .and_then(|at| said.get(at + "</think>".len()..))
        .unwrap_or(said);
    while let Some(call) = call_in(rest) {
        // Past this call: the function block's close or the object's end,
        // whichever the call was read from.
        let past = if let Some(at) = rest.find("</function>") {
            at + "</function>".len()
        } else if let Some(object) = crate::probes::tools::first_object(rest) {
            rest.find(&object)
                .map_or(rest.len(), |at| at + object.len())
        } else {
            rest.len()
        };
        found.push(call);
        let Some(after) = rest.get(past..) else { break };
        if after.is_empty() || past == 0 {
            break;
        }
        rest = after;
    }
    found
}

/// The first call in what the model said, in either form a template
/// writes — a JSON object naming a tool, or a function block — or none.
pub(crate) fn call_in(said: &str) -> Option<Call> {
    // The answer, not the thought before it: a model that drafts its call
    // inside its thinking is read where it made the call.
    let said = said
        .rfind("</think>")
        .and_then(|at| said.get(at + "</think>".len()..))
        .unwrap_or(said);
    if let Some(call) = function_block(said) {
        return Some(call);
    }
    let object = crate::probes::tools::first_object(said)?;
    let value = mcf_record::json::parse(&object).ok()?;
    let name = value.get("name").and_then(Value::as_text)?.to_owned();
    let arguments = value
        .get("arguments")
        .or_else(|| value.get("parameters"))
        .cloned()
        .unwrap_or(Value::Map(BTreeMap::new()));
    // Arguments written as a string holding JSON are read as JSON.
    let arguments = match arguments {
        Value::Text(text) => mcf_record::json::parse(&text).unwrap_or(Value::Text(text)),
        other => other,
    };
    let Value::Map(arguments) = arguments else {
        return Some(Call {
            name,
            arguments: BTreeMap::new(),
        });
    };
    Some(Call { name, arguments })
}

/// A `<function=name>` block with its `<parameter=key>value</parameter>`
/// children, as several templates write a call.
fn function_block(said: &str) -> Option<Call> {
    let at = said.find("<function=")?.checked_add("<function=".len())?;
    let rest = said.get(at..)?;
    let end = rest.find('>')?;
    let name = rest.get(..end)?.trim().to_owned();
    let body = rest.get(end + 1..)?;
    let body = body
        .find("</function>")
        .and_then(|close| body.get(..close))
        .unwrap_or(body);
    let mut arguments = BTreeMap::new();
    let mut cursor = body;
    while let Some(open) = cursor.find("<parameter=") {
        let Some(rest) = cursor.get(open + "<parameter=".len()..) else {
            break;
        };
        let Some(close) = rest.find('>') else { break };
        let key = rest.get(..close).unwrap_or_default().trim().to_owned();
        let Some(after) = rest.get(close + 1..) else {
            break;
        };
        let value_end = after.find("</parameter>").unwrap_or(after.len());
        let raw = after.get(..value_end).unwrap_or_default().trim();
        // A whole number written bare is a number; anything else is text.
        let value = raw
            .parse::<i64>()
            .map_or_else(|_| Value::text(raw.to_owned()), Value::Integer);
        let _was = arguments.insert(key, value);
        cursor = after.get(value_end..).unwrap_or_default();
    }
    Some(Call { name, arguments })
}

/// Whether one argument the model gave matches the one expected.
pub(crate) fn argument_matches(given: &Value, expected: Arg) -> bool {
    match expected {
        Arg::Text(want) => given
            .as_text()
            .is_some_and(|got| got.trim().eq_ignore_ascii_case(want)),
        Arg::Expression(want) => given.as_text().is_some_and(|got| {
            got.chars()
                .filter(|c| !c.is_whitespace())
                .collect::<String>()
                == want
        }),
        Arg::Integer(want) => given.as_integer() == Some(want),
    }
}

/// What one trial of one task read: each a whole number a row holds.
#[derive(Debug, Default, Clone, Copy)]
#[allow(
    clippy::struct_excessive_bools,
    reason = "four yes-or-no facts about one call, each its own row, not a state machine"
)]
struct Trial {
    called: bool,
    right_tool: bool,
    keys_match: bool,
    args_match: bool,
    extra_keys: usize,
    /// `None` where the task expects no call, or the call was not right
    /// enough to answer.
    result_carried: Option<bool>,
    call_tokens: usize,
    call_ns: u64,
    answer_tokens: usize,
    answer_ns: u64,
}

/// Reads a call against what a task expects.
fn judged(call: Option<&Call>, expects: Expect) -> (bool, bool, bool, bool, usize) {
    let called = call.is_some();
    let Expect::Call {
        tool, arguments, ..
    } = expects
    else {
        return (called, false, false, false, 0);
    };
    let Some(call) = call else {
        return (false, false, false, false, 0);
    };
    let right_tool = call.name == tool;
    let keys_match = arguments
        .iter()
        .all(|(key, _)| call.arguments.contains_key(*key));
    let args_match = keys_match
        && arguments.iter().all(|(key, want)| {
            call.arguments
                .get(*key)
                .is_some_and(|got| argument_matches(got, *want))
        });
    let extra_keys = call
        .arguments
        .keys()
        .filter(|key| !arguments.iter().any(|(want, _)| want == key))
        .count();
    (called, right_tool, keys_match, args_match, extra_keys)
}

/// Runs it.
#[must_use]
#[allow(
    clippy::too_many_lines,
    reason = "one measurement read straight through: what it started, what it read, the rows it kept"
)]
pub fn measure(site: &Site<'_>) -> Found {
    let engine = match site.server(&Startup {
        projector: None,
        ..site.startup()
    }) {
        Ok(engine) => engine,
        Err(why) => return Found::could_not_tell(&why),
    };
    let mut rows = Vec::new();
    let mut lines = vec![format!(
        "  {} task(s), {TRIALS} trial(s) each — one greedy, two drawn at temperature 0.7 — \
         the tools declared through the model's own template",
        TASKS.len()
    )];
    let mut called_right = 0_usize;
    let mut args_matched = 0_usize;
    let mut held_back = 0_usize;
    let mut carried = 0_usize;
    let mut asked = 0_usize;
    for task in &TASKS {
        let mut said = Vec::new();
        for trial in 0..TRIALS {
            if site.asker_gone() {
                return Found::could_not_tell(crate::served::CLIENT_LEFT);
            }
            let read = match one_trial(site, &engine, task, trial) {
                Ok(read) => read,
                Err(why) => return Found::could_not_tell(&why),
            };
            asked = asked.saturating_add(1);
            let dims = [
                ("task", Value::text(task.name)),
                ("trial", Value::Integer(as_integer(trial))),
            ];
            let flag = |held: bool| i64::from(held);
            rows.push(Reading::new(&dims, "called", flag(read.called), "bool"));
            match task.expects {
                Expect::NoCall => {
                    if !read.called {
                        held_back = held_back.saturating_add(1);
                    }
                    rows.push(Reading::new(&dims, "held_back", flag(!read.called), "bool"));
                    said.push(if read.called { "called" } else { "held back" });
                }
                Expect::Call { .. } => {
                    if read.right_tool {
                        called_right = called_right.saturating_add(1);
                    }
                    if read.args_match {
                        args_matched = args_matched.saturating_add(1);
                    }
                    rows.push(Reading::new(
                        &dims,
                        "right_tool",
                        flag(read.right_tool),
                        "bool",
                    ));
                    rows.push(Reading::new(
                        &dims,
                        "keys_match",
                        flag(read.keys_match),
                        "bool",
                    ));
                    rows.push(Reading::new(
                        &dims,
                        "args_match",
                        flag(read.args_match),
                        "bool",
                    ));
                    rows.push(Reading::new(
                        &dims,
                        "extra_keys",
                        as_integer(read.extra_keys),
                        "count",
                    ));
                    if let Some(got) = read.result_carried {
                        if got {
                            carried = carried.saturating_add(1);
                        }
                        rows.push(Reading::new(&dims, "result_carried", flag(got), "bool"));
                        rows.push(Reading::new(
                            &dims,
                            "answer_tokens",
                            as_integer(read.answer_tokens),
                            "tokens",
                        ));
                        rows.push(Reading::new(
                            &dims,
                            "answer_ns",
                            i64::try_from(read.answer_ns).unwrap_or(i64::MAX),
                            "ns",
                        ));
                    }
                    said.push(
                        match (read.right_tool, read.args_match, read.result_carried) {
                            (true, true, Some(true)) => "right, carried",
                            (true, true, _) => "right, not carried",
                            (true, false, _) => "tool right, arguments not",
                            (false, _, _) if read.called => "wrong tool",
                            _ => "no call",
                        },
                    );
                }
            }
            rows.push(Reading::new(
                &dims,
                "call_tokens",
                as_integer(read.call_tokens),
                "tokens",
            ));
            rows.push(Reading::new(
                &dims,
                "call_ns",
                i64::try_from(read.call_ns).unwrap_or(i64::MAX),
                "ns",
            ));
        }
        lines.push(format!("  {:<22} {}", task.name, said.join(" · ")));
    }
    let of_calls = asked.saturating_sub(TRIALS);
    lines.push(format!(
        "  right tool in {called_right} of {of_calls}; arguments matched in {args_matched}; \
         result carried in {carried}; held back when nothing fit in {held_back} of {TRIALS}"
    ));
    Found {
        lines,
        fields: vec![
            ("tasks", Value::Integer(as_integer(TASKS.len()))),
            ("trials", Value::Integer(as_integer(TRIALS))),
            ("asked", Value::Integer(as_integer(asked))),
            ("right_tool", Value::Integer(as_integer(called_right))),
            ("args_matched", Value::Integer(as_integer(args_matched))),
            ("result_carried", Value::Integer(as_integer(carried))),
            ("held_back", Value::Integer(as_integer(held_back))),
        ],
        rows,
    }
}

/// One trial: the call, judged, and where it was right enough, the tool's
/// result fed back and the answer read for it.
fn one_trial(site: &Site<'_>, engine: &Served, task: &Task, trial: usize) -> Result<Trial, String> {
    let said = |failure: mcf_core::Failure| failure.detail().to_owned();
    let tools = Value::List(task.tools.iter().map(tool_value).collect());
    let user = Value::map([
        ("role", Value::text("user")),
        ("content", Value::text(task.asks)),
    ]);
    let rendered = engine
        .render_with_tools(Value::List(vec![user.clone()]), tools.clone())
        .map_err(said)?;
    let draw = if trial == 0 {
        Draw::greedy(0)
    } else {
        Draw {
            seed: u64::try_from(trial).unwrap_or(0),
            temperature: Thousandths(TEMPERATURE),
            truncation: Truncation::OFF,
        }
    };
    let ids = as_read(engine, &rendered)?;
    let (done, call_ns) = timed(|| {
        engine.complete(
            Prompt::Identifiers(&ids),
            CALL_BUDGET,
            draw,
            false,
            site.timed(),
        )
    });
    let completed = done.map_err(said)?;
    let call = call_in(&completed.text);
    let (called, right_tool, keys_match, args_match, extra_keys) =
        judged(call.as_ref(), task.expects);
    let mut read = Trial {
        called,
        right_tool,
        keys_match,
        args_match,
        extra_keys,
        result_carried: None,
        call_tokens: completed.predicted,
        call_ns,
        answer_tokens: 0,
        answer_ns: 0,
    };
    let (
        Expect::Call {
            tool,
            result,
            carries,
            ..
        },
        Some(call),
    ) = (task.expects, call)
    else {
        return Ok(read);
    };
    if !right_tool {
        return Ok(read);
    }
    let rendered = engine
        .render_with_tools(second_turn(user, tool, &call, result), tools)
        .map_err(said)?;
    let ids = as_read(engine, &rendered)?;
    let (done, answer_ns) = timed(|| {
        engine.complete(
            Prompt::Identifiers(&ids),
            ANSWER_BUDGET,
            draw,
            false,
            site.timed(),
        )
    });
    let completed = done.map_err(said)?;
    let spoken = completed
        .text
        .rfind("</think>")
        .and_then(|at| completed.text.get(at + "</think>".len()..))
        .unwrap_or(&completed.text);
    read.result_carried = Some(spoken.contains(carries));
    read.answer_tokens = completed.predicted;
    read.answer_ns = answer_ns;
    Ok(read)
}

/// The second turn's messages: the request, the model's own call, then
/// the tool's result, as a caller of the hosted server would send them
/// back.
fn second_turn(user: Value, tool: &str, call: &Call, result: &str) -> Value {
    let arguments = Value::Map(call.arguments.clone()).to_line();
    let assistant = Value::map([
        ("role", Value::text("assistant")),
        ("content", Value::text("")),
        (
            "tool_calls",
            Value::List(vec![Value::map([
                ("id", Value::text("call_1")),
                ("type", Value::text("function")),
                (
                    "function",
                    Value::map([
                        ("name", Value::text(tool)),
                        ("arguments", Value::text(arguments)),
                    ]),
                ),
            ])]),
        ),
    ]);
    let answer = Value::map([
        ("role", Value::text("tool")),
        ("tool_call_id", Value::text("call_1")),
        ("name", Value::text(tool)),
        ("content", Value::text(result)),
    ]);
    Value::List(vec![user, assistant, answer])
}

/// A rendered prompt as the engine reads it: its markers as markers, its
/// beginning as the model's convention.
pub(crate) fn as_read(engine: &Served, rendered: &str) -> Result<Vec<usize>, String> {
    engine
        .tokenize(rendered, true, true)
        .map(|tokens| tokens.into_iter().map(|token| token.id).collect())
        .map_err(|failure| failure.detail().to_owned())
}
