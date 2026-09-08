use mcf_standin::gguf::{Model, Value};

const EXPECTED: [(&str, &str); 6] = [
    ("block_count", "blocks"),
    ("embedding_length", "embedding width"),
    ("attention.head_count", "attention heads"),
    ("attention.head_count_kv", "key/value heads"),
    ("context_length", "context length"),
    ("feed_forward_length", "feed-forward width"),
];

const FILE_TYPES: [(i64, &str); 36] = [
    (0, "F32"),
    (1, "F16"),
    (2, "Q4_0"),
    (3, "Q4_1"),
    (7, "Q8_0"),
    (8, "Q5_0"),
    (9, "Q5_1"),
    (10, "Q2_K"),
    (11, "Q3_K_S"),
    (12, "Q3_K_M"),
    (13, "Q3_K_L"),
    (14, "Q4_K_S"),
    (15, "Q4_K_M"),
    (16, "Q5_K_S"),
    (17, "Q5_K_M"),
    (18, "Q6_K"),
    (19, "IQ2_XXS"),
    (20, "IQ2_XS"),
    (21, "Q2_K_S"),
    (22, "IQ3_XS"),
    (23, "IQ3_XXS"),
    (24, "IQ1_S"),
    (25, "IQ4_NL"),
    (26, "IQ3_S"),
    (27, "IQ3_M"),
    (28, "IQ2_S"),
    (29, "IQ2_M"),
    (30, "IQ4_XS"),
    (31, "IQ1_M"),
    (32, "BF16"),
    (33, "Q4_0_4_4"),
    (34, "Q4_0_4_8"),
    (35, "Q4_0_8_8"),
    (36, "TQ1_0"),
    (37, "TQ2_0"),
    (38, "MXFP4"),
];

pub(crate) fn declared(file: &Model) -> Vec<(&'static str, String)> {
    let architecture = file.architecture().unwrap_or("unstated").to_owned();
    let under = |suffix: &str| file.get(&format!("{architecture}.{suffix}")).map(shown);
    let general = |suffix: &str| file.get(&format!("general.{suffix}")).map(shown);
    let mut rows = vec![
        ("format version", file.version.to_string()),
        ("architecture", architecture.clone()),
    ];
    for (key, label) in EXPECTED {
        rows.push((
            label,
            under(key).unwrap_or_else(|| "the file does not say".to_owned()),
        ));
    }
    rows.push((
        "vocabulary",
        match file.get("tokenizer.ggml.tokens") {
            Some(Value::List(tokens)) => format!("{} tokens", tokens.len()),
            _ => "the file does not say".to_owned(),
        },
    ));
    rows.extend(shape(file, &architecture));
    let mut named = |label, held: Option<String>| rows.extend(held.map(|held| (label, held)));
    named("size label", general("size_label"));
    named("parameter count", general("parameter_count"));
    named(
        "quantized as",
        file.get("general.file_type")
            .and_then(Value::as_integer)
            .map(|number| {
                FILE_TYPES
                    .iter()
                    .find(|(held, _)| *held == number)
                    .map_or_else(
                        || format!("file type {number}, which MCF does not name"),
                        |(_, name)| format!("{name} (file type {number})"),
                    )
            }),
    );
    named("quantized by", general("quantized_by"));
    named("importance matrix", imatrix(file));
    named("base model", base_model(file));
    named(
        "publisher's name for it",
        Some(general("name").unwrap_or_else(|| "unstated".to_owned())),
    );
    rows
}

fn shape(file: &Model, architecture: &str) -> Vec<(&'static str, String)> {
    let under = |suffix: &str| file.get(&format!("{architecture}.{suffix}")).map(shown);
    let mut rows = Vec::new();
    if let Some(key) = under("attention.key_length") {
        let value = under("attention.value_length").unwrap_or_else(|| key.clone());
        rows.push((
            "head width",
            if key == value {
                key
            } else {
                format!("{key} for keys, {value} for values")
            },
        ));
    }
    if let Some(count) = under("expert_count") {
        let used = under("expert_used_count").map_or_else(
            || "the file does not say how many a token visits".to_owned(),
            |used| format!("{used} visited per token"),
        );
        let shared = under("expert_shared_count")
            .filter(|held| held != "0")
            .map_or_else(String::new, |shared| {
                format!(", {shared} shared by every token")
            });
        rows.push(("experts", format!("{count}, {used}{shared}")));
    }
    let mut named = |label, held: Option<String>| rows.extend(held.map(|held| (label, held)));
    named(
        "expert feed-forward width",
        under("expert_feed_forward_length"),
    );
    named(
        "dense blocks before the experts",
        under("leading_dense_block_count"),
    );
    named("sliding window", under("attention.sliding_window"));
    named(
        "latent attention ranks",
        under("attention.kv_lora_rank").map(|kv| {
            under("attention.q_lora_rank").map_or_else(
                || format!("{kv} for keys and values"),
                |q| format!("{q} for queries, {kv} for keys and values"),
            )
        }),
    );
    named("rope base frequency", under("rope.freq_base"));
    named("rope dimensions", under("rope.dimension_count"));
    named(
        "rope scaling",
        under("rope.scaling.type").map(|kind| {
            let factor = under("rope.scaling.factor")
                .map_or_else(String::new, |factor| format!(" ×{factor}"));
            let from = under("rope.scaling.original_context_length")
                .map_or_else(String::new, |from| format!(" from a trained {from}"));
            format!("{kind}{factor}{from}")
        }),
    );
    named("norm epsilon", under("attention.layer_norm_rms_epsilon"));
    rows
}

fn imatrix(file: &Model) -> Option<String> {
    let dataset = file
        .get("quantize.imatrix.dataset")
        .and_then(Value::as_text)?;
    let chunks = file
        .get("quantize.imatrix.chunks_count")
        .and_then(Value::as_integer)
        .map_or_else(String::new, |chunks| format!(", {chunks} chunks"));
    Some(format!("{dataset}{chunks}"))
}

fn base_model(file: &Model) -> Option<String> {
    let text = |key: &str| {
        file.get(&format!("general.base_model.0.{key}"))
            .and_then(Value::as_text)
    };
    let name = text("name")?;
    Some(match text("organization") {
        Some(organization) => format!("{name}, by {organization}"),
        None => name.to_owned(),
    })
}

fn shown(value: &Value) -> String {
    match value {
        Value::Integer(number) => number.to_string(),
        Value::Text(text) => text.clone(),
        Value::Bool(flag) => if *flag { "yes" } else { "no" }.to_owned(),
        Value::Float(held) if held.abs() < 0.001 && *held != 0.0 => format!("{held:e}"),
        Value::Float(held) => format!("{held}"),
        Value::List(items) => format!("a list of {}", items.len()),
        other => format!("{other:?}"),
    }
}
