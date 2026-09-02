//! What an explanation shows, and what it refuses to claim.

use super::run;

/// The three columns are all there, and each says what kind of thing it is:
/// declared, read, or chosen. A table whose provenance a reader has to guess is
/// the thing this surface exists to replace (A21, §3.15).
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

    // The declarations are the fixture's own.
    assert!(said.contains("llama"), "{said}");
    assert!(said.contains("20 tokens"), "{said}");
    // What MCF read is of the bytes in front of it.
    assert!(said.contains("sha256"), "{said}");
    assert!(said.contains("11 tensors"), "{said}");
    assert!(said.contains("F32"), "{said}");
    // And the choices name where they are written down, so a reader can go and
    // disagree with them.
    assert!(said.contains("greedy"), "{said}");
    assert!(said.contains("crates/mcf-cli/src/run.rs"), "{said}");

    let _cleared = std::fs::remove_dir_all(&scratch);
}

/// Nothing in the report runs two columns together, and nothing runs past a
/// terminal.
///
/// The value column used to be exactly as wide as its widest value, so that one
/// row read `32 unless --limit saystokens rather than seconds` — two columns
/// with nothing between them. A report a reader has to decode is one they stop
/// reading (D7).
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
    // The row that used to collide, with the space that keeps it apart.
    assert!(said.contains("says tokens rather than seconds"), "{said}");

    let _cleared = std::fs::remove_dir_all(&scratch);
}

/// The terms are shown where somebody decides whether to run it (§III, B-023),
/// and an artifact nothing accounts for says its terms are unknown rather than
/// leaving the line out — an absent line reads as *no restrictions* (A7).
#[test]
fn the_terms_are_shown_and_an_unaccounted_model_says_they_are_unknown() {
    let scratch = std::env::temp_dir().join(format!("mcf-explain-terms-{}", std::process::id()));
    std::fs::create_dir_all(&scratch).expect("a scratch directory");
    let model = scratch.join("model.gguf");
    std::fs::write(&model, mcf_lab::fixture::a_model_that_runs()).expect("a model file");

    let bare = run(model.to_str().unwrap_or_default()).text;
    assert!(bare.contains("terms"), "{bare}");
    assert!(bare.contains("unknown"), "{bare}");

    // With provenance beside it, the identifier the repository declared and the
    // family its own name puts it in — and nothing further (B-023).
    let provenance = mcf_core::provenance::Provenance::acquired(
        mcf_core::provenance::Origin::hub(
            mcf_core::provenance::Repository::new("a-publisher/a-model"),
            None,
        ),
        mcf_core::time::Timestamp::now(),
    )
    // Through `recognize`, which is the only way a licence enters MCF: it
    // normalizes the case an identifier is written in, and a `Spdx` built by
    // hand from a differently-cased string is one nothing would match.
    .with_licence(mcf_hub::licence::recognize("Apache-2.0").expect("a known identifier"));
    mcf_hub::store::record_provenance(&model, &provenance).expect("a sidecar");

    let said = run(model.to_str().unwrap_or_default()).text;
    assert!(said.contains("apache-2.0"), "{said}");
    assert!(said.contains("permissive"), "{said}");

    let _cleared = std::fs::remove_dir_all(&scratch);
}

/// The unanswered questions are named with their reasons, because a defaults
/// screen that listed only what MCF chose would imply it had a basis for
/// choosing (§6.5, C7).
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

/// A file that is not a model is refused rather than explained, and the refusal
/// says what MCF reads.
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

/// A model that is not there is said, not explained.
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

/// What `explain` says MCF would choose is what MCF would choose.
///
/// **Both of the choices that matter were wrong here.** The engine row read
/// *more than one is provisioned and MCF will not choose*, and the placement
/// row read *the processor — MCF's stand-in has no accelerator path*. Neither
/// had been true since engine resolution landed: MCF does choose, by
/// arithmetic over the model's header and what each device has free, and on a
/// machine with a card it chooses the card. A screen whose whole job is *no
/// hidden choices* being wrong about the choice is worse than no screen
/// (§3.15, B-038, F133).
///
/// So this asserts the two are the same function's answer rather than two
/// answers that happen to agree today.
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
            // And neither is the sentence that was wrong: a resolved model is
            // never described as running on the stand-in's processor.
            let placement = value("placement").unwrap_or_default();
            assert_ne!(placement, "the processor", "the stale placement is back");
        }
        Err(why) => {
            // A7: MCF could not work it out is not *the processor*.
            assert_eq!(
                value("placement").as_deref(),
                Some("Unknown"),
                "a placement MCF could not resolve is being reported as one it could"
            );
            // And a reason travels with it: `Unknown` alone cannot tell a
            // reader whether to provision an engine or find a smaller model
            // (F138).
            //
            // That a reason is *there*, not that it is the same text. The page
            // and this test resolve the model at two different moments, and a
            // refusal names how much memory was free — a figure that moves
            // between them on a machine doing anything at all. Comparing the
            // strings tested the machine's idleness rather than MCF (F140).
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

/// The smallest model file this machine is holding, if any.
///
/// **Smallest rather than first, because the assertion is about agreement and
/// not about the boundary.** `the_placement_shown_is_the_placement_resolved`
/// resolves a model twice — once through the page, once directly — and a model
/// whose weights sit near what this machine can hold resolves differently
/// depending on how much memory was free at each moment. Under a workspace run
/// several test binaries compete for it, so the two calls straddled the
/// boundary and the test failed on a real machine doing real work rather than
/// on a defect. The smallest model held is nowhere near the boundary, which
/// makes the comparison about what it is supposed to be about.
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
            // A projector belongs to a model rather than being one, and has
            // no context length for a placement to be worked out from.
            if mcf_hub::store::is_a_companion(&path) {
                continue;
            }
            // By the whole set, not the file. Picking the smallest *file*
            // chose the 10.9 MB first part of a 111.92 GB four-part model —
            // the largest thing held here, wearing the smallest file's size,
            // which is F138 catching this test out the same way it caught the
            // console.
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

/// A model's header, from a bounded prefix.
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

/// The model is counted from its directory and costed from its header, and
/// each of those sections says what kind of thing it is: counted, compared,
/// computed (A21, A20).
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
    // The fixture's header agrees with its own directory, and the report says
    // so in one line rather than leaving a reader to check the verdicts.
    assert!(said.contains("every figure"), "{said}");
    assert!(!said.contains("DISAGREE"), "{said}");

    let _cleared = std::fs::remove_dir_all(&scratch);
}
