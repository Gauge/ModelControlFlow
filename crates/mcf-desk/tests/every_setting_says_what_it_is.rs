#![allow(clippy::panic, clippy::expect_used)]

use mcf_serve::hosting::Hosting;

fn settings() -> Hosting {
    Hosting::recommended("llama.cpp", "a card", true, 32_768, Some(8), true, None)
}

/// Every name the Configure tab puts in front of a person, in the order it draws them.
const SHOWN: &[&str] = &[
    "Device",
    "Context window",
    "Cache width",
    "Split mode",
    "Experts",
    "Main device",
    "Devices",
    "Dense layers on the processor",
    "Tensors placed by hand",
    "Cache in system memory",
    "Memory lock",
    "How it loads",
    "Large tensors",
    "Prompt batch",
    "Micro-batch",
    "Threads",
    "Threads for reading a prompt",
    "Flash attention",
    "Thinking budget",
    "Thinking level",
    "Temperature",
    "Top-p",
    "Top-k",
    "Draft depth",
    "Prompt cache",
    "Prompt cache memory",
    "Prefix reuse",
    "Keep idle slots",
    "Checkpoints",
    "Checkpoint spacing",
    "Context shift",
    "Tokens kept in front",
    "Name callers use",
    "Conversations at once",
    "Port",
    "Reachable from the network",
    "Answer kind",
    "Pooling",
    "API key",
];

fn because_of(name: &str) -> Option<&'static str> {
    let held = settings();
    held.listed(&held)
        .into_iter()
        .find(|setting| setting.name.eq_ignore_ascii_case(name))
        .map(|setting| setting.because)
}

#[test]
fn every_setting_the_window_shows_has_something_to_say_about_itself() {
    let mut silent = Vec::new();
    for name in SHOWN {
        match because_of(name) {
            Some(because) if !because.trim().is_empty() => {}
            _ => silent.push(*name),
        }
    }
    assert!(
        silent.is_empty(),
        "a setting a person can change and nothing explains is a setting they have to guess \
         at: {silent:?}"
    );
}

#[test]
fn a_description_is_a_sentence_rather_than_a_restatement_of_the_name() {
    for name in SHOWN {
        let because = because_of(name).unwrap_or_default();
        assert!(
            because.split_whitespace().count() >= 6,
            "{name:?} says only {because:?}"
        );
        assert!(
            !because.eq_ignore_ascii_case(name),
            "{name:?} explains itself with its own name"
        );
    }
}

#[test]
fn the_names_are_short_enough_to_read_at_a_glance() {
    for name in SHOWN {
        assert!(
            name.split_whitespace().count() <= 5,
            "{name:?} is a sentence, not a label"
        );
        assert!(!name.ends_with('.'), "{name:?}");
    }
}

#[test]
fn nothing_is_named_twice() {
    let mut seen: Vec<String> = SHOWN.iter().map(|name| name.to_lowercase()).collect();
    seen.sort();
    let all = seen.len();
    seen.dedup();
    assert_eq!(all, seen.len(), "two rows share a name");
}

#[test]
fn the_thinking_budget_is_among_the_settings_a_person_can_set() {
    assert!(SHOWN.contains(&"Thinking budget"));
    let because = because_of("Thinking budget").unwrap_or_default();
    assert!(
        because.contains("engine"),
        "the budget is counted by the engine, and saying so is the point of it: {because}"
    );
}

#[test]
fn a_setting_named_in_the_window_matches_the_one_the_record_names() {
    let held = settings();
    let named: Vec<&str> = held.listed(&held).into_iter().map(|one| one.name).collect();
    for name in SHOWN {
        assert!(
            named.iter().any(|held| held.eq_ignore_ascii_case(name)),
            "{name:?} is drawn by the window and known to nothing else"
        );
    }
}

#[test]
fn every_setting_the_optimizer_dials_can_also_be_set_by_hand() {
    for dial in mcf_optimize::dial::Dial::ALL {
        let wanted = match dial {
            mcf_optimize::dial::Dial::ThinkingBudget => "Thinking budget",
            mcf_optimize::dial::Dial::ThinkingLevel => "Thinking level",
            mcf_optimize::dial::Dial::Temperature => "Temperature",
            mcf_optimize::dial::Dial::TopP => "Top-p",
            mcf_optimize::dial::Dial::TopK => "Top-k",
            mcf_optimize::dial::Dial::MicroBatch => "Micro-batch",
            mcf_optimize::dial::Dial::Batch => "Prompt batch",
            mcf_optimize::dial::Dial::CacheWidth => "Cache width",
            mcf_optimize::dial::Dial::Experts => "Experts",
            mcf_optimize::dial::Dial::FlashAttention => "Flash attention",
            mcf_optimize::dial::Dial::ThreadsForAPrompt => "Threads for reading a prompt",
            mcf_optimize::dial::Dial::DraftDepth => "Draft depth",
        };
        assert!(
            SHOWN.contains(&wanted),
            "{} can be dialled in and not set, which leaves nowhere to put the answer",
            dial.label()
        );
        assert!(
            because_of(wanted).is_some(),
            "{wanted:?} is drawn and explained by nothing"
        );
    }
}

#[test]
fn the_thinking_level_is_a_setting_a_person_can_choose_before_hosting() {
    let because = because_of("Thinking level").expect("the level explains itself");
    assert!(
        because.contains("request") || because.contains("template"),
        "the level is asked of the model rather than enforced, and saying so is the point: \
         {because}"
    );
}

#[test]
fn the_level_reaches_the_engine_as_a_flag_of_its_own() {
    let started = mcf_serve::declared::Started {
        effort: Some("high".to_owned()),
        ..mcf_serve::declared::Started::default()
    };
    let said = started.arguments();
    let at = said
        .iter()
        .position(|held| held == "--reasoning-effort")
        .expect("the flag is written");
    assert_eq!(
        said.get(at.saturating_add(1)).map(String::as_str),
        Some("high")
    );
}

#[test]
fn no_level_asked_for_means_no_flag_written() {
    let started = mcf_serve::declared::Started::default();
    assert!(
        !started
            .arguments()
            .iter()
            .any(|held| held == "--reasoning-effort"),
        "leaving it alone leaves the model's own default alone"
    );
}

#[test]
fn the_template_box_starts_on_the_one_the_model_was_published_with() {
    let mut desk = mcf_desk::Desk::new(std::path::PathBuf::from("/tmp/mcf-template-test.sock"));
    desk.declared = Some(mcf_serve::declared::Declared {
        template: Some("{{ the file's own }}".to_owned()),
        ..mcf_serve::declared::Declared::default()
    });
    desk.settings = Some(Hosting::recommended(
        "llama.cpp",
        "a card",
        true,
        4096,
        Some(8),
        true,
        None,
    ));
    assert_eq!(
        desk.template_now(),
        "{{ the file's own }}",
        "a model is addressed the way it was published to be addressed until somebody says \
         otherwise"
    );
    assert!(desk.template_is_the_model_s_own());

    desk.act(mcf_desk::Act::Edit(
        mcf_desk::Field::ChatTemplate,
        mcf_desk::ui::Touched::At(0),
    ));
    desk.typing()
        .set("{%- if a, b %}\n  mine_own\n{%- endif %}");
    desk.apply_edit();
    assert_eq!(
        desk.template_now(),
        "{%- if a, b %}\n  mine_own\n{%- endif %}",
        "commas, underscores and newlines all mean something in Jinja, and the box that \
         takes numbers strips every one of them"
    );
    assert!(!desk.template_is_the_model_s_own());

    desk.act(mcf_desk::Act::TemplateAsPublished);
    assert_eq!(desk.template_now(), "{{ the file's own }}");
    assert!(
        desk.settings
            .as_ref()
            .is_some_and(|held| held.template.is_none()),
        "put back means nothing of its own, so nothing is written out and the engine reads \
         the file"
    );
}

/// Off is MCF's own word for no thinking at all, and two crates hold it because neither
/// depends on the other. They have to hold the same word: a template that checks its own
/// vocabulary refuses the whole request over one it does not know, which is how a hold made
/// with `--reasoning-effort off` answered every trial of every setting in seven milliseconds
/// with nothing in it.
#[test]
fn the_word_for_no_thinking_is_the_same_word_in_both_places() {
    assert_eq!(
        mcf_optimize::dial::Dial::OFF,
        mcf_serve::thinking::OFF,
        "the sweep and the engine flags would disagree about what off means"
    );
}

#[test]
fn mcf_s_own_word_for_no_thinking_never_reaches_a_template() {
    let held = mcf_serve::declared::Started {
        effort: Some(mcf_serve::thinking::OFF.to_owned()),
        ..mcf_serve::declared::Started::default()
    };
    let said = held.arguments();
    assert!(
        !said.iter().any(|flag| flag == "--reasoning-effort"),
        "no template knows this word: one that checks its vocabulary raises on it and \
         refuses the request, and one that does not falls back to its own default: {said:?}"
    );

    let real = mcf_serve::declared::Started {
        effort: Some("low".to_owned()),
        ..mcf_serve::declared::Started::default()
    };
    assert!(
        real.arguments()
            .iter()
            .any(|flag| flag == "--reasoning-effort"),
        "and a level the model does name goes through"
    );
}

#[test]
fn choosing_off_by_hand_turns_it_off_the_way_a_sweep_does() {
    let mut desk = mcf_desk::Desk::new(std::path::PathBuf::from("/tmp/mcf-off-test.sock"));
    desk.declared = Some(mcf_serve::declared::Declared {
        thinking: mcf_serve::thinking::Thinking::in_template(
            "{% if reasoning_effort not in ('xhigh','medium','low') %}{{ raise_exception('no') }}\
             {% endif %}<think>",
        ),
        ..mcf_serve::declared::Declared::default()
    });
    desk.settings = Some(Hosting::recommended(
        "llama.cpp",
        "a card",
        true,
        4096,
        Some(8),
        true,
        None,
    ));
    let levels = desk.levels_of_the_model();
    let at = levels
        .iter()
        .position(|held| held == mcf_serve::thinking::OFF)
        .expect("a template that opens a thinking section is offered off");
    desk.act(mcf_desk::Act::ThinkingLevel(at + 1));
    let started = &desk.settings.as_ref().expect("settings").started;
    assert_eq!(
        started.effort, None,
        "off is not a level to name, it is a budget of nothing"
    );
    assert_eq!(started.thinking, Some(0));
    assert!(
        !started
            .arguments()
            .iter()
            .any(|flag| flag == "--reasoning-effort"),
        "{:?}",
        started.arguments()
    );
}

/// What a model's own chat template will read becomes controls, named as the template
/// names them, and what is set is handed to the engine as the template's arguments.
mod what_the_template_takes {
    use super::settings;
    use mcf_desk::{Desk, Page, Switch};

    /// Nemotron-3's shape: a switch that is on unless asked off, one that is off unless
    /// asked on, and a third about the history.
    fn a_desk_holding_a_template_that_takes_things() -> Desk {
        let mut desk = Desk::new(std::path::PathBuf::from("/nowhere"));
        desk.page = Page::Host;
        desk.declared = Some(mcf_serve::declared::Declared {
            template: Some(
                "{%- set enable_thinking = enable_thinking if enable_thinking is defined \
                 else True %}\
                 {%- set low_effort = low_effort if low_effort is defined else False %}\
                 {%- if enable_thinking %}{{- '<think>' }}{%- endif %}"
                    .to_owned(),
            ),
            ..mcf_serve::declared::Declared::default()
        });
        desk.settings = Some(settings());
        desk
    }

    #[test]
    fn the_template_is_read_for_what_it_takes() {
        let desk = a_desk_holding_a_template_that_takes_things();
        let names: Vec<String> = desk
            .template_takes()
            .into_iter()
            .map(|held| held.name)
            .collect();
        assert_eq!(names, vec!["enable_thinking", "low_effort"]);
    }

    /// Nothing is sent until somebody asks for something, so a hold set up and left alone
    /// is addressed exactly as the model's own template would address it.
    #[test]
    fn nothing_is_sent_until_something_is_asked_for() {
        let desk = a_desk_holding_a_template_that_takes_things();
        let started = &desk.settings.as_ref().expect("settings").started;
        assert!(started.template_taken.is_empty());
        assert!(
            !started
                .arguments()
                .iter()
                .any(|flag| flag == "--chat-template-kwargs")
        );
    }

    #[test]
    fn turning_one_the_other_way_is_what_gets_sent() {
        let mut desk = a_desk_holding_a_template_that_takes_things();
        // The second is off unless asked on, so one turn asks for it on.
        desk.act(mcf_desk::Act::Switch(Switch::TemplateTakes(1)));
        let started = &desk.settings.as_ref().expect("settings").started;
        assert_eq!(
            started.template_taken,
            vec![("low_effort".to_owned(), mcf_record::json::Value::Bool(true))]
        );
        let said = started.arguments();
        let at = said
            .iter()
            .position(|held| held == "--chat-template-kwargs")
            .expect("handed to the engine");
        assert!(
            said.get(at + 1)
                .is_some_and(|held| held.contains("low_effort")),
            "{said:?}"
        );
    }

    /// Turning it back to what the template does on its own stops sending it, so leaving
    /// a control alone and setting it to its own default are the same thing.
    #[test]
    fn turning_it_back_stops_sending_it() {
        let mut desk = a_desk_holding_a_template_that_takes_things();
        desk.act(mcf_desk::Act::Switch(Switch::TemplateTakes(0)));
        assert_eq!(
            desk.settings
                .as_ref()
                .expect("settings")
                .started
                .template_taken
                .len(),
            1,
            "asked for the opposite of what the template does"
        );
        desk.act(mcf_desk::Act::Switch(Switch::TemplateTakes(0)));
        assert!(
            desk.settings
                .as_ref()
                .expect("settings")
                .started
                .template_taken
                .is_empty(),
            "and back again is the template left alone"
        );
    }

    /// A model addressed plainly is offered nothing, rather than an empty heading.
    #[test]
    fn a_template_that_takes_nothing_is_offered_nothing() {
        let mut desk = a_desk_holding_a_template_that_takes_things();
        desk.declared = Some(mcf_serve::declared::Declared {
            template: Some(
                "{%- for message in messages %}{{ message.role }}{%- endfor %}".to_owned(),
            ),
            ..mcf_serve::declared::Declared::default()
        });
        assert!(desk.template_takes().is_empty());
    }
}
