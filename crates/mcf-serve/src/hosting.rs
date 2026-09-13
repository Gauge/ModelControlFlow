use mcf_core::configuration::CacheType;
use mcf_record::json::Value;
use mcf_standin::anatomy::grouped;

pub const LOOPBACK: &str = "127.0.0.1";

pub const DEFAULT_PORT: u16 = 17817;

pub const ALL_LAYERS: u32 = 999;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Answers {
    #[default]
    Chat,
    Embeddings,
    Reranking,
}

impl Answers {
    pub const ALL: [Self; 3] = [Self::Chat, Self::Embeddings, Self::Reranking];

    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Chat => "chat",
            Self::Embeddings => "embeddings",
            Self::Reranking => "reranking",
        }
    }

    #[must_use]
    pub fn parse(said: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|held| held.as_str() == said)
    }

    #[must_use]
    pub const fn said(self) -> &'static str {
        match self {
            Self::Chat => "chat completions, and nothing else",
            Self::Embeddings => "embeddings only, which is what an embedding model is for",
            Self::Reranking => "reranking, scoring documents against a query",
        }
    }

    #[must_use]
    pub const fn keeps_a_conversation(self) -> bool {
        matches!(self, Self::Chat)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Pooling {
    #[default]
    TheModels,
    None,
    Mean,
    Cls,
    Last,
    Rank,
}

impl Pooling {
    pub const ALL: [Self; 6] = [
        Self::TheModels,
        Self::None,
        Self::Mean,
        Self::Cls,
        Self::Last,
        Self::Rank,
    ];

    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::TheModels => "the model's own",
            Self::None => "none",
            Self::Mean => "mean",
            Self::Cls => "cls",
            Self::Last => "last",
            Self::Rank => "rank",
        }
    }

    #[must_use]
    pub fn parse(said: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|held| held.as_str() == said)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Loading {
    #[default]
    Auto,
    None,
    Mapped,
    Locked,
    MappedAndLocked,
    Direct,
}

impl Loading {
    pub const ALL: [Self; 6] = [
        Self::Auto,
        Self::None,
        Self::Mapped,
        Self::Locked,
        Self::MappedAndLocked,
        Self::Direct,
    ];

    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::None => "none",
            Self::Mapped => "mmap",
            Self::Locked => "mlock",
            Self::MappedAndLocked => "mmap+mlock",
            Self::Direct => "dio",
        }
    }

    #[must_use]
    pub fn parse(said: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|held| held.as_str() == said)
    }

    #[must_use]
    pub const fn said(self) -> &'static str {
        match self {
            Self::Auto => "mapped, unless a device cannot take it",
            Self::None => "read plainly, neither mapped nor held",
            Self::Mapped => "mapped from the file",
            Self::Locked => "held in memory rather than paged out",
            Self::MappedAndLocked => "mapped, and held in memory",
            Self::Direct => "read straight from the device, past the page cache",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Lazily {
    #[default]
    Auto,
    On,
    Off,
}

impl Lazily {
    pub const ALL: [Self; 3] = [Self::Auto, Self::On, Self::Off];

    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::On => "on",
            Self::Off => "off",
        }
    }

    #[must_use]
    pub fn parse(said: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|held| held.as_str() == said)
    }

    #[must_use]
    pub const fn said(self) -> &'static str {
        match self {
            Self::Auto => "on for tensors above four gigabytes",
            Self::On => "rows read from disk as they are needed",
            Self::Off => "everything resident",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Split {
    #[default]
    Layer,
    None,
    Row,
    Tensor,
}

impl Split {
    pub const ALL: [Self; 4] = [Self::Layer, Self::None, Self::Row, Self::Tensor];

    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Layer => "layer",
            Self::None => "none",
            Self::Row => "row",
            Self::Tensor => "tensor",
        }
    }

    #[must_use]
    pub fn parse(said: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|held| held.as_str() == said)
    }

    #[must_use]
    pub const fn said(self) -> &'static str {
        match self {
            Self::Layer => "layers and cache divided between the cards",
            Self::None => "one card only",
            Self::Row => "each weight divided across the cards by rows",
            Self::Tensor => "weights and cache divided across the cards",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Experts {
    #[default]
    WithTheModel,
    FirstLayers(u32),
    OnTheProcessor,
}

impl Experts {
    #[must_use]
    pub fn said(self) -> String {
        match self {
            Self::WithTheModel => "wherever the model is".to_owned(),
            Self::FirstLayers(layers) => {
                format!("the first {layers} layers' experts on the processor")
            }
            Self::OnTheProcessor => "all of them on the processor".to_owned(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Spread {
    pub cache_on_processor: bool,
    pub split: Split,
    pub experts: Experts,
    pub ffn_layers_on_processor: u32,
    pub main_device: u32,
    pub devices: Option<String>,
    pub override_tensors: Option<String>,
}

impl Spread {
    #[must_use]
    pub fn arguments(&self) -> Vec<String> {
        let mut out = vec![
            "--split-mode".to_owned(),
            self.split.as_str().to_owned(),
            "--main-gpu".to_owned(),
            self.main_device.to_string(),
        ];
        if self.cache_on_processor {
            out.push("--no-kv-offload".to_owned());
        }
        match self.experts {
            Experts::WithTheModel => {}
            Experts::OnTheProcessor => out.push("--cpu-moe".to_owned()),
            Experts::FirstLayers(layers) => {
                out.push("--n-cpu-moe".to_owned());
                out.push(layers.to_string());
            }
        }
        if self.ffn_layers_on_processor > 0 {
            out.push("--n-cpu-ffn".to_owned());
            out.push(self.ffn_layers_on_processor.to_string());
        }
        if let Some(devices) = &self.devices {
            out.push("--device".to_owned());
            out.push(devices.clone());
        }
        if let Some(overridden) = &self.override_tensors {
            out.push("--override-tensor".to_owned());
            out.push(overridden.clone());
        }
        out
    }

    #[must_use]
    pub fn from_value(value: &Value, recommended: &Self) -> Self {
        let number = |key: &str| value.get(key).and_then(Value::as_integer);
        Self {
            cache_on_processor: match value.get("cache_on_processor") {
                Some(Value::Bool(held)) => *held,
                _ => recommended.cache_on_processor,
            },
            split: value
                .get("split_mode")
                .and_then(Value::as_text)
                .and_then(Split::parse)
                .unwrap_or(recommended.split),
            experts: match value.get("experts") {
                Some(Value::Text(said)) if said == "all" => Experts::OnTheProcessor,
                Some(Value::Integer(layers)) => {
                    u32::try_from(*layers).map_or(Experts::WithTheModel, Experts::FirstLayers)
                }
                Some(Value::Null) => Experts::WithTheModel,
                _ => recommended.experts,
            },
            ffn_layers_on_processor: number("ffn_layers_on_processor")
                .and_then(|held| u32::try_from(held).ok())
                .unwrap_or(recommended.ffn_layers_on_processor),
            main_device: number("main_device")
                .and_then(|held| u32::try_from(held).ok())
                .unwrap_or(recommended.main_device),
            devices: match value.get("devices") {
                None => recommended.devices.clone(),
                Some(Value::Text(named)) if !named.is_empty() => Some(named.clone()),
                Some(_) => None,
            },
            override_tensors: match value.get("override_tensors") {
                None => recommended.override_tensors.clone(),
                Some(Value::Text(said)) if !said.is_empty() => Some(said.clone()),
                Some(_) => None,
            },
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Reuse {
    pub prompt_cache: bool,
    pub idle_slots: bool,
    pub context_shift: bool,
    pub prompt_cache_mib: i64,
    pub cache_reuse: u32,
    pub checkpoints: u32,
    pub checkpoint_min_step: u32,
    pub keep: i64,
}

impl Reuse {
    #[must_use]
    pub fn arguments(&self) -> Vec<String> {
        let mut out = Vec::new();
        out.push(
            if self.prompt_cache {
                "--cache-prompt"
            } else {
                "--no-cache-prompt"
            }
            .to_owned(),
        );
        out.push("--cache-ram".to_owned());
        out.push(self.prompt_cache_mib.to_string());
        out.push("--cache-reuse".to_owned());
        out.push(self.cache_reuse.to_string());
        out.push(
            if self.idle_slots {
                "--cache-idle-slots"
            } else {
                "--no-cache-idle-slots"
            }
            .to_owned(),
        );
        out.push(
            if self.context_shift {
                "--context-shift"
            } else {
                "--no-context-shift"
            }
            .to_owned(),
        );
        out.push("--ctx-checkpoints".to_owned());
        out.push(self.checkpoints.to_string());
        out.push("--checkpoint-min-step".to_owned());
        out.push(self.checkpoint_min_step.to_string());
        out.push("--keep".to_owned());
        out.push(self.keep.to_string());
        out
    }

    #[must_use]
    pub fn from_value(value: &Value, recommended: Self) -> Self {
        let number = |key: &str| value.get(key).and_then(Value::as_integer);
        let flag = |key: &str, fallback: bool| match value.get(key) {
            Some(Value::Bool(held)) => *held,
            _ => fallback,
        };
        Self {
            prompt_cache: flag("prompt_cache", recommended.prompt_cache),
            idle_slots: flag("idle_slots", recommended.idle_slots),
            context_shift: flag("context_shift", recommended.context_shift),
            prompt_cache_mib: number("prompt_cache_mib").unwrap_or(recommended.prompt_cache_mib),
            cache_reuse: number("cache_reuse")
                .and_then(|held| u32::try_from(held).ok())
                .unwrap_or(recommended.cache_reuse),
            checkpoints: number("checkpoints")
                .and_then(|held| u32::try_from(held).ok())
                .unwrap_or(recommended.checkpoints),
            checkpoint_min_step: number("checkpoint_min_step")
                .and_then(|held| u32::try_from(held).ok())
                .unwrap_or(recommended.checkpoint_min_step),
            keep: number("keep").unwrap_or(recommended.keep),
        }
    }
}

impl Default for Reuse {
    fn default() -> Self {
        Self {
            prompt_cache: true,
            idle_slots: true,
            context_shift: false,
            prompt_cache_mib: 8_192,
            cache_reuse: 256,
            checkpoints: 32,
            checkpoint_min_step: 8_192,
            keep: 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hosting {
    pub context: u64,
    pub gpu_layers: u32,
    pub engine: String,
    pub device: String,
    pub threads: u32,
    pub batch: u32,
    pub flash_attention: bool,
    pub answers: Answers,
    pub pooling: Pooling,
    pub alias: Option<String>,
    pub adapters: Vec<String>,
    pub loading: Loading,
    pub lazily: Lazily,
    pub ubatch: u32,
    pub threads_batch: u32,
    pub cache: CacheType,
    pub slots: u32,
    pub reuse: Reuse,
    pub spread: Spread,
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

fn merged(one: Value, two: Value) -> Value {
    match (one, two) {
        (Value::Map(mut into), Value::Map(from)) => {
            for (key, value) in from {
                let _replaced = into.insert(key, value);
            }
            Value::Map(into)
        }
        (one, _) => one,
    }
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
            answers: Answers::default(),
            pooling: Pooling::default(),
            alias: None,
            adapters: Vec::new(),
            loading: Loading::default(),
            lazily: Lazily::default(),
            ubatch: 512,
            threads_batch: cores
                .and_then(|cores| u32::try_from(cores).ok())
                .unwrap_or(4)
                .max(1),
            cache: CacheType::default(),
            slots: 1,
            reuse: Reuse::default(),
            spread: Spread::default(),
            keep_resident: false,
            port: DEFAULT_PORT,
            api_key: None,
            open: false,
            projector: projector.map(|path| path.display().to_string()),
            started: crate::declared::Started::default(),
            tensor_split: Vec::new(),
        }
    }

    /// The settings a file's own header argues for, on top of what the machine
    /// argues for. Each one is a measurement, and each says what it rests on.
    #[must_use]
    pub fn tuned_for(mut self, file: &mcf_standin::gguf::Model) -> Self {
        let architecture = file.architecture().map(str::to_owned);
        let under = |suffix: &str| {
            architecture.as_ref().and_then(|held| {
                file.get(&format!("{held}.{suffix}"))
                    .and_then(mcf_standin::gguf::Value::as_integer)
                    .and_then(|held| u64::try_from(held).ok())
            })
        };
        let sparse = under("expert_count").is_some_and(|count| count > 1);
        self.ubatch = if sparse { 1024 } else { 256 };
        self.batch = self.batch.max(self.ubatch);
        self.cache = CacheType::Q8_0;
        if under("nextn_predict_layers").is_some_and(|layers| layers > 0) {
            self.started.draft_head = true;
            self.started.drafted = Some(2);
            self.reuse.prompt_cache_mib = 0;
            self.reuse.checkpoints = 0;
        }
        self.started.architecture = architecture;
        self
    }

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
    pub fn arguments(
        &self,
        model: &str,
        bind: &str,
        key_file: Option<&std::path::Path>,
    ) -> Vec<String> {
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
        out.extend(self.spread.arguments());
        out.extend(self.reuse.arguments());
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
        match self.answers {
            Answers::Chat => {}
            Answers::Embeddings => out.push("--embeddings".to_owned()),
            Answers::Reranking => out.push("--rerank".to_owned()),
        }
        if self.pooling != Pooling::TheModels {
            out.push("--pooling".to_owned());
            out.push(self.pooling.as_str().to_owned());
        }
        out.push("--alias".to_owned());
        out.push(self.alias.clone().unwrap_or_else(|| {
            model
                .rsplit('/')
                .next()
                .unwrap_or(model)
                .trim_end_matches(".gguf")
                .to_owned()
        }));
        for adapter in &self.adapters {
            out.push("--lora".to_owned());
            out.push(adapter.clone());
        }
        out.push("--fit".to_owned());
        out.push("off".to_owned());
        out.push("--load-mode".to_owned());
        out.push(
            if self.keep_resident && self.loading == Loading::Auto {
                Loading::MappedAndLocked
            } else {
                self.loading
            }
            .as_str()
            .to_owned(),
        );
        out.push("--lazy-mode".to_owned());
        out.push(self.lazily.as_str().to_owned());
        out.push("--ubatch-size".to_owned());
        out.push(self.ubatch.min(self.batch).to_string());
        out.push("--threads-batch".to_owned());
        out.push(self.threads_batch.max(1).to_string());
        if let Some(key_file) = key_file {
            out.push("--api-key-file".to_owned());
            out.push(key_file.display().to_string());
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
                because: if self.answers.keeps_a_conversation() {
                    "how long a conversation it can hold. Every token of it costs \
                     memory on the device the model runs on, so MCF holds it at the \
                     largest window whose cache stays within the model's own size; \
                     --context sets it to anything that fits. Where more than one slot \
                     is asked for they share it, and `per conversation` is what each gets"
                } else {
                    "how long a passage it can read. This hold keeps nothing between \
                     requests, so the window is the size of one passage rather than a \
                     conversation that grows, and no cache is reserved against it"
                },
            },
            Setting {
                name: "where it runs",
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
                because: "which build of the engine MCF starts. Builds differ in what hardware \
                          they can compute on, so this decides what a card can be used for",
            },
            Setting {
                name: "device",
                value: self.device.clone(),
                recommended: against.device.clone(),
                because: "the card or processor this hold runs on, as the engine names it. \
                          Where a machine has more than one, this is the one the model is put \
                          on unless the split settings say otherwise",
            },
            Setting {
                name: "cards to spread over",
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
                name: "prompt batch",
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
                name: "cache in system memory",
                value: yes_no(self.spread.cache_on_processor),
                recommended: yes_no(against.spread.cache_on_processor),
                because: "hold the conversation in system memory rather than on the card. \
                          The card then has its whole pool for the weights, which is what \
                          fits a longer window on a machine whose card memory is the smaller \
                          half; reading it back costs time on every token",
            },
            Setting {
                name: "split mode",
                value: self.spread.split.as_str().to_owned(),
                recommended: against.spread.split.as_str().to_owned(),
                because: "how a model on more than one card is divided: by layer, by rows of \
                          each weight, by tensor, or not at all",
            },
            Setting {
                name: "experts",
                value: self.spread.experts.said(),
                recommended: against.spread.experts.said(),
                because: "where a mixture-of-experts model keeps its experts. Holding them in \
                          system memory fits a model on a card that could not otherwise take \
                          it, and the experts are the part least worth the card",
            },
            Setting {
                name: "dense layers on the processor",
                value: self.spread.ffn_layers_on_processor.to_string(),
                recommended: against.spread.ffn_layers_on_processor.to_string(),
                because: "how many of the first layers keep their dense feed-forward weights \
                          in system memory, for the same reason",
            },
            Setting {
                name: "main device",
                value: self.spread.main_device.to_string(),
                recommended: against.spread.main_device.to_string(),
                because: "which card holds the model where the split mode is none",
            },
            Setting {
                name: "devices",
                value: self
                    .spread
                    .devices
                    .clone()
                    .unwrap_or_else(|| "every one MCF found".to_owned()),
                recommended: against
                    .spread
                    .devices
                    .clone()
                    .unwrap_or_else(|| "every one MCF found".to_owned()),
                because: "which devices may be used at all, named as the engine names them",
            },
            Setting {
                name: "tensors placed by hand",
                value: self
                    .spread
                    .override_tensors
                    .clone()
                    .unwrap_or_else(|| "none".to_owned()),
                recommended: against
                    .spread
                    .override_tensors
                    .clone()
                    .unwrap_or_else(|| "none".to_owned()),
                because: "a pattern matching tensor names to the memory they are put in, for \
                          a placement none of the settings above expresses",
            },
            Setting {
                name: "prefix reuse",
                value: self.reuse.cache_reuse.to_string(),
                recommended: against.reuse.cache_reuse.to_string(),
                because: "the smallest run of tokens the engine will recover from what it \
                          already read, rather than reading the conversation again. The engine \
                          leaves this off; MCF asks for it, because a conversation that comes \
                          back after a gap is the case it exists for. Zero turns it off",
            },
            Setting {
                name: "prompt cache",
                value: yes_no(self.reuse.prompt_cache),
                recommended: yes_no(against.reuse.prompt_cache),
                because: "whether what was read for one message is kept for the next",
            },
            Setting {
                name: "prompt cache memory",
                value: match self.reuse.prompt_cache_mib {
                    -1 => "no limit".to_owned(),
                    0 => "none".to_owned(),
                    held => format!("{} MiB", grouped(held.unsigned_abs())),
                },
                recommended: match against.reuse.prompt_cache_mib {
                    -1 => "no limit".to_owned(),
                    0 => "none".to_owned(),
                    held => format!("{} MiB", grouped(held.unsigned_abs())),
                },
                because: "how much system memory the kept prompts may take. A conversation \
                          whose cache is larger than this does not fit in it, and comes back \
                          from a gap by being read again rather than restored",
            },
            Setting {
                name: "keep idle slots",
                value: yes_no(self.reuse.idle_slots),
                recommended: yes_no(against.reuse.idle_slots),
                because: "whether a conversation nobody is using is written to the prompt \
                          cache so its place is held while something else runs",
            },
            Setting {
                name: "context shift",
                value: yes_no(self.reuse.context_shift),
                recommended: yes_no(against.reuse.context_shift),
                because: "whether a conversation that fills the window carries on by dropping \
                          its oldest tokens, rather than stopping",
            },
            Setting {
                name: "checkpoints",
                value: self.reuse.checkpoints.to_string(),
                recommended: against.reuse.checkpoints.to_string(),
                because: "how many places in a conversation the engine can return to without \
                          reading from the start again",
            },
            Setting {
                name: "checkpoint spacing",
                value: format!(
                    "{} tokens",
                    grouped(u64::from(self.reuse.checkpoint_min_step))
                ),
                recommended: format!(
                    "{} tokens",
                    grouped(u64::from(against.reuse.checkpoint_min_step))
                ),
                because: "how far apart those places are put",
            },
            Setting {
                name: "tokens kept in front",
                value: match self.reuse.keep {
                    -1 => "all of it".to_owned(),
                    held => format!("{} tokens", grouped(held.unsigned_abs())),
                },
                recommended: match against.reuse.keep {
                    -1 => "all of it".to_owned(),
                    held => format!("{} tokens", grouped(held.unsigned_abs())),
                },
                because: "how much of the opening of a conversation survives a context shift",
            },
            Setting {
                name: "window per conversation",
                value: format!("{} tokens", grouped(self.per_conversation())),
                recommended: format!("{} tokens", grouped(against.per_conversation())),
                because: "the window one conversation actually gets: the whole of it on one \
                          slot, and its share where more were asked for",
            },
            Setting {
                name: "conversations at once",
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
                name: "memory lock",
                value: yes_no(self.keep_resident),
                recommended: yes_no(against.keep_resident),
                because: "keep the weights in memory rather than letting the system page them \
                          out to disk. It stops a long pause the first time a paged-out model \
                          is asked for, and it needs the memory to be free to begin with",
            },
            Setting {
                name: "answer kind",
                value: self.answers.as_str().to_owned(),
                recommended: against.answers.as_str().to_owned(),
                because: "what the endpoint serves. The same engine and the same file will \
                          answer chat, embeddings or reranking, and a model published for one \
                          of the last two is held by saying so",
            },
            Setting {
                name: "pooling",
                value: self.pooling.as_str().to_owned(),
                recommended: against.pooling.as_str().to_owned(),
                because: "how the vectors for a passage are reduced to one, where the hold \
                          answers embeddings",
            },
            Setting {
                name: "name callers use",
                value: self
                    .alias
                    .clone()
                    .unwrap_or_else(|| "its file, without the suffix".to_owned()),
                recommended: against
                    .alias
                    .clone()
                    .unwrap_or_else(|| "its file, without the suffix".to_owned()),
                because: "the name a client asks for. Without one a caller sees the path on \
                          this disk, which is not a name anybody chose",
            },
            Setting {
                name: "adapters",
                value: if self.adapters.is_empty() {
                    "none".to_owned()
                } else {
                    self.adapters.join(", ")
                },
                recommended: if against.adapters.is_empty() {
                    "none".to_owned()
                } else {
                    against.adapters.join(", ")
                },
                because: "low-rank adapters applied over the weights, each a file beside them",
            },
            Setting {
                name: "who sizes it",
                value: "MCF".to_owned(),
                recommended: "MCF".to_owned(),
                because: "the engine can adjust settings it was not given, to fit the devices \
                          it finds. MCF turns that off and plans the hold itself, so that what \
                          it printed is what ran; two fitters with no knowledge of each other \
                          is how a reported figure and a real one come apart",
            },
            Setting {
                name: "how it loads",
                value: self.loading.as_str().to_owned(),
                recommended: against.loading.as_str().to_owned(),
                because: "how the weights are read from the file: mapped, held in memory, \
                          both, read straight from the device past the page cache, or plainly. \
                          On a file of tens of gigabytes this is the difference between a fast \
                          first load and a slow one",
            },
            Setting {
                name: "large tensors",
                value: self.lazily.as_str().to_owned(),
                recommended: against.lazily.as_str().to_owned(),
                because: "whether the rows of a very large tensor are read as they are needed \
                          rather than all held: less memory for a slower first pass",
            },
            Setting {
                name: "micro-batch",
                value: self.ubatch.to_string(),
                recommended: against.ubatch.to_string(),
                because: "how much of a batch the engine actually computes in one pass. This \
                          is how fast a prompt is read and not how fast an answer is written: \
                          an answer is one token at a time whatever this is. The compute \
                          buffers are built for this rather than for the batch size, so it is \
                          a memory setting as much as a speed one, and lowering it is what to \
                          reach for when a hold is a little short of fitting",
            },
            Setting {
                name: "threads for reading a prompt",
                value: self.threads_batch.to_string(),
                recommended: against.threads_batch.to_string(),
                because: "how many processor threads read a prompt, which the engine counts \
                          separately from the threads that generate",
            },
            Setting {
                name: "thinking budget",
                value: self.started.thinking.map_or_else(
                    || "unrestricted".to_owned(),
                    |held| format!("{held} tokens"),
                ),
                recommended: against.started.thinking.map_or_else(
                    || "unrestricted".to_owned(),
                    |held| format!("{held} tokens"),
                ),
                because: "how many tokens a model may spend thinking before the engine closes \
                          the thinking off and makes it answer. Zero ends it at once. This is \
                          counted by the engine rather than asked of the model, so it holds \
                          whatever the model would rather do — but it needs a model whose \
                          template marks where thinking starts and ends, and does nothing for \
                          one that has no thinking at all",
            },
            Setting {
                name: "temperature",
                value: self
                    .started
                    .temperature
                    .map_or_else(|| "the engine's own".to_owned(), |held| held.to_string()),
                recommended: against
                    .started
                    .temperature
                    .map_or_else(|| "the engine's own".to_owned(), |held| held.to_string()),
                because: "how much the model is allowed to wander when it picks each token. \
                          Zero takes the likeliest every time, which is what makes two runs \
                          comparable; higher wanders further. This is the default for callers \
                          who name none of their own, and a caller that names one overrides it",
            },
            Setting {
                name: "top-p",
                value: self
                    .started
                    .top_p
                    .map_or_else(|| "the engine's own".to_owned(), |held| held.to_string()),
                recommended: against
                    .started
                    .top_p
                    .map_or_else(|| "the engine's own".to_owned(), |held| held.to_string()),
                because: "keep only the likeliest tokens whose chances add up to this much, and \
                          draw from those. It bounds how far the temperature can wander. Like \
                          the temperature, it is a default a caller can override",
            },
            Setting {
                name: "top-k",
                value: self
                    .started
                    .top_k
                    .map_or_else(|| "the engine's own".to_owned(), |held| held.to_string()),
                recommended: against
                    .started
                    .top_k
                    .map_or_else(|| "the engine's own".to_owned(), |held| held.to_string()),
                because: "keep only this many of the likeliest tokens and draw from those. \
                          Zero leaves the count unbounded and lets top-p decide alone",
            },
            Setting {
                name: "thinking level",
                value: self
                    .started
                    .effort
                    .clone()
                    .unwrap_or_else(|| "the model's own".to_owned()),
                recommended: against
                    .started
                    .effort
                    .clone()
                    .unwrap_or_else(|| "the model's own".to_owned()),
                because: "how hard to ask the model to think. This is a word MCF puts in the \
                          prompt through the model's own template, so it is a request rather \
                          than a cap the engine enforces, and the words on offer are the ones \
                          that model's template accepts",
            },
            Setting {
                name: "draft depth",
                value: self.started.drafted.map_or_else(
                    || "the engine's own".to_owned(),
                    |held| format!("{held} tokens"),
                ),
                recommended: against.started.drafted.map_or_else(
                    || "the engine's own".to_owned(),
                    |held| format!("{held} tokens"),
                ),
                because: "how many tokens the draft head guesses ahead before the model checks \
                          them. Guessing further wins more when the guesses are right and costs \
                          more when they are wrong",
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
        let hold = Value::map([
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
        ]);
        let grouped = Value::map([
            ("prompt_cache", Value::Bool(self.reuse.prompt_cache)),
            (
                "prompt_cache_mib",
                Value::Integer(self.reuse.prompt_cache_mib),
            ),
            (
                "cache_reuse",
                Value::Integer(i64::from(self.reuse.cache_reuse)),
            ),
            ("idle_slots", Value::Bool(self.reuse.idle_slots)),
            ("context_shift", Value::Bool(self.reuse.context_shift)),
            (
                "checkpoints",
                Value::Integer(i64::from(self.reuse.checkpoints)),
            ),
            (
                "checkpoint_min_step",
                Value::Integer(i64::from(self.reuse.checkpoint_min_step)),
            ),
            ("keep", Value::Integer(self.reuse.keep)),
            (
                "cache_on_processor",
                Value::Bool(self.spread.cache_on_processor),
            ),
            ("split_mode", Value::text(self.spread.split.as_str())),
            (
                "experts",
                match self.spread.experts {
                    Experts::WithTheModel => Value::Null,
                    Experts::OnTheProcessor => Value::text("all"),
                    Experts::FirstLayers(layers) => Value::Integer(i64::from(layers)),
                },
            ),
            (
                "ffn_layers_on_processor",
                Value::Integer(i64::from(self.spread.ffn_layers_on_processor)),
            ),
            (
                "main_device",
                Value::Integer(i64::from(self.spread.main_device)),
            ),
            (
                "devices",
                self.spread.devices.clone().map_or(Value::Null, Value::text),
            ),
            (
                "override_tensors",
                self.spread
                    .override_tensors
                    .clone()
                    .map_or(Value::Null, Value::text),
            ),
        ]);
        merged(hold, merged(grouped, self.tail_of_to_value()))
    }

    #[must_use]
    fn tail_of_to_value(&self) -> Value {
        Value::map([
            ("keep_resident", Value::Bool(self.keep_resident)),
            ("answers", Value::text(self.answers.as_str())),
            ("pooling", Value::text(self.pooling.as_str())),
            ("alias", self.alias.clone().map_or(Value::Null, Value::text)),
            (
                "adapters",
                Value::List(self.adapters.iter().cloned().map(Value::text).collect()),
            ),
            ("loading", Value::text(self.loading.as_str())),
            ("lazily", Value::text(self.lazily.as_str())),
            ("ubatch", Value::Integer(i64::from(self.ubatch))),
            (
                "threads_batch",
                Value::Integer(i64::from(self.threads_batch)),
            ),
            ("open", Value::Bool(self.open)),
            ("port", Value::Integer(i64::from(self.port))),
            ("api_key_set", Value::Bool(self.api_key.is_some())),
            (
                "projector",
                self.projector.clone().map_or(Value::Null, Value::text),
            ),
            ("draft_head", Value::Bool(self.started.draft_head)),
            (
                "drafted",
                self.started
                    .drafted
                    .map_or(Value::Null, |held| Value::Integer(i64::from(held))),
            ),
            (
                "trained",
                self.started.trained.map_or(Value::Null, |held| {
                    Value::Integer(i64::try_from(held).unwrap_or(i64::MAX))
                }),
            ),
            (
                "lift",
                self.started.lift.map_or(Value::Null, |held| {
                    Value::Integer(i64::try_from(held).unwrap_or(i64::MAX))
                }),
            ),
            (
                "architecture",
                self.started
                    .architecture
                    .clone()
                    .map_or(Value::Null, Value::text),
            ),
            (
                "thinking",
                self.started
                    .thinking
                    .map_or(Value::Null, |held| Value::Integer(i64::from(held))),
            ),
            (
                "effort",
                self.started.effort.clone().map_or(Value::Null, Value::text),
            ),
            (
                "temperature",
                self.started
                    .temperature
                    .map_or(Value::Null, |held| Value::Integer(i64::from(held.0))),
            ),
            (
                "top_p",
                self.started
                    .top_p
                    .map_or(Value::Null, |held| Value::Integer(i64::from(held.0))),
            ),
            (
                "top_k",
                self.started
                    .top_k
                    .map_or(Value::Null, |held| Value::Integer(i64::from(held))),
            ),
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
            reuse: Reuse::from_value(value, recommended.reuse),
            spread: Spread::from_value(value, &recommended.spread),
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
            answers: value
                .get("answers")
                .and_then(Value::as_text)
                .and_then(Answers::parse)
                .unwrap_or(recommended.answers),
            pooling: value
                .get("pooling")
                .and_then(Value::as_text)
                .and_then(Pooling::parse)
                .unwrap_or(recommended.pooling),
            alias: match value.get("alias") {
                None => recommended.alias.clone(),
                Some(Value::Text(named)) if !named.is_empty() => Some(named.clone()),
                Some(_) => None,
            },
            adapters: match value.get("adapters").and_then(Value::as_list) {
                Some(listed) => listed
                    .iter()
                    .filter_map(Value::as_text)
                    .map(str::to_owned)
                    .collect(),
                None => recommended.adapters.clone(),
            },
            loading: value
                .get("loading")
                .and_then(Value::as_text)
                .and_then(Loading::parse)
                .unwrap_or(recommended.loading),
            lazily: value
                .get("lazily")
                .and_then(Value::as_text)
                .and_then(Lazily::parse)
                .unwrap_or(recommended.lazily),
            ubatch: number("ubatch")
                .and_then(|held| u32::try_from(held).ok())
                .unwrap_or(recommended.ubatch)
                .max(1),
            threads_batch: number("threads_batch")
                .and_then(|held| u32::try_from(held).ok())
                .unwrap_or(recommended.threads_batch)
                .max(1),
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
