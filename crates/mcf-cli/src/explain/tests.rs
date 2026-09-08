use super::run;

#[test]
fn an_explanation_separates_declared_read_and_chosen() {
    let scratch = std::env::temp_dir().join(format!("mcf-explain-{}", std::process::id()));
    std::fs::create_dir_all(&scratch).expect("a scratch directory");
    let model = scratch.join("model.gguf");
    std::fs::write(&model, mcf_lab::fixture::a_model_that_runs()).expect("a model file");

    let response = run(model.to_str().unwrap_or_default());
    assert!(response.served, "{}", response.text);
    let said = response.text;

    assert!(said.contains("WHAT THE FILE DECLARES"), "{said}");
    assert!(said.contains("WHAT MCF READ FROM THE BYTES"), "{said}");
    assert!(said.contains("WHAT MCF WOULD CHOOSE"), "{said}");
    assert!(said.contains("WHAT MCF CANNOT TELL YOU"), "{said}");

    assert!(said.contains("llama"), "{said}");
    assert!(said.contains("20 tokens"), "{said}");
    assert!(said.contains("sha256"), "{said}");
    assert!(said.contains("11 tensors"), "{said}");
    assert!(said.contains("F32"), "{said}");
    assert!(said.contains("greedy"), "{said}");
    assert!(said.contains("crates/mcf-cli/src/run.rs"), "{said}");

    let _cleared = std::fs::remove_dir_all(&scratch);
}

#[test]
fn the_columns_do_not_collide_and_the_lines_do_not_run_off() {
    let scratch = std::env::temp_dir().join(format!("mcf-explain-columns-{}", std::process::id()));
    std::fs::create_dir_all(&scratch).expect("a scratch directory");
    let model = scratch.join("model.gguf");
    std::fs::write(&model, mcf_lab::fixture::a_model_that_runs()).expect("a model file");

    let said = run(model.to_str().unwrap_or_default()).text;
    for line in said.lines() {
        assert!(
            line.chars().count() <= 120,
            "a line runs past any terminal ({} characters): {line}",
            line.chars().count()
        );
    }
    assert!(said.contains("says tokens rather than seconds"), "{said}");

    let _cleared = std::fs::remove_dir_all(&scratch);
}

#[test]
fn the_terms_are_shown_and_an_unaccounted_model_says_they_are_unknown() {
    let scratch = std::env::temp_dir().join(format!("mcf-explain-terms-{}", std::process::id()));
    std::fs::create_dir_all(&scratch).expect("a scratch directory");
    let model = scratch.join("model.gguf");
    std::fs::write(&model, mcf_lab::fixture::a_model_that_runs()).expect("a model file");

    let bare = run(model.to_str().unwrap_or_default()).text;
    assert!(bare.contains("terms"), "{bare}");
    assert!(bare.contains("unknown"), "{bare}");

    let provenance = mcf_core::provenance::Provenance::acquired(
        mcf_core::provenance::Origin::hub(
            mcf_core::provenance::Repository::new("a-publisher/a-model"),
            None,
        ),
        mcf_core::time::Timestamp::now(),
    )
    .with_licence(mcf_hub::licence::recognize("Apache-2.0").expect("a known identifier"));
    mcf_hub::store::record_provenance(&model, &provenance).expect("a sidecar");

    let said = run(model.to_str().unwrap_or_default()).text;
    assert!(said.contains("apache-2.0"), "{said}");
    assert!(said.contains("permissive"), "{said}");

    let _cleared = std::fs::remove_dir_all(&scratch);
}

#[test]
fn what_mcf_cannot_say_is_said_with_the_reason() {
    let scratch = std::env::temp_dir().join(format!("mcf-explain-why-{}", std::process::id()));
    std::fs::create_dir_all(&scratch).expect("a scratch directory");
    let model = scratch.join("model.gguf");
    std::fs::write(&model, mcf_lab::fixture::a_model_that_runs()).expect("a model file");

    let said = run(model.to_str().unwrap_or_default()).text;
    assert!(said.contains("Which quantization should I run?"), "{said}");
    assert!(said.contains("DEC-002"), "{said}");
    assert!(said.contains("How fast is it on this machine?"), "{said}");
    assert!(said.contains("B65"), "{said}");

    let _cleared = std::fs::remove_dir_all(&scratch);
}

#[test]
fn something_that_is_not_a_model_is_refused() {
    let scratch = std::env::temp_dir().join(format!("mcf-explain-not-{}", std::process::id()));
    std::fs::create_dir_all(&scratch).expect("a scratch directory");
    let model = scratch.join("model.gguf");
    std::fs::write(&model, b"not a model at all").expect("a file");

    let response = run(model.to_str().unwrap_or_default());
    assert!(!response.served);
    assert!(response.text.contains("GGUF"), "{}", response.text);

    let _cleared = std::fs::remove_dir_all(&scratch);
}

#[test]
fn a_model_that_is_not_there_is_said() {
    let response = run("owner/model:absent.gguf");
    assert!(!response.served);
    assert!(
        response.text.contains("there is no model"),
        "{}",
        response.text
    );
}

#[test]
fn the_placement_shown_is_the_placement_resolved() {
    let Some(path) = a_model_on_this_machine() else {
        eprintln!("skipped: this machine holds no model");
        return;
    };
    let Some(file) = header_of(&path) else {
        eprintln!("skipped: {} has no header MCF can read", path.display());
        return;
    };
    let rows = super::chosen(&path, &file);
    let value = |name: &str| {
        rows.iter()
            .find(|(row, _, _)| *row == name)
            .map(|(_, value, _)| value.clone())
    };

    match super::resolved_here(&path, &file) {
        Ok(choice) => {
            assert_eq!(
                value("placement").as_deref(),
                Some(choice.device.name.as_str()),
                "the placement shown is not the placement resolved"
            );
            assert_eq!(
                value("engine").as_deref(),
                Some(choice.engine.as_str()),
                "the engine shown is not the engine resolved"
            );
            let placement = value("placement").unwrap_or_default();
            assert_ne!(placement, "the processor", "the stale placement is back");
        }
        Err(why) => {
            assert_eq!(
                value("placement").as_deref(),
                Some("Unknown"),
                "a placement MCF could not resolve is being reported as one it could"
            );
            let said = rows
                .iter()
                .find(|(row, _, _)| *row == "placement")
                .map(|(_, _, note)| note.clone())
                .unwrap_or_default();
            assert!(
                said.len() > "MCF could not work out where this would run: ".len(),
                "the placement could not be resolved and no reason travelled with it; this                  call was refused with: {why}"
            );
        }
    }
}

fn a_model_on_this_machine() -> Option<std::path::PathBuf> {
    let root = crate::models::default_root()?;
    let mut looking = vec![root];
    let mut smallest: Option<(u64, std::path::PathBuf)> = None;
    while let Some(directory) = looking.pop() {
        let Ok(entries) = std::fs::read_dir(&directory) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if entry.file_type().is_ok_and(|kind| kind.is_dir()) {
                looking.push(path);
                continue;
            }
            if !path
                .extension()
                .is_some_and(|held| held.eq_ignore_ascii_case("gguf"))
            {
                continue;
            }
            if mcf_hub::store::is_a_companion(&path) {
                continue;
            }
            let Ok(size) = mcf_hub::store::bytes_of_the_whole(&path) else {
                continue;
            };
            if smallest.as_ref().is_none_or(|(held, _)| size < *held) {
                smallest = Some((size, path));
            }
        }
    }
    smallest.map(|(_, path)| path)
}

fn header_of(path: &std::path::Path) -> Option<mcf_standin::gguf::Model> {
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
fn an_explanation_counts_compares_and_costs_the_model() {
    let scratch = std::env::temp_dir().join(format!("mcf-explain-anatomy-{}", std::process::id()));
    std::fs::create_dir_all(&scratch).expect("a scratch directory");
    let model = scratch.join("model.gguf");
    std::fs::write(&model, mcf_lab::fixture::a_model_that_runs()).expect("a model file");

    let said = run(model.to_str().unwrap_or_default()).text;

    assert!(said.contains("parameters, counted"), "{said}");
    assert!(said.contains("by part"), "{said}");
    assert!(said.contains("by block"), "{said}");
    assert!(said.contains("1 block(s), 0:"), "{said}");
    assert!(said.contains("by encoding"), "{said}");
    assert!(
        said.contains("WHAT THE HEADER DECLARES, AGAINST WHAT THE DIRECTORY HOLDS"),
        "{said}"
    );
    assert!(
        said.contains("WHAT ONE TOKEN COSTS, IN ARITHMETIC"),
        "{said}"
    );
    assert!(said.contains("multiply-adds through the weights"), "{said}");
    assert!(said.contains("key/value cache"), "{said}");
    assert!(said.contains("WHAT THE VOCABULARY IS"), "{said}");
    assert!(said.contains("named tokens"), "{said}");
    assert!(said.contains("every figure"), "{said}");
    assert!(!said.contains("DISAGREE"), "{said}");

    let _cleared = std::fs::remove_dir_all(&scratch);
}
