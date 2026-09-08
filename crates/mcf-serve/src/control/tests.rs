use super::{Answer, REQUEST_CEILING, Request, VERSION};
use mcf_core::failure::Category;
use mcf_record::json::Value;

#[test]
fn every_request_round_trips() {
    for request in [
        Request::Status,
        Request::Holding,
        Request::Failures { last: 12 },
        Request::Stop {
            reason: "the operator asked".to_owned(),
        },
    ] {
        let line = request.to_line();
        assert_eq!(Request::read(&line).expect("it reads back"), request);
        assert!(!line.contains('\n'), "a message is one line: {line}");
    }
}

#[test]
fn a_protocol_this_build_does_not_speak_is_said_rather_than_guessed() {
    let later = Value::map([
        ("protocol", Value::Integer(VERSION + 1)),
        ("ask", Value::text("status")),
    ])
    .to_line();
    let failure = Request::read(&later).expect_err("a later protocol");
    assert_eq!(failure.category(), Category::ExchangeSchemaUnreadable);
    assert_eq!(
        failure.context_value("this_build_speaks"),
        Some(VERSION.to_string().as_str())
    );

    let none = Value::map([("ask", Value::text("status"))]).to_line();
    assert_eq!(
        Request::read(&none).expect_err("no protocol").category(),
        Category::ConfigInvalid
    );
}

#[test]
fn what_is_not_a_request_is_refused_by_name() {
    for rubbish in [
        "",
        "{",
        "not json at all",
        r#"{"protocol":1}"#,
        r#"{"protocol":1,"ask":"serve"}"#,
        r#"{"protocol":1,"ask":42}"#,
    ] {
        let failure = Request::read(rubbish).expect_err(rubbish);
        assert_eq!(failure.category(), Category::ConfigInvalid, "{rubbish}");
    }
}

#[test]
fn a_request_mcf_does_not_have_names_it() {
    let asked = Value::map([
        ("protocol", Value::Integer(VERSION)),
        ("ask", Value::text("serve")),
    ])
    .to_line();
    let failure = Request::read(&asked).expect_err("no such request");
    assert!(
        failure
            .context()
            .iter()
            .any(|entry| entry.value.contains("serve")),
        "the refusal does not say what was asked for"
    );
}

#[test]
fn a_request_larger_than_the_ceiling_is_refused_and_not_quoted_whole() {
    let enormous = format!(
        r#"{{"protocol":1,"ask":"{}"}}"#,
        "x".repeat(REQUEST_CEILING)
    );
    let failure = Request::read(&enormous).expect_err("too large");
    assert_eq!(failure.category(), Category::ConfigInvalid);
    let quoted: usize = failure
        .context()
        .iter()
        .map(|entry| entry.value.chars().count())
        .sum();
    assert!(quoted < 500, "the refusal carries {quoted} characters back");
}

#[test]
fn an_answer_says_whether_it_is_one() {
    let served = Answer::served(Value::map([("up", Value::Bool(true))]));
    let read = Answer::read(&served.to_line()).expect("it reads back");
    assert!(read.served);
    assert_eq!(read.body.get("up"), Some(&Value::Bool(true)));

    let failure = Request::read("nonsense").expect_err("not a request");
    let refused = Answer::refused(&failure);
    let read = Answer::read(&refused.to_line()).expect("it reads back");
    assert!(!read.served);
    assert_eq!(
        read.body.get("category").and_then(Value::as_text),
        Some("config.invalid")
    );
}

#[test]
fn the_new_requests_survive_the_wire() {
    let asked = [
        Request::Offered {
            reference: "owner/repository".to_owned(),
            from: None,
            fresh: false,
        },
        Request::Offered {
            reference: "owner/repository".to_owned(),
            from: Some("https://elsewhere.example/".to_owned()),
            fresh: true,
        },
        Request::Acquire {
            reference: "owner/repository".to_owned(),
            file: "a-model.gguf".to_owned(),
            from: None,
        },
        Request::Measure {
            started: crate::declared::Started::default(),
            model: "a-model.gguf".to_owned(),
            engine: None,
            on: None,
            deepest: 8192,
        },
        Request::Measure {
            started: crate::declared::Started::default(),
            model: "a-model.gguf".to_owned(),
            engine: Some("stand-in".to_owned()),
            on: Some(super::On::Processor),
            deepest: 512,
        },
        Request::Provision { component: None },
        Request::Provision {
            component: Some("llama.cpp".to_owned()),
        },
        Request::Anatomy {
            model: "a-model.gguf".to_owned(),
        },
        Request::CrossCheck {
            model: "a-model.gguf".to_owned(),
        },
        Request::Generate {
            model: "a-model.gguf".to_owned(),
            prompt: String::new(),
            limit: Some(17),
            seed: 0,
            tokens: Some(vec![1; 4]),
            pieces: None,
            engine: None,
            whose: mcf_record::content::Whose::Fixture,
            pinned: true,
            turn: None,
            image: None,
            started: crate::declared::Started::default(),
        },
        Request::Generate {
            model: "a-model.gguf".to_owned(),
            prompt: "yes".to_owned(),
            limit: None,
            seed: 7,
            tokens: None,
            pieces: Some(vec![
                mcf_standin::tokenizer::Piece::Marker("<|im_start|>".to_owned()),
                mcf_standin::tokenizer::Piece::Text("user\nyes".to_owned()),
            ]),
            engine: Some("stand-in".to_owned()),
            whose: mcf_record::content::Whose::User,
            pinned: false,
            turn: None,
            image: None,
            started: crate::declared::Started::default(),
        },
        Request::PromptReport {
            turn: None,
            model: "a-model.gguf".to_owned(),
            prompt: "Be brief.\n\nBe right.".to_owned(),
            by: None,
            most: None,
            extras: crate::prompt::Extras::NONE,
            temperature: None,
            seed: 41,
        },
        Request::PromptReport {
            turn: None,
            model: "a-model.gguf".to_owned(),
            prompt: "Be brief.\n\nBe right.".to_owned(),
            by: Some(crate::prompt::Unit::Sentence),
            most: Some(40),
            extras: crate::prompt::Extras::NONE
                .with(crate::prompt::Extra::Floors, true)
                .with(crate::prompt::Extra::Alone, true)
                .with(crate::prompt::Extra::Prefixes, true),
            temperature: Some(mcf_core::configuration::Thousandths(700)),
            seed: 41,
        },
    ];
    for request in asked {
        let line = request.to_line();
        let read = Request::read(&line).expect("a request MCF wrote is a request MCF reads");
        assert_eq!(format!("{request:?}"), format!("{read:?}"), "{line}");
    }
}

#[test]
fn a_turn_of_pieces_and_a_count_survive_the_wire() {
    let asked = [
        Request::Generate {
            whose: mcf_record::content::Whose::Fixture,
            model: "a-model.gguf".to_owned(),
            prompt: String::new(),
            limit: Some(8),
            seed: 7,
            tokens: None,
            pieces: Some(vec![
                mcf_standin::tokenizer::Piece::Marker("<|im_start|>".to_owned()),
                mcf_standin::tokenizer::Piece::Text("user\n<|im_start|>".to_owned()),
                mcf_standin::tokenizer::Piece::Marker("<|im_end|>".to_owned()),
            ]),
            engine: Some("llama-server".to_owned()),
            pinned: false,
            turn: None,
            image: None,
            started: crate::declared::Started::default(),
        },
        Request::Tokenize {
            model: "a-model.gguf".to_owned(),
            text: "Le rapide renard brun".to_owned(),
            engine: None,
            beginning: false,
        },
        Request::Tokenize {
            model: "a-model.gguf".to_owned(),
            text: "The quick brown fox".to_owned(),
            engine: Some("stand-in".to_owned()),
            beginning: true,
        },
    ];
    for request in asked {
        let line = request.to_line();
        let read = Request::read(&line).expect("a request MCF wrote is a request MCF reads");
        assert_eq!(format!("{request:?}"), format!("{read:?}"), "{line}");
    }
    assert!(
        Request::read(r#"{"protocol":1,"ask":"tokenize","model":"a-model.gguf"}"#).is_err(),
        "a count of nothing is refused rather than answered nought"
    );
}

#[test]
fn a_measurement_must_say_how_deep() {
    let line = r#"{"protocol":1,"ask":"measure","model":"a-model.gguf"}"#;
    assert!(
        Request::read(line).is_err(),
        "a measurement with no depth was accepted"
    );
}

#[test]
fn an_acquisition_must_name_the_file() {
    let line = r#"{"protocol":1,"ask":"acquire","reference":"owner/repository"}"#;
    assert!(
        Request::read(line).is_err(),
        "an acquisition with no file was accepted"
    );
}

#[test]
fn a_generation_that_does_not_say_is_not_pinned() {
    let line = Value::map([
        ("protocol", Value::Integer(VERSION)),
        ("ask", Value::text("generate")),
        ("model", Value::text("a-model.gguf")),
        ("prompt", Value::text("yes")),
        ("limit", Value::Integer(4)),
    ])
    .to_line();
    match Request::read(&line).expect("a generation") {
        Request::Generate { pinned, limit, .. } => {
            assert!(!pinned, "{line}");
            assert_eq!(limit, Some(4));
        }
        other => panic!("not a generation: {other:?}"),
    }
}

#[test]
fn a_prompt_report_by_an_unknown_unit_is_refused() {
    let by_letter =
        r#"{"protocol":1,"ask":"prompt-report","model":"m","prompt":"A. B.","by":"letter"}"#;
    let refused = Request::read(by_letter).expect_err("by letter is not a unit");
    assert!(
        format!("{refused:?}").contains("none of word, phrase, sentence or paragraph"),
        "{refused:?}"
    );
    let none = r#"{"protocol":1,"ask":"prompt-report","model":"m","prompt":"A. B.","most":0}"#;
    let refused = Request::read(none).expect_err("removing nothing is not a report");
    assert!(format!("{refused:?}").contains("no parts"), "{refused:?}");
    let cold = r#"{"protocol":1,"ask":"prompt-report","model":"m","prompt":"A. B.","temperature_thousandths":0}"#;
    let refused =
        Request::read(cold).expect_err("nought is greedy, not a temperature to settle at");
    assert!(
        format!("{refused:?}").contains("not above nought"),
        "{refused:?}"
    );
}

#[test]
fn a_measurement_says_where_the_model_goes() {
    let asked = super::Request::Measure {
        model: "a-model.gguf".to_owned(),
        engine: None,
        on: Some(super::On::Card),
        deepest: 1024,
        started: crate::declared::Started::default(),
    };
    let read = super::Request::read(&asked.to_line()).expect("its own line reads back");
    assert_eq!(read, asked);
    let cpu = r#"{"protocol":1,"ask":"measure","model":"a-model.gguf","deepest":512,"on":"cpu"}"#;
    match super::Request::read(cpu) {
        Ok(super::Request::Measure { on, .. }) => assert_eq!(on, Some(super::On::Processor)),
        other => panic!("cpu is a word people type: {other:?}"),
    }
    let elsewhere =
        r#"{"protocol":1,"ask":"measure","model":"a-model.gguf","deepest":512,"on":"cloud"}"#;
    assert!(
        super::Request::read(elsewhere).is_err(),
        "a place MCF does not put a model is refused"
    );
}

#[test]
fn a_probe_request_round_trips() {
    let asked = Request::Probe {
        model: "m".to_owned(),
        engine: Some("provisioned".to_owned()),
        apply: true,
        up_to: Some(4096),
        only: vec!["chat-template".to_owned(), "thinking".to_owned()],
    };
    let read = Request::read(&asked.to_line()).unwrap_or_else(|why| panic!("{why}"));
    assert_eq!(read, asked);
    let bare = r#"{"protocol":1,"ask":"probe","model":"m"}"#;
    let Ok(Request::Probe {
        engine,
        apply,
        up_to,
        only,
        ..
    }) = Request::read(bare)
    else {
        panic!("a bare probe request did not read");
    };
    assert_eq!((engine, apply, up_to), (None, false, None));
    assert!(only.is_empty());
    assert!(Request::read(r#"{"protocol":1,"ask":"probe"}"#).is_err());
}

#[test]
fn an_examine_request_round_trips() {
    let asked = Request::Examine {
        model: "m".to_owned(),
        engine: Some("provisioned".to_owned()),
        only: vec!["determinism".to_owned(), "cold-start".to_owned()],
    };
    let read = Request::read(&asked.to_line()).unwrap_or_else(|why| panic!("{why}"));
    assert_eq!(read, asked);
    let bare = r#"{"protocol":1,"ask":"examine","model":"m"}"#;
    let Ok(Request::Examine { engine, only, .. }) = Request::read(bare) else {
        panic!("a bare examine request did not read");
    };
    assert_eq!(engine, None);
    assert!(only.is_empty());
    assert!(Request::read(r#"{"protocol":1,"ask":"examine"}"#).is_err());
}

#[test]
fn a_readings_request_round_trips() {
    let asked = Request::Readings {
        model: "m".to_owned(),
        method: Some("prefill-saturation".to_owned()),
    };
    let read = Request::read(&asked.to_line()).unwrap_or_else(|why| panic!("{why}"));
    assert_eq!(read, asked);
    let Ok(Request::Readings { method, .. }) =
        Request::read(r#"{"protocol":1,"ask":"readings","model":"m"}"#)
    else {
        panic!("a bare readings request did not read");
    };
    assert_eq!(method, None);
}

#[test]
fn a_hub_request_carries_whether_it_wants_the_hub_asked_again() {
    let asked = Request::Search {
        query: "gemma".to_owned(),
        from: None,
        fresh: true,
    };
    let read = Request::read(&asked.to_line()).unwrap_or_else(|why| panic!("{why}"));
    assert_eq!(read, asked);
    let bare = r#"{"protocol":1,"ask":"offered","reference":"owner/name"}"#;
    let Ok(Request::Offered { fresh, .. }) = Request::read(bare) else {
        panic!("a bare listing did not read");
    };
    assert!(!fresh, "a line without the flag asks for what was kept");
}
