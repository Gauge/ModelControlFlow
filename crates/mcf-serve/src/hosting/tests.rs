use super::{DEFAULT_PORT, Hosting, LOOPBACK};
use mcf_record::json::Value;

fn on_a_card() -> Hosting {
    Hosting::recommended(
        "llama.cpp-cuda",
        "NVIDIA",
        true,
        32_768,
        Some(32),
        true,
        None,
    )
}

#[test]
fn a_model_that_fits_on_the_card_is_put_on_the_card() {
    let recommended = on_a_card();
    assert!(
        recommended.gpu_layers > 0,
        "a model that fits on the card was recommended onto the processor"
    );
    let arguments = recommended.arguments("/models/a.gguf", "127.0.0.1", None, None);
    let at = arguments
        .iter()
        .position(|held| held == "--n-gpu-layers")
        .expect("the layer count is passed to the engine");
    assert_ne!(
        arguments.get(at + 1).map(String::as_str),
        Some("0"),
        "the engine is still being told to use no layers: {arguments:?}"
    );
}

#[test]
fn a_model_that_does_not_fit_stays_on_the_processor() {
    let recommended = Hosting::recommended(
        "llama.cpp-cuda",
        "NVIDIA",
        true,
        8_192,
        Some(32),
        false,
        None,
    );
    assert_eq!(recommended.gpu_layers, 0);
    assert!(!recommended.flash_attention);
}

#[test]
fn every_setting_is_shown_with_what_was_recommended() {
    let recommended = on_a_card();
    let listed = recommended.listed(&recommended);
    assert!(
        listed.len() >= 10,
        "only {} settings are listed",
        listed.len()
    );
    for setting in &listed {
        assert!(!setting.name.is_empty());
        assert!(!setting.value.is_empty(), "{} has no value", setting.name);
        assert!(
            !setting.because.is_empty(),
            "{} says nothing about what it does",
            setting.name
        );
    }
    assert!(recommended.differs_from(&recommended).is_empty());
}

#[test]
fn a_setting_that_was_moved_says_so() {
    let recommended = on_a_card();
    let mut chosen = recommended.clone();
    chosen.gpu_layers = 0;
    chosen.context = 4_096;
    let moved = chosen.differs_from(&recommended);
    assert_eq!(moved.len(), 3, "{moved:?}");
    assert!(
        moved.iter().any(|said| said.contains("where it runs")),
        "{moved:?}"
    );
    assert!(
        moved.iter().any(|said| said.contains("context window")),
        "{moved:?}"
    );
    assert!(
        moved.iter().any(|said| said.contains("per conversation")),
        "what one conversation gets follows the window it is a share of: {moved:?}"
    );

    let mut shared = recommended.clone();
    shared.slots = 4;
    assert_eq!(
        shared.per_conversation().saturating_mul(4),
        recommended.context
    );
    let moved = shared.differs_from(&recommended);
    assert!(
        moved
            .iter()
            .any(|said| said.contains("conversations at once"))
            && moved
                .iter()
                .any(|said| said.contains("window per conversation")),
        "asking for more slots divides the window and says so: {moved:?}"
    );
}

#[test]
fn a_key_never_reaches_the_command_line() {
    let mut chosen = on_a_card();
    chosen.api_key = Some("a-secret-nobody-should-see".to_owned());
    let bare = chosen.arguments("/model.gguf", "127.0.0.1", None, None);
    assert!(
        !bare.iter().any(|held| held.contains("a-secret")),
        "a key reached the arguments: {bare:?}"
    );
    assert!(
        !bare.iter().any(|held| held == "--api-key"),
        "the flag that puts a key in the process list is still passed: {bare:?}"
    );

    let named = std::path::Path::new("/run/user/1000/mcf/a-key");
    let with = chosen.arguments("/model.gguf", "127.0.0.1", Some(named), None);
    let at = with
        .iter()
        .position(|held| held == "--api-key-file")
        .expect("the key is passed as a file");
    assert_eq!(with.get(at + 1).map(String::as_str), named.to_str());
    assert!(
        !with.iter().any(|held| held.contains("a-secret")),
        "a key reached the arguments beside the file: {with:?}"
    );
}

#[test]
fn a_key_is_recorded_as_present_and_never_as_itself() {
    let mut chosen = on_a_card();
    chosen.api_key = Some("a-secret-nobody-should-see".to_owned());
    let written = chosen.to_value().to_line();
    assert!(
        !written.contains("a-secret-nobody-should-see"),
        "the key itself reached the record: {written}"
    );
    assert!(written.contains("api_key_set"), "{written}");
    let listed = chosen.listed(&chosen);
    let key = listed
        .iter()
        .find(|setting| setting.name == "API key")
        .expect("the key is listed");
    assert_eq!(key.value, "set");
}

#[test]
fn an_unmentioned_setting_keeps_its_recommendation() {
    let recommended = on_a_card();
    let nothing: [(&str, Value); 0] = [];
    let read = Hosting::from_value(&Value::map(nothing), &recommended);
    assert_eq!(read, recommended);

    let asked = Value::map([("gpu_layers", Value::Integer(0))]);
    let read = Hosting::from_value(&asked, &recommended);
    assert_eq!(read.gpu_layers, 0, "an explicit zero was overridden");
    assert_eq!(
        read.context, recommended.context,
        "an unmentioned field moved"
    );
}

#[test]
fn a_hosted_model_is_on_this_computer_only() {
    let recommended = on_a_card();
    assert_eq!(recommended.port, DEFAULT_PORT);
    assert!(
        recommended
            .address()
            .starts_with(&format!("http://{LOOPBACK}:"))
    );
    let arguments = recommended.arguments("/models/a.gguf", LOOPBACK, None, None);
    let at = arguments
        .iter()
        .position(|held| held == "--host")
        .expect("the engine is told what to bind");
    assert_eq!(arguments.get(at + 1).map(String::as_str), Some(LOOPBACK));
}

#[test]
fn a_hold_open_to_the_network_binds_every_address_and_names_its_own() {
    let recommended = on_a_card();
    assert!(!recommended.open, "off unless somebody turned it on");
    assert_eq!(recommended.bind(), LOOPBACK);
    assert_eq!(recommended.network_address(), None);
    let asked = Value::map([("open", Value::Bool(true)), ("api_key", Value::text("k"))]);
    let open = Hosting::from_value(&asked, &recommended);
    assert!(open.open);
    assert_eq!(open.bind(), "0.0.0.0");
    assert!(
        open.address().starts_with(&format!("http://{LOOPBACK}:")),
        "the loopback address is still named"
    );
    match open.network_address() {
        Some(named) => {
            assert!(named.starts_with("http://"), "{named}");
            assert!(named.ends_with(&format!(":{}", open.port)), "{named}");
            assert!(
                !named.contains("127.0.0.1"),
                "the network address is not the loopback one"
            );
        }
        None => eprintln!("this machine has no route out; no network address to name"),
    }
    assert_eq!(open.to_value().get("open"), Some(&Value::Bool(true)));
    assert!(
        open.listed(&recommended)
            .iter()
            .any(|setting| setting.name == "reachable from the network" && setting.value == "on"),
        "the switch is a listed setting"
    );
}

#[test]
fn a_hold_asked_for_carries_its_key_and_the_record_does_not() {
    let mut chosen = on_a_card();
    chosen.api_key = Some("a-secret-nobody-should-see".to_owned());
    chosen.open = true;
    let asked = chosen.to_request();
    assert_eq!(
        asked.get("api_key").and_then(Value::as_text),
        Some("a-secret-nobody-should-see")
    );
    let read = Hosting::from_value(&asked, &on_a_card());
    assert_eq!(read.api_key.as_deref(), Some("a-secret-nobody-should-see"));
    assert!(read.open);
    assert!(
        !chosen.to_value().to_line().contains("nobody-should-see"),
        "the record's form keeps the key out"
    );
}

#[test]
fn the_sampling_a_hold_was_given_survives_being_written_down_and_read_back() {
    let recommended = on_a_card();
    let mut chosen = recommended.clone();
    chosen.started.temperature = Some(mcf_core::configuration::Thousandths(200));
    chosen.started.top_p = Some(mcf_core::configuration::Thousandths(950));
    chosen.started.top_k = Some(40);
    chosen.started.effort = Some("low".to_owned());
    let back = Hosting::from_value(&chosen.to_value(), &recommended);
    assert_eq!(back.started.temperature, chosen.started.temperature);
    assert_eq!(back.started.top_p, chosen.started.top_p);
    assert_eq!(back.started.top_k, chosen.started.top_k);
    assert_eq!(back.started.effort, chosen.started.effort);
}

#[test]
fn sampling_a_person_chose_reaches_the_engine_on_its_command_line() {
    let mut chosen = on_a_card();
    chosen.started.temperature = Some(mcf_core::configuration::Thousandths(200));
    chosen.started.top_p = Some(mcf_core::configuration::Thousandths(950));
    chosen.started.top_k = Some(40);
    let said = chosen.started.arguments();
    for (flag, value) in [("--temp", "0.200"), ("--top-p", "0.950"), ("--top-k", "40")] {
        let at = said
            .iter()
            .position(|held| held == flag)
            .unwrap_or_else(|| panic!("{flag} is written: {said:?}"));
        assert_eq!(
            said.get(at.saturating_add(1)).map(String::as_str),
            Some(value)
        );
    }
}

#[test]
fn sampling_nobody_chose_writes_no_flags_and_leaves_the_engine_its_own() {
    let plain = on_a_card();
    let said = plain.started.arguments();
    for flag in ["--temp", "--top-p", "--top-k", "--reasoning-effort"] {
        assert!(!said.iter().any(|held| held == flag), "{flag} in {said:?}");
    }
}

fn a_file_declaring(pairs: &[(&str, mcf_standin::gguf::Value)]) -> mcf_standin::gguf::Model {
    let mut metadata = std::collections::BTreeMap::new();
    metadata.insert(
        "general.architecture".to_owned(),
        mcf_standin::gguf::Value::Text("a-family".to_owned()),
    );
    for (key, value) in pairs {
        metadata.insert((*key).to_owned(), value.clone());
    }
    mcf_standin::gguf::Model {
        version: 3,
        metadata,
        tensors: Vec::new(),
        data_offset: 0,
        alignment: 32,
    }
}

/// A model is published with the draw its makers found it answers best under, written into
/// its own header. MCF read those numbers and showed them, and held the model at the
/// engine's defaults anyway.
#[test]
fn a_model_is_held_at_the_sampling_its_own_file_asks_for() {
    use mcf_standin::gguf::Value;
    let file = a_file_declaring(&[
        ("general.sampling.temp", Value::Float(0.7)),
        ("general.sampling.top_p", Value::Float(0.8)),
        ("general.sampling.top_k", Value::Integer(20)),
    ]);
    let settings =
        super::Hosting::recommended("llama.cpp", "a card", true, 4096, Some(8), true, None)
            .tuned_for(&file);
    assert_eq!(
        settings.started.temperature,
        Some(mcf_core::configuration::Thousandths(700))
    );
    assert_eq!(
        settings.started.top_p,
        Some(mcf_core::configuration::Thousandths(800))
    );
    assert_eq!(settings.started.top_k, Some(20));
    let said = settings.started.arguments();
    for flag in ["--temp", "--top-p", "--top-k"] {
        assert!(
            said.iter().any(|held| held == flag),
            "{flag} never reaches the engine: {said:?}"
        );
    }
}

#[test]
fn a_model_that_asks_for_nothing_is_left_at_the_engine_s_own_defaults() {
    let settings =
        super::Hosting::recommended("llama.cpp", "a card", true, 4096, Some(8), true, None)
            .tuned_for(&a_file_declaring(&[]));
    assert_eq!(settings.started.temperature, None);
    assert_eq!(settings.started.top_p, None);
    assert_eq!(
        settings.started.top_k, None,
        "a number MCF made up is not a default, it is a decision nobody took"
    );
}

#[test]
fn a_model_is_held_under_its_own_template_unless_one_is_left_in_the_settings() {
    let plain = super::Hosting::recommended("llama.cpp", "a card", true, 4096, Some(8), true, None);
    assert_eq!(
        plain.template, None,
        "nothing here means the file's own, which is what it was published with"
    );
    let said = plain.arguments("/model.gguf", "127.0.0.1", None, None);
    assert!(
        !said.iter().any(|held| held == "--chat-template-file"),
        "and the engine is told nothing, so it reads the one in the file: {said:?}"
    );

    let mut edited = plain.clone();
    edited.template = Some("{{ messages }}".to_owned());
    let at = std::path::Path::new("/tmp/a-template.jinja");
    let said = edited.arguments("/model.gguf", "127.0.0.1", None, Some(at));
    let at_flag = said.iter().position(|held| held == "--chat-template-file");
    assert!(at_flag.is_some(), "{said:?}");
    assert_eq!(
        at_flag
            .and_then(|at| said.get(at.saturating_add(1)))
            .map(String::as_str),
        Some("/tmp/a-template.jinja"),
        "the engine reads a template from a file, so an edited one has to be on the disk"
    );
}

#[test]
fn a_template_survives_the_round_trip_through_the_socket() {
    let recommended =
        super::Hosting::recommended("llama.cpp", "a card", true, 4096, Some(8), true, None);
    let mut held = recommended.clone();
    held.template = Some("{%- if x %}\n  {{ y }},{{ z }}_w\n{%- endif %}".to_owned());
    let back = super::Hosting::from_value(&held.to_value(), &recommended);
    assert_eq!(
        back.template, held.template,
        "a template is Jinja: its newlines, its commas and its underscores are all load \\
         bearing, and a round trip that trims or strips any of them is a round trip that \\
         changes how the model is addressed"
    );

    let mut blank = recommended.clone();
    blank.template = Some("   \n  ".to_owned());
    assert_eq!(
        super::Hosting::from_value(&blank.to_value(), &recommended).template,
        None,
        "a template emptied out is asking for the file's own back"
    );
}

#[test]
fn what_the_template_was_asked_to_read_survives_the_round_trip_through_the_socket() {
    let recommended =
        super::Hosting::recommended("llama.cpp", "a card", true, 4096, Some(8), true, None);
    let mut held = recommended.clone();
    held.started.template_taken = vec![
        ("enable_thinking".to_owned(), Value::Bool(false)),
        ("low_effort".to_owned(), Value::Bool(true)),
    ];
    let back = super::Hosting::from_value(&held.to_request(), &recommended);
    assert_eq!(
        back.started.template_taken, held.started.template_taken,
        "a switch ticked under what this template takes is saved and held with, not dropped \
         on the way to the daemon"
    );
    assert!(
        back.started
            .arguments()
            .iter()
            .any(|argument| argument.contains("low_effort")),
        "and the engine is started with it: {:?}",
        back.started.arguments()
    );
    assert!(
        recommended.to_value().get("template_taken").is_none(),
        "settings that ask the template for nothing are written as they always were"
    );
}

/// A budget of nothing and no budget at all are opposites, and the engine spells them the
/// same way if MCF is not careful: it reads `--reasoning-budget 0` as no budget, and lets
/// the model think until it is done. Measured on a model whose template opens its own
/// thinking section: two hundred and seventy three characters of thinking at nought, none
/// at one. On a set of eight tasks the same nought filled sixteen thousand tokens over
/// twelve minutes and never answered.
#[test]
fn asking_for_no_thinking_is_not_asking_for_no_budget() {
    let none_at_all = crate::declared::Started {
        thinking: Some(0),
        ..crate::declared::Started::default()
    };
    let said = none_at_all.arguments();
    let at = said.iter().position(|held| held == "--reasoning-budget");
    assert_eq!(
        at.and_then(|at| said.get(at.saturating_add(1)))
            .map(String::as_str),
        Some("1"),
        "nothing is asked for as the smallest budget there is, not as nought: {said:?}"
    );

    let some = crate::declared::Started {
        thinking: Some(4096),
        ..crate::declared::Started::default()
    };
    let said = some.arguments();
    let at = said.iter().position(|held| held == "--reasoning-budget");
    assert_eq!(
        at.and_then(|at| said.get(at.saturating_add(1)))
            .map(String::as_str),
        Some("4096"),
        "and every other budget is itself: {said:?}"
    );

    let said = crate::declared::Started::default().arguments();
    assert!(
        !said.iter().any(|held| held == "--reasoning-budget"),
        "no budget at all is asked for by saying nothing, which is what the engine does by \
         default anyway: {said:?}"
    );
}
