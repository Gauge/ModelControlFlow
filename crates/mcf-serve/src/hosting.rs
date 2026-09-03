//! Every setting a hosted model runs under, and what MCF recommends.
//!
//! **Nothing here was exposed before, and one of the hidden values was
//! wrong.** The engine was started with `--ctx-size 0 -ngl 0` written into the
//! source: no context of MCF's choosing, and *no layers on the graphics card*.
//! So MCF resolved a model to a card, said so, and then ran it on the
//! processor — a stated condition that was not the condition (A6, A12). On
//! this machine that cost 4.9× (F133).
//!
//! **A default is a decision, so it is one somebody can see and change.**
//! §3.15: MCF doing something other than the plain thing must never be
//! invisible. Every field below is settable, every field has a recommendation
//! MCF computed from the model and the machine, and [`Hosting::differs_from`]
//! says which of them somebody has moved — so the record can carry what was
//! chosen *and* what was recommended, and the two can disagree in writing.
//!
//! **Sampling is not here, and once was.** This module briefly carried its own
//! `Sampling` type — temperature, top-p, top-k, a repetition penalty and a
//! seed, held in thousandths as whole numbers so that no float reached a
//! record. Every part of that was right and all of it already existed:
//! `mcf_core::configuration::Sampling` holds the same five things in
//! `Thousandths`, which even renders as the same decimal. It also holds each
//! of them as [`mcf_core::attested::Attested`], which the copy did not — and
//! that is the distinction B-281 turns on, because a sampling value a
//! publisher declared and one a sweep measured are not the same kind of fact.
//! The copy was never wired to anything, so what it cost was a second answer
//! to a question already answered (B-419, A1).
//!
//! **The API is the engine's, and MCF says so.** MCF does not implement an
//! inference API; it provisions an engine that has one and supervises it. What
//! hosting does is bind that engine to a port with these settings and write
//! down what it started. Claiming the API as MCF's own would be claiming
//! authorship of the thing A19 says not to advertise.

use mcf_record::json::Value;

/// Where a hosted model listens.
///
/// The loopback address only, and stated rather than defaulted silently: a
/// model bound to every interface is a model on the network, and that is a
/// decision somebody makes rather than one MCF makes for them (§3.7).
pub const LOOPBACK: &str = "127.0.0.1";

/// The port MCF asks for when nobody has said.
///
/// **Not a port anything else is known to want.** The first value chosen here
/// was 11434, on the reasoning that nothing common uses it — which was wrong:
/// it is Ollama's default, and on the machine this was written on Ollama had
/// it. The lesson is not the number but that the guess was made instead of
/// the check, so [`Hosting::port_is_free`] now runs before an engine is
/// started and the refusal says what happened (A2).
pub const DEFAULT_PORT: u16 = 17817;

/// How a model is held and answered with.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hosting {
    /// How long a conversation it can hold, in tokens.
    ///
    /// A power of two, never past what the model was trained for and never
    /// past what the device's memory holds — the same arithmetic
    /// [`crate::engines::resolve`] does, because it is the same question.
    pub context: u64,
    /// How many of the model's layers go on the graphics card.
    ///
    /// The one that was hardcoded to zero. `0` is the processor; a number at
    /// or above the model's layer count is all of it.
    pub gpu_layers: u32,
    /// Which provisioned engine.
    pub engine: String,
    /// Which device, as the engine names it.
    pub device: String,
    /// How many processor threads the engine uses.
    pub threads: u32,
    /// How many tokens of prompt are read at once.
    pub batch: u32,
    /// Whether to use the attention kernel that reads less memory.
    pub flash_attention: bool,
    /// Whether to keep the model's pages resident rather than paged.
    pub keep_resident: bool,
    /// The port it listens on.
    pub port: u16,
    /// A key callers must present, where somebody set one.
    ///
    /// `None` is no key, which on the loopback address is the ordinary case
    /// and is stated rather than assumed.
    pub api_key: Option<String>,
    /// The projector loaded beside the model, where it has one.
    ///
    /// **Half of a multimodal model travels as a second file, and a host that
    /// leaves it on disk hosts half the model.** The publisher's `mmproj`
    /// beside the weights is what lets a picture, a video or a sound reach a
    /// model that was trained to take them; without it the engine answers
    /// text and quietly declines the rest. `None` is text only — the
    /// recommendation where no projector sits beside the model, and a choice
    /// somebody can make where one does.
    pub projector: Option<String>,
    /// What the engine is started with beyond the plain load: the model's
    /// own draft head, a rope scaling.
    ///
    /// **A file can declare more than an engine starts, and MCF starts
    /// neither of these by itself.** A draft head is left in the file unless
    /// it is asked for, and a rope scaling the file does not declare is a
    /// change to how the model reads position that somebody chooses and the
    /// account then carries. The recommendation is nothing asked for, so a
    /// model hosted with either says so in the settings that moved (B-456,
    /// D43, §3.15).
    pub started: crate::declared::Started,
}

/// How a hosted model turns a prompt into text.
///
/// Separate from [`Hosting`] because these can change per request and those
/// cannot: a context size is chosen when the model is loaded, and a
/// temperature is chosen when somebody asks a question.
/// A projector as a setting names it: the file, not the whole path.
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

/// What one setting is, for a surface that lists them all.
#[derive(Debug, Clone)]
pub struct Setting {
    /// What it is called.
    pub name: &'static str,
    /// What it is set to now.
    pub value: String,
    /// What MCF recommended.
    pub recommended: String,
    /// What it does, in a sentence.
    pub because: &'static str,
}

impl Hosting {
    /// What MCF recommends for this model on this machine.
    ///
    /// **Every value here is derived from something measured, and the ones
    /// that are not are named.** The context is arithmetic over the model's
    /// own header and the device's free memory. The layer count is all of
    /// them where the weights and the cache fit on the card and none where
    /// they do not — there is no half-way that is not a guess. The thread
    /// count is the machine's cores. The batch size is the engine's own
    /// default, restated here so that changing it is a change to a number
    /// somebody can see rather than to a default nobody knew about.
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
            // The whole model, or none of it. A partial offload is a real
            // configuration and a poor default: it is slower than the card
            // and harder to account for than the processor, and choosing how
            // many layers needs a measurement nobody has taken yet.
            gpu_layers: if on_a_card && fits_on_the_card {
                999
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
            // Reads less memory per token at the same answer, where the
            // engine has it. Off where the model runs on the processor,
            // because that is where it has been seen to help least.
            flash_attention: on_a_card && fits_on_the_card,
            keep_resident: false,
            port: DEFAULT_PORT,
            api_key: None,
            projector: projector.map(|path| path.display().to_string()),
            // Never on by themselves: a draft head changes what the tokens
            // are drawn from, and a stretched rope changes what the model
            // makes of a position. Both are somebody's decision (D43).
            started: crate::declared::Started::default(),
        }
    }

    /// The arguments this becomes on the engine's command line.
    ///
    /// One place, so that what MCF asked for and what MCF records are built
    /// from the same values — a record assembled separately from the command
    /// is a record that can describe a run that did not happen (A6).
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
            // MCF draws its own interface and reads its own answers; a web
            // page served alongside is a surface nobody asked for.
            "--no-webui".to_owned(),
        ];
        if self.flash_attention {
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

    /// Every setting, in the order a person reads them, with what MCF
    /// recommended beside what it is set to.
    #[must_use]
    pub fn listed(&self, against: &Self) -> Vec<Setting> {
        let layers = |held: u32| {
            if held == 0 {
                "none — the processor".to_owned()
            } else if held >= 999 {
                "all of them".to_owned()
            } else {
                held.to_string()
            }
        };
        let yes_no = |held: bool| if held { "on" } else { "off" }.to_owned();
        vec![
            Setting {
                name: "context window",
                value: format!("{} tokens", self.context),
                recommended: format!("{} tokens", against.context),
                because: "how long a conversation it can hold. Every token of it costs \
                          memory on the device the model runs on",
            },
            Setting {
                name: "layers on the card",
                value: layers(self.gpu_layers),
                recommended: layers(against.gpu_layers),
                because: "how much of the model the graphics card holds. All of it is \
                          several times quicker where it fits",
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
                name: "keep resident",
                value: yes_no(self.keep_resident),
                recommended: yes_no(against.keep_resident),
                because: "hold the model's pages in memory rather than letting them page out",
            },
            Setting {
                name: "port",
                value: self.port.to_string(),
                recommended: against.port.to_string(),
                because: "where the API listens, on this computer only",
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

    /// Which settings have been moved off what MCF recommended.
    ///
    /// The record carries both, because *what was chosen* and *what was
    /// advised* are two facts and a run under a changed setting is not a run
    /// under the recommended one (A6, §3.15).
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

    /// As the record and the control plane carry it.
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
            ("keep_resident", Value::Bool(self.keep_resident)),
            ("port", Value::Integer(i64::from(self.port))),
            // The key itself is never written down. That it exists is a
            // condition of the hosting; what it is is a secret, and a record
            // is a thing MCF publishes (A25, §3.20).
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

    /// Read back from what the control plane carried.
    ///
    /// Absent fields keep the recommendation rather than a zero: a client that
    /// did not mention a setting has not asked for its lowest value (A7, D43).
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
            keep_resident: flag("keep_resident", recommended.keep_resident),
            port: number("port")
                .and_then(|held| u16::try_from(held).ok())
                .unwrap_or(recommended.port),
            api_key: value
                .get("api_key")
                .and_then(Value::as_text)
                .map(str::to_owned),
            // Three states, told apart: unmentioned keeps the projector MCF
            // found, a path names one, and an explicit null or empty name is
            // *text only* asked for — which is a choice, not an omission.
            projector: match value.get("projector") {
                None => recommended.projector.clone(),
                Some(Value::Text(path)) if !path.is_empty() => Some(path.clone()),
                Some(_) => None,
            },
            // Read from the same three names the record carries, so that a
            // hosting read back is the hosting that was written down.
            started: crate::declared::Started::from_value(value),
        }
    }

    /// Whether anything is already listening where this would bind.
    ///
    /// Asked before the engine is started, because an engine that exits
    /// because its port was taken exits with a status and no sentence — and
    /// *the provisioned server stopped before it began answering* is a true
    /// report of the wrong thing (A2).
    #[must_use]
    pub fn port_is_free(&self) -> bool {
        std::net::TcpListener::bind((LOOPBACK, self.port)).is_ok()
    }

    /// Where a caller reaches a model hosted under these settings.
    #[must_use]
    pub fn address(&self) -> String {
        format!("http://{LOOPBACK}:{}", self.port)
    }
}

#[cfg(test)]
mod tests;
