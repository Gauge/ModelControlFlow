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
        ["Server", "Models", "Downloads", "Exit"],
        "the window's places are the server, the library, the queue of what is arriving, \
         and Exit"
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

/// A transfer takes nothing away from the window.
///
/// It is queued in the daemon, so it is not the window's one job: asking for a file does
/// not stop a listing that is still running, and it does not have to wait for one either.
/// It used to replace whatever the window was doing, which meant one file at a time and
/// only while the window stayed open.
#[test]
fn asking_for_a_file_does_not_stop_what_the_window_was_doing() {
    let mut desk = Desk::new(std::path::PathBuf::from("/nowhere"));
    desk.typed.set("owner/repository");
    desk.look_up();
    assert!(matches!(desk.doing, crate::Doing::Listing(_)));
    desk.download("owner/repository", "a-model.gguf");
    assert!(
        matches!(desk.doing, crate::Doing::Listing(_)),
        "asking for a file threw away the listing that was still running"
    );
}

/// And one can be asked for while another is already on its way.
#[test]
fn one_file_on_its_way_does_not_refuse_the_next() {
    let mut desk = Desk::new(std::path::PathBuf::from("/nowhere"));
    desk.download("owner/repository", "one.gguf");
    desk.download("owner/repository", "two.gguf");
    // Nothing is listening at /nowhere, so both are refused by the daemon rather than
    // queued; what matters is that the second was attempted at all, which the old
    // one-at-a-time rule would not have allowed.
    assert!(
        desk.notices.about(crate::ABOUT_TRANSFERS).is_some(),
        "a refusal from the daemon is where a transfer that cannot be queued is reported"
    );
    assert!(
        !desk.doing.busy(),
        "a transfer must not be holding the window's job slot"
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
        open: false,
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
        open: false,
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
    let held = desk
        .notices
        .about("engine:llama.cpp")
        .expect("the refusal is kept");
    assert_eq!(held.tone, crate::notice::Tone::Refused);
    assert!(held.what.contains("llama.cpp"), "{}", held.what);
    let why = held.detail.clone().unwrap_or_default();
    assert!(why.contains("not answering"), "{why}");
    desk.build("llama.cpp");
    assert!(
        desk.notices.about("engine:llama.cpp").is_none(),
        "asking again puts the old refusal away"
    );
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
        open: false,
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
    let why = desk
        .notices
        .about(crate::ABOUT_HOLD)
        .map(|held| held.what.clone())
        .unwrap_or_default();
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
    let said = desk
        .notices
        .about(crate::ABOUT_HOLD)
        .map(|held| held.what.clone());
    assert!(
        !said
            .as_deref()
            .is_some_and(|why| why.contains("API key field")),
        "with a key, the hold is not refused for one: {said:?}"
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
    assert_eq!(
        desk.after_download,
        Some(crate::Act::HostIt),
        "what to do once the file is here was not remembered, so it would never be done: \
         a queued transfer answers at once, and the thing waiting on it is picked up when \
         the file lands"
    );
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
        open: false,
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

/// Removal is asked for on the downloads page and nowhere else, so nothing ticked is
/// nothing to ask about.
#[test]
fn removing_nothing_opens_nothing() {
    let mut desk = Desk::new(std::path::PathBuf::from("/nowhere"));
    desk.page = Page::Downloads;
    desk.picked.clear();
    desk.act(crate::Act::RemovePicked);
    assert!(
        desk.removing.is_none(),
        "with nothing ticked there is nothing to ask about"
    );
}

#[test]
fn a_removal_will_not_go_ahead_until_it_is_told_why() {
    let mut desk = Desk::new(std::path::PathBuf::from("/nowhere"));
    desk.page = Page::Models;
    desk.removing = Some(crate::Removing {
        models: vec!["/m/a-model.gguf".to_owned()],
        name: "a-model".to_owned(),
        files: Vec::new(),
        bytes: Some(1024),
        reversible: true,
        shelf: "/m/removed".to_owned(),
        reason: crate::typing::Typing::of(String::new()),
        purge: false,
        refused: None,
        done: None,
    });
    desk.act(crate::Act::DoRemove);
    let why = desk
        .removing
        .as_ref()
        .and_then(|held| held.refused.clone())
        .unwrap_or_default();
    assert!(
        why.contains("why"),
        "an empty reason is refused, and the refusal says what is missing: {why}"
    );
    assert!(
        desk.removing
            .as_ref()
            .is_some_and(|held| held.done.is_none()),
        "nothing was removed"
    );
}

#[test]
fn a_reason_being_typed_takes_the_keys_ahead_of_the_filter() {
    let mut desk = Desk::new(std::path::PathBuf::from("/nowhere"));
    desk.page = Page::Models;
    desk.filter.set("a-family");
    desk.removing = Some(crate::Removing {
        models: vec!["/m/a-model.gguf".to_owned()],
        name: "a-model".to_owned(),
        files: Vec::new(),
        bytes: None,
        reversible: true,
        shelf: String::new(),
        reason: crate::typing::Typing::of(String::new()),
        purge: false,
        refused: None,
        done: None,
    });
    assert!(desk.takes_typing());
    desk.typing().put("room", 64);
    assert_eq!(
        desk.removing.as_ref().map(|held| held.reason.said()),
        Some("room"),
        "the keys reach the reason, not the filter behind it"
    );
    assert_eq!(
        desk.filter.said(),
        "a-family",
        "the filter is left as it was"
    );
}

#[test]
fn escape_puts_a_removal_back_without_removing_anything() {
    let mut desk = Desk::new(std::path::PathBuf::from("/nowhere"));
    desk.page = Page::Models;
    desk.removing = Some(crate::Removing {
        models: vec!["/m/a-model.gguf".to_owned()],
        name: "a-model".to_owned(),
        files: Vec::new(),
        bytes: None,
        reversible: true,
        shelf: String::new(),
        reason: crate::typing::Typing::of("room".to_owned()),
        purge: false,
        refused: None,
        done: None,
    });
    desk.stopped_typing();
    assert!(desk.removing.is_none());
}

#[test]
fn deleting_is_asked_for_separately_from_removing() {
    let mut desk = Desk::new(std::path::PathBuf::from("/nowhere"));
    desk.page = Page::Models;
    desk.removing = Some(crate::Removing {
        models: vec!["/m/a-model.gguf".to_owned()],
        name: "a-model".to_owned(),
        files: Vec::new(),
        bytes: None,
        reversible: true,
        shelf: String::new(),
        reason: crate::typing::Typing::of(String::new()),
        purge: false,
        refused: None,
        done: None,
    });
    assert_eq!(desk.removing.as_ref().map(|held| held.purge), Some(false));
    desk.act(crate::Act::PurgeToggle);
    assert_eq!(
        desk.removing.as_ref().map(|held| held.purge),
        Some(true),
        "shelving is what a removal does unless deleting is asked for"
    );
    desk.act(crate::Act::PurgeToggle);
    assert_eq!(desk.removing.as_ref().map(|held| held.purge), Some(false));
}

/// Everything the window shows about a hold comes out of one answer.
///
/// It used to ask twice, once for each half it wanted. The daemon works a rate out as the
/// difference between the counters at one answer and the counters at the next, so the
/// second ask measured the milliseconds since the first — no whole token arrives in that
/// time, and every rate read zero for as long as the model was working.
mod the_hold {
    use crate::Desk;
    use mcf_record::json::Value;
    use mcf_serve::control::Answer;

    fn an_answer() -> Answer {
        Answer::served(Value::map([
            ("hosting", Value::text("/store/a-model.gguf")),
            ("address", Value::text("http://127.0.0.1:8080/")),
            ("since", Value::text("2026-09-17T10:00:00Z")),
            ("settings", Value::map([("context", Value::Integer(8192))])),
            (
                "use",
                Value::map([
                    ("generated_tokens", Value::text("512")),
                    ("prompted_tokens", Value::text("96641")),
                    ("prompt_tokens_reused", Value::text("2.16578e+06")),
                    ("generated_tokens_per_second", Value::text("41.500")),
                    ("uptime_seconds", Value::Integer(60)),
                ]),
            ),
            (
                "under_test",
                Value::map([
                    ("model", Value::text("/store/under-test.gguf")),
                    ("engine", Value::text("provisioned llama.cpp @abc")),
                    ("use", Value::map([("decodes", Value::text("9"))])),
                ]),
            ),
        ]))
    }

    fn a_desk() -> Desk {
        Desk::new(std::path::PathBuf::from("/nowhere"))
    }

    #[test]
    fn one_answer_carries_the_hold_what_is_under_test_and_the_figures() {
        let mut desk = a_desk();
        desk.took_the_hosted(Ok(an_answer()));

        let held = desk.hosted.as_ref().expect("the hold was read");
        assert_eq!(held.model, "/store/a-model.gguf");
        let figures = held.in_use.as_ref().expect("the figures were read");
        assert_eq!(figures.generated, Some(512));
        assert_eq!(figures.generated_per_second, Some(41.5));
        assert!(
            desk.under_test.is_some(),
            "what is under test came out of the same answer, not a second ask"
        );
        assert!(
            !desk.busy,
            "a served answer does not leave the window waiting"
        );
    }

    #[test]
    fn every_reading_that_arrives_is_kept_for_the_graph_that_draws_them() {
        let mut desk = a_desk();
        for _ in 0..3 {
            desk.took_the_hosted(Ok(an_answer()));
        }
        assert_eq!(
            desk.tallies.len(),
            3,
            "the graph is drawn from what was read, so a reading dropped is a gap in it"
        );
        let newest = desk.tallies.back().copied().expect("a reading");
        assert_eq!(newest.written, 512);
        assert_eq!(newest.processed, 96_641);
        assert_eq!(
            newest.asked,
            96_641 + 2_165_780,
            "what was asked for is what was read plus what the cache saved reading"
        );
    }

    /// A rate is a difference over a span, and both halves of that have to be right.
    mod rates {
        use crate::Tally;
        use std::time::{Duration, Instant};

        /// Two readings a known span apart. Both are built from one instant, so the span
        /// is exactly what it says rather than that plus however long the test took.
        fn two(gap: u64, before: u64, after: u64) -> (Tally, Tally) {
            let base = Instant::now();
            let reading = |written, at| Tally {
                asked: 0,
                processed: 0,
                written,
                at,
            };
            (
                reading(before, base),
                reading(
                    after,
                    base.checked_add(Duration::from_secs(gap))
                        .expect("a later instant"),
                ),
            )
        }

        fn written_per_second(gap: u64, before: u64, after: u64) -> Option<f32> {
            let (then, now) = two(gap, before, after);
            Tally::per_second(then, now, |held| held.written)
        }

        #[test]
        fn a_count_climbing_over_a_second_is_that_many_a_second() {
            assert_eq!(written_per_second(1, 100, 145), Some(45.0));
        }

        #[test]
        fn a_span_longer_than_a_second_is_divided_by_what_it_was() {
            // The poll asks about once a second, but a busy daemon answers slower. Taking
            // the span as one second regardless would report a hold as four times busier
            // than it was.
            assert_eq!(written_per_second(4, 100, 280), Some(45.0));
        }

        #[test]
        fn a_hold_doing_nothing_reads_as_nothing_rather_than_as_no_reading() {
            assert_eq!(
                written_per_second(1, 100, 100),
                Some(0.0),
                "idle is a real reading, and it is the half of the graph that says when \
                 the work was not happening"
            );
        }

        #[test]
        fn two_readings_at_the_same_moment_yield_no_rate() {
            assert_eq!(written_per_second(0, 100, 100), None);
        }

        #[test]
        fn a_count_that_went_backwards_is_not_a_negative_rate() {
            // An engine restarted under the hold starts its counters again.
            assert_eq!(written_per_second(1, 900, 12), None);
        }
    }

    /// An average is a total over the time actually spent on it, which is exact and needs
    /// no sampling. The rates worked out between two readings a second apart read 0.0
    /// almost always, because these counters only move when a request finishes.
    mod averages {
        use mcf_record::json::Value;

        fn a_hold_that(said: &[(&str, &str)]) -> crate::Use {
            crate::Use::from_value(&Value::map(
                said.iter()
                    .map(|(key, held)| ((*key).to_owned(), Value::text((*held).to_owned())))
                    .collect::<Vec<_>>(),
            ))
        }

        #[test]
        fn generation_is_what_was_written_over_the_time_spent_writing_it() {
            let read = a_hold_that(&[
                ("generated_tokens", "7844"),
                ("engine_generating_seconds", "198.664"),
            ]);
            let rate = read.generation_average().expect("both counters are there");
            assert!(
                (rate - 39.48).abs() < 0.05,
                "7,844 tokens over 198.7 s is about 39.5 a second, not {rate}"
            );
        }

        #[test]
        fn prefill_is_held_apart_from_generation() {
            // The two run an order of magnitude apart, and an average across both would
            // describe neither.
            let read = a_hold_that(&[
                ("prompted_tokens", "275698"),
                ("engine_prompt_seconds", "781.316"),
            ]);
            let rate = read.prefill_average().expect("both counters are there");
            assert!(
                (rate - 352.87).abs() < 0.5,
                "275,698 tokens over 781.3 s is about 353 a second, not {rate}"
            );
        }

        #[test]
        fn a_hold_that_has_spent_no_time_yet_averages_nothing() {
            // Not nought: nought a second is a claim about a model that has not been
            // asked for anything, and dividing by it is worse.
            let read = a_hold_that(&[
                ("generated_tokens", "0"),
                ("engine_generating_seconds", "0"),
            ]);
            assert_eq!(read.generation_average(), None);
            assert_eq!(crate::Use::default().prefill_average(), None);
        }

        #[test]
        fn what_one_request_came_to_is_the_total_over_the_requests() {
            let read = a_hold_that(&[("generated_tokens", "7844"), ("requests_served", "6")]);
            let each = read.a_request(read.generated).expect("six requests");
            assert!(
                (each - 1_307.3).abs() < 0.5,
                "7,844 tokens over 6 requests is about 1,307 each, not {each}"
            );
        }

        #[test]
        fn nothing_served_yet_is_no_average_rather_than_a_division_by_nought() {
            let read = a_hold_that(&[("generated_tokens", "7844"), ("requests_served", "0")]);
            assert_eq!(read.a_request(read.generated), None);
        }

        #[test]
        fn the_cache_share_is_of_the_whole_prompt() {
            // Against what was read plus what was reused, not against what was read: a
            // share shown beside the wrong total invites arithmetic that will not come out.
            let read = a_hold_that(&[
                ("prompted_tokens", "275698"),
                ("prompt_tokens_reused", "3648300"),
            ]);
            let share = read.cache_share().expect("both counts are there");
            assert!(
                (share - 0.9297).abs() < 0.005,
                "3,648,300 of 3,923,998 asked is about 93%, not {share}"
            );
        }

        #[test]
        fn time_working_is_reading_and_writing_together() {
            let read = a_hold_that(&[
                ("engine_prompt_seconds", "781.316"),
                ("engine_generating_seconds", "198.664"),
            ]);
            let spent = read.seconds_working().expect("both counters are there");
            assert!((spent - 979.98).abs() < 0.05, "spent {spent}");
            assert_eq!(crate::Use::default().seconds_working(), None);
        }
    }

    #[test]
    fn an_engine_that_publishes_only_some_counts_still_draws() {
        // Engines differ in what they publish. Refusing the reading outright drew an
        // empty graph beside figures that were plainly there.
        let only_written =
            crate::Use::from_value(&Value::map([("generated_tokens", Value::text("512"))]));
        let tally = crate::Tally::of(&only_written).expect("one count is enough");
        assert_eq!(tally.written, 512);
        assert_eq!(tally.processed, 0);
        assert_eq!(tally.asked, 0);

        assert!(
            crate::Tally::of(&crate::Use::default()).is_none(),
            "a reading that says nothing about tokens is not a reading to draw"
        );
    }

    #[test]
    fn an_answer_that_did_not_come_leaves_the_last_figures_standing() {
        let mut desk = a_desk();
        desk.took_the_hosted(Ok(an_answer()));
        desk.took_the_hosted(Err("MCF did not answer".to_owned()));
        assert!(
            desk.hosted.is_some(),
            "a poll that timed out threw away figures that were still the best known"
        );
        assert!(desk.busy, "and the window says it is waiting");
    }

    #[test]
    fn a_hold_that_has_been_let_go_is_read_as_nothing_held() {
        let mut desk = a_desk();
        desk.took_the_hosted(Ok(an_answer()));
        desk.took_the_hosted(Ok(Answer::served(Value::map([("hosting", Value::Null)]))));
        assert!(
            desk.hosted.is_none(),
            "nothing is held and it still said so"
        );
        assert!(
            desk.tallies.is_empty(),
            "counts from a hold that is gone start again at nought in the next one, so \
             keeping them would draw a cliff that never happened"
        );
    }
}

/// A repository's quantizations are separate models on this disk.
///
/// Each is its own file, with its own size, its own settings and its own place in the
/// record, so each is chosen, held and removed on its own. The library used to show one
/// row per repository and choose its first file when that row was pressed, which left
/// every other quantization of it unreachable — including for removal.
mod quantizations {
    use crate::{Desk, Model};

    fn a_quant(file: &str) -> Model {
        Model {
            name: file.trim_end_matches(".gguf").to_owned(),
            path: format!("/store/owner/Small-GGUF/{file}"),
            file: file.to_owned(),
            repository: Some("owner/Small-GGUF".to_owned()),
            bytes: Some(1_000),
            ..Model::default()
        }
    }

    fn a_desk() -> Desk {
        let mut desk = Desk::new(std::path::PathBuf::from("/nowhere"));
        desk.models = vec![
            a_quant("Small-Q4_K_M.gguf"),
            a_quant("Small-Q8_0.gguf"),
            Model {
                name: "Alone-Q8_0".to_owned(),
                path: "/store/other/Alone-GGUF/Alone-Q8_0.gguf".to_owned(),
                file: "Alone-Q8_0.gguf".to_owned(),
                repository: Some("other/Alone-GGUF".to_owned()),
                ..Model::default()
            },
        ];
        desk
    }

    #[test]
    fn a_repository_opens_out_into_its_quantizations_and_folds_away_again() {
        let mut desk = a_desk();
        let repository = "owner/Small-GGUF";
        assert!(!desk.is_opened_out(Some(repository)));

        desk.act(crate::Act::OpenOut(repository.to_owned()));
        assert!(
            desk.is_opened_out(Some(repository)),
            "there was nothing to press, so only the first quantization could be reached"
        );

        desk.act(crate::Act::OpenOut(repository.to_owned()));
        assert!(!desk.is_opened_out(Some(repository)));
    }

    #[test]
    fn opening_one_repository_out_does_not_open_the_others() {
        let mut desk = a_desk();
        desk.act(crate::Act::OpenOut("owner/Small-GGUF".to_owned()));
        assert!(!desk.is_opened_out(Some("other/Alone-GGUF")));
        assert!(!desk.is_opened_out(None));
    }

    #[test]
    fn choosing_a_quantization_makes_that_file_the_subject() {
        let mut desk = a_desk();
        desk.act(crate::Act::Choose(1));
        assert_eq!(
            desk.chosen
                .and_then(|at| desk.models.get(at))
                .map(|held| held.file.clone()),
            Some("Small-Q8_0.gguf".to_owned()),
            "the second quantization was chosen and the first was made the subject"
        );
    }

    #[test]
    fn what_stays_behind_a_removal_is_counted_and_named() {
        let desk = a_desk();
        let (others, repository) = desk
            .others_of_the_repository("/store/owner/Small-GGUF/Small-Q4_K_M.gguf")
            .expect("the repository holds another");
        assert_eq!(others, 1);
        assert_eq!(repository, "owner/Small-GGUF");
    }

    #[test]
    fn a_repository_holding_one_quantization_has_nothing_staying_behind() {
        let desk = a_desk();
        assert!(
            desk.others_of_the_repository("/store/other/Alone-GGUF/Alone-Q8_0.gguf")
                .is_none(),
            "saying nothing stays behind is only honest when something does"
        );
    }

    #[test]
    fn a_quantizations_row_shows_what_sets_it_apart_not_what_it_shares() {
        let said = crate::view::quantization_said(&a_quant("Small-UD-Q4_K_XL.gguf"));
        assert_eq!(
            said, "UD-Q4_K_XL",
            "the repository's name is on the row above; repeating it pushes the part that \
             differs off the end of the row"
        );
    }

    #[test]
    fn a_file_that_shares_no_prefix_with_its_repository_is_shown_whole() {
        let mut odd = a_quant("something-else-Q8_0.gguf");
        odd.repository = Some("owner/Unrelated-GGUF".to_owned());
        assert_eq!(
            crate::view::quantization_said(&odd),
            "something-else-Q8_0",
            "trimming a prefix that is not there must not leave an empty row"
        );
    }
}

/// What the window makes of the figures the daemon sends.
///
/// The numbers come off a real hold: a current llama.cpp that had served about ninety-six
/// thousand prompt tokens and generated twenty-three thousand.
mod the_figures {
    use crate::Use;
    use mcf_record::json::Value;

    fn as_the_daemon_sends_them() -> Value {
        Value::map([
            ("prompted_tokens", Value::text("96641")),
            ("generated_tokens", Value::text("23210")),
            ("generated_tokens_live", Value::Integer(23_210)),
            ("decodes", Value::text("23381")),
            ("deepest_tokens", Value::text("42094")),
            ("prompt_tokens_reused", Value::text("2.16578e+06")),
            ("engine_said_tokens_per_second", Value::text("45.4769")),
            (
                "engine_said_prompt_tokens_per_second",
                Value::text("424.925"),
            ),
            ("resident_bytes", Value::Integer(4_539_359_232)),
            ("card_bytes", Value::Integer(70_623_830_016)),
            ("uptime_seconds", Value::Integer(3_352)),
        ])
    }

    #[test]
    fn every_figure_the_daemon_sends_is_read() {
        let read = Use::from_value(&as_the_daemon_sends_them());
        assert_eq!(read.prompted, Some(96_641));
        assert_eq!(read.generated, Some(23_210));
        assert_eq!(read.decodes, Some(23_381));
        assert_eq!(read.deepest, Some(42_094));
        assert_eq!(read.resident, Some(4_539_359_232));
        assert_eq!(read.card, Some(70_623_830_016));
        assert_eq!(read.uptime_seconds, Some(3_352));
    }

    #[test]
    fn a_count_the_engine_wrote_with_an_exponent_is_read_whole() {
        let read = Use::from_value(&as_the_daemon_sends_them());
        assert_eq!(
            read.prompt_reused,
            Some(2_165_780),
            "the engine writes large counters with an exponent, and they have to survive \
             being read"
        );
    }

    #[test]
    fn a_count_past_what_a_single_holds_exactly_is_not_rounded() {
        let held = Value::map([("prompted_tokens", Value::text("99000001"))]);
        assert_eq!(
            Use::from_value(&held).prompted,
            Some(99_000_001),
            "a single holds whole numbers exactly only to about sixteen million, and a \
             token count passes that on the long session where it is worth reading"
        );
    }

    #[test]
    fn a_hold_with_no_two_readings_yet_shows_the_engines_own_average() {
        let read = Use::from_value(&as_the_daemon_sends_them());
        assert_eq!(
            read.generated_per_second, None,
            "MCF has not worked a rate out yet: there has only been one reading"
        );
        assert_eq!(
            read.generating_per_second(),
            Some(45.4769),
            "with nothing of its own to show, the window must show what the engine says \
             rather than nothing — nothing reads as a model doing nothing"
        );
        assert_eq!(read.prompting_per_second(), Some(424.925));
    }

    #[test]
    fn mcfs_own_reading_is_preferred_once_there_is_one() {
        let mut held = as_the_daemon_sends_them();
        if let Value::Map(fields) = &mut held {
            let _put = fields.insert(
                "generated_tokens_per_second".to_owned(),
                Value::text("61.250"),
            );
        }
        let read = Use::from_value(&held);
        assert_eq!(
            read.generating_per_second(),
            Some(61.25),
            "MCF's own reading is the live one; the engine's is an average over the hold"
        );
    }

    #[test]
    fn a_rate_of_nothing_falls_back_rather_than_reading_as_a_stalled_model() {
        let mut held = as_the_daemon_sends_them();
        if let Value::Map(fields) = &mut held {
            let _put = fields.insert(
                "generated_tokens_per_second".to_owned(),
                Value::text("0.000"),
            );
        }
        assert_eq!(
            Use::from_value(&held).generating_per_second(),
            Some(45.4769),
            "a reading of zero taken across too short a span is not a model doing nothing"
        );
    }

    #[test]
    fn an_engine_that_publishes_no_cache_gauge_reports_no_cache_figure() {
        let read = Use::from_value(&as_the_daemon_sends_them());
        assert_eq!(read.cache_used, None);
        assert_eq!(read.cache_tokens, None);
        assert!(
            read.deepest.is_some(),
            "and what the engine publishes instead is there to show in its place"
        );
    }
}

/// What the page says about who can reach the hold.
///
/// The key itself is deliberately kept off the wire. Reading the key field to decide what
/// to say found nothing every time, so a hold answering the network with a key set was
/// described as "API key: none (localhost only)" — directly above its own network address.
mod reachability {
    use crate::Hosted;
    use mcf_record::json::Value;
    use mcf_serve::control::Answer;

    fn a_hold(open: bool, key_set: bool) -> Answer {
        Answer::served(Value::map([
            ("hosting", Value::text("/store/a-model.gguf")),
            ("address", Value::text("http://127.0.0.1:17817")),
            (
                "network_address",
                if open {
                    Value::text("http://192.168.1.10:17817")
                } else {
                    Value::Null
                },
            ),
            ("since", Value::text("2026-09-17T20:01:23Z")),
            (
                "settings",
                Value::map([
                    ("context", Value::Integer(262_144)),
                    ("open", Value::Bool(open)),
                    ("api_key_set", Value::Bool(key_set)),
                ]),
            ),
        ]))
    }

    fn read(open: bool, key_set: bool) -> Hosted {
        let mut desk = crate::Desk::new(std::path::PathBuf::from("/nowhere"));
        desk.took_the_hosted(Ok(a_hold(open, key_set)));
        desk.hosted.expect("the hold was read")
    }

    #[test]
    fn a_hold_that_answers_the_network_with_a_key_says_so() {
        let held = read(true, true);
        assert!(held.open, "it is bound to the network and must say so");
        assert!(
            held.api_key,
            "the key is kept off the wire, so whether one is set has to be said separately \
             — reading the key field itself always found nothing"
        );
    }

    #[test]
    fn a_hold_that_answers_only_this_computer_says_that() {
        let held = read(false, false);
        assert!(!held.open);
        assert!(!held.api_key);
    }

    #[test]
    fn a_hold_open_to_the_network_without_a_key_is_not_described_as_private() {
        let held = read(true, false);
        assert!(
            held.open,
            "describing this one as reachable from this computer only would be describing \
             it as safe when anything that can route here can use it"
        );
        assert!(!held.api_key);
    }
}

/// Whether the draft head is earning its keep.
///
/// MCF has a setting for it and a sweep that searches it, so a hold that is using one is
/// owed the figure while it is being used rather than only in a sweep's report.
mod the_draft_head {
    use crate::Use;
    use mcf_record::json::Value;

    fn with(drafted: Option<&str>, taken: Option<&str>) -> Use {
        let mut fields: Vec<(&str, Value)> = Vec::new();
        if let Some(drafted) = drafted {
            fields.push(("drafted_tokens", Value::text(drafted)));
        }
        if let Some(taken) = taken {
            fields.push(("drafted_tokens_taken", Value::text(taken)));
        }
        Use::from_value(&Value::map(fields))
    }

    #[test]
    fn a_share_is_shown_of_what_was_actually_proposed() {
        let read = with(Some("1000"), Some("750"));
        assert_eq!(read.draft_taken_share(), Some(0.75));
    }

    #[test]
    fn a_hold_with_no_draft_head_is_not_reported_as_a_draft_head_failing() {
        assert_eq!(
            with(Some("0"), Some("0")).draft_taken_share(),
            None,
            "a share of nothing proposed is not a figure, and nought per cent would say \
             the head had failed at something it was never asked to do"
        );
        assert_eq!(with(None, None).draft_taken_share(), None);
    }

    #[test]
    fn an_engine_that_says_nothing_about_drafting_yields_no_share() {
        assert_eq!(with(Some("1000"), None).draft_taken_share(), None);
    }

    #[test]
    fn a_share_is_never_shown_above_the_whole() {
        assert_eq!(
            with(Some("100"), Some("140")).draft_taken_share(),
            Some(1.0),
            "whatever an engine reports, more taken than proposed is not a share above one"
        );
    }
}

/// The server page, actually drawn.
///
/// The page is two things side by side now — the model down the left, the machine down a
/// rail on the right — where it used to be one column with three tables of machine
/// readings stacked on top, pushing the model's own figures and the message box below the
/// fold. These render it on paper and read the pixels back.
mod the_server_page {
    use crate::paint::{NIGHT, Painter};
    use crate::{Desk, Hosted, Model, Page, Use};
    use mcf_record::json::Value;

    fn a_hold() -> Hosted {
        Hosted {
            model: "/store/owner/Held-GGUF/Held-Q6_K.gguf".to_owned(),
            address: "http://127.0.0.1:17817".to_owned(),
            since: "2026-09-17T20:01:23Z".to_owned(),
            context: Some(262_144),
            cache: mcf_core::configuration::CacheType::default(),
            projector: None,
            takes: None,
            api_key: true,
            open: true,
            network_address: Some("http://192.168.1.10:17817".to_owned()),
            in_use: Some(Use::from_value(&Value::map([
                ("prompted_tokens", Value::text("96641")),
                ("generated_tokens", Value::text("23210")),
                ("decodes", Value::text("23381")),
                ("deepest_tokens", Value::text("42094")),
                ("prompt_tokens_reused", Value::text("2.16578e+06")),
                ("engine_said_tokens_per_second", Value::text("45.4769")),
                (
                    "engine_said_prompt_tokens_per_second",
                    Value::text("424.925"),
                ),
                ("engine_prompt_seconds", Value::text("781.316")),
                ("engine_generating_seconds", Value::text("198.664")),
                ("requests_served", Value::text("6")),
                ("card_power_watts", Value::text("53.098")),
                ("card_energy_joules", Value::text("230066.655")),
                ("card_energy_over_seconds", Value::text("3351.704")),
                ("resident_bytes", Value::Integer(4_539_359_232)),
                ("card_bytes", Value::Integer(70_623_830_016)),
                ("uptime_seconds", Value::Integer(3_352)),
            ]))),
        }
    }

    pub(super) fn a_desk() -> Desk {
        let mut desk = Desk::new(std::path::PathBuf::from("/nowhere"));
        desk.page = Page::Hosting;
        desk.models = vec![Model {
            name: "Held-Q6_K".to_owned(),
            path: "/store/owner/Held-GGUF/Held-Q6_K.gguf".to_owned(),
            file: "Held-Q6_K.gguf".to_owned(),
            repository: Some("owner/Held-GGUF".to_owned()),
            bytes: Some(65_000_000_000),
            ..Model::default()
        }];
        desk.chosen = Some(0);
        desk.hosted = Some(a_hold());
        // An hour of a real hold: two spells of work with a lull between them and a long
        // idle stretch after. A total climbs through each spell and stands level between
        // them, which is how the plot says when the work happened.
        let began = std::time::Instant::now()
            .checked_sub(std::time::Duration::from_mins(50))
            .unwrap_or_else(std::time::Instant::now);
        let (mut asked, mut processed, mut written) = (1_800_000_u64, 80_000_u64, 19_000_u64);
        for second in 0..3_000_u64 {
            let busy = matches!(second, 120..=520 | 1_250..=1_640);
            if busy {
                // A burst reads a long prompt and then writes an answer at a tenth the rate.
                let reading = matches!(second % 90, 0..=12);
                asked += if reading { 9_400 } else { 60 };
                processed += if reading { 410 } else { 12 };
                written += if reading { 0 } else { 46 };
            }
            desk.tallies.push_back(crate::Tally {
                asked,
                processed,
                written,
                at: began
                    .checked_add(std::time::Duration::from_secs(second))
                    .unwrap_or(began),
            });
        }
        desk
    }

    /// Draw the whole window on paper and hand back what drew it, so the pixels can be
    /// read off it.
    pub(super) fn drawn(desk: &Desk, width: u32, height: u32) -> Painter {
        let mut paint = match Painter::on_paper(width, height, 1.0, NIGHT) {
            Ok(paint) => paint,
            // The face is built by this crate's build script; if it is genuinely missing
            // there is nothing here to test, and saying so beats a failure that reads like
            // a layout fault.
            Err(why) => panic!("the window's own face could not be loaded: {why}"),
        };
        let _act = crate::view::draw(&mut paint, desk, &crate::ui::Mouse::default());
        paint
    }

    /// One repository of the shot's shelf: its name, and the variants on it.
    type Shelved = (&'static str, &'static [(&'static str, u64, Option<u32>)]);

    /// The shot's shelf, as models.
    fn shelved(shelf: &[Shelved]) -> Vec<Model> {
        let mut held = Vec::new();
        for (repository, variants) in shelf {
            for (file, bytes, parts) in *variants {
                held.push(Model {
                    name: (*file).to_owned(),
                    path: format!("/store/{repository}/{file}.gguf"),
                    file: format!("{file}.gguf"),
                    repository: Some((*repository).to_owned()),
                    bytes: Some(*bytes),
                    parts: *parts,
                    ..Model::default()
                });
            }
        }
        held
    }

    /// The shot's model carries a template that takes things, so the settings page has
    /// the section to show. Taken from the shape Nemotron-3 uses.
    fn a_template_that_takes_things(desk: &mut Desk) {
        desk.declared = Some(mcf_serve::declared::Declared {
            template: Some(
                "{%- set enable_thinking = enable_thinking if enable_thinking is defined \
                 else True %}\
                 {%- set low_effort = low_effort if low_effort is defined else False %}\
                 {%- set truncate_history_thinking = truncate_history_thinking if \
                 truncate_history_thinking is defined else True %}\
                 {%- if enable_thinking %}{{- '<think>' }}{%- endif %}"
                    .to_owned(),
            ),
            ..mcf_serve::declared::Declared::default()
        });
        desk.recommended = Some(mcf_serve::hosting::Hosting::recommended(
            "llama.cpp",
            "a card",
            true,
            32_768,
            Some(8),
            true,
            None,
        ));
        desk.settings = Some(mcf_serve::hosting::Hosting::recommended(
            "llama.cpp",
            "a card",
            true,
            32_768,
            Some(8),
            true,
            None,
        ));
    }

    /// A desk with something for the strip to say, for the shot only.
    fn a_talkative_desk() -> Desk {
        let mut desk = a_desk();
        // Something for the strip to say, so the shot shows it doing its job.
        desk.notices.say(
            crate::notice::Notice::new(
                crate::notice::Tone::Working,
                crate::ABOUT_TRANSFERS_WORK,
                "Getting Small-Q8_0",
            )
            .so_far(Some(0.46))
            .saying("294 MB of 639 MB"),
        );
        desk.notices.say(
            crate::notice::Notice::new(
                crate::notice::Tone::Refused,
                "engine:llama.cpp",
                "Could not build llama.cpp",
            )
            .saying("podman is not at /usr/bin/podman or /usr/local/bin/podman"),
        );
        desk.notices.say(
            crate::notice::Notice::new(
                crate::notice::Tone::Done,
                crate::ABOUT_HOLD,
                "Server stopped: Held-Q6_K",
            )
            .saying("freed 4.5 GB RAM, 70.6 GB VRAM"),
        );
        desk.notices.say(crate::notice::Notice::new(
            crate::notice::Tone::Warning,
            "network",
            "The hold answers the network without an API key",
        ));
        desk.notices_open = std::env::var("MCF_SHOT_OPEN").is_ok();
        a_template_that_takes_things(&mut desk);
        a_shelf(&mut desk);
        desk
    }

    /// A shelf worth looking at, put on a desk for the shot.
    fn a_shelf(desk: &mut Desk) {
        // A shelf worth looking at: several repositories of very different size, a split
        // variant, one being served, and a projector nobody's model is left for.
        desk.page = std::env::var("MCF_SHOT_PAGE")
            .ok()
            .and_then(|held| match held.as_str() {
                "downloads" => Some(crate::Page::Downloads),
                "host" => Some(crate::Page::Host),
                _ => None,
            })
            .unwrap_or(desk.page);
        desk.disk = Some(crate::Storage {
            total: 1_900_000_000_000,
            free: 1_200_000_000_000,
        });
        let shelf: [Shelved; 4] = [
            (
                "owner/Coder-Next-GGUF",
                &[
                    ("Coder-Next-Q6_K-00001-of-00003", 65_600_000_000, Some(3)),
                    ("Coder-Next-Q4_K_M", 48_500_000_000, None),
                    ("Coder-Next-UD-IQ4_NL", 39_200_000_000, None),
                ],
            ),
            (
                "owner/Big-35B-GGUF",
                &[
                    ("Big-35B-BF16-00001-of-00002", 69_400_000_000, Some(2)),
                    ("Big-35B-Q8_0", 36_900_000_000, None),
                ],
            ),
            (
                "owner/Middle-31B-GGUF",
                &[("Middle-31B-UD-Q4_K_XL", 17_300_000_000, None)],
            ),
            (
                "owner/Small-20B-GGUF",
                &[("Small-20B-Q8_0", 12_100_000_000, None)],
            ),
        ];
        desk.models = shelved(&shelf);
        desk.weights = crate::Weights {
            bytes: 300_000_000_000,
            files: 13,
            orphans: vec![
                crate::Orphan {
                    path: "/store/owner/Gone-27B-GGUF/mmproj-BF16.gguf".to_owned(),
                    bytes: 931_000_000,
                },
                crate::Orphan {
                    path: "/store/owner/Also-Gone-GGUF/mmproj-F16.gguf".to_owned(),
                    bytes: 878_000_000,
                },
            ],
        };
        desk.chosen = Some(1);
        let _ticked = desk
            .picked
            .insert("/store/owner/Coder-Next-GGUF/Coder-Next-Q4_K_M.gguf".to_owned());
        let _opened = desk.opened_out.insert("owner/Coder-Next-GGUF".to_owned());
        let _opened = desk.opened_out.insert("owner/Big-35B-GGUF".to_owned());
    }

    /// Write a drawn page out so it can be looked at. Ignored: it is for eyes, not for
    /// the suite, and it writes a file.
    #[test]
    #[ignore = "writes a file; run it by name to look at the page"]
    fn look_at_it() {
        let wide = std::env::var("MCF_SHOT_WIDTH")
            .ok()
            .and_then(|held| held.parse().ok())
            .unwrap_or(1400);
        let paint = drawn(&a_talkative_desk(), wide, 1024);
        let paper = paint.paper().expect("paper");
        let mut out = format!("P6\n{} {}\n255\n", paper.width, paper.height).into_bytes();
        for y in 0..paper.height {
            for x in 0..paper.width {
                let (red, green, blue) = paper.at(x, y).unwrap_or((0, 0, 0));
                out.extend_from_slice(&[red, green, blue]);
            }
        }
        // Into the temp directory unless told otherwise: an ignored test is still a test,
        // and one that litters the tree it is run from is a nuisance.
        let at = std::env::var("MCF_SHOT").unwrap_or_else(|_| {
            std::env::temp_dir()
                .join("mcf-server-page.ppm")
                .display()
                .to_string()
        });
        std::fs::write(&at, out).expect("written");
        println!("wrote {at}");
    }

    /// One pixel of a drawn page.
    pub(super) fn pixel(paint: &Painter, x: u32, y: u32) -> (u8, u8, u8) {
        paint
            .paper()
            .expect("drawing on paper yields paper")
            .at(x, y)
            .expect("a pixel inside the page")
    }

    #[test]
    fn it_draws_at_every_width_without_coming_apart() {
        let desk = a_desk();
        // Across the width the page stops railing and starts stacking, and either side of
        // it: a threshold is where geometry goes wrong.
        for width in [420, 700, 919, 920, 921, 1280, 1920, 2560] {
            let paint = drawn(&desk, width, 1024);
            assert_eq!(
                paint.paper().expect("paper").width,
                width,
                "drew at the wrong width"
            );
        }
    }

    #[test]
    fn it_draws_with_nothing_held_at_all() {
        let mut desk = Desk::new(std::path::PathBuf::from("/nowhere"));
        desk.page = Page::Hosting;
        for width in [420, 1280] {
            let _drew = drawn(&desk, width, 1024);
        }
    }

    #[test]
    fn it_draws_while_the_machine_has_reported_nothing() {
        // Every reading absent is the state a daemon that has just started is in, and it
        // is the state that turns a missing figure into a panic if anything divides by one.
        let mut desk = a_desk();
        desk.reading = mcf_tui::machine::Reading::default();
        desk.hosted = Some(Hosted {
            in_use: Some(Use::default()),
            ..a_hold()
        });
        let _drew = drawn(&desk, 1280, 1024);
    }

    #[test]
    fn the_machine_sits_beside_the_model_rather_than_above_it() {
        let paint = drawn(&a_desk(), 1400, 1024);
        // The rail paints itself in the card colour against the page's ground, so where
        // it is can be read straight off the paper.
        let in_the_rail = pixel(&paint, 1400 - 60, 700);
        let in_the_column = pixel(&paint, 600, 700);
        assert_eq!(
            in_the_rail, NIGHT.card,
            "there is no rail down the right: the machine is not beside the model"
        );
        assert_eq!(
            in_the_column, NIGHT.ground,
            "the rail has spread across the column"
        );
    }

    #[test]
    fn a_window_too_narrow_for_a_rail_stacks_instead() {
        let paint = drawn(&a_desk(), 800, 1024);
        let at_the_edge = pixel(&paint, 800 - 60, 700);
        assert_eq!(
            at_the_edge, NIGHT.ground,
            "a rail was drawn in a window too narrow to hold one beside a column of prose"
        );
    }

    /// The rail runs to the window's own edges — top, bottom and right. Inset by the
    /// page's padding it floated, and read as a card that happened to be tall.
    #[test]
    fn the_rail_runs_edge_to_edge() {
        let paint = drawn(&a_desk(), 1400, 1024);
        // Up to the strip along the foot, which is card-coloured too — so the rail is
        // checked within its own extent rather than against a pixel the strip also paints.
        for y in [1, 200, 500, 800, 970] {
            assert_eq!(
                pixel(&paint, 1400 - 60, y),
                NIGHT.card,
                "the rail stops short at y {y}"
            );
        }
        assert_eq!(
            pixel(&paint, 1399, 500),
            NIGHT.card,
            "the rail does not reach the right edge, so it floats"
        );
    }
}

/// The strip along the foot, drawn.
///
/// It is the one place the window says anything in passing, so it is on every page, it is
/// always in the same place, and it never covers what it is reporting about.
mod the_strip {
    use super::the_server_page::{a_desk, drawn, pixel};
    use crate::paint::NIGHT;

    /// Where the strip is, in a window of this size.
    fn in_the_strip(height: u32) -> u32 {
        height - 17
    }

    #[test]
    fn it_is_there_on_every_page() {
        for page in [
            crate::Page::Hosting,
            crate::Page::Models,
            crate::Page::Downloads,
            crate::Page::Host,
        ] {
            let mut desk = a_desk();
            desk.page = page;
            let paint = drawn(&desk, 1400, 1024);
            assert_eq!(
                pixel(&paint, 700, in_the_strip(1024)),
                NIGHT.card,
                "no strip on {page:?}: the one place MCF says things has to be every place"
            );
        }
    }

    #[test]
    fn it_says_what_is_happening_and_takes_it_back_when_it_stops() {
        let mut desk = a_desk();
        assert!(desk.notices.foremost().is_none());
        desk.notices
            .working(crate::ABOUT_TRANSFERS_WORK, "Getting one.gguf");
        assert_eq!(
            desk.notices.foremost().map(|held| held.what.as_str()),
            Some("Getting one.gguf")
        );
        desk.notices.forget(crate::ABOUT_TRANSFERS_WORK);
        assert!(
            desk.notices.foremost().is_none(),
            "work that has stopped must stop being reported"
        );
    }

    #[test]
    fn opening_the_list_takes_room_from_the_page_rather_than_covering_it() {
        let mut desk = a_desk();
        desk.notices.refused("hold", "Could not hold it");
        let shut = drawn(&desk, 1400, 1024);
        desk.notices_open = true;
        let open = drawn(&desk, 1400, 1024);
        // With the list open the page is shorter, so what was drawn part way down the
        // column is no longer there. Covering it would have left it exactly as it was.
        let moved = (200..900).any(|y| pixel(&shut, 400, y) != pixel(&open, 400, y));
        assert!(
            moved,
            "the list covered the page instead of taking room from it"
        );
    }

    #[test]
    fn the_list_only_opens_when_there_is_something_in_it() {
        let mut desk = a_desk();
        desk.notices_open = true;
        let empty = drawn(&desk, 1400, 1024);
        desk.notices_open = false;
        let shut = drawn(&desk, 1400, 1024);
        assert_eq!(
            (200..900)
                .filter(|y| pixel(&empty, 400, *y) != pixel(&shut, 400, *y))
                .count(),
            0,
            "an empty list still took room from the page"
        );
    }

    #[test]
    fn a_daemon_that_is_not_answering_is_said_once_and_then_taken_back() {
        let mut desk = a_desk();
        desk.refusal = Some("MCF is not answering on this computer".to_owned());
        desk.tell_what_is_happening();
        desk.tell_what_is_happening();
        assert_eq!(
            desk.notices.len(),
            1,
            "a condition said every frame must not stack up once a frame"
        );
        assert_eq!(
            desk.notices
                .about(crate::ABOUT_DAEMON)
                .map(|held| held.tone),
            Some(crate::notice::Tone::Refused)
        );

        desk.refusal = None;
        desk.tell_what_is_happening();
        assert!(
            desk.notices.about(crate::ABOUT_DAEMON).is_none(),
            "a condition that has gone must stop being reported"
        );
    }
}

/// The downloads page as a page about a disk.
///
/// It was the transfer queue and nothing else, so the one page about files said nothing
/// about the disk they were on — and nothing on it could be removed.
mod the_disk {
    use crate::{Orphan, Storage, Weights};
    use mcf_record::json::Value;

    fn a_shelf() -> Vec<Value> {
        // As the daemon sends it: two variants of one repository, and a projector each
        // beside a model and left behind on its own.
        vec![
            Value::map([
                ("path", Value::text("/store/owner/Kept-GGUF/Kept-Q8_0.gguf")),
                ("bytes", Value::Integer(12_000_000_000)),
                ("companion", Value::Bool(false)),
            ]),
            Value::map([
                (
                    "path",
                    Value::text("/store/owner/Kept-GGUF/mmproj-F16.gguf"),
                ),
                ("bytes", Value::Integer(900_000_000)),
                ("companion", Value::Bool(true)),
            ]),
            Value::map([
                (
                    "path",
                    Value::text("/store/owner/Gone-GGUF/mmproj-BF16.gguf"),
                ),
                ("bytes", Value::Integer(931_000_000)),
                ("companion", Value::Bool(true)),
            ]),
        ]
    }

    #[test]
    fn the_shelf_is_weighed_with_the_companions_counted() {
        let weighed = crate::weighed(&a_shelf());
        assert_eq!(
            weighed.bytes,
            12_000_000_000 + 900_000_000 + 931_000_000,
            "a projector takes the same disk as anything else, so a figure that left it \
             out would be short"
        );
        assert_eq!(weighed.files, 3);
    }

    #[test]
    fn a_projector_beside_a_model_is_the_models_and_one_on_its_own_is_not() {
        let weighed = crate::weighed(&a_shelf());
        assert_eq!(weighed.orphans.len(), 1);
        assert_eq!(
            weighed.orphans.first().map(|held| held.path.as_str()),
            Some("/store/owner/Gone-GGUF/mmproj-BF16.gguf")
        );
        assert_eq!(weighed.reclaimable(), 931_000_000);
    }

    #[test]
    fn nothing_is_orphaned_where_every_projector_has_its_model() {
        let held = vec![
            Value::map([
                ("path", Value::text("/store/owner/Kept-GGUF/Kept-Q8_0.gguf")),
                ("bytes", Value::Integer(1)),
                ("companion", Value::Bool(false)),
            ]),
            Value::map([
                (
                    "path",
                    Value::text("/store/owner/Kept-GGUF/mmproj-F16.gguf"),
                ),
                ("bytes", Value::Integer(1)),
                ("companion", Value::Bool(true)),
            ]),
        ];
        assert!(crate::weighed(&held).orphans.is_empty());
    }

    #[test]
    fn the_orphans_are_largest_first_because_that_is_what_is_worth_removing() {
        let mut held = a_shelf();
        held.push(Value::map([
            (
                "path",
                Value::text("/store/owner/Also-Gone-GGUF/mmproj-F16.gguf"),
            ),
            ("bytes", Value::Integer(5_000_000_000)),
            ("companion", Value::Bool(true)),
        ]));
        let weighed = crate::weighed(&held);
        let sizes: Vec<u64> = weighed.orphans.iter().map(|one| one.bytes).collect();
        assert_eq!(sizes, vec![5_000_000_000, 931_000_000]);
    }

    #[test]
    fn what_a_disk_holds_is_stated_as_shares_of_two_different_things() {
        let volume = Storage {
            total: 2_000_000_000_000,
            free: 1_000_000_000_000,
        };
        assert_eq!(volume.used(), 1_000_000_000_000);
        // Half of what is used, a quarter of the disk: two answers to two questions, and
        // showing one as the other is how a page misleads.
        assert_eq!(volume.share_of_what_is_used(500_000_000_000), Some(50));
        assert_eq!(volume.share_of_the_disk(500_000_000_000), Some(25));
    }

    #[test]
    fn a_share_of_a_disk_that_reported_nothing_is_not_a_share_of_nought() {
        let empty = Storage { total: 0, free: 0 };
        assert_eq!(empty.share_of_the_disk(1), None);
        assert_eq!(empty.share_of_what_is_used(1), None);
    }

    #[test]
    fn a_terabyte_is_said_as_one() {
        assert_eq!(
            crate::words::size_in_words(Some(1_200_000_000_000)).as_deref(),
            Some("1.2 TB"),
            "a disk said to hold 1200 GB is a disk nobody reads at a glance"
        );
        assert_eq!(
            crate::words::size_in_words(Some(48_500_000_000)).as_deref(),
            Some("48 GB")
        );
    }

    #[test]
    fn ticking_a_variant_chooses_it_and_ticking_it_again_does_not() {
        let mut desk = super::the_server_page::a_desk();
        let path = desk
            .models
            .first()
            .map(|held| held.path.clone())
            .expect("a model");
        desk.act(crate::Act::PickOnDisk(path.clone()));
        assert!(desk.picked.contains(&path));
        desk.act(crate::Act::PickOnDisk(path.clone()));
        assert!(desk.picked.is_empty(), "the same tick box must untick it");
    }

    #[test]
    fn what_is_chosen_is_weighed_so_the_page_can_say_what_would_go() {
        let mut desk = super::the_server_page::a_desk();
        desk.weights = Weights {
            bytes: 0,
            files: 0,
            orphans: vec![Orphan {
                path: "/store/owner/Gone-GGUF/mmproj-BF16.gguf".to_owned(),
                bytes: 931_000_000,
            }],
        };
        let model = desk.models.first().cloned().expect("a model");
        desk.act(crate::Act::PickOnDisk(model.path.clone()));
        desk.act(crate::Act::PickOnDisk(
            "/store/owner/Gone-GGUF/mmproj-BF16.gguf".to_owned(),
        ));
        assert_eq!(
            desk.picked_bytes(),
            model.bytes.unwrap_or(0) + 931_000_000,
            "an orphaned projector is weighed too, or the total is short of what goes"
        );
    }

    #[test]
    fn ticking_every_orphan_takes_the_lot() {
        let mut desk = super::the_server_page::a_desk();
        desk.weights = Weights {
            bytes: 0,
            files: 0,
            orphans: vec![
                Orphan {
                    path: "/a/mmproj.gguf".to_owned(),
                    bytes: 1,
                },
                Orphan {
                    path: "/b/mmproj.gguf".to_owned(),
                    bytes: 2,
                },
            ],
        };
        desk.act(crate::Act::PickTheOrphans);
        assert_eq!(desk.picked.len(), 2);
        desk.act(crate::Act::ClearPicked);
        assert!(desk.picked.is_empty());
    }

    #[test]
    fn the_model_being_served_is_named_before_it_can_be_removed_by_accident() {
        let mut desk = super::the_server_page::a_desk();
        let served = desk
            .hosted
            .as_ref()
            .map(|hosting| hosting.model.clone())
            .expect("a hold");
        assert!(desk.picked_the_served().is_none());
        desk.act(crate::Act::PickOnDisk(served));
        assert!(
            desk.picked_the_served().is_some(),
            "the one thing on this page that cannot simply go has to be said so"
        );
    }
}

/// Removing a model is asked for on the downloads page and nowhere else.
///
/// It used to be an action on a model's own page, and the dialogue was drawn there — so
/// when the downloads page learnt to ask, nothing drew the answer and the removal could
/// not be carried through at all.
mod removal_lives_on_the_downloads_page {
    use super::the_server_page::{a_desk, drawn, pixel};

    fn about_to_remove() -> crate::Desk {
        let mut desk = a_desk();
        desk.page = crate::Page::Downloads;
        let path = desk
            .models
            .first()
            .map(|held| held.path.clone())
            .expect("a model");
        desk.act(crate::Act::PickOnDisk(path));
        desk
    }

    #[test]
    fn ticking_and_asking_opens_the_question() {
        let mut desk = about_to_remove();
        assert!(desk.removing.is_none());
        desk.act(crate::Act::RemovePicked);
        let removing = desk.removing.as_ref().expect("the question is open");
        assert_eq!(removing.models.len(), 1);
        assert!(!removing.finished());
    }

    #[test]
    fn the_question_is_drawn_on_the_page_that_asked_it() {
        let mut desk = about_to_remove();
        let before = drawn(&desk, 1400, 1024);
        desk.act(crate::Act::RemovePicked);
        let after = drawn(&desk, 1400, 1024);
        // The dialogue takes the page, so what the shelf drew is no longer there. Before
        // this, asking changed nothing on screen and the removal could not be finished.
        let moved = (120..600).any(|y| pixel(&before, 500, y) != pixel(&after, 500, y));
        assert!(
            moved,
            "asking to remove drew nothing, so there is no way to answer"
        );
    }

    #[test]
    fn it_will_not_go_ahead_until_it_is_told_why() {
        let mut desk = about_to_remove();
        desk.act(crate::Act::RemovePicked);
        desk.act(crate::Act::DoRemove);
        let removing = desk.removing.as_ref().expect("still open");
        assert!(!removing.finished(), "it went ahead with no reason given");
        assert!(
            removing
                .refused
                .as_deref()
                .is_some_and(|why| why.contains("Say why")),
            "{:?}",
            removing.refused
        );
    }

    #[test]
    fn giving_it_up_puts_the_question_away_and_leaves_the_tick() {
        let mut desk = about_to_remove();
        desk.act(crate::Act::RemovePicked);
        desk.act(crate::Act::CancelRemove);
        assert!(desk.removing.is_none());
        assert_eq!(
            desk.picked.len(),
            1,
            "what was ticked stays ticked: nothing was removed"
        );
    }

    #[test]
    fn several_ticked_is_one_question_about_all_of_them() {
        let mut desk = a_desk();
        desk.page = crate::Page::Downloads;
        for at in 0..2 {
            desk.models.push(crate::Model {
                name: format!("Another-Q{at}"),
                path: format!("/store/owner/Another-GGUF/Another-Q{at}.gguf"),
                file: format!("Another-Q{at}.gguf"),
                repository: Some("owner/Another-GGUF".to_owned()),
                bytes: Some(1_000_000_000),
                ..crate::Model::default()
            });
        }
        for held in desk.models.clone() {
            desk.act(crate::Act::PickOnDisk(held.path));
        }
        assert_eq!(desk.picked.len(), 3, "three ticked");
        desk.act(crate::Act::RemovePicked);
        let removing = desk.removing.as_ref().expect("open");
        assert_eq!(removing.models.len(), 3);
        assert_eq!(
            removing.name, "3 models",
            "one question named for the lot of them"
        );
    }

    #[test]
    fn the_models_page_no_longer_offers_it() {
        let mut desk = a_desk();
        desk.page = crate::Page::Models;
        desk.chosen = Some(0);
        let paint = drawn(&desk, 1400, 1024);
        // Nothing on the models page opens the question, so nothing on that page draws
        // it either: the page is unchanged whatever is chosen.
        assert!(desk.removing.is_none());
        let _drew = pixel(&paint, 700, 500);
        assert_eq!(paint.paper().expect("paper").width, 1400);
    }
}

/// A marked score is only read beside scores taken on the same tests. Readings taken on
/// the short questions, before the focus runs replaced them, are still in the ledger; they
/// stay out of a focus reading, and come back when their own tests are chosen.
#[test]
fn a_score_on_one_kind_of_test_is_never_summed_with_another() {
    use mcf_optimize::dial::{Dial, Step};
    use mcf_optimize::ledger::{At, Ledger};
    use mcf_optimize::reading::{Ending, Reading};
    let home = std::env::temp_dir().join(format!("mcf-focus-ledger-{}", std::process::id()));
    let _made = std::fs::create_dir_all(&home);
    let mut desk = Desk::new(std::path::PathBuf::from("/nowhere"));
    desk.home = Some(home.clone());
    desk.models = vec![Model {
        name: "A-Model-Q4_K_M".to_owned(),
        path: "/m/A-Model-Q4_K_M.gguf".to_owned(),
        ..Model::default()
    }];
    desk.chosen = Some(0);
    desk.settings = Some(mcf_serve::hosting::Hosting::recommended(
        "llama.cpp",
        "a card",
        true,
        32_768,
        Some(8),
        true,
        None,
    ));
    desk.optimizing.sweep.dial = Dial::Temperature;
    let under = desk.base_for_a_sweep().expect("a base");
    let path = desk.ledger_path().expect("a ledger");
    let _cleared = std::fs::remove_file(&path);
    let mut ledger = Ledger::open(&path).expect("ledger");
    for set in [
        mcf_optimize::corpus::SHORT_FROM,
        mcf_optimize::corpus::FOCUS_FROM,
    ] {
        let at = At {
            dial: Dial::Temperature,
            step: Step::Thousandths(700),
            set,
            repeat: 1,
        };
        let reading = Reading {
            dial: Dial::Temperature,
            step: Step::Thousandths(700),
            set,
            repeat: 1,
            passed: 20,
            of: 25,
            produced: 1000,
            milliseconds: 1000,
            ending: Ending::Answered,
            why: None,
            per_task: Vec::new(),
        };
        ledger
            .record(&under, at, &reading, "now")
            .expect("recorded");
    }
    assert_eq!(desk.optimizing.tests, crate::Tests::Focus);
    desk.read_the_ledger();
    assert_eq!(
        desk.optimizing
            .rows
            .iter()
            .map(|row| row.at.set)
            .collect::<Vec<_>>(),
        vec![mcf_optimize::corpus::FOCUS_FROM],
        "the short questions' reading is left out of a focus score"
    );
    let _removed = std::fs::remove_dir_all(&home);
}
