use mcf_record::json::Value;
use mcf_standin::anatomy::blocks::{Feed, Mixing, ranges};
use mcf_standin::anatomy::vocabulary::{self, Vocabulary};
use mcf_standin::anatomy::work::Cache;
use mcf_standin::anatomy::{self, Agreement, Share};
use mcf_standin::gguf::Model;

fn count(held: u64) -> Value {
    Value::Integer(i64::try_from(held).unwrap_or(i64::MAX))
}

fn maybe(held: Option<u64>) -> Value {
    held.map_or(Value::Null, count)
}

fn share(held: &Share) -> Vec<(&'static str, Value)> {
    vec![
        ("tensors", count(held.tensors)),
        ("elements", count(held.elements)),
        ("bytes", maybe(held.bytes)),
    ]
}

fn agreement(held: &Agreement) -> Value {
    Value::map([
        ("what", Value::text(held.what)),
        (
            "declared",
            held.declared
                .as_deref()
                .map_or(Value::Null, |text| Value::text(text.to_owned())),
        ),
        (
            "observed",
            held.observed
                .as_deref()
                .map_or(Value::Null, |text| Value::text(text.to_owned())),
        ),
        ("agrees", held.agrees.map_or(Value::Null, Value::Bool)),
    ])
}

fn cache(held: &Cache) -> Value {
    match held {
        Cache::Sized {
            per_token,
            key_heads,
            per_head,
            latent,
            at_context,
            sliding_window,
            attending,
            recurrent,
        } => Value::map([
            ("sized", Value::Bool(true)),
            ("per_token", count(*per_token)),
            ("key_heads", count(*key_heads)),
            ("per_head", count(*per_head)),
            ("latent", Value::Bool(*latent)),
            ("kept", Value::text(held.kept().unwrap_or_default())),
            ("context", maybe(at_context.map(|(tokens, _)| tokens))),
            ("at_context", maybe(at_context.map(|(_, bytes)| bytes))),
            ("sliding_window", maybe(*sliding_window)),
            ("attending", count(attending.0)),
            ("blocks", count(attending.1)),
            ("recurrent", count(*recurrent)),
        ]),
        Cache::Unsized(why) => Value::map([
            ("sized", Value::Bool(false)),
            ("why", Value::text((*why).to_owned())),
        ]),
    }
}

#[must_use]
pub fn encode(named: &str, model: &Model) -> Value {
    let body = anatomy::of(model);
    let work = anatomy::work::of(model, &body);
    let mut counted = vec![
        ("elements", count(body.elements)),
        ("bytes", maybe(body.bytes)),
        ("unsized_tensors", count(body.unsized_tensors)),
        ("blocks", count(body.blocks)),
        ("output_tied", Value::Bool(body.output_tied)),
        (
            "active",
            body.active.as_ref().map_or(Value::Null, |active| {
                Value::map([
                    ("elements", count(active.elements)),
                    ("experts", count(active.experts)),
                    ("used", count(active.used)),
                ])
            }),
        ),
    ];
    counted.push((
        "parts",
        Value::List(
            body.roles
                .iter()
                .map(|(role, held)| {
                    let mut fields = vec![("part", Value::text(role.as_str()))];
                    fields.extend(share(held));
                    Value::map(fields)
                })
                .collect(),
        ),
    ));
    counted.push((
        "encodings",
        Value::List(
            body.kinds
                .iter()
                .map(|(kind, held)| {
                    let mut fields = vec![("encoding", Value::text(kind.to_string()))];
                    fields.extend(share(held));
                    Value::map(fields)
                })
                .collect(),
        ),
    ));
    counted.push((
        "block_shapes",
        Value::List(body.census.families.iter().map(family).collect()),
    ));
    counted.push(("attending", count(body.census.attending)));
    counted.push(("recurrent", count(body.census.recurrent)));
    Value::map([
        ("model", Value::text(named.to_owned())),
        ("counted", Value::map(counted)),
        (
            "agreements",
            Value::List(body.agreements.iter().map(agreement).collect()),
        ),
        (
            "work",
            Value::map([
                ("multiply_adds", count(work.multiply_adds)),
                ("head_width", maybe(work.head_width)),
                ("queries_per_key", maybe(work.queries_per_key)),
                ("attention_at_context", maybe(work.attention_at_context)),
                ("cache", cache(&work.cache)),
            ]),
        ),
        ("vocabulary", spoken(&vocabulary::of(model))),
    ])
}

fn spoken(held: &Vocabulary) -> Value {
    let text = |held: &str| Value::text(held.to_owned());
    let named: Vec<Value> = held
        .named
        .iter()
        .map(|named| {
            Value::map([
                ("what", text(named.what)),
                ("identifier", Value::Integer(named.identifier)),
                (
                    "spelled",
                    named.spelled.as_deref().map_or(Value::Null, text),
                ),
                (
                    "beyond",
                    held.beyond(named).as_deref().map_or(Value::Null, text),
                ),
            ])
        })
        .collect();
    let template = held.template.as_ref().map_or(Value::Null, |template| {
        Value::map([
            ("bytes", count(template.bytes)),
            (
                "mentions",
                Value::List(template.mentions.iter().map(|name| text(name)).collect()),
            ),
            (
                "markers",
                template.markers.as_ref().map_or(Value::Null, |markers| {
                    Value::List(markers.iter().map(|marker| text(marker)).collect())
                }),
            ),
            (
                "no_markers",
                template.no_markers().map_or(Value::Null, text),
            ),
        ])
    });
    Value::map([
        ("tokens", count(held.tokens)),
        ("segmentation", Value::text(held.segmentation())),
        ("merges", maybe(held.merges)),
        (
            "kinds",
            held.kinds.as_ref().map_or(Value::Null, |kinds| {
                Value::List(
                    kinds
                        .iter()
                        .map(|(kind, held)| {
                            Value::map([
                                ("kind", Value::text(kind.as_str())),
                                ("count", count(*held)),
                            ])
                        })
                        .collect(),
                )
            }),
        ),
        ("word_starts", count(held.word_starts)),
        ("digit_tokens", count(held.digit_tokens.0)),
        ("longest_digits", count(held.digit_tokens.1)),
        ("digits", Value::text(held.digits_said())),
        (
            "longest",
            held.longest.as_ref().map_or(Value::Null, |(token, bytes)| {
                Value::map([("token", text(token)), ("bytes", count(*bytes))])
            }),
        ),
        ("named", Value::List(named)),
        (
            "adds_beginning",
            held.adds_beginning.map_or(Value::Null, Value::Bool),
        ),
        ("beginning", text(held.beginning_said())),
        ("template", template),
        ("no_template", text(vocabulary::NO_TEMPLATE)),
    ])
}

fn family(held: &anatomy::blocks::Family) -> Value {
    let mut fields = vec![
        (
            "blocks",
            Value::List(held.blocks.iter().map(|block| count(*block)).collect()),
        ),
        (
            "mixing",
            Value::text(match held.shape.mixing {
                Mixing::Attention => "attention",
                Mixing::Recurrent => "recurrent",
                Mixing::Nothing => "none",
            }),
        ),
        (
            "feed",
            match held.shape.feed {
                Feed::Dense => Value::text("dense"),
                Feed::Experts { .. } => Value::text("experts"),
                Feed::Nothing => Value::text("none"),
            },
        ),
        (
            "experts",
            match held.shape.feed {
                Feed::Experts { count: held, .. } => count(held),
                Feed::Dense | Feed::Nothing => Value::Null,
            },
        ),
        (
            "shared_expert",
            Value::Bool(matches!(
                held.shape.feed,
                Feed::Experts { shared: true, .. }
            )),
        ),
        ("said", Value::text(held.shape.said())),
        ("ranged", Value::text(ranges(&held.blocks, 6))),
        (
            "bits_hundredths",
            held.bits.map_or(Value::Null, |(least, most)| {
                Value::List(vec![count(least), count(most)])
            }),
        ),
    ];
    fields.extend(share(&held.share));
    Value::map(fields)
}

fn read_count(held: &Value, key: &str) -> Option<u64> {
    held.get(key)
        .and_then(Value::as_integer)
        .and_then(|held| u64::try_from(held).ok())
}

fn read_text(held: &Value, key: &str) -> Option<String> {
    held.get(key).and_then(Value::as_text).map(str::to_owned)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SaidShare {
    pub name: String,
    pub tensors: u64,
    pub elements: u64,
    pub bytes: Option<u64>,
}

impl SaidShare {
    fn read(held: &Value, name: &str) -> Option<Self> {
        Some(Self {
            name: read_text(held, name)?,
            tensors: read_count(held, "tensors")?,
            elements: read_count(held, "elements")?,
            bytes: read_count(held, "bytes"),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SaidFamily {
    pub blocks: Vec<u64>,
    pub said: String,
    pub ranged: String,
    pub share: SaidShare,
    pub bits: Option<(u64, u64)>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SaidAgreement {
    pub what: String,
    pub declared: Option<String>,
    pub observed: Option<String>,
    pub agrees: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SaidCache {
    Sized {
        per_token: u64,
        key_heads: u64,
        kept: String,
        at_context: Option<(u64, u64)>,
        sliding_window: Option<u64>,
        attending: (u64, u64),
        recurrent: u64,
    },
    Unsized(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SaidNamed {
    pub what: String,
    pub identifier: i64,
    pub spelled: Option<String>,
    pub beyond: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SaidTemplate {
    pub bytes: u64,
    pub mentions: Vec<String>,
    pub markers: Result<Vec<String>, String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SaidVocabulary {
    pub tokens: u64,
    pub segmentation: String,
    pub merges: Option<u64>,
    pub kinds: Option<Vec<(String, u64)>>,
    pub word_starts: u64,
    pub digits: String,
    pub longest: Option<(String, u64)>,
    pub named: Vec<SaidNamed>,
    pub beginning: String,
    pub template: Result<SaidTemplate, String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Said {
    pub model: String,
    pub elements: u64,
    pub bytes: Option<u64>,
    pub unsized_tensors: u64,
    pub blocks: u64,
    pub output_tied: bool,
    pub active: Option<(u64, u64, u64)>,
    pub parts: Vec<SaidShare>,
    pub encodings: Vec<SaidShare>,
    pub families: Vec<SaidFamily>,
    pub attending: u64,
    pub recurrent: u64,
    pub agreements: Vec<SaidAgreement>,
    pub multiply_adds: u64,
    pub head_width: Option<u64>,
    pub queries_per_key: Option<u64>,
    pub attention_at_context: Option<u64>,
    pub cache: SaidCache,
    pub vocabulary: SaidVocabulary,
}

impl Said {
    #[must_use]
    pub fn from_value(held: &Value) -> Option<Self> {
        let counted = held.get("counted")?;
        let work = held.get("work")?;
        let shares = |key: &str, name: &str| -> Option<Vec<SaidShare>> {
            counted
                .get(key)?
                .as_list()?
                .iter()
                .map(|each| SaidShare::read(each, name))
                .collect()
        };
        Some(Self {
            model: read_text(held, "model")?,
            elements: read_count(counted, "elements")?,
            bytes: read_count(counted, "bytes"),
            unsized_tensors: read_count(counted, "unsized_tensors")?,
            blocks: read_count(counted, "blocks")?,
            output_tied: counted.get("output_tied") == Some(&Value::Bool(true)),
            active: counted.get("active").and_then(|active| {
                Some((
                    read_count(active, "elements")?,
                    read_count(active, "experts")?,
                    read_count(active, "used")?,
                ))
            }),
            parts: shares("parts", "part")?,
            encodings: shares("encodings", "encoding")?,
            families: counted
                .get("block_shapes")?
                .as_list()?
                .iter()
                .map(read_family)
                .collect::<Option<Vec<_>>>()?,
            attending: read_count(counted, "attending")?,
            recurrent: read_count(counted, "recurrent")?,
            agreements: held
                .get("agreements")?
                .as_list()?
                .iter()
                .map(|each| {
                    Some(SaidAgreement {
                        what: read_text(each, "what")?,
                        declared: read_text(each, "declared"),
                        observed: read_text(each, "observed"),
                        agrees: match each.get("agrees") {
                            Some(Value::Bool(agrees)) => Some(*agrees),
                            _ => None,
                        },
                    })
                })
                .collect::<Option<Vec<_>>>()?,
            multiply_adds: read_count(work, "multiply_adds")?,
            head_width: read_count(work, "head_width"),
            queries_per_key: read_count(work, "queries_per_key"),
            attention_at_context: read_count(work, "attention_at_context"),
            cache: read_cache(work.get("cache")?)?,
            vocabulary: read_vocabulary(held.get("vocabulary")?)?,
        })
    }
}

fn read_vocabulary(held: &Value) -> Option<SaidVocabulary> {
    let texts = |held: &Value, key: &str| -> Option<Vec<String>> {
        held.get(key)?
            .as_list()?
            .iter()
            .map(|each| each.as_text().map(str::to_owned))
            .collect()
    };
    let template = match held.get("template") {
        Some(Value::Null) | None => Err(read_text(held, "no_template")?),
        Some(template) => Ok(SaidTemplate {
            bytes: read_count(template, "bytes")?,
            mentions: texts(template, "mentions")?,
            markers: match read_text(template, "no_markers") {
                Some(why) => Err(why),
                None => Ok(texts(template, "markers")?),
            },
        }),
    };
    Some(SaidVocabulary {
        tokens: read_count(held, "tokens")?,
        segmentation: read_text(held, "segmentation")?,
        merges: read_count(held, "merges"),
        kinds: held.get("kinds").and_then(|kinds| {
            kinds
                .as_list()?
                .iter()
                .map(|each| Some((read_text(each, "kind")?, read_count(each, "count")?)))
                .collect()
        }),
        word_starts: read_count(held, "word_starts")?,
        digits: read_text(held, "digits")?,
        longest: held.get("longest").and_then(|longest| {
            Some((read_text(longest, "token")?, read_count(longest, "bytes")?))
        }),
        named: held
            .get("named")?
            .as_list()?
            .iter()
            .map(|each| {
                Some(SaidNamed {
                    what: read_text(each, "what")?,
                    identifier: each.get("identifier")?.as_integer()?,
                    spelled: read_text(each, "spelled"),
                    beyond: read_text(each, "beyond"),
                })
            })
            .collect::<Option<Vec<_>>>()?,
        beginning: read_text(held, "beginning")?,
        template,
    })
}

fn read_family(held: &Value) -> Option<SaidFamily> {
    let counts = |key: &str| -> Option<Vec<u64>> {
        held.get(key)?
            .as_list()?
            .iter()
            .map(|each| each.as_integer().and_then(|held| u64::try_from(held).ok()))
            .collect()
    };
    Some(SaidFamily {
        blocks: counts("blocks")?,
        said: read_text(held, "said")?,
        ranged: read_text(held, "ranged")?,
        share: SaidShare::read(held, "said")?,
        bits: counts("bits_hundredths").and_then(|held| match held.as_slice() {
            [least, most] => Some((*least, *most)),
            _ => None,
        }),
    })
}

fn read_cache(held: &Value) -> Option<SaidCache> {
    if held.get("sized") != Some(&Value::Bool(true)) {
        return Some(SaidCache::Unsized(read_text(held, "why")?));
    }
    Some(SaidCache::Sized {
        per_token: read_count(held, "per_token")?,
        key_heads: read_count(held, "key_heads")?,
        kept: read_text(held, "kept")?,
        at_context: read_count(held, "context").zip(read_count(held, "at_context")),
        sliding_window: read_count(held, "sliding_window"),
        attending: (read_count(held, "attending")?, read_count(held, "blocks")?),
        recurrent: read_count(held, "recurrent")?,
    })
}

#[cfg(test)]
mod tests;
