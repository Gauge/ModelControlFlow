//! Multi-step tool use: chains where each argument comes from the last
//! result, parallel calls in one turn, and an error result the model
//! must react to (B-520, D55, A19).
//!
//! **The agent question.** The tool-use suite asks for one call and one
//! answer. An agent is a model that makes the next call from what the
//! last one returned, makes two calls at once when two are wanted, and
//! does something sensible when a tool says no. Each is a parser's
//! question put over several turns: at every step the call the model made
//! is judged by exact match against the step's expected tool and
//! arguments, the tool's stated result goes back as a caller of the hosted
//! server would send it, and the run ends where the model stops calling
//! rightly. Every step is a row.

use mcf_record::json::Value;

use super::tooluse::{Arg, Call, Tool, argument_matches, as_read, calls_in, tool_value};
use super::{Found, Reading, Site, as_integer, timed};
use crate::generation::{Draw, Truncation};
use crate::served::{Prompt, Served, Startup};
use mcf_core::configuration::Thousandths;

/// The measurement's name.
pub const NAME: &str = "multi-step-tool-use";

/// How many trials each task has: one greedy, one drawn.
pub const TRIALS: usize = 2;

/// How many tokens a turn may take.
const TURN_BUDGET: usize = 220;

/// One call a step expects, and what the tool answers it with.
#[derive(Debug, Clone, Copy)]
struct Expected {
    tool: &'static str,
    arguments: &'static [(&'static str, Arg)],
    result: &'static str,
}

/// One step: the calls the turn should make, at once where there are
/// several.
#[derive(Debug, Clone, Copy)]
struct Step {
    calls: &'static [Expected],
}

/// One task.
#[derive(Debug, Clone, Copy)]
struct Task {
    name: &'static str,
    /// What kind of ask: for the rows and the report.
    kind: &'static str,
    tools: &'static [Tool],
    asks: &'static str,
    steps: &'static [Step],
    /// What the final answer must carry, each from a tool's result.
    carries: &'static [&'static str],
}

const LOOKUP_CITY: Tool = Tool {
    name: "lookup_city",
    description: "Find a city's identifier by its name.",
    parameters: &[("name", "string", true)],
};
const WEATHER_BY_ID: Tool = Tool {
    name: "weather_by_id",
    description: "Look up the current weather for a city identifier.",
    parameters: &[("city_id", "integer", true)],
};
const FIND_USER: Tool = Tool {
    name: "find_user",
    description: "Find a user's identifier by email address.",
    parameters: &[("email", "string", true)],
};
const LIST_ORDERS: Tool = Tool {
    name: "list_orders",
    description: "List a user's orders, newest first, by user identifier.",
    parameters: &[("user_id", "integer", true)],
};
const ORDER_STATUS: Tool = Tool {
    name: "order_status",
    description: "The shipping status of an order, by order identifier.",
    parameters: &[("order_id", "string", true)],
};
const EXCHANGE_RATE: Tool = Tool {
    name: "exchange_rate",
    description: "The exchange rate from one currency to another, in basis points (1 unit = 10000).",
    parameters: &[("from", "string", true), ("to", "string", true)],
};
const CONVERT: Tool = Tool {
    name: "convert_amount",
    description: "Convert an amount using a rate in basis points.",
    parameters: &[("amount", "integer", true), ("rate_bp", "integer", true)],
};

const TASKS: [Task; 5] = [
    Task {
        name: "city-then-weather",
        kind: "chain",
        tools: &[LOOKUP_CITY, WEATHER_BY_ID],
        asks: "What is the weather in Lisbon? Look the city up to get its identifier first, then \
               use the identifier.",
        steps: &[
            Step {
                calls: &[Expected {
                    tool: "lookup_city",
                    arguments: &[("name", Arg::Text("Lisbon"))],
                    result: "{\"city_id\": 8814}",
                }],
            },
            Step {
                calls: &[Expected {
                    tool: "weather_by_id",
                    arguments: &[("city_id", Arg::Integer(8814))],
                    result: "{\"temperature_c\": 19, \"sky\": \"clear\"}",
                }],
            },
        ],
        carries: &["19"],
    },
    Task {
        name: "three-steps",
        kind: "chain",
        tools: &[FIND_USER, LIST_ORDERS, ORDER_STATUS],
        asks: "When will the latest order for ana@example.com arrive? Find the user, list the \
               orders, then check the latest order's status.",
        steps: &[
            Step {
                calls: &[Expected {
                    tool: "find_user",
                    arguments: &[("email", Arg::Text("ana@example.com"))],
                    result: "{\"user_id\": 42}",
                }],
            },
            Step {
                calls: &[Expected {
                    tool: "list_orders",
                    arguments: &[("user_id", Arg::Integer(42))],
                    result: "{\"orders\": [{\"order_id\": \"ORD77\", \"placed\": \"2026-09-28\"}, {\"order_id\": \"ORD51\", \"placed\": \"2026-08-02\"}]}",
                }],
            },
            Step {
                calls: &[Expected {
                    tool: "order_status",
                    arguments: &[("order_id", Arg::Text("ORD77"))],
                    result: "{\"status\": \"shipped\", \"arrives\": \"2026-10-02\"}",
                }],
            },
        ],
        carries: &["2026-10-02"],
    },
    Task {
        name: "rate-then-convert",
        kind: "chain",
        tools: &[EXCHANGE_RATE, CONVERT],
        asks: "Convert 250 EUR to JPY: get the exchange rate first, then convert with it.",
        steps: &[
            Step {
                calls: &[Expected {
                    tool: "exchange_rate",
                    arguments: &[("from", Arg::Text("EUR")), ("to", Arg::Text("JPY"))],
                    result: "{\"rate_bp\": 1631000}",
                }],
            },
            Step {
                calls: &[Expected {
                    tool: "convert_amount",
                    arguments: &[
                        ("amount", Arg::Integer(250)),
                        ("rate_bp", Arg::Integer(1_631_000)),
                    ],
                    result: "{\"value\": 40775}",
                }],
            },
        ],
        carries: &["40775"],
    },
    Task {
        name: "two-at-once",
        kind: "parallel",
        tools: &[super::tooluse::WEATHER],
        asks: "What is the weather in Paris and in Tokyo right now? Look both up.",
        steps: &[Step {
            calls: &[
                Expected {
                    tool: "get_weather",
                    arguments: &[("city", Arg::Text("Paris"))],
                    result: "{\"temperature_c\": 17, \"sky\": \"overcast\"}",
                },
                Expected {
                    tool: "get_weather",
                    arguments: &[("city", Arg::Text("Tokyo"))],
                    result: "{\"temperature_c\": 22, \"sky\": \"clear\"}",
                },
            ],
        }],
        carries: &["17", "22"],
    },
    Task {
        name: "an-error-to-recover-from",
        kind: "error",
        tools: &[super::tooluse::WEATHER],
        asks: "What is the weather in Springfield, the one in Illinois?",
        steps: &[
            Step {
                calls: &[Expected {
                    tool: "get_weather",
                    arguments: &[],
                    result: "{\"error\": \"ambiguous city\", \"choices\": [\"Springfield, IL\", \"Springfield, MA\"]}",
                }],
            },
            Step {
                calls: &[Expected {
                    tool: "get_weather",
                    arguments: &[("city", Arg::Text("Springfield, IL"))],
                    result: "{\"temperature_c\": 12, \"sky\": \"rain\"}",
                }],
            },
        ],
        carries: &["12"],
    },
];

/// What one trial read.
#[derive(Debug, Default)]
struct Read {
    /// Per step: how many of its expected calls were made, and rightly.
    steps: Vec<(usize, usize, usize)>,
    /// How many calls the first turn made.
    first_turn_calls: usize,
    completed: bool,
    carried: bool,
    tokens: usize,
    ns: u64,
}

/// Whether a call matches an expected one.
fn matches(call: &Call, expected: &Expected) -> (bool, bool) {
    let tool = call.name == expected.tool;
    let args = tool
        && expected.arguments.iter().all(|(key, want)| {
            call.arguments
                .get(*key)
                .is_some_and(|got| argument_matches(got, *want))
        });
    (tool, args)
}

/// The calls a turn made against the calls a step expected: how many
/// named the right tool, how many with the right arguments, and the
/// messages that send each right call back with its result.
fn matched(calls: &[Call], step: &Step) -> (usize, usize, Vec<Value>, Vec<Value>) {
    let mut made = 0_usize;
    let mut right = 0_usize;
    let mut tool_calls = Vec::new();
    let mut results = Vec::new();
    for (which, expected) in step.calls.iter().enumerate() {
        // The nth call naming this tool, for a step that wants the same
        // tool twice; any nth call otherwise.
        let named: Vec<&Call> = calls
            .iter()
            .filter(|call| call.name == expected.tool)
            .collect();
        let Some(call) = named.get(which).copied().or_else(|| calls.get(which)) else {
            continue;
        };
        let (tool, args) = matches(call, expected);
        if !tool {
            continue;
        }
        made = made.saturating_add(1);
        if args {
            right = right.saturating_add(1);
        }
        let id = format!("call_{}", tool_calls.len() + 1);
        tool_calls.push(Value::map([
            ("id", Value::text(id.clone())),
            ("type", Value::text("function")),
            (
                "function",
                Value::map([
                    ("name", Value::text(call.name.clone())),
                    (
                        "arguments",
                        Value::text(Value::Map(call.arguments.clone()).to_line()),
                    ),
                ]),
            ),
        ]));
        results.push(Value::map([
            ("role", Value::text("tool")),
            ("tool_call_id", Value::text(id)),
            ("name", Value::text(call.name.clone())),
            ("content", Value::text(expected.result)),
        ]));
    }
    (made, right, tool_calls, results)
}

/// Runs one trial of one task: turn by turn until a step is missed.
fn one_trial(site: &Site<'_>, engine: &Served, task: &Task, trial: usize) -> Result<Read, String> {
    let said = |failure: mcf_core::Failure| failure.detail().to_owned();
    let tools = Value::List(task.tools.iter().map(tool_value).collect());
    let draw = if trial == 0 {
        Draw::greedy(0)
    } else {
        Draw {
            seed: u64::try_from(trial).unwrap_or(0),
            temperature: Thousandths(super::tooluse::TEMPERATURE),
            truncation: Truncation::OFF,
        }
    };
    let mut messages = vec![Value::map([
        ("role", Value::text("user")),
        ("content", Value::text(task.asks)),
    ])];
    let mut read = Read::default();
    let turn = |messages: &[Value]| -> Result<(String, usize, u64), String> {
        let rendered = engine
            .render_with_tools(Value::List(messages.to_vec()), tools.clone())
            .map_err(said)?;
        let ids = as_read(engine, &rendered)?;
        let (done, ns) = timed(|| {
            engine.complete(
                Prompt::Identifiers(&ids),
                TURN_BUDGET,
                draw,
                false,
                site.timed(),
            )
        });
        let completed = done.map_err(said)?;
        Ok((completed.text, completed.predicted, ns))
    };
    for (at, step) in task.steps.iter().enumerate() {
        let (text, tokens, ns) = turn(&messages)?;
        read.tokens = read.tokens.saturating_add(tokens);
        read.ns = read.ns.saturating_add(ns);
        let calls = calls_in(&text);
        if at == 0 {
            read.first_turn_calls = calls.len();
        }
        let (made, right, tool_calls, results) = matched(&calls, step);
        read.steps.push((step.calls.len(), made, right));
        if made < step.calls.len() {
            return Ok(read);
        }
        messages.push(Value::map([
            ("role", Value::text("assistant")),
            ("content", Value::text("")),
            ("tool_calls", Value::List(tool_calls)),
        ]));
        messages.extend(results);
    }
    read.completed = true;
    let (text, tokens, ns) = turn(&messages)?;
    read.tokens = read.tokens.saturating_add(tokens);
    read.ns = read.ns.saturating_add(ns);
    let spoken = text
        .rfind("</think>")
        .and_then(|at| text.get(at + "</think>".len()..))
        .unwrap_or(&text);
    read.carried = task.carries.iter().all(|carry| spoken.contains(carry));
    Ok(read)
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
        "  {} task(s), {TRIALS} trial(s) each — one greedy, one drawn — each step's call judged \
         by exact match and the tool's result sent back",
        TASKS.len()
    )];
    let (mut completed, mut carried, mut asked) = (0_usize, 0_usize, 0_usize);
    for (at, task) in TASKS.iter().enumerate() {
        site.progress(at, TASKS.len(), task.name);
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
            if read.completed {
                completed = completed.saturating_add(1);
            }
            if read.carried {
                carried = carried.saturating_add(1);
            }
            let flag = |held: bool| i64::from(held);
            for (at, (wanted, made, right)) in read.steps.iter().enumerate() {
                let dims = [
                    ("task", Value::text(task.name)),
                    ("kind", Value::text(task.kind)),
                    ("trial", Value::Integer(as_integer(trial))),
                    ("step", Value::Integer(as_integer(at))),
                ];
                rows.push(Reading::new(
                    &dims,
                    "calls_wanted",
                    as_integer(*wanted),
                    "count",
                ));
                rows.push(Reading::new(
                    &dims,
                    "calls_made",
                    as_integer(*made),
                    "count",
                ));
                rows.push(Reading::new(
                    &dims,
                    "calls_right",
                    as_integer(*right),
                    "count",
                ));
            }
            let dims = [
                ("task", Value::text(task.name)),
                ("kind", Value::text(task.kind)),
                ("trial", Value::Integer(as_integer(trial))),
            ];
            rows.push(Reading::new(
                &dims,
                "steps_completed",
                as_integer(read.steps.iter().filter(|(w, m, _)| m >= w).count()),
                "count",
            ));
            rows.push(Reading::new(
                &dims,
                "steps_wanted",
                as_integer(task.steps.len()),
                "count",
            ));
            rows.push(Reading::new(
                &dims,
                "first_turn_calls",
                as_integer(read.first_turn_calls),
                "count",
            ));
            rows.push(Reading::new(
                &dims,
                "completed",
                flag(read.completed),
                "bool",
            ));
            rows.push(Reading::new(
                &dims,
                "result_carried",
                flag(read.carried),
                "bool",
            ));
            rows.push(Reading::new(
                &dims,
                "tokens",
                as_integer(read.tokens),
                "tokens",
            ));
            rows.push(Reading::new(
                &dims,
                "ns",
                i64::try_from(read.ns).unwrap_or(i64::MAX),
                "ns",
            ));
            said.push(match (read.completed, read.carried) {
                (true, true) => "completed, carried".to_owned(),
                (true, false) => "completed, not carried".to_owned(),
                (false, _) => format!(
                    "stopped at step {} of {}",
                    read.steps.len(),
                    task.steps.len()
                ),
            });
        }
        lines.push(format!("  {:<26} {}", task.name, said.join(" · ")));
    }
    lines.push(format!(
        "  completed the chain in {completed} of {asked}; carried every result in {carried}"
    ));
    Found {
        lines,
        fields: vec![
            ("tasks", Value::Integer(as_integer(TASKS.len()))),
            ("trials", Value::Integer(as_integer(TRIALS))),
            ("asked", Value::Integer(as_integer(asked))),
            ("completed", Value::Integer(as_integer(completed))),
            ("result_carried", Value::Integer(as_integer(carried))),
        ],
        rows,
    }
}
