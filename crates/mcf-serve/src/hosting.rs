use mcf_core::configuration::CacheType;
use mcf_record::json::Value;
use mcf_standin::anatomy::grouped;

pub const LOOPBACK: &str = "127.0.0.1";

pub const DEFAULT_PORT: u16 = 17817;

pub const ALL_LAYERS: u32 = 999;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hosting {
    pub context: u64,
    pub gpu_layers: u32,
    pub engine: String,
    pub device: String,
    pub threads: u32,
    pub batch: u32,
    pub flash_attention: bool,
    pub cache: CacheType,
    pub slots: u32,
    pub keep_resident: bool,
    pub port: u16,
    pub api_key: Option<String>,
    pub open: bool,
    pub projector: Option<String>,
    pub started: crate::declared::Started,
    pub tensor_split: Vec<u64>,
}

fn spread(split: &[u64]) -> String {
    if split.len() < 2 {
        return "one device, the whole model on it".to_owned();
    }
    let total = split
        .iter()
        .fold(0_u64, |sum, free| sum.saturating_add(*free));
    let shares: Vec<String> = split
        .iter()
        .map(|free| {
            let percent = free
                .saturating_mul(100)
                .checked_div(total.max(1))
                .unwrap_or(0);
            format!("{percent}%")
        })
        .collect();
    format!("{} devices, {}", split.len(), shares.join(" / "))
}

fn projector_named(projector: Option<&str>) -> String {
    projector.map_or_else(
        || "none — text only".to_owned(),
        |path| {
            path.rsplit('/')
                .next()
                .filter(|name| !name.is_empty())
                .unwrap_or(path)
                .to_owned()
        },
    )
}

#[derive(Debug, Clone)]
pub struct Setting {
    pub name: &'static str,
    pub value: String,
    pub recommended: String,
    pub because: &'static str,
}

impl Hosting {
    #[must_use]
    pub fn recommended(
        engine: &str,
        device: &str,
        on_a_card: bool,
        context: u64,
        cores: Option<usize>,
        fits_on_the_card: bool,
        projector: Option<&std::path::Path>,
    ) -> Self {
        Self {
            context,
            gpu_layers: if on_a_card && fits_on_the_card {
                ALL_LAYERS
            } else {
                0
            },
            engine: engine.to_owned(),
            device: device.to_owned(),
            threads: cores
                .and_then(|cores| u32::try_from(cores).ok())
                .unwrap_or(4)
                .max(1),
            batch: 2048,
            flash_attention: on_a_card && fits_on_the_card,
            cache: CacheType::default(),
            slots: 1,
            keep_resident: false,
            port: DEFAULT_PORT,
            api_key: None,
            open: false,
            projector: projector.map(|path| path.display().to_string()),
            started: crate::declared::Started::default(),
            tensor_split: Vec::new(),
        }
    }

    /// What one conversation gets of the window the slots share.
    #[must_use]
    pub fn per_conversation(&self) -> u64 {
        self.context
            .checked_div(u64::from(self.slots.max(1)))
            .unwrap_or(self.context)
    }

    #[must_use]
    pub fn spread_over(mut self, free_per_device: Vec<u64>) -> Self {
        if free_per_device.len() > 1 {
            self.gpu_layers = ALL_LAYERS;
            self.flash_attention = true;
            self.tensor_split = free_per_device;
        }
        self
    }

    #[must_use]
    pub fn shares(&self) -> String {
        self.tensor_split
            .iter()
            .map(|free| (free >> 20).to_string())
            .collect::<Vec<String>>()
            .join(",")
    }

    #[must_use]
    pub fn arguments(&self, model: &str, bind: &str) -> Vec<String> {
        let mut out = vec![
            "--model".to_owned(),
            model.to_owned(),
            "--host".to_owned(),
            bind.to_owned(),
            "--ctx-size".to_owned(),
            self.context.to_string(),
            "--n-gpu-layers".to_owned(),
            self.gpu_layers.to_string(),
            "--threads".to_owned(),
            self.threads.to_string(),
            "--batch-size".to_owned(),
            self.batch.to_string(),
            "--no-webui".to_owned(),
            "--metrics".to_owned(),
        ];
        if self.tensor_split.len() > 1 {
            out.push("--tensor-split".to_owned());
            out.push(self.shares());
        }
        out.push("--parallel".to_owned());
        out.push(self.slots.max(1).to_string());
        out.push("--cache-type-k".to_owned());
        out.push(self.cache.as_str().to_owned());
        out.push("--cache-type-v".to_owned());
        out.push(self.cache.as_str().to_owned());
        if self.flash_attention || self.cache.is_quantized() {
            out.push("--flash-attn".to_owned());
            out.push("on".to_owned());
        }
        if self.keep_resident {
            out.push("--mlock".to_owned());
        }
        if let Some(key) = &self.api_key {
            out.push("--api-key".to_owned());
            out.push(key.clone());
        }
        if let Some(projector) = &self.projector {
            out.push("--mmproj".to_owned());
            out.push(projector.clone());
        }
        out.extend(self.started.arguments());
        out
    }

    #[must_use]
    #[allow(
        clippy::too_many_lines,
        reason = "one setting a row, each with its because"
    )]
    pub fn listed(&self, against: &Self) -> Vec<Setting> {
        let layers = |held: u32| {
            if held == 0 {
                "the processor, nothing on a card".to_owned()
            } else if held >= 999 {
                "the card, the whole model on it".to_owned()
            } else {
                format!("the card, {held} layers of the model on it")
            }
        };
        let yes_no = |held: bool| if held { "on" } else { "off" }.to_owned();
        vec![
            Setting {
                name: "context window",
                value: format!("{} tokens", grouped(self.context)),
                recommended: format!("{} tokens", grouped(against.context)),
                because: "how long a conversation it can hold. Every token of it costs \
                          memory on the device the model runs on, so MCF holds it at the \
                          largest window whose cache stays within the model's own size; \
                          --context sets it to anything that fits. Where more than one slot \
                          is asked for they share it, and `per conversation` is what each gets",
            },
            Setting {
                name: "put it on",
                value: layers(self.gpu_layers),
                recommended: layers(against.gpu_layers),
                because: "where the model goes: the whole of it on the card is several times \
                          quicker where it fits, and the build that drives the card is chosen \
                          with it",
            },
            Setting {
                name: "engine",
                value: self.engine.clone(),
                recommended: against.engine.clone(),
                because: "which build MCF starts. They differ in what they can compute on",
            },
            Setting {
                name: "device",
                value: self.device.clone(),
                recommended: against.device.clone(),
                because: "what it runs on",
            },
            Setting {
                name: "spread over",
                value: spread(&self.tensor_split),
                recommended: spread(&against.tensor_split),
                because: "a model too large for any one card is divided across several, in \
                          proportion to what each has free. One card holds the whole model \
                          wherever it fits, because crossing between cards costs time",
            },
            Setting {
                name: "threads",
                value: self.threads.to_string(),
                recommended: against.threads.to_string(),
                because: "how many processor threads the engine uses",
            },
            Setting {
                name: "batch size",
                value: self.batch.to_string(),
                recommended: against.batch.to_string(),
                because: "how many tokens of a prompt are read at once",
            },
            Setting {
                name: "flash attention",
                value: yes_no(self.flash_attention),
                recommended: yes_no(against.flash_attention),
                because: "an attention kernel that reads less memory for the same answer",
            },
            Setting {
                name: "per conversation",
                value: format!("{} tokens", grouped(self.per_conversation())),
                recommended: format!("{} tokens", grouped(against.per_conversation())),
                because: "the window one conversation actually gets: the whole of it on one \
                          slot, and its share where more were asked for",
            },
            Setting {
                name: "slots",
                value: self.slots.to_string(),
                recommended: against.slots.to_string(),
                because: "how many conversations the engine holds at once. The window is the \
                          pool they share, so two slots give each of them half of it; MCF asks \
                          for one so the window it reports is the window one conversation gets, \
                          rather than leaving the number to the engine",
            },
            Setting {
                name: "cache width",
                value: self.cache.as_str().to_owned(),
                recommended: against.cache.as_str().to_owned(),
                because: "how wide the engine holds each cached token. Every token of the \
                          window costs this much, so a narrower one fits a longer conversation \
                          in the same memory and answers from a shorter arithmetic; --cache \
                          sets it, and a narrow one is held with flash attention because the \
                          engine reads it no other way",
            },
            Setting {
                name: "keep resident",
                value: yes_no(self.keep_resident),
                recommended: yes_no(against.keep_resident),
                because: "hold the model's pages in memory rather than letting them page out",
            },
            Setting {
                name: "reachable from the network",
                value: yes_no(self.open),
                recommended: yes_no(against.open),
                because: "answer every address this machine has, not the loopback one alone; \
                          a key is required with it",
            },
            Setting {
                name: "port",
                value: self.port.to_string(),
                recommended: against.port.to_string(),
                because: if self.open {
                    "where the API listens, on every address this machine has"
                } else {
                    "where the API listens, on this computer only"
                },
            },
            Setting {
                name: "API key",
                value: self
                    .api_key
                    .as_ref()
                    .map_or_else(|| "none".to_owned(), |_| "set".to_owned()),
                recommended: "none".to_owned(),
                because: "a key callers must present. On this computer's own address, \
                          usually not needed",
            },
            Setting {
                name: "started with",
                value: self.started.said(),
                recommended: against.started.said(),
                because: "what the engine is started with beyond the plain load: the draft head \
                          some publishers train into the file, which the engine leaves there \
                          unless it is asked for it, and a scaling for positions past the \
                          conversation the model was trained on",
            },
            Setting {
                name: "projector",
                value: projector_named(self.projector.as_deref()),
                recommended: projector_named(against.projector.as_deref()),
                because: "the encoder that turns a picture, a video or a sound into what the \
                          model reads. Its publisher ships it as a second file beside the \
                          weights; without it the model takes text only",
            },
        ]
    }

    #[must_use]
    pub fn differs_from(&self, recommended: &Self) -> Vec<String> {
        self.listed(recommended)
            .into_iter()
            .filter(|setting| setting.value != setting.recommended)
            .map(|setting| {
                format!(
                    "{}: {} rather than {}",
                    setting.name, setting.value, setting.recommended
                )
            })
            .collect()
    }

    #[must_use]
    pub fn to_value(&self) -> Value {
        Value::map([
            (
                "context",
                Value::Integer(i64::try_from(self.context).unwrap_or(i64::MAX)),
            ),
            ("gpu_layers", Value::Integer(i64::from(self.gpu_layers))),
            ("engine", Value::text(self.engine.clone())),
            ("device", Value::text(self.device.clone())),
            ("threads", Value::Integer(i64::from(self.threads))),
            ("batch", Value::Integer(i64::from(self.batch))),
            ("flash_attention", Value::Bool(self.flash_attention)),
            ("cache", Value::text(self.cache.as_str())),
            ("slots", Value::Integer(i64::from(self.slots))),
            ("keep_resident", Value::Bool(self.keep_resident)),
            ("open", Value::Bool(self.open)),
            ("port", Value::Integer(i64::from(self.port))),
            ("api_key_set", Value::Bool(self.api_key.is_some())),
            (
                "projector",
                self.projector.clone().map_or(Value::Null, Value::text),
            ),
            ("draft_head", Value::Bool(self.started.draft_head)),
            (
                "rope_scaling",
                self.started
                    .rope
                    .map_or(Value::Null, |rope| Value::text(rope.as_str())),
            ),
            (
                "rope_scale",
                self.started
                    .factor
                    .map_or(Value::Null, |factor| Value::Integer(i64::from(factor))),
            ),
        ])
    }

    #[must_use]
    pub fn from_value(value: &Value, recommended: &Self) -> Self {
        let number = |key: &str| value.get(key).and_then(Value::as_integer);
        let flag = |key: &str, fallback: bool| match value.get(key) {
            Some(Value::Bool(held)) => *held,
            _ => fallback,
        };
        Self {
            context: number("context")
                .and_then(|held| u64::try_from(held).ok())
                .unwrap_or(recommended.context),
            gpu_layers: number("gpu_layers")
                .and_then(|held| u32::try_from(held).ok())
                .unwrap_or(recommended.gpu_layers),
            engine: value
                .get("engine")
                .and_then(Value::as_text)
                .unwrap_or(&recommended.engine)
                .to_owned(),
            device: value
                .get("device")
                .and_then(Value::as_text)
                .unwrap_or(&recommended.device)
                .to_owned(),
            threads: number("threads")
                .and_then(|held| u32::try_from(held).ok())
                .unwrap_or(recommended.threads)
                .max(1),
            batch: number("batch")
                .and_then(|held| u32::try_from(held).ok())
                .unwrap_or(recommended.batch)
                .max(1),
            flash_attention: flag("flash_attention", recommended.flash_attention),
            slots: number("slots")
                .and_then(|held| u32::try_from(held).ok())
                .unwrap_or(recommended.slots)
                .max(1),
            cache: value
                .get("cache")
                .and_then(Value::as_text)
                .and_then(CacheType::parse)
                .unwrap_or(recommended.cache),
            keep_resident: flag("keep_resident", recommended.keep_resident),
            open: flag("open", recommended.open),
            port: number("port")
                .and_then(|held| u16::try_from(held).ok())
                .unwrap_or(recommended.port),
            api_key: value
                .get("api_key")
                .and_then(Value::as_text)
                .map(str::to_owned),
            projector: match value.get("projector") {
                None => recommended.projector.clone(),
                Some(Value::Text(path)) if !path.is_empty() => Some(path.clone()),
                Some(_) => None,
            },
            started: crate::declared::Started::from_value(value),
            tensor_split: recommended.tensor_split.clone(),
        }
    }

    #[must_use]
    pub fn port_is_free(&self) -> bool {
        std::net::TcpListener::bind((LOOPBACK, self.port)).is_ok()
    }

    #[must_use]
    pub fn to_request(&self) -> Value {
        let mut asked = self.to_value();
        if let (Value::Map(fields), Some(key)) = (&mut asked, &self.api_key) {
            let _key = fields.insert("api_key".to_owned(), Value::text(key.clone()));
        }
        asked
    }

    #[must_use]
    pub fn address(&self) -> String {
        format!("http://{LOOPBACK}:{}", self.port)
    }

    #[must_use]
    pub const fn bind(&self) -> &'static str {
        if self.open { "0.0.0.0" } else { LOOPBACK }
    }

    #[must_use]
    pub fn network_address(&self) -> Option<String> {
        if !self.open {
            return None;
        }
        Some(format!("http://{}:{}", machine_address()?, self.port))
    }
}

#[must_use]
pub fn machine_address() -> Option<String> {
    let trie = std::fs::read_to_string("/proc/net/fib_trie").ok()?;
    let mut last: Option<&str> = None;
    for line in trie.lines() {
        let trimmed = line.trim();
        if trimmed == "/32 host LOCAL"
            && let Some(address) = last
            && !address.starts_with("127.")
        {
            return Some(address.to_owned());
        }
        last = trimmed
            .strip_prefix("|-- ")
            .or_else(|| trimmed.strip_prefix("+-- "));
    }
    None
}

#[cfg(test)]
mod tests;
