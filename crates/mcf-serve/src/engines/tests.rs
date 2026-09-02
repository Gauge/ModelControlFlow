use super::*;

/// A device line from a real engine parses into a device.
#[test]
fn a_reported_device_parses() {
    let device =
        parse_device("CUDA0: NVIDIA GeForce RTX 5080 (15877 MiB, 13773 MiB free)").expect("parses");
    assert_eq!(device.kind, Kind::Gpu);
    assert_eq!(device.name, "NVIDIA GeForce RTX 5080");
    assert_eq!(device.free, Some(13_773 << 20));
}

/// A line with no free-memory figure gives a device with unknown memory, not
/// a device with none (A7).
#[test]
fn a_device_that_does_not_say_its_memory_is_unknown_not_zero() {
    let device = parse_device("CUDA0: Some Card").expect("parses");
    assert_eq!(device.free, None, "unknown memory must not become zero");
}

/// The window is always a power of two, never above what the model was
/// trained for, and never larger than the memory holds.
#[test]
fn the_window_is_a_power_of_two_within_both_limits() {
    let weights = 5_000_000_000;
    let per_token = 114_688;
    let trained = 40_960;
    for free in [8_u64, 16, 32, 64, 128].map(|gb| gb * 1_000_000_000) {
        let context = largest_context(weights, per_token, free, trained);
        if context == 0 {
            continue;
        }
        assert!(
            context.is_power_of_two(),
            "{context} is not a power of two at {free} free"
        );
        assert!(
            context <= trained,
            "{context} exceeds the trained {trained}"
        );
        let needed = weights + OVERHEAD + context * per_token;
        #[allow(clippy::integer_division, reason = "the same arithmetic the code does")]
        let allowed = free * HEADROOM_NUMERATOR / HEADROOM_DENOMINATOR;
        assert!(
            needed <= allowed,
            "{context} needs {needed} of {allowed} allowed"
        );
    }
}

/// A device that cannot hold the weights gets no window at all.
#[test]
fn weights_that_do_not_fit_give_no_window() {
    assert_eq!(
        largest_context(20_000_000_000, 114_688, 2_000_000_000, 40_960),
        0
    );
}

/// An architecture with no growing cache is bounded only by what it was
/// trained for.
#[test]
fn no_cache_means_the_model_is_the_only_limit() {
    assert_eq!(
        largest_context(300_000_000, 0, 90_000_000_000, 4_096),
        4_096
    );
}

/// The card wins a tie, because it is the same window and will be quicker —
/// the only thing MCF can say about speed before measuring any.
#[test]
fn a_tie_goes_to_the_card() {
    let engine = Engine {
        name: "an engine".to_owned(),
        prefix: PathBuf::from("/nowhere"),
        commit: "abc".to_owned(),
    };
    let plenty = 90_000_000_000;
    let devices = vec![
        Device {
            kind: Kind::Cpu,
            name: "CPU".to_owned(),
            free: Some(plenty),
        },
        Device {
            kind: Kind::Gpu,
            name: "A Card".to_owned(),
            free: Some(plenty),
        },
    ];
    let choice =
        resolve(&[(engine, devices)], 500_000_000, Some(22_528), 8_192).expect("something runs it");
    assert_eq!(choice.device.kind, Kind::Gpu);
    assert_eq!(choice.context, 8_192, "both could hold the trained context");
}

/// The larger window wins over the faster device, because capacity is
/// arithmetic MCF always has and speed is a measurement it may not.
#[test]
fn the_larger_window_beats_the_card() {
    let engine = |name: &str| Engine {
        name: name.to_owned(),
        prefix: PathBuf::from("/nowhere"),
        commit: "abc".to_owned(),
    };
    let engines = vec![
        (
            engine("the processor one"),
            vec![Device {
                kind: Kind::Cpu,
                name: "CPU".to_owned(),
                free: Some(90_000_000_000),
            }],
        ),
        (
            engine("the card one"),
            vec![Device {
                kind: Kind::Gpu,
                name: "A Small Card".to_owned(),
                free: Some(6_000_000_000),
            }],
        ),
    ];
    let choice = resolve(&engines, 1_000_000_000, Some(114_688), 40_960).expect("runs");
    assert_eq!(choice.device.kind, Kind::Cpu);
    assert_eq!(choice.engine, "the processor one");
}

/// Nothing provisioned is its own refusal, and it offers the way out.
#[test]
fn no_engine_says_what_to_do() {
    let refusal = resolve(&[], 1, Some(1), 1).expect_err("nothing runs it");
    assert_eq!(refusal, Refused::NoEngine);
    assert!(refusal.says().contains("build one"), "{}", refusal.says());
}

/// A model too big for every device says how big and how much there is.
#[test]
fn a_model_that_does_not_fit_says_by_how_much() {
    let engines = vec![(
        Engine {
            name: "an engine".to_owned(),
            prefix: PathBuf::from("/nowhere"),
            commit: "abc".to_owned(),
        },
        vec![Device {
            kind: Kind::Cpu,
            name: "CPU".to_owned(),
            free: Some(2_000_000_000),
        }],
    )];
    let refusal = resolve(&engines, 20_000_000_000, Some(114_688), 40_960).expect_err("too big");
    let said = refusal.says();
    // **What it needs, which is more than what it weighs.** Twenty gigabytes
    // of weights, the engine's overhead beside them, and the smallest window
    // worth opening: about thirty. The figure a reader needs is what running
    // it costs, and quoting the weights alone was the shape of the error that
    // had MCF proposing windows the engine could not hold (F144).
    assert!(said.contains("30.0"), "{said}");
    assert!(said.contains("2.00 GB"), "{said}");
    assert!(
        said.contains("85%"),
        "the refusal names two numbers that do not entail it without the fraction: {said}"
    );
}

/// Every refusal is written for a person: no rule identifiers, no clause
/// numbers, nothing that sends somebody to a document they have never seen.
#[test]
fn no_refusal_cites_a_document() {
    let refusals = [
        Refused::NoEngine,
        Refused::DoesNotFit {
            largest_device: 1,
            needs: 2,
        },
        Refused::HeaderIncomplete,
    ];
    for refusal in refusals {
        let said = refusal.says();
        assert!(!said.contains('§'), "cites a clause: {said}");
        for word in said.split(|c: char| !c.is_ascii_alphanumeric()) {
            let looks_like_a_rule = (2..=5).contains(&word.len())
                && word.starts_with(|c: char| c.is_ascii_uppercase())
                && word.chars().skip(1).all(|c| c.is_ascii_digit());
            assert!(!looks_like_a_rule, "cites {word}: {said}");
        }
    }
}

/// A size a person reads is exact, and never a rounded float.
#[test]
fn a_size_is_exact() {
    assert_eq!(gigabytes(5_020_000_000), "5.02 GB");
    assert_eq!(gigabytes(90_000_000), "0.09 GB");
    assert_eq!(gigabytes(0), "0.00 GB");
}

/// Discovery finds nothing where there is nothing, rather than failing.
#[test]
fn discovery_of_an_empty_home_is_empty() {
    assert!(discover(Path::new("/nowhere/at/all")).is_empty());
}

/// Not a check — a look at what this machine actually resolves to.
///
/// Ignored by default because it reads the operator's own store, which no tier
/// may depend on. Run with `--ignored --nocapture` to see it.
#[test]
#[ignore = "reads this machine's store"]
fn what_this_machine_resolves() {
    let Ok(home) = std::env::var("HOME") else {
        return;
    };
    let home = std::path::PathBuf::from(home).join(".local/share/mcf");
    let engines = discover(&home);
    println!("\ndiscovered {} engine(s):", engines.len());
    let mut held = Vec::new();
    for engine in engines {
        let devices = engine.devices(Some(75_000_000_000)).unwrap_or_default();
        println!(
            "  {}  @{}",
            engine.name,
            &engine.commit[..12.min(engine.commit.len())]
        );
        for d in &devices {
            let free = d.free.map_or_else(
                || "unknown".to_owned(),
                |b| format!("{} free", gigabytes(b)),
            );
            println!("      {:?}   {}   {free}", d.kind, d.name);
        }
        held.push((engine, devices));
    }
    let mut names: Vec<std::path::PathBuf> = Vec::new();
    let mut stack = vec![home.join("models")];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().is_some_and(|e| e == "gguf") {
                names.push(path);
            }
        }
    }
    names.sort();
    println!("\n{} model(s):", names.len());
    for path in names.iter().take(6).chain(names.iter().rev().take(2)) {
        let Ok(model) = mcf_standin::gguf::read(path) else {
            continue;
        };
        let weights = std::fs::metadata(path).map_or(0, |m| m.len());
        let per = cache_bytes_per_token(&model);
        let trained = model
            .architecture()
            .and_then(|a| model.get(&format!("{a}.context_length")))
            .and_then(mcf_standin::gguf::Value::as_integer)
            .and_then(|v| u64::try_from(v).ok())
            .unwrap_or(0);
        let short = path.file_name().and_then(|n| n.to_str()).unwrap_or("?");
        print!("  {short:<40}");
        match resolve(&held, weights, per, trained) {
            Ok(c) => println!("{} on {} — window {}", c.engine, c.device.name, c.context),
            Err(why) => println!("{}", why.says()),
        }
    }
}

/// A model that keeps no growing cache costs nothing per token, rather than
/// being unreadable.
///
/// Its header states no attention geometry because it has none. Read as a
/// missing field that is *the file does not say how it is shaped* — true of the
/// fields, false about the model, which runs perfectly well.
#[test]
fn a_state_space_model_has_no_cache_rather_than_no_answer() {
    for architecture in NO_GROWING_CACHE {
        assert!(
            !architecture.is_empty(),
            "an empty architecture would match every file with no header"
        );
    }
    // The window such a model gets is bounded by what it was trained for and
    // by nothing else.
    assert_eq!(
        largest_context(300_000_000, 0, 90_000_000_000, 4_096),
        4_096
    );
}

/// A model's shape comes out of its own header.
///
/// **Because most repositories publish no configuration.** `config.json` is
/// where the fitness judgement looks first and nearly every repository that
/// publishes GGUFs has none — so *will this run here* came back as *MCF cannot
/// say* for almost everything somebody would try to download (B-413, F16).
#[test]
fn a_shape_is_read_from_a_header() {
    let Some(path) = a_model_on_this_machine() else {
        // No model here is a fact about the machine, not a failure of the
        // check (A9). It is announced rather than passing silently.
        eprintln!("skipped: this machine holds no model to read");
        return;
    };
    let Some(model) = read_header(&path) else {
        eprintln!("skipped: {} has no header MCF could read", path.display());
        return;
    };
    let Some(shape) = super::shape_of(&model) else {
        panic!(
            "{} has a header and no shape came out of it",
            path.display()
        );
    };
    assert!(shape.blocks > 0, "a model with no blocks");
    assert!(shape.key_value_heads > 0, "a model with no key/value heads");
    assert!(shape.per_head > 0, "a model with heads that keep nothing");
    // Half precision, the same parameter a configuration is read with, so the
    // two sources give comparable answers.
    assert_eq!(shape.bytes_per_element, 2);

    // And the shape agrees with what the cache arithmetic says independently:
    // both read the same header, so a disagreement would mean one of them is
    // reading it wrong.
    if let Some(per_token) = super::cache_bytes_per_token(&model) {
        let from_shape = shape
            .blocks
            .saturating_mul(shape.key_value_heads)
            .saturating_mul(shape.per_head)
            .saturating_mul(shape.bytes_per_element);
        assert_eq!(
            from_shape, per_token,
            "the shape and the cache arithmetic disagree about the same header"
        );
    }
}

/// A hybrid's cache is sized by the blocks that attend, not by the header's
/// block count.
///
/// Three recurrent blocks to every one that attends put the cache at four
/// times its size, which refused contexts that fit (F150).
#[test]
fn a_hybrid_caches_only_in_the_blocks_that_attend() {
    use mcf_standin::gguf::{Model, Tensor, TensorKind, Value};
    let tensor = |name: String, dimensions: &[u64]| Tensor {
        name,
        dimensions: dimensions.to_vec(),
        kind: TensorKind::Q4_K,
        offset: 0,
    };
    let mut metadata = std::collections::BTreeMap::new();
    for (key, value) in [
        ("general.architecture", Value::Text("hybrid".to_owned())),
        ("hybrid.block_count", Value::Integer(4)),
        ("hybrid.embedding_length", Value::Integer(64)),
        ("hybrid.attention.head_count", Value::Integer(4)),
        ("hybrid.attention.head_count_kv", Value::Integer(2)),
        ("hybrid.attention.key_length", Value::Integer(16)),
    ] {
        metadata.insert(key.to_owned(), value);
    }
    let mut tensors = Vec::new();
    for block in 0..4_u64 {
        if block == 3 {
            tensors.push(tensor(format!("blk.{block}.attn_k.weight"), &[64, 32]));
        } else {
            tensors.push(tensor(format!("blk.{block}.ssm_out.weight"), &[64, 64]));
        }
    }
    let model = Model {
        version: 3,
        metadata,
        tensors,
        data_offset: 0,
        alignment: 32,
    };
    // 2 heads × (16 + 16) × 2 bytes, in the ONE block that attends.
    assert_eq!(cache_bytes_per_token(&model), Some(2 * 32 * 2));
    assert_eq!(shape_of(&model).map(|held| held.blocks), Some(1));

    let mut only_recurrent = model.clone();
    only_recurrent.tensors.truncate(3);
    assert_eq!(cache_bytes_per_token(&only_recurrent), Some(0));

    // A latent-attention model keeps the latent as its key and no value:
    // 1 head × 16 × the one block that attends × 2 bytes (F151).
    let mut latent = model;
    for (key, value) in [
        ("hybrid.attention.kv_lora_rank", 12),
        ("hybrid.attention.value_length", 12),
        ("hybrid.attention.head_count_kv", 1),
    ] {
        latent
            .metadata
            .insert(key.to_owned(), Value::Integer(value));
    }
    assert_eq!(cache_bytes_per_token(&latent), Some(16 * 2));
}

/// A header that says nothing yields no shape, and never a zero.
#[test]
fn a_header_that_says_nothing_yields_no_shape() {
    // Not a GGUF at all: what comes back is `None`, which is *MCF cannot say*
    // and not *a model with no layers* (A7).
    assert!(mcf_standin::gguf::parse(b"not a gguf at all").is_err());
}

/// The first model file this machine is holding, if any.
fn a_model_on_this_machine() -> Option<std::path::PathBuf> {
    let home = std::env::var_os("HOME").map(std::path::PathBuf::from)?;
    let root = home.join(".local/share/mcf/models");
    let mut looking = vec![root];
    while let Some(directory) = looking.pop() {
        let entries = std::fs::read_dir(&directory).ok()?;
        for entry in entries.flatten() {
            let path = entry.path();
            if entry.file_type().is_ok_and(|kind| kind.is_dir()) {
                looking.push(path);
                continue;
            }
            // **A shard is not a model.** A multi-part GGUF names itself
            // `...-00001-of-00004.gguf`, and the first part carries the
            // header for the whole set — so its declared tensor bytes describe
            // four files and its own length describes one. Picking one here
            // made this test assert that a header must accept a file it does
            // not describe, which is the opposite of the claim.
            //
            // Skipped rather than accommodated: MCF has no concept of a
            // sharded model yet (B-422), and a test that quietly worked around
            // that would hide it.
            let is_a_shard = path
                .file_stem()
                .and_then(|held| held.to_str())
                .is_some_and(|held| {
                    held.rsplit_once("-of-")
                        .is_some_and(|(_, tail)| tail.chars().all(|c| c.is_ascii_digit()))
                });
            if !is_a_shard
                && path
                    .extension()
                    .is_some_and(|held| held.eq_ignore_ascii_case("gguf"))
            {
                return Some(path);
            }
        }
    }
    None
}

/// A model's header, from a bounded prefix of the file.
fn read_header(path: &std::path::Path) -> Option<mcf_standin::gguf::Model> {
    use std::io::Read as _;
    let held = std::fs::metadata(path).map_or(0, |about| about.len());
    for cap in [4_u64 << 20, 16 << 20] {
        let take = cap.min(held);
        let mut prefix = Vec::new();
        std::fs::File::open(path)
            .and_then(|handle| handle.take(take).read_to_end(&mut prefix))
            .ok()?;
        if let Ok(model) = mcf_standin::gguf::parse(&prefix) {
            return Some(model);
        }
        if take >= held {
            return None;
        }
    }
    None
}

/// A header that is not describing the file it came with is refused.
///
/// **A shape fetched from a hub is a claim, and this is the one part of it MCF
/// can check without the file.** The header's own tensor table says where the
/// last tensor ends, and that has to be inside the published file and account
/// for nearly all of it — measured across seven architectures on this machine,
/// the declared extent is between 96.2% and 100.0% of the published size.
///
/// The deception that matters is a small shape against a large file: it makes
/// a forty-gigabyte model look like it needs almost nothing, and MCF would
/// answer *fits* about something that does not (§3.7, B-022, A21).
#[test]
fn a_header_that_does_not_describe_its_file_is_refused() {
    let Some(path) = a_model_on_this_machine() else {
        eprintln!("skipped: this machine holds no model");
        return;
    };
    let Some(model) = read_header(&path) else {
        eprintln!("skipped: {} has no header MCF can read", path.display());
        return;
    };
    let held = std::fs::metadata(&path).map_or(0, |about| about.len());
    let Some(declared) = model.data_bytes_required() else {
        eprintln!(
            "skipped: {} uses a quantization MCF cannot size",
            path.display()
        );
        return;
    };

    // Against the file it actually came with: accepted.
    assert!(
        crate::daemon::header_describes_this_file(&model, held),
        "a header was refused for the file it is actually describing"
    );

    // The deception: the same truthful header, offered for a file a thousand
    // times larger. Every number in it is correct and none is about that file.
    assert!(
        !crate::daemon::header_describes_this_file(&model, held.saturating_mul(1_000)),
        "a header describing a fraction of the published file was accepted, so a large model \
         can be made to look small by publishing a small model's header"
    );

    // And a header claiming more data than the file holds, which cannot be
    // true of any file.
    assert!(
        !crate::daemon::header_describes_this_file(&model, declared.saturating_sub(1)),
        "a header claiming more data than the file has was accepted"
    );
    assert!(!crate::daemon::header_describes_this_file(&model, 0));
}

/// An engine is named by what it is, not by what its family is called.
///
/// **Every generation recorded `provisioned llama.cpp` however it was built.**
/// A run on the CUDA build and a run on the processor build wrote the same
/// engine name, and the path printed beside it said otherwise — so the record
/// held two engines under one identity while contradicting itself in the same
/// sentence. `engine_ran` is the field a timing's honesty rests on (B65, D31),
/// and a comparison between two names for one engine reports a moved
/// condition that did not move (F45, A6, B-420).
#[test]
fn an_engine_records_its_own_component_name() {
    let Some(home) = std::env::var_os("HOME") else {
        eprintln!("skipped: this machine says nothing about where its home is");
        return;
    };
    let home = std::path::PathBuf::from(home).join(".local/share/mcf");
    let engines = super::discover(&home);
    if engines.is_empty() {
        eprintln!("skipped: nothing is provisioned on this machine");
        return;
    }
    for engine in &engines {
        // The name is the component's, and the prefix is named for it: a
        // prefix that does not carry its own component's name would mean the
        // two came from different places.
        let prefix = engine.prefix.display().to_string();
        assert!(
            prefix.contains(&engine.name),
            "{} is named {} and lives in {prefix}, so the name and the place disagree",
            engine.commit,
            engine.name
        );
    }
    // And where two are provisioned, their names differ — which is the whole
    // point: one recorded identity for two engines is what this prevents.
    if engines.len() > 1 {
        let mut names: Vec<&str> = engines.iter().map(|engine| engine.name.as_str()).collect();
        names.sort_unstable();
        let held = names.len();
        names.dedup();
        assert_eq!(
            names.len(),
            held,
            "two provisioned engines share a name, so a record cannot tell them apart"
        );
    }
}

/// The engine a machine needs is decided by its driver: the accelerator
/// build where one is loaded, the processor build otherwise — and both are
/// components MCF knows how to build.
#[test]
fn the_required_engine_follows_the_driver() {
    let with = super::required(true).expect("the table names the accelerator build");
    let without = super::required(false).expect("the table names the processor build");
    assert_eq!(with.name, "llama.cpp-cuda");
    assert_eq!(without.name, "llama.cpp");
    assert!(
        with.targets.contains(&"llama-server") && without.targets.contains(&"llama-server"),
        "both builds are the same server"
    );
}
