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
    assert!(said.contains("20."), "{said}");
    assert!(said.contains("2.00 GB"), "{said}");
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
