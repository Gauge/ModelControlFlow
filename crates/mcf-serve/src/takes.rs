use mcf_record::json::{self, Value};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Takes {
    pub images: Option<bool>,
    pub video: Option<bool>,
    pub audio: Option<bool>,
    pub tools: Option<bool>,
    pub tool_calls: Option<bool>,
    pub parallel_tool_calls: Option<bool>,
    pub system_role: Option<bool>,
    pub reasoning_effort: Option<bool>,
    pub preserve_reasoning: Option<bool>,
    pub thinking: Option<Thinking>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Thinking {
    NoSwitch,
    OnUnlessAskedOff { off_ends: String },
    OffUnlessAskedOn { on_ends: String },
    ThreeWays,
}

impl Takes {
    #[must_use]
    pub fn asked_on(port: u16) -> Option<Self> {
        let props =
            over_tcp(port, "GET", "/props", None).and_then(|body| json::parse(&body).ok())?;
        let modality = |name: &str| {
            props
                .get("modalities")
                .and_then(|held| held.get(name))
                .and_then(Value::as_bool)
        };
        let template = |name: &str| {
            props
                .get("chat_template_caps")
                .and_then(|held| held.get(name))
                .and_then(Value::as_bool)
        };
        Some(Self {
            images: modality("vision"),
            video: modality("video"),
            audio: modality("audio"),
            tools: template("supports_tools"),
            tool_calls: template("supports_tool_calls"),
            parallel_tool_calls: template("supports_parallel_tool_calls"),
            system_role: template("supports_system_role"),
            reasoning_effort: template("supports_reasoning_effort"),
            preserve_reasoning: template("supports_preserve_reasoning"),
            thinking: thinking_on(port),
        })
    }

    #[must_use]
    pub fn to_value(&self) -> Value {
        let flag = |held: Option<bool>| held.map_or(Value::Null, Value::Bool);
        let thinking = match &self.thinking {
            None => Value::Null,
            Some(Thinking::NoSwitch) => Value::map([("switch", Value::Bool(false))]),
            Some(Thinking::OnUnlessAskedOff { off_ends }) => Value::map([
                ("switch", Value::Bool(true)),
                ("unasked", Value::text("on")),
                ("off_ends", Value::text(off_ends.clone())),
            ]),
            Some(Thinking::OffUnlessAskedOn { on_ends }) => Value::map([
                ("switch", Value::Bool(true)),
                ("unasked", Value::text("off")),
                ("on_ends", Value::text(on_ends.clone())),
            ]),
            Some(Thinking::ThreeWays) => Value::map([
                ("switch", Value::Bool(true)),
                ("unasked", Value::text("neither")),
            ]),
        };
        Value::map([
            ("images", flag(self.images)),
            ("video", flag(self.video)),
            ("audio", flag(self.audio)),
            ("tools", flag(self.tools)),
            ("tool_calls", flag(self.tool_calls)),
            ("parallel_tool_calls", flag(self.parallel_tool_calls)),
            ("system_role", flag(self.system_role)),
            ("reasoning_effort", flag(self.reasoning_effort)),
            ("preserve_reasoning", flag(self.preserve_reasoning)),
            ("thinking", thinking),
            (
                "asked",
                Value::text("the engine's /props, and /apply-template for the thinking switch"),
            ),
        ])
    }

    #[must_use]
    pub fn from_value(value: &Value) -> Self {
        let flag = |name: &str| value.get(name).and_then(Value::as_bool);
        let thinking = value.get("thinking").and_then(|held| {
            let ends = |name: &str| {
                held.get(name)
                    .and_then(Value::as_text)
                    .unwrap_or_default()
                    .to_owned()
            };
            match (
                held.get("switch").and_then(Value::as_bool)?,
                held.get("unasked").and_then(Value::as_text),
            ) {
                (false, _) => Some(Thinking::NoSwitch),
                (true, Some("on")) => Some(Thinking::OnUnlessAskedOff {
                    off_ends: ends("off_ends"),
                }),
                (true, Some("off")) => Some(Thinking::OffUnlessAskedOn {
                    on_ends: ends("on_ends"),
                }),
                (true, _) => Some(Thinking::ThreeWays),
            }
        });
        Self {
            images: flag("images"),
            video: flag("video"),
            audio: flag("audio"),
            tools: flag("tools"),
            tool_calls: flag("tool_calls"),
            parallel_tool_calls: flag("parallel_tool_calls"),
            system_role: flag("system_role"),
            reasoning_effort: flag("reasoning_effort"),
            preserve_reasoning: flag("preserve_reasoning"),
            thinking,
        }
    }

    #[must_use]
    pub fn media(&self) -> String {
        let mut held = vec!["text"];
        if self.images == Some(true) {
            held.push("images");
        }
        if self.video == Some(true) {
            held.push("video");
        }
        if self.audio == Some(true) {
            held.push("audio");
        }
        joined(&held)
    }

    #[must_use]
    pub fn template(&self) -> String {
        let mut held = Vec::new();
        if self.system_role == Some(true) {
            held.push("a system turn");
        }
        match (self.tools, self.parallel_tool_calls) {
            (Some(true), Some(true)) => held.push("tools, several in a turn"),
            (Some(true), _) => held.push("tools"),
            _ => {}
        }
        if self.reasoning_effort == Some(true) {
            held.push("a reasoning effort");
        }
        if self.preserve_reasoning == Some(true) {
            held.push("earlier reasoning kept");
        }
        if held.is_empty() {
            "the engine did not say".to_owned()
        } else {
            joined(&held)
        }
    }

    #[must_use]
    pub fn thinking_said(&self) -> String {
        match &self.thinking {
            None => "the engine did not render the switch".to_owned(),
            Some(Thinking::NoSwitch) => "the template has no thinking switch".to_owned(),
            Some(Thinking::OnUnlessAskedOff { off_ends }) => format!(
                "on unless asked off — enable_thinking false ends the prompt {}",
                shown(off_ends)
            ),
            Some(Thinking::OffUnlessAskedOn { on_ends }) => format!(
                "off unless asked on — enable_thinking true ends the prompt {}",
                shown(on_ends)
            ),
            Some(Thinking::ThreeWays) => {
                "the switch changes the prompt, and unasked is a third rendering".to_owned()
            }
        }
    }
}

fn joined(words: &[&str]) -> String {
    match words {
        [] => String::new(),
        [one] => (*one).to_owned(),
        [head @ .., last] => format!("{} and {last}", head.join(", ")),
    }
}

fn shown(text: &str) -> String {
    format!("`{}`", text.replace('\n', "\\n"))
}

fn thinking_on(port: u16) -> Option<Thinking> {
    let render = |kwargs: Option<Value>| {
        let mut body = vec![(
            "messages",
            Value::List(vec![Value::map([
                ("role", Value::text("user")),
                ("content", Value::text("test")),
            ])]),
        )];
        if let Some(kwargs) = kwargs {
            body.push(("chat_template_kwargs", kwargs));
        }
        over_tcp(
            port,
            "POST",
            "/apply-template",
            Some(&Value::map(body).to_line()),
        )
        .and_then(|answer| json::parse(&answer).ok())
        .and_then(|answer| {
            answer
                .get("prompt")
                .and_then(Value::as_text)
                .map(str::to_owned)
        })
    };
    let unsaid = render(None)?;
    let on = render(Some(Value::map([("enable_thinking", Value::Bool(true))])))?;
    let off = render(Some(Value::map([("enable_thinking", Value::Bool(false))])))?;
    Some(compared(&unsaid, &on, &off))
}

fn compared(unsaid: &str, on: &str, off: &str) -> Thinking {
    if on == off {
        Thinking::NoSwitch
    } else if unsaid == on {
        Thinking::OnUnlessAskedOff {
            off_ends: past_the_shared(on, off),
        }
    } else if unsaid == off {
        Thinking::OffUnlessAskedOn {
            on_ends: past_the_shared(off, on),
        }
    } else {
        Thinking::ThreeWays
    }
}

fn past_the_shared(one: &str, other: &str) -> String {
    let shared = one
        .char_indices()
        .zip(other.chars())
        .take_while(|((_, a), b)| a == b)
        .last()
        .map_or(0, |((at, held), _)| at.saturating_add(held.len_utf8()));
    other.get(shared..).unwrap_or_default().to_owned()
}

fn over_tcp(port: u16, method: &str, path: &str, body: Option<&str>) -> Option<String> {
    use std::io::{Read as _, Write as _};
    let mut connection = std::net::TcpStream::connect((crate::hosting::LOOPBACK, port)).ok()?;
    connection
        .set_read_timeout(Some(std::time::Duration::from_secs(10)))
        .ok()?;
    let body = body.unwrap_or("");
    write!(
        connection,
        "{method} {path} HTTP/1.1\r\nHost: localhost\r\nContent-Type: application/json\r\n\
         Content-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    )
    .and_then(|()| connection.flush())
    .ok()?;
    let mut answer = Vec::new();
    connection.take(1 << 20).read_to_end(&mut answer).ok()?;
    let answer = String::from_utf8_lossy(&answer).into_owned();
    let (head, body) = answer.split_once("\r\n\r\n")?;
    head.contains(" 200 ").then(|| body.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_template_without_a_switch_renders_the_same_either_way() {
        assert_eq!(compared("a", "a", "a"), Thinking::NoSwitch);
    }

    #[test]
    fn a_model_that_thinks_unasked_shows_what_turns_it_off() {
        let on = "<|im_start|>assistant\n";
        let off = "<|im_start|>assistant\n<think>\n\n</think>\n\n";
        assert_eq!(
            compared(on, on, off),
            Thinking::OnUnlessAskedOff {
                off_ends: "<think>\n\n</think>\n\n".to_owned()
            }
        );
    }

    #[test]
    fn a_model_that_thinks_only_when_asked_shows_what_turns_it_on() {
        let on = "assistant\n<think>\n";
        let off = "assistant\n";
        assert_eq!(
            compared(off, on, off),
            Thinking::OffUnlessAskedOn {
                on_ends: "<think>\n".to_owned()
            }
        );
    }

    #[test]
    fn a_third_rendering_is_not_folded_into_either() {
        assert_eq!(compared("c", "a", "b"), Thinking::ThreeWays);
    }

    #[test]
    fn the_record_carries_it_back_unchanged() {
        let takes = Takes {
            images: Some(true),
            video: Some(true),
            audio: Some(false),
            tools: Some(true),
            tool_calls: Some(true),
            parallel_tool_calls: Some(true),
            system_role: Some(true),
            reasoning_effort: Some(true),
            preserve_reasoning: None,
            thinking: Some(Thinking::OnUnlessAskedOff {
                off_ends: "<think>\n\n</think>\n\n".to_owned(),
            }),
        };
        assert_eq!(Takes::from_value(&takes.to_value()), takes);
        assert_eq!(takes.media(), "text, images and video");
        assert_eq!(
            takes.template(),
            "a system turn, tools, several in a turn and a reasoning effort"
        );
        assert!(takes.thinking_said().starts_with("on unless asked off"));
    }

    #[test]
    fn an_engine_that_said_nothing_claims_nothing() {
        let takes = Takes::from_value(&Value::map::<&str>([]));
        assert_eq!(takes.images, None);
        assert_eq!(takes.media(), "text");
        assert_eq!(takes.template(), "the engine did not say");
        assert_eq!(
            takes.thinking_said(),
            "the engine did not render the switch"
        );
    }
}
