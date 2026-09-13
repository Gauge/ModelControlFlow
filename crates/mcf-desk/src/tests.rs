use super::{ACTIONS, Desk, Model, Page, model_from};
use mcf_record::json::Value;

#[test]
fn every_action_reaches_a_request_or_asks_nothing() {
    assert!(!ACTIONS.is_empty());
    for action in ACTIONS {
        assert!(!action.key.is_empty());
        assert!(!action.does.is_empty());
        if let Some(reaches) = action.reaches {
            assert!(
                matches!(
                    reaches,
                    "Status" | "Holding" | "Stop" | "Components" | "Anatomy" | "Failures"
                ),
                "{} reaches {reaches}, which this surface cannot build",
                action.key
            );
        }
    }
    assert!(ACTIONS.iter().any(|action| action.reaches.is_some()));
}

#[test]
fn nothing_in_the_menu_leads_nowhere() {
    assert!(!Page::MENU.is_empty());
    for (page, label) in Page::MENU {
        assert!(!label.is_empty());
        assert_eq!(
            page.section(),
            *page,
            "{label} is not a section, so it cannot be lit when you are on it"
        );
    }
    assert_eq!(Page::Adding.section(), Page::Models);
    assert_eq!(Page::Hosting.section(), Page::Hosting);
    assert_eq!(Page::Anatomy.section(), Page::Models);
    assert_eq!(Page::Vocabulary.section(), Page::Models);
    assert_eq!(Page::Host.section(), Page::Models);
    assert!(
        !Page::MENU.iter().any(|(page, _)| *page == Page::Host),
        "Host is the list Models shows, not a second entry for it"
    );
}

#[test]
fn a_model_that_will_not_run_is_never_drawn_as_ready() {
    let refused = Model {
        name: "too big".to_owned(),
        refused: Some("this model needs more memory than this computer has".to_owned()),
        ..Model::default()
    };
    assert!(!refused.will_run());
    assert!(refused.in_a_sentence().contains("more memory"));
    assert!(refused.where_it_runs().contains("more memory"));
}

#[test]
fn a_refusal_travels_from_the_daemon_word_for_word() {
    let answered = Value::map([
        ("path", Value::text("/models/enormous.gguf")),
        ("bytes", Value::Integer(400_000_000_000)),
        (
            "runs",
            Value::map([(
                "resolved",
                Value::map([
                    ("known", Value::Bool(false)),
                    ("why", Value::text("no engine here has room for this model")),
                ]),
            )]),
        ),
    ]);
    let held = model_from(&answered);
    assert!(!held.will_run());
    assert_eq!(
        held.refused.as_deref(),
        Some("no engine here has room for this model"),
        "the window must not rewrite the daemon's reason"
    );
}

#[test]
fn an_unreadable_machine_is_never_reported_as_an_empty_one() {
    let desk = Desk::new(std::path::PathBuf::from("/nowhere/control.sock"));
    assert_eq!(desk.memory_sentence(), crate::words::UNMEASURED);
    assert_eq!(desk.processor_sentence(), crate::words::UNMEASURED);
    assert_eq!(desk.card_sentence(), "None found");
    assert!(
        desk.capability_sentence().contains("not been able to read"),
        "{}",
        desk.capability_sentence()
    );
}

#[test]
fn a_daemon_that_is_not_there_is_said_in_words() {
    let mut desk = Desk::new(std::path::PathBuf::from("/nowhere/control.sock"));
    desk.refresh();
    let why = desk.refusal.clone().unwrap_or_default();
    assert!(why.contains("not answering"), "{why}");
    assert!(
        !why.contains("/nowhere"),
        "the path is shown to a person: {why}"
    );
    assert!(
        !why.contains("sock"),
        "the socket is shown to a person: {why}"
    );
    assert_eq!(desk.doing_sentence(), "MCF is not answering");
}

#[test]
fn every_menu_entry_reaches_something_built() {
    let named: Vec<&str> = Page::MENU.iter().map(|(_, label)| *label).collect();
    assert_eq!(
        named,
        ["Server", "Models", "Exit"],
        "the window's places are the server, the library, and Exit"
    );
    for (page, label) in Page::MENU {
        assert_eq!(page.section(), *page, "{label} is not a section of its own");
    }
}

#[test]
fn an_empty_field_asks_for_nothing() {
    let mut desk = Desk::new(std::path::PathBuf::from("/nowhere"));
    desk.page = Page::Adding;
    desk.typed.set("   ");
    desk.look_up();
    assert!(
        matches!(desk.doing, crate::Doing::Nothing),
        "a lookup was started for an empty name"
    );
    desk.models = vec![Model::default()];
    desk.chosen = Some(0);
    desk.ask(0);
    assert!(
        matches!(desk.doing, crate::Doing::Nothing),
        "a question was asked with nothing in it"
    );
}

#[test]
fn only_one_thing_runs_at_a_time() {
    let mut desk = Desk::new(std::path::PathBuf::from("/nowhere"));
    desk.typed.set("owner/repository");
    desk.look_up();
    assert!(matches!(desk.doing, crate::Doing::Listing(_)));
    desk.download("owner/repository", "a-model.gguf");
    assert!(
        matches!(desk.doing, crate::Doing::Downloading(_)),
        "starting a download did not replace what was running"
    );
}

#[test]
fn a_window_is_picked_and_never_cycled() {
    let mut desk = Desk::new(std::path::PathBuf::from("/nowhere"));
    let offered = crate::windows();
    assert!(
        offered.contains(&desk.window),
        "the window it starts on is not one of the ones offered"
    );
    for wanted in offered {
        desk.act(crate::Act::Open(crate::Picker::Window));
        desk.act(crate::Act::SetWindow(wanted));
        assert_eq!(desk.window, wanted);
        assert_eq!(desk.open, None, "picking did not shut the list");
    }
    for held in offered {
        assert!(held.is_power_of_two(), "{held} is not a power of two");
    }
}

#[test]
fn a_pasted_reference_is_taken_as_a_value() {
    let mut desk = Desk::new(std::path::PathBuf::from("/nowhere/control.sock"));
    desk.page = Page::Adding;

    desk.paste("an-owner/a-repository-GGUF");
    assert_eq!(
        desk.typed.said(),
        "an-owner/a-repository-GGUF",
        "a plain reference is kept as it is"
    );

    desk.typed.clear();
    desk.paste("an-owner/a-repository-GGUF\n");
    assert_eq!(desk.typed.said(), "an-owner/a-repository-GGUF");

    desk.typed.clear();
    desk.paste("an-owner/a-repository-GGUF\nand a second line\n");
    assert_eq!(desk.typed.said(), "an-owner/a-repository-GGUF");

    desk.typed.clear();
    desk.paste("\tan-owner/a-repository-GGUF\r");
    assert_eq!(desk.typed.said(), "an-owner/a-repository-GGUF");

    desk.typed.set("an-owner/");
    desk.paste("   \n  ");
    assert_eq!(
        desk.typed.said(),
        "an-owner/",
        "an empty paste changes nothing"
    );

    desk.typed.set("an-owner/");
    desk.paste("a-repository-GGUF");
    assert_eq!(desk.typed.said(), "an-owner/a-repository-GGUF");
}

#[test]
fn a_paste_is_bounded() {
    let mut desk = Desk::new(std::path::PathBuf::from("/nowhere/control.sock"));
    desk.page = Page::Adding;
    desk.paste(&"a".repeat(4096));
    assert_eq!(
        desk.typed.chars().count(),
        512,
        "a paste is capped rather than accepted whole"
    );
    desk.paste("bbbb");
    assert_eq!(desk.typed.chars().count(), 512);
}

#[test]
fn what_is_hosted_carries_where_it_answers() {
    let mut desk = Desk::new(std::path::PathBuf::from("/nowhere/control.sock"));
    assert!(
        desk.hosted.is_none(),
        "nothing is hosted until the daemon says so"
    );
    desk.hosted = Some(crate::Hosted {
        model: "/a/store/an-owner/a-repository/a-file.gguf".to_owned(),
        address: "http://127.0.0.1:8080/v1".to_owned(),
        since: "2026-08-31T00:00:00Z".to_owned(),
        context: Some(8192),
        cache: mcf_core::configuration::CacheType::default(),
        projector: None,
        takes: None,
        api_key: false,
        network_address: None,
        in_use: None,
    });
    let hosting = desk.hosted.as_ref().expect("just set");
    assert_eq!(
        hosting.model.rsplit('/').next(),
        Some("a-file.gguf"),
        "the monitor names what is answering, not where it sits"
    );
    assert_eq!(hosting.context, Some(8192));
}

#[test]
fn a_busy_daemon_is_not_a_missing_one() {
    let mut desk = Desk::new(std::path::PathBuf::from("/nowhere/control.sock"));

    desk.refresh();
    assert!(
        desk.refusal.is_some(),
        "a socket with nothing behind it is a refusal"
    );
    assert!(!desk.busy, "and not merely busy");
    let (word, _) = desk.state_line();
    assert_eq!(word, "NOT UP");

    desk.refusal = None;
    desk.busy = true;
    desk.doing = crate::Doing::Hosting(crate::job::Job::start(
        std::path::Path::new("/nowhere/control.sock"),
        mcf_serve::control::Request::Hosted,
        "holding a-model".to_owned(),
    ));
    let (word, said) = desk.state_line();
    assert_eq!(word, "HOLDING", "it is holding, not down");
    assert!(said.contains("holding a-model"), "{said}");
    assert!(
        said.contains("so far"),
        "how long it has waited, not a promise about how long it will take: {said}"
    );
}

#[test]
fn an_unanswered_poll_keeps_what_was_hosted() {
    let mut desk = Desk::new(std::path::PathBuf::from("/nowhere/control.sock"));
    desk.hosted = Some(crate::Hosted {
        model: "/a/model.gguf".to_owned(),
        address: "http://127.0.0.1:8080/v1".to_owned(),
        since: "2026-09-01T00:00:00Z".to_owned(),
        context: Some(4096),
        cache: mcf_core::configuration::CacheType::default(),
        projector: None,
        takes: None,
        api_key: false,
        network_address: None,
        in_use: None,
    });
    desk.read_hosted();
    assert!(
        desk.hosted.is_some(),
        "an unanswered poll says nothing about what is held"
    );
    assert!(desk.busy, "and it is recorded as busy");
}

#[test]
fn an_unanswered_reading_keeps_the_models_it_had() {
    let mut desk = Desk::new(std::path::PathBuf::from("/nowhere/control.sock"));
    desk.models = vec![
        Model {
            name: "a-model".to_owned(),
            ..Model::default()
        },
        Model {
            name: "another".to_owned(),
            ..Model::default()
        },
    ];
    desk.refresh();
    assert_eq!(
        desk.models.len(),
        2,
        "a reading that did not arrive says nothing about what is held"
    );
}

#[test]
fn a_companion_file_is_never_offered_as_a_model() {
    let entry = |path: &str, companion: bool| {
        Value::map([
            ("path", Value::text(path)),
            ("bytes", Value::Integer(1_000)),
            ("companion", Value::Bool(companion)),
        ])
    };
    let answered = Value::map([(
        "models",
        Value::List(vec![
            entry("/m/a-model.gguf", false),
            entry("/m/mmproj-F16.gguf", true),
        ]),
    )]);
    let listed: Vec<Model> = answered
        .get("models")
        .and_then(Value::as_list)
        .map(|held| {
            held.iter()
                .filter(|entry| !matches!(entry.get("companion"), Some(Value::Bool(true))))
                .map(model_from)
                .collect()
        })
        .unwrap_or_default();
    assert_eq!(listed.len(), 1, "the projector is not a model");
    assert_eq!(
        listed.first().map(|held| held.name.as_str()),
        Some("a-model")
    );
}

#[test]
fn hosting_with_no_engine_builds_the_one_mcf_named() {
    let mut desk = Desk::new(std::path::PathBuf::from("/nowhere"));
    let answered = Value::map([
        ("path", Value::text("/models/small.gguf")),
        ("bytes", Value::Integer(270_000_000)),
        ("runs", Value::map([("architecture", Value::text("llama"))])),
    ]);
    desk.models = vec![model_from(&answered)];
    desk.chosen = Some(0);
    desk.settings = None;
    desk.no_settings = Some("no engine is installed yet — MCF can build one for you".to_owned());
    desk.needs_engine = Some("llama.cpp".to_owned());

    desk.host_it();

    let crate::Doing::Provisioning(job) = &desk.doing else {
        panic!("Host with no engine must build one, not {:?}", desk.doing);
    };
    assert!(job.what.contains("llama.cpp"), "{}", job.what);
    assert!(job.what.contains("small"), "for which model: {}", job.what);
    let (word, said) = {
        desk.busy = true;
        desk.state_line()
    };
    assert_eq!(word, "BUILDING");
    assert!(said.contains("so far"), "{said}");
}

#[test]
fn hosting_with_a_nameless_refusal_builds_nothing() {
    let mut desk = Desk::new(std::path::PathBuf::from("/nowhere"));
    let answered = Value::map([
        ("path", Value::text("/models/small.gguf")),
        ("bytes", Value::Integer(270_000_000)),
        ("runs", Value::map([("architecture", Value::text("llama"))])),
    ]);
    desk.models = vec![model_from(&answered)];
    desk.chosen = Some(0);
    desk.settings = None;
    desk.no_settings = Some("a model whose header does not say how long".to_owned());
    desk.needs_engine = None;

    desk.host_it();

    assert!(
        matches!(desk.doing, crate::Doing::Nothing),
        "{:?}",
        desk.doing
    );
    assert!(desk.no_settings.is_some());
}

#[test]
fn asking_for_a_component_to_be_built_builds_that_one() {
    let mut desk = Desk::new(std::path::PathBuf::from("/nowhere"));
    desk.act(super::Act::Build("llama.cpp".to_owned()));

    let super::Doing::Provisioning(job) = &desk.doing else {
        panic!("Build must build, not {:?}", desk.doing);
    };
    assert_eq!(job.what, "building llama.cpp");
    assert_eq!(desk.building.as_deref(), Some("llama.cpp"));

    let before = desk.doing.job().map(|job| job.what.clone());
    desk.act(super::Act::Build("sdl3".to_owned()));
    let after = desk.doing.job().map(|job| job.what.clone());
    assert!(
        after == before || desk.doing.job().is_some_and(|job| job.finished),
        "{before:?} then {after:?}"
    );
}

#[test]
fn a_refused_build_lands_on_its_own_card() {
    let mut desk = Desk::new(std::path::PathBuf::from("/nowhere"));
    desk.build("llama.cpp");
    let started = std::time::Instant::now();
    while !desk.doing.job().is_some_and(|job| job.finished) {
        assert!(
            started.elapsed() < std::time::Duration::from_secs(10),
            "a job at /nowhere must refuse itself promptly"
        );
        if !desk.hear() {
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
    }
    assert!(desk.building.is_none(), "{:?}", desk.building);
    let (failed, why) = desk.build_failed.clone().expect("the refusal is kept");
    assert_eq!(failed, "llama.cpp");
    assert!(why.contains("not answering"), "{why}");
    desk.build("llama.cpp");
    assert!(desk.build_failed.is_none());
}

#[test]
fn what_is_in_it_is_asked_of_the_daemon_and_never_counted_here() {
    let mut desk = Desk::new(std::path::PathBuf::from("/nowhere/control.sock"));
    desk.act(crate::Act::Go(Page::Anatomy));
    assert_eq!(desk.page, Page::Anatomy);
    assert!(desk.anatomy.is_none());
    let why = desk.no_anatomy.clone().unwrap_or_default();
    assert!(why.contains("Choose a model"), "{why}");

    let mut desk = Desk::new(std::path::PathBuf::from("/nowhere/control.sock"));
    desk.models = vec![Model {
        name: "a-model".to_owned(),
        path: "/nowhere/a-model.gguf".to_owned(),
        bytes: Some(4_000_000_000),
        ..Model::default()
    }];
    desk.chosen = Some(0);
    desk.act(crate::Act::Go(Page::Anatomy));
    assert!(desk.anatomy.is_none());
    let why = desk.no_anatomy.clone().unwrap_or_default();
    assert!(why.contains("not answering"), "{why}");
    assert!(
        !why.contains("sock"),
        "the socket is shown to a person: {why}"
    );

    desk.no_anatomy = None;
    desk.act(crate::Act::Go(Page::Vocabulary));
    assert_eq!(desk.page, Page::Vocabulary);
    assert!(desk.anatomy.is_none());
    let why = desk.no_anatomy.clone().unwrap_or_default();
    assert!(why.contains("not answering"), "{why}");
}

#[test]
fn an_anatomy_answer_is_read_as_the_daemon_wrote_it() {
    let line = r#"{"model":"m.gguf","counted":{"elements":100,"bytes":50,"unsized_tensors":0,"blocks":2,"output_tied":true,"active":null,"parts":[{"part":"embedding","tensors":1,"elements":40,"bytes":20}],"encodings":[{"encoding":"Q4_K","tensors":3,"elements":100,"bytes":50}],"block_shapes":[{"blocks":[0,1],"mixing":"attention","feed":"dense","experts":null,"shared_expert":false,"said":"attention over the context, keys and values kept per position; one feed-forward every token passes","ranged":"0–1","bits_hundredths":[400,400],"tensors":2,"elements":60,"bytes":30}],"attending":2,"recurrent":0},"agreements":[{"what":"blocks","declared":"2","observed":"2","agrees":true}],"work":{"multiply_adds":100,"head_width":8,"queries_per_key":1,"attention_at_context":null,"cache":{"sized":true,"per_token":64,"key_heads":1,"per_head":16,"latent":false,"kept":"16 for a key and a value","context":null,"at_context":null,"sliding_window":null,"attending":2,"blocks":2,"recurrent":0}},"vocabulary":{"tokens":3,"segmentation":"llama","merges":null,"kinds":[{"kind":"text","count":2},{"kind":"control","count":1}],"word_starts":1,"digit_tokens":0,"longest_digits":0,"digits":"none: every digit is spelled some other way","longest":{"token":"▁the","bytes":6},"named":[{"what":"end of text","identifier":2,"spelled":"</s>","beyond":null}],"adds_beginning":true,"beginning":"yes, the file says so","template":null,"no_template":"none in the file — a chat turn has no framing the file states"}}"#;
    let value = mcf_record::json::parse(line).expect("the line parses");
    let said = mcf_serve::anatomy::Said::from_value(&value).expect("the value reads");
    assert_eq!(said.elements, 100);
    assert_eq!(said.families.len(), 1);
    assert_eq!(
        said.agreements.first().and_then(|held| held.agrees),
        Some(true)
    );
    assert!(matches!(
        said.cache,
        mcf_serve::anatomy::SaidCache::Sized { per_token: 64, .. }
    ));
    assert_eq!(said.vocabulary.tokens, 3);
    assert_eq!(said.vocabulary.segmentation, "llama");
    assert_eq!(
        said.vocabulary
            .named
            .first()
            .and_then(|n| n.spelled.as_deref()),
        Some("</s>")
    );
    assert!(said.vocabulary.template.is_err());
}

#[test]
fn a_typed_setting_is_taken_or_refused_with_the_word() {
    let mut desk = Desk::new(std::path::PathBuf::from("/nowhere/control.sock"));
    let recommended = mcf_serve::hosting::Hosting::recommended(
        "llama.cpp",
        "CPU",
        false,
        4096,
        Some(8),
        true,
        None,
    );
    desk.settings = Some(recommended.clone());
    desk.recommended = Some(recommended);
    desk.page = Page::Models;

    desk.edit(crate::Field::Context, crate::ui::Touched::No);
    assert!(
        desk.takes_typing(),
        "a field being typed into takes the keys"
    );
    desk.typing().set("32,768");
    desk.apply_edit();
    assert_eq!(
        desk.settings.as_ref().map(|held| held.context),
        Some(32_768)
    );
    assert!(desk.edit_refused.is_none());

    desk.edit(crate::Field::Context, crate::ui::Touched::No);
    desk.typing().set("lots");
    desk.apply_edit();
    assert_eq!(
        desk.settings.as_ref().map(|held| held.context),
        Some(32_768),
        "a word is not a window, and the window stays as it was"
    );
    let why = desk.edit_refused.clone().unwrap_or_default();
    assert!(
        why.contains("lots"),
        "the refusal names what was typed: {why}"
    );

    desk.edit(crate::Field::Port, crate::ui::Touched::No);
    desk.typing().set("80");
    desk.apply_edit();
    assert!(
        desk.edit_refused
            .as_deref()
            .is_some_and(|why| why.contains("1024")),
        "a port MCF cannot bind without rights is refused, not tried"
    );

    desk.flip(crate::Switch::FlashAttention);
    assert_eq!(
        desk.settings.as_ref().map(|held| held.flash_attention),
        Some(true)
    );
    assert!(
        desk.editing.is_none(),
        "nothing is being typed into a setting now"
    );
    assert!(
        desk.takes_typing(),
        "the library's search field still takes typing (D51)"
    );
}

#[test]
fn a_run_on_the_hosted_model_is_said_to_cost_a_second_copy() {
    let mut desk = Desk::new(std::path::PathBuf::from("/nowhere/control.sock"));
    desk.models = vec![
        Model {
            name: "a".to_owned(),
            path: "/m/a.gguf".to_owned(),
            bytes: Some(17_665_334_432),
            ..Model::default()
        },
        Model {
            name: "b".to_owned(),
            path: "/m/b.gguf".to_owned(),
            bytes: None,
            ..Model::default()
        },
    ];
    desk.chosen = Some(0);
    assert_eq!(desk.second_copy(), None, "nothing hosted, nothing to say");
    desk.hosted = Some(crate::Hosted {
        model: "/m/a.gguf".to_owned(),
        address: "127.0.0.1:8080".to_owned(),
        since: "2026-09-06T13:56:00Z".to_owned(),
        context: Some(4096),
        cache: mcf_core::configuration::CacheType::default(),
        projector: None,
        takes: None,
        api_key: false,
        network_address: None,
        in_use: None,
    });
    let said = desk.second_copy().unwrap_or_default();
    assert!(
        said.contains("lets the hold go") && said.contains("one copy"),
        "a run on the hosted model lets the hold go and takes it up again: {said}"
    );
    desk.chosen = Some(1);
    let said = desk.second_copy().unwrap_or_default();
    assert!(
        said.contains("one model runs at a time") && said.contains("a is hosted"),
        "a run on another model is the rule, said before Run: {said}"
    );
}

#[test]
fn start_server_takes_the_key_being_typed_and_names_the_field_it_needs() {
    let mut desk = Desk::new(std::path::PathBuf::from("/nowhere/control.sock"));
    let recommended = mcf_serve::hosting::Hosting::recommended(
        "llama.cpp",
        "CPU",
        false,
        4096,
        Some(8),
        true,
        None,
    );
    desk.settings = Some(recommended.clone());
    desk.recommended = Some(recommended);
    desk.models = vec![Model {
        name: "a".to_owned(),
        path: "/m/a.gguf".to_owned(),
        ..Model::default()
    }];
    desk.chosen = Some(0);
    desk.page = Page::Models;
    desk.flip(crate::Switch::Open);
    assert!(desk.settings.as_ref().is_some_and(|held| held.open));
    desk.host_it();
    let why = desk.host_refused.clone().unwrap_or_default();
    assert!(why.contains("API key field"), "{why}");
    assert!(
        !matches!(desk.doing, crate::Doing::Hosting(_)),
        "nothing was asked of the daemon"
    );
    desk.edit(crate::Field::ApiKey, crate::ui::Touched::No);
    desk.typing().set("mcf-home");
    desk.host_it();
    assert_eq!(
        desk.settings
            .as_ref()
            .and_then(|held| held.api_key.clone())
            .as_deref(),
        Some("mcf-home"),
        "the key typed is the key the hold gets"
    );
    assert!(
        !desk
            .host_refused
            .as_deref()
            .is_some_and(|why| why.contains("API key field")),
        "with a key, the hold is not refused for one: {:?}",
        desk.host_refused
    );
}

#[test]
fn the_last_hold_settings_come_back_with_one_press() {
    let mut desk = Desk::new(std::path::PathBuf::from("/nowhere"));
    let recommended = mcf_serve::hosting::Hosting::recommended(
        "llama.cpp",
        "CPU",
        false,
        32_768,
        Some(8),
        false,
        None,
    );
    let mut last = recommended.clone();
    last.context = 8_192;
    desk.recommended = Some(recommended.clone());
    desk.settings = Some(recommended.clone());
    desk.last_settings = Some((last.clone(), "2026-09-05".to_owned()));
    desk.act(crate::Act::LastSettings);
    assert_eq!(desk.settings.as_ref().map(|held| held.context), Some(8_192));
    desk.act(crate::Act::Recommended);
    assert_eq!(
        desk.settings.as_ref().map(|held| held.context),
        Some(32_768)
    );
}

#[test]
fn the_search_field_narrows_the_library() {
    let mut desk = Desk::new(std::path::PathBuf::from("/nowhere"));
    let model = |name: &str, architecture: &str| Model {
        name: name.to_owned(),
        path: format!("/store/owner/{name}-GGUF/{name}.gguf"),
        architecture: Some(architecture.to_owned()),
        ..Model::default()
    };
    desk.models = vec![
        model("Assistant-8B-Q4_K_M", "llama"),
        model("Coder-30B-Q4_K_XL", "a-moe-architecture"),
        model("Tiny-135M", "llama"),
    ];
    let members = |desk: &Desk| -> Vec<usize> {
        desk.library()
            .into_iter()
            .flat_map(|group| group.members)
            .collect()
    };
    assert_eq!(members(&desk), vec![0, 1, 2]);
    desk.filter.set("LLAMA");
    assert_eq!(
        members(&desk),
        vec![0, 2],
        "the architecture counts, case aside"
    );
    desk.filter.set("q4_k");
    assert_eq!(members(&desk), vec![0, 1]);
    desk.filter.set("coder xl");
    assert_eq!(members(&desk), vec![1], "every word must match");
    desk.filter.set("gemma");
    assert!(desk.library().is_empty());
    assert!(!desk.hub_matches(), "nothing has been asked of the hub");
    desk.page = Page::Models;
    desk.entered();
    assert!(
        matches!(desk.doing, crate::Doing::Listing(_)),
        "Return on words nothing matches did not search the hub"
    );
}

#[test]
fn the_hubs_answer_is_kept_for_its_words() {
    use mcf_record::json::Value;
    let mut desk = Desk::new(std::path::PathBuf::from("/nowhere"));
    desk.filter.set("gemma");
    let mut job = crate::job::Job::already(
        "searching".to_owned(),
        vec![Value::map([
            ("query", Value::text("gemma")),
            (
                "repositories",
                Value::List(vec![Value::map([
                    ("id", Value::text("someone/gemma-GGUF")),
                    ("downloads", Value::Integer(1_295_081)),
                ])]),
            ),
            ("done", Value::Bool(true)),
        ])],
    );
    job.finished = true;
    desk.doing = crate::Doing::Listing(job);
    desk.hear_for_review();
    let hub = desk.hub.as_ref().expect("the hub's answer is kept");
    assert_eq!(hub.query, "gemma");
    assert_eq!(hub.repositories.len(), 1);
    assert_eq!(hub.repositories[0].downloads, Some(1_295_081));
    assert!(desk.hub_matches());
    desk.filter.set("gemma 4");
    assert!(!desk.hub_matches(), "other words are another question");
}

#[test]
fn a_model_is_a_repository_with_its_quantizations() {
    let mut desk = Desk::new(std::path::PathBuf::from("/nowhere"));
    let held = |file: &str, repository: Option<&str>| Model {
        name: file.trim_end_matches(".gguf").to_owned(),
        path: format!("/store/{file}"),
        file: file.to_owned(),
        repository: repository.map(str::to_owned),
        bytes: Some(1),
        ..Model::default()
    };
    desk.models = vec![
        held("Model-Q4_K_M.gguf", Some("owner/Model-GGUF")),
        held("Other.gguf", None),
        held("Model-Q8_0.gguf", Some("owner/Model-GGUF")),
    ];
    let groups = desk.library();
    assert_eq!(groups.len(), 2, "{groups:?}");
    assert_eq!(groups[0].members, vec![0, 2]);
    assert_eq!(groups[0].name(&desk.models), "Model-GGUF");
    assert_eq!(groups[1].name(&desk.models), "Other");
    desk.chosen = Some(0);
    let _replaced = desk.offered.insert(
        "owner/Model-GGUF".to_owned(),
        vec![
            crate::OfferedFile {
                file: "Model-Q8_0.gguf".to_owned(),
                bytes: Some(2),
                fits: Some(true),
            },
            crate::OfferedFile {
                file: "Model-BF16.gguf".to_owned(),
                bytes: Some(4),
                fits: Some(false),
            },
        ],
    );
    let listed = desk.quantizations();
    let files: Vec<&str> = listed.iter().map(|quant| quant.file.as_str()).collect();
    assert_eq!(
        files,
        vec!["Model-Q4_K_M.gguf", "Model-Q8_0.gguf", "Model-BF16.gguf"],
        "here first, then the hub's, and a file here is not listed twice"
    );
    assert_eq!(desk.quantization_at(), Some(0));
    desk.pick_quantization(1);
    assert_eq!(
        desk.chosen,
        Some(2),
        "a quantization here becomes the subject"
    );
    assert!(desk.pending.is_none());
    desk.pick_quantization(2);
    assert_eq!(
        desk.pending.as_ref().map(|pending| pending.file.as_str()),
        Some("Model-BF16.gguf"),
        "a quantization on the hub is the subject as not downloaded"
    );
    assert_eq!(desk.quantization_at(), Some(2));
    desk.pick_quantization(0);
    assert!(desk.pending.is_none());
    assert_eq!(desk.chosen, Some(0));
}

#[test]
fn a_pick_not_here_downloads_first_and_then_does_the_thing() {
    let mut desk = Desk::new(std::path::PathBuf::from("/nowhere"));
    desk.pending = Some(crate::Pending {
        repository: "owner/Model-GGUF".to_owned(),
        file: "Model-Q8_0.gguf".to_owned(),
        bytes: Some(2),
        fits: Some(true),
    });
    desk.act(crate::Act::DownloadThen(std::boxed::Box::new(
        crate::Act::HostIt,
    )));
    assert!(
        matches!(desk.doing, crate::Doing::Downloading(_)),
        "the download did not go first"
    );
    assert_eq!(desk.after_download, Some(crate::Act::HostIt));
    desk.models = vec![Model {
        name: "Model-Q8_0".to_owned(),
        path: "/store/owner/Model-GGUF/Model-Q8_0.gguf".to_owned(),
        file: "Model-Q8_0.gguf".to_owned(),
        repository: Some("owner/Model-GGUF".to_owned()),
        ..Model::default()
    }];
    desk.settle_download();
    assert_eq!(desk.chosen, Some(0), "the file here is the subject now");
    assert!(desk.pending.is_none());
    assert!(
        desk.after_download.is_none(),
        "the thing named was not done"
    );
    assert_eq!(desk.page, Page::Hosting, "the server was not started");
}

#[test]
fn the_filters_narrow_the_library_with_the_words() {
    let mut desk = Desk::new(std::path::PathBuf::from("/nowhere"));
    let model = |name: &str, architecture: &str, bytes: u64, refused: Option<&str>| Model {
        name: name.to_owned(),
        path: format!("/store/{name}.gguf"),
        architecture: Some(architecture.to_owned()),
        bytes: Some(bytes),
        refused: refused.map(str::to_owned),
        ..Model::default()
    };
    desk.models = vec![
        model("Small", "llama", 5_000_000_000, None),
        model("Large", "llama", 40_000_000_000, Some("too large")),
        model("Moe", "a-moe", 18_000_000_000, None),
    ];
    let members = |desk: &Desk| -> Vec<usize> {
        desk.library()
            .into_iter()
            .flat_map(|group| group.members)
            .collect()
    };
    assert_eq!(desk.architectures(), vec!["a-moe", "llama"]);
    assert!(!desk.filters.any_set());
    desk.act(crate::Act::SetArchitecture(2));
    assert_eq!(desk.filters.architecture.as_deref(), Some("llama"));
    assert_eq!(members(&desk), vec![0, 1]);
    desk.act(crate::Act::SetFits(1));
    assert_eq!(members(&desk), vec![0], "only what will run here");
    desk.act(crate::Act::SetArchitecture(0));
    assert_eq!(members(&desk), vec![0, 2], "any architecture again");
    desk.act(crate::Act::SetSize(2));
    assert_eq!(desk.filters.size, Some(20_000_000_000));
    assert_eq!(members(&desk), vec![0, 2]);
    desk.act(crate::Act::SetSize(1));
    assert_eq!(members(&desk), vec![0], "up to 8 GB");
    desk.filter.set("moe");
    assert!(
        members(&desk).is_empty(),
        "the words and the filters both apply"
    );
    desk.act(crate::Act::SetSize(0));
    desk.act(crate::Act::SetFits(0));
    assert_eq!(members(&desk), vec![2]);
    assert!(!desk.filters.any_set());
    desk.act(crate::Act::ToggleFilters);
    assert!(desk.filters.open);
}

#[test]
fn every_taxonomy_category_renders_with_its_subsystem_and_context() {
    use mcf_core::failure::{Attribution, Category, Disposition, Failure, Subsystem};
    for category in Category::ALL {
        let failure = Failure::new(
            category,
            Attribution::Machine,
            Disposition::Refused,
            Subsystem::new("mcf-serve::daemon"),
            "what happened, in a sentence",
        )
        .with_context("looked_for", "/somewhere/on/the/disk");
        let entry = Value::map([
            ("body", mcf_record::encode::failure(&failure)),
            (
                "recorded_at",
                Value::text("2026-09-07T15:20:04.250382271Z (local offset -07:00)"),
            ),
        ]);
        let fault = super::fault_from(&entry);
        assert_eq!(fault.category, category.code());
        assert_eq!(fault.meaning, category.meaning());
        assert_eq!(fault.at, "2026-09-07T15:20:04");
        let lines = super::fault_lines(&fault);
        assert!(
            lines[0].contains(category.code()) && lines[0].contains(category.meaning()),
            "{}: {lines:?}",
            category.code()
        );
        assert!(
            lines
                .iter()
                .any(|line| line.contains("in mcf-serve::daemon")),
            "{}: the subsystem is not named: {lines:?}",
            category.code()
        );
        assert!(
            lines
                .iter()
                .any(|line| line == "looked_for: /somewhere/on/the/disk"),
            "{}: the context is not there: {lines:?}",
            category.code()
        );
        assert!(
            lines
                .iter()
                .any(|line| line == "what happened, in a sentence"),
            "{}: the detail is not there: {lines:?}",
            category.code()
        );
    }
}

#[test]
fn a_failures_cause_and_axes_are_said_plainly() {
    use mcf_core::failure::{Attribution, Category, Disposition, Failure, Subsystem};
    let cause = Failure::new(
        Category::EngineExitImmediate,
        Attribution::Machine,
        Disposition::Partial,
        Subsystem::new("mcf-serve::served"),
        "the engine left at once",
    );
    let failure = Failure::new(
        Category::EngineSpawnRefused,
        Attribution::User,
        Disposition::Refused,
        Subsystem::new("mcf-serve::daemon"),
        "the hold could not start",
    )
    .caused_by(cause);
    let mut fault = super::fault_from(&mcf_record::encode::failure(&failure));
    fault.asked = "host".to_owned();
    let lines = super::fault_lines(&fault);
    assert!(lines.contains(&"asked: host".to_owned()), "{lines:?}");
    assert!(
        lines[1].starts_with("the operator's doing; refused; in mcf-serve::daemon"),
        "{lines:?}"
    );
    assert_eq!(
        lines.last().map(String::as_str),
        Some("because: engine.exit.immediate — the engine left at once")
    );
    assert_eq!(
        super::fault_lines(&super::Fault::default())[1],
        "unattributed; no disposition; in "
    );
}

#[test]
fn closing_lets_go_of_what_is_held_and_of_nothing_else() {
    let mut desk = Desk::new(std::path::PathBuf::from("/nowhere"));
    assert_eq!(desk.to_let_go(), None);

    desk.hosted = Some(super::Hosted {
        model: "/models/Assistant-2B-Instruct-Q4_K_M.gguf".to_owned(),
        address: "http://127.0.0.1:17817".to_owned(),
        since: String::new(),
        context: Some(8192),
        cache: mcf_core::configuration::CacheType::default(),
        projector: None,
        takes: None,
        api_key: false,
        network_address: None,
        in_use: None,
    });
    assert_eq!(
        desk.to_let_go().as_deref(),
        Some("Assistant-2B-Instruct-Q4_K_M")
    );

    desk.hosted = None;
    desk.under_test = Some(super::UnderTest {
        model: "/models/Assistant-2B-Instruct-Q4_K_M.gguf".to_owned(),
        engine: "provisioned llama.cpp".to_owned(),
        window: Some(4096),
        in_use: super::Use::default(),
    });
    assert_eq!(desk.to_let_go(), None);
}

#[test]
fn a_build_reads_as_its_version_and_the_first_of_its_revision() {
    assert_eq!(
        super::said_of("0.1.0-m0", "52b825a5d1f8685f0a048c99f034225405802c2a"),
        "0.1.0-m0 · 52b825a"
    );
    assert_eq!(super::said_of("0.1.0-m0", "unknown"), "0.1.0-m0");
    assert_eq!(super::said_of("0.1.0-m0", ""), "0.1.0-m0");
    let mine = Desk::build_said();
    assert!(
        mine.starts_with(env!("CARGO_PKG_VERSION")),
        "the window does not name its own version: {mine}"
    );
}

#[test]
fn a_daemon_of_another_age_is_shown_and_one_of_the_same_age_is_not() {
    let mut desk = Desk::new(std::path::PathBuf::from("/nowhere"));
    assert_eq!(desk.daemon_build, None);
    desk.daemon_build = Some(Desk::build_said());
    assert_eq!(
        desk.daemon_build
            .as_ref()
            .filter(|held| **held != Desk::build_said()),
        None,
        "a daemon of this window's own age is not called out"
    );
    desk.daemon_build = Some("0.1.0-m0 · 74ca562".to_owned());
    assert!(
        desk.daemon_build
            .as_ref()
            .is_some_and(|held| *held != Desk::build_said()),
        "a daemon of another age is not shown as one"
    );
}
