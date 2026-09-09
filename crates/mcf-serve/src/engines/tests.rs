use super::*;

#[test]
fn a_reported_device_parses() {
    let device =
        parse_device("CUDA0: NVIDIA GeForce RTX 5080 (15877 MiB, 13773 MiB free)").expect("parses");
    assert_eq!(device.kind, Kind::Gpu);
    assert_eq!(device.name, "NVIDIA GeForce RTX 5080");
    assert_eq!(device.free, Some(13_773 << 20));
}

#[test]
fn a_device_that_does_not_say_its_memory_is_unknown_not_zero() {
    let device = parse_device("CUDA0: Some Card").expect("parses");
    assert_eq!(device.free, None, "unknown memory must not become zero");
}

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
        let allowed = free * headroom_percent() / HEADROOM_DENOMINATOR;
        assert!(
            needed <= allowed,
            "{context} needs {needed} of {allowed} allowed"
        );
    }
}

#[test]
fn weights_that_do_not_fit_give_no_window() {
    assert_eq!(
        largest_context(20_000_000_000, 114_688, 2_000_000_000, 40_960),
        0
    );
}

#[test]
fn no_cache_means_the_model_is_the_only_limit() {
    assert_eq!(
        largest_context(300_000_000, 0, 90_000_000_000, 4_096),
        4_096
    );
}

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
                free: Some(4_500_000_000),
            }],
        ),
    ];
    let choice = resolve(&engines, 1_000_000_000, Some(114_688), 40_960).expect("runs");
    assert_eq!(choice.device.kind, Kind::Cpu);
    assert_eq!(choice.engine, "the processor one");
}

#[test]
fn no_engine_says_what_to_do() {
    let refusal = resolve(&[], 1, Some(1), 1).expect_err("nothing runs it");
    assert_eq!(refusal, Refused::NoEngine);
    assert!(refusal.says().contains("build one"), "{}", refusal.says());
}

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
    assert!(said.contains("30.0"), "{said}");
    assert!(said.contains("2.00 GB"), "{said}");
    assert!(
        said.contains(&format!("{}%", headroom_percent())),
        "the refusal names two numbers that do not entail it without the fraction: {said}"
    );
}

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

#[test]
fn a_size_is_exact() {
    assert_eq!(gigabytes(5_020_000_000), "5.02 GB");
    assert_eq!(gigabytes(90_000_000), "0.09 GB");
    assert_eq!(gigabytes(0), "0.00 GB");
}

#[test]
fn discovery_of_an_empty_home_is_empty() {
    assert!(discover(Path::new("/nowhere/at/all")).is_empty());
}

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

#[test]
fn a_state_space_model_has_no_cache_rather_than_no_answer() {
    for architecture in NO_GROWING_CACHE {
        assert!(
            !architecture.is_empty(),
            "an empty architecture would match every file with no header"
        );
    }
    assert_eq!(
        largest_context(300_000_000, 0, 90_000_000_000, 4_096),
        4_096
    );
}

#[test]
fn a_shape_is_read_from_a_header() {
    let Some(path) = a_model_on_this_machine() else {
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
    assert_eq!(shape.bytes_per_element, 2);

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
    assert_eq!(cache_bytes_per_token(&model), Some(2 * 32 * 2));
    assert_eq!(shape_of(&model).map(|held| held.blocks), Some(1));

    let mut only_recurrent = model.clone();
    only_recurrent.tensors.truncate(3);
    assert_eq!(cache_bytes_per_token(&only_recurrent), Some(0));

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

#[test]
fn a_header_that_says_nothing_yields_no_shape() {
    assert!(mcf_standin::gguf::parse(b"not a gguf at all").is_err());
}

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

    assert!(
        crate::daemon::header_describes_this_file(&model, held),
        "a header was refused for the file it is actually describing"
    );

    assert!(
        !crate::daemon::header_describes_this_file(&model, held.saturating_mul(1_000)),
        "a header describing a fraction of the published file was accepted, so a large model \
         can be made to look small by publishing a small model's header"
    );

    assert!(
        !crate::daemon::header_describes_this_file(&model, declared.saturating_sub(1)),
        "a header claiming more data than the file has was accepted"
    );
    assert!(!crate::daemon::header_describes_this_file(&model, 0));
}

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
        let prefix = engine.prefix.display().to_string();
        assert!(
            prefix.contains(&engine.name),
            "{} is named {} and lives in {prefix}, so the name and the place disagree",
            engine.commit,
            engine.name
        );
    }
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

#[test]
fn the_required_engine_follows_the_driver() {
    let cuda = super::required(super::Backend::Cuda).expect("the table names the CUDA build");
    let vulkan = super::required(super::Backend::Vulkan).expect("the table names the Vulkan build");
    let without =
        super::required(super::Backend::None).expect("the table names the processor build");
    assert_eq!(cuda.name, "llama.cpp-cuda");
    assert_eq!(vulkan.name, "llama.cpp-vulkan");
    assert_eq!(without.name, "llama.cpp");
    for build in [cuda, vulkan, without] {
        assert!(
            build.targets.contains(&"llama-server"),
            "every build is the same server: {}",
            build.name
        );
    }
    assert!(
        super::wanted_for(super::Backend::None).is_none(),
        "a machine with no card wants nothing built for one"
    );
}

#[test]
fn a_card_is_driven_through_what_the_system_installed_for_it() {
    let root = std::env::temp_dir().join(format!("mcf-backend-{}", std::process::id()));
    let _cleared = std::fs::remove_dir_all(&root);
    let drm = root.join("drm");
    let icds = root.join("icd.d");
    let driver = root.join("drivers").join("amdgpu");
    std::fs::create_dir_all(drm.join("card0").join("device")).expect("a fake card");
    std::fs::create_dir_all(drm.join("card0-DP-1")).expect("a connector");
    std::fs::create_dir_all(&driver).expect("a fake driver");
    std::fs::create_dir_all(&icds).expect("a loader table");
    std::os::unix::fs::symlink(&driver, drm.join("card0").join("device").join("driver"))
        .expect("the driver link");

    assert_eq!(
        super::backend_from(false, &drm, &icds),
        super::Backend::None,
        "a card without its Vulkan driver is not driven through Vulkan"
    );
    std::fs::write(icds.join("radeon_icd.json"), "{}").expect("the driver's entry");
    assert_eq!(
        super::backend_from(false, &drm, &icds),
        super::Backend::Vulkan
    );
    assert_eq!(
        super::backend_from(true, &drm, &icds),
        super::Backend::Cuda,
        "NVIDIA's driver wins where it is loaded"
    );
    assert_eq!(
        super::backend_from(false, &root.join("nowhere"), &icds),
        super::Backend::None
    );
    let _cleared = std::fs::remove_dir_all(&root);
}

#[test]
fn a_cards_sensors_are_read_from_its_hardware_monitor() {
    let root = std::env::temp_dir().join(format!("mcf-sensors-{}", std::process::id()));
    let monitor = root
        .join("card0")
        .join("device")
        .join("hwmon")
        .join("hwmon3");
    std::fs::create_dir_all(&monitor).unwrap();
    std::fs::create_dir_all(root.join("card0-DP-1")).unwrap();
    std::fs::write(monitor.join("temp1_input"), "50000\n").unwrap();
    std::fs::write(monitor.join("freq1_input"), "625000000\n").unwrap();
    std::fs::write(monitor.join("power1_average"), "68011000\n").unwrap();
    let sensors = card_sensors_under(&root);
    assert_eq!(
        sensors,
        CardSensors {
            power_label: None,
            temperature_millic: Some(50_000),
            clock_hz: Some(625_000_000),
            power_uw: Some(68_011_000),
        }
    );
    assert!(sensors.any());
    let _gone = std::fs::remove_dir_all(&root);
    assert_eq!(card_sensors_under(&root), CardSensors::default());
    assert!(!CardSensors::default().any());
}

#[test]
fn a_cards_free_memory_and_whether_it_is_the_hosts_are_read() {
    let root = std::env::temp_dir().join(format!("mcf-cardmem-{}", std::process::id()));
    let device = root.join("card0").join("device");
    std::fs::create_dir_all(&device).unwrap();
    std::fs::write(device.join("mem_info_vram_total"), "536870912\n").unwrap();
    std::fs::write(device.join("mem_info_vram_used"), "433000448\n").unwrap();
    assert_eq!(
        card_memory_free_under(&root),
        Some(536_870_912 - 433_000_448)
    );
    assert!(
        card_memory_is_the_hosts_under(&root),
        "half a gigabyte is a carve-out"
    );
    std::fs::write(device.join("mem_info_vram_total"), "25769803776\n").unwrap();
    assert!(
        !card_memory_is_the_hosts_under(&root),
        "twenty-four gigabytes is a card's own"
    );
    let _gone = std::fs::remove_dir_all(&root);
    assert_eq!(card_memory_free_under(&root), None);
    assert!(
        card_memory_is_the_hosts_under(&root),
        "no card: nothing beside the host"
    );
}

#[test]
fn a_hold_defaults_to_the_window_whose_cache_stays_within_the_weights() {
    use super::held_at;
    assert_eq!(held_at(262_144, 1_500_000_000, 112 * 1024), 8_192);
    assert_eq!(held_at(262_144, 29_000_000_000, 68 * 1024), 262_144);
    assert_eq!(held_at(32_768, 29_000_000_000, 68 * 1024), 32_768);
    assert_eq!(held_at(262_144, 10_000_000, 112 * 1024), 4_096);
    assert_eq!(held_at(2_048, 10_000_000, 112 * 1024), 2_048);
    assert_eq!(held_at(262_144, 1_500_000_000, 0), 262_144);
}

#[test]
fn whose_draw_it_is_comes_from_the_drivers_own_label() {
    let package = CardSensors {
        power_uw: Some(90_000_000),
        power_label: Some("PPT".to_owned()),
        ..CardSensors::default()
    };
    assert_eq!(package.power_named(), "package");
    assert!(package.power_is().contains("processor"));

    let card = CardSensors {
        power_uw: Some(90_000_000),
        power_label: Some("GFX".to_owned()),
        ..CardSensors::default()
    };
    assert_eq!(card.power_named(), "card");
    assert_eq!(card.power_is(), "the graphics device");

    let unlabelled = CardSensors {
        power_uw: Some(90_000_000),
        ..CardSensors::default()
    };
    assert_eq!(unlabelled.power_named(), "card");
}

#[test]
fn the_power_label_is_read_from_the_file_beside_the_reading() {
    let root = std::env::temp_dir().join(format!("mcf-label-{}", std::process::id()));
    let monitor = root
        .join("card0")
        .join("device")
        .join("hwmon")
        .join("hwmon3");
    std::fs::create_dir_all(&monitor).expect("a fixture card");
    std::fs::write(monitor.join("power1_average"), "132032000\n").expect("a draw");
    std::fs::write(monitor.join("power1_label"), "PPT\n").expect("a label");
    let sensors = card_sensors_under(&root);
    assert_eq!(sensors.power_uw, Some(132_032_000));
    assert_eq!(sensors.power_label.as_deref(), Some("PPT"));
    assert_eq!(sensors.power_named(), "package");
    let _gone = std::fs::remove_dir_all(&root);
}

#[test]
fn a_model_larger_than_any_one_card_is_spread_across_them() {
    let engine = |name: &str| Engine {
        name: name.to_owned(),
        prefix: PathBuf::from("/nowhere"),
        commit: "abc".to_owned(),
    };
    let engines = vec![(
        engine("the card one"),
        vec![
            Device {
                kind: Kind::Cpu,
                name: "CPU".to_owned(),
                free: Some(8_000_000_000),
            },
            Device {
                kind: Kind::Gpu,
                name: "Card A".to_owned(),
                free: Some(116_000_000_000),
            },
            Device {
                kind: Kind::Gpu,
                name: "Card B".to_owned(),
                free: Some(101_000_000_000),
            },
        ],
    )];
    let choice = resolve(&engines, 135_000_000_000, Some(114_688), 40_960)
        .expect("two cards together hold it");
    assert!(choice.is_spread(), "it was not spread: {choice:?}");
    assert_eq!(choice.across.len(), 2, "the processor is not a card");
    assert_eq!(choice.device.name, "Card A", "the largest card leads");
    assert!(choice.context > 0);
}

#[test]
fn one_card_that_holds_it_is_never_spread() {
    let engine = |name: &str| Engine {
        name: name.to_owned(),
        prefix: PathBuf::from("/nowhere"),
        commit: "abc".to_owned(),
    };
    let engines = vec![(
        engine("the card one"),
        vec![
            Device {
                kind: Kind::Gpu,
                name: "Card A".to_owned(),
                free: Some(90_000_000_000),
            },
            Device {
                kind: Kind::Gpu,
                name: "Card B".to_owned(),
                free: Some(90_000_000_000),
            },
        ],
    )];
    let choice = resolve(&engines, 1_000_000_000, Some(114_688), 40_960).expect("runs");
    assert!(!choice.is_spread(), "one card was enough: {choice:?}");
}

#[test]
fn a_spread_names_a_share_for_each_card() {
    let hosting = crate::hosting::Hosting::recommended(
        "an engine",
        "Card A",
        true,
        4096,
        Some(8),
        true,
        None,
    )
    .spread_over(vec![116_000_000_000, 101_000_000_000]);
    let arguments = hosting.arguments("/model.gguf", "127.0.0.1");
    let at = arguments
        .iter()
        .position(|held| held == "--tensor-split")
        .expect("the split is passed to the engine");
    assert_eq!(arguments[at + 1], "110626,96321");
    assert_eq!(hosting.gpu_layers, crate::hosting::ALL_LAYERS);
}

#[test]
fn a_model_trained_shorter_than_the_smallest_step_still_fits() {
    let trained = 16;
    let context = largest_context(1_000_000, 114_688, 64_000_000_000, trained);
    assert_eq!(
        context, trained,
        "a short window read as no window, which is a memory refusal for a model that fits"
    );
}

#[test]
fn a_model_trained_for_nothing_asks_for_nothing() {
    assert_eq!(largest_context(1_000_000, 114_688, 64_000_000_000, 0), 0);
}
