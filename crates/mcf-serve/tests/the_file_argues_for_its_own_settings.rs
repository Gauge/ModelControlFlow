use std::collections::BTreeMap;

use mcf_core::configuration::CacheType;
use mcf_serve::hosting::Hosting;
use mcf_standin::gguf::{Model, Value};

fn file(pairs: &[(&str, i64)]) -> Model {
    let mut metadata = BTreeMap::new();
    metadata.insert(
        "general.architecture".to_owned(),
        Value::Text("testarch".to_owned()),
    );
    for (key, count) in pairs {
        metadata.insert((*key).to_owned(), Value::Integer(*count));
    }
    Model {
        version: 3,
        metadata,
        tensors: Vec::new(),
        data_offset: 0,
        alignment: mcf_standin::gguf::DEFAULT_ALIGNMENT,
    }
}

fn bare() -> Hosting {
    Hosting::recommended("llama.cpp", "gpu", true, 4096, Some(8), true, None)
}

#[test]
fn a_dense_file_asks_for_the_narrow_micro_batch_that_measured_fastest() {
    let tuned = bare().tuned_for(&file(&[]));
    assert_eq!(tuned.ubatch, 256);
    assert!(tuned.batch >= tuned.ubatch);
}

#[test]
fn a_mixture_of_experts_asks_for_a_wider_one() {
    let tuned = bare().tuned_for(&file(&[("testarch.expert_count", 128)]));
    assert_eq!(tuned.ubatch, 1024);
    assert!(tuned.batch >= tuned.ubatch);
}

#[test]
fn one_expert_is_not_a_mixture() {
    assert_eq!(
        bare()
            .tuned_for(&file(&[("testarch.expert_count", 1)]))
            .ubatch,
        256
    );
}

#[test]
fn an_expert_count_filed_under_another_name_is_not_read() {
    assert_eq!(
        bare()
            .tuned_for(&file(&[("elsewhere.expert_count", 128)]))
            .ubatch,
        256
    );
}

#[test]
fn the_cache_is_halved_because_that_is_what_lets_a_long_window_fit() {
    assert_eq!(bare().tuned_for(&file(&[])).cache, CacheType::Q8_0);
}

#[test]
fn a_file_that_carries_a_draft_head_has_it_turned_on_with_what_it_needs() {
    let tuned = bare().tuned_for(&file(&[("testarch.nextn_predict_layers", 1)]));
    assert!(tuned.started.draft_head);
    assert_eq!(tuned.started.drafted, Some(2));
    assert_eq!(
        (tuned.reuse.prompt_cache_mib, tuned.reuse.checkpoints),
        (0, 0)
    );
}

#[test]
fn a_file_without_a_draft_head_keeps_the_caches_it_would_otherwise_have() {
    let plain = bare();
    let tuned = bare().tuned_for(&file(&[]));
    assert!(!tuned.started.draft_head);
    assert_eq!(tuned.reuse.prompt_cache_mib, plain.reuse.prompt_cache_mib);
    assert_eq!(tuned.reuse.checkpoints, plain.reuse.checkpoints);
}

#[test]
fn a_file_declaring_no_draft_layers_does_not_get_a_draft_head() {
    let tuned = bare().tuned_for(&file(&[("testarch.nextn_predict_layers", 0)]));
    assert!(!tuned.started.draft_head);
    assert_eq!(tuned.started.drafted, None);
}

#[test]
fn the_architecture_is_kept_so_a_window_can_be_widened_later() {
    assert_eq!(
        bare().tuned_for(&file(&[])).started.architecture.as_deref(),
        Some("testarch")
    );
}

#[test]
fn a_file_naming_no_architecture_is_left_alone_apart_from_the_hardware_settings() {
    let nameless = Model {
        version: 3,
        metadata: BTreeMap::new(),
        tensors: Vec::new(),
        data_offset: 0,
        alignment: mcf_standin::gguf::DEFAULT_ALIGNMENT,
    };
    let tuned = bare().tuned_for(&nameless);
    assert_eq!(tuned.started.architecture, None);
    assert_eq!(tuned.ubatch, 256);
    assert!(!tuned.started.draft_head);
}

#[test]
fn tuning_changes_nothing_the_machine_decided() {
    let plain = bare();
    let tuned = bare().tuned_for(&file(&[("testarch.expert_count", 128)]));
    assert_eq!(tuned.context, plain.context);
    assert_eq!(tuned.gpu_layers, plain.gpu_layers);
    assert_eq!(tuned.threads, plain.threads);
    assert_eq!(tuned.flash_attention, plain.flash_attention);
    assert_eq!(tuned.engine, plain.engine);
    assert_eq!(tuned.device, plain.device);
}

#[test]
fn what_was_tuned_survives_the_wire() {
    let tuned = bare().tuned_for(&file(&[("testarch.nextn_predict_layers", 1)]));
    let back = Hosting::from_value(&tuned.to_value(), &bare());
    assert_eq!(back.ubatch, tuned.ubatch);
    assert_eq!(back.cache, tuned.cache);
    assert_eq!(back.started.draft_head, tuned.started.draft_head);
    assert_eq!(back.started.drafted, tuned.started.drafted);
    assert_eq!(back.started.architecture, tuned.started.architecture);
}
