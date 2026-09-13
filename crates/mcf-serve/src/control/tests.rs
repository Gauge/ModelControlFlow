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

#[test]
fn a_removal_preview_survives_the_wire() {
    let asked = Request::Removal {
        model: "a-model.gguf".to_owned(),
    };
    let back = Request::read(&asked.to_line()).expect("a preview is readable");
    assert_eq!(back, asked);
}

#[test]
fn a_removal_carries_its_reason_and_whether_it_deletes() {
    for purge in [false, true] {
        let asked = Request::Remove {
            model: "a-model.gguf".to_owned(),
            reason: "making room for the one being measured".to_owned(),
            purge,
        };
        let back = Request::read(&asked.to_line()).expect("a removal is readable");
        assert_eq!(back, asked);
    }
}

#[test]
fn a_removal_naming_no_model_is_refused_rather_than_guessed_at() {
    let line = mcf_record::json::Value::map([
        ("protocol", mcf_record::json::Value::Integer(1)),
        ("ask", mcf_record::json::Value::text("remove")),
        ("reason", mcf_record::json::Value::text("because")),
    ])
    .to_line();
    assert!(Request::read(&line).is_err());
}

#[test]
fn a_removal_with_no_reason_on_the_wire_reads_as_no_reason_not_as_one() {
    let line = mcf_record::json::Value::map([
        ("protocol", mcf_record::json::Value::Integer(1)),
        ("ask", mcf_record::json::Value::text("remove")),
        ("model", mcf_record::json::Value::text("a-model.gguf")),
    ])
    .to_line();
    let Ok(Request::Remove { reason, purge, .. }) = Request::read(&line) else {
        panic!("a removal naming a model is readable");
    };
    assert!(reason.is_empty(), "the daemon refuses it later, by name");
    assert!(!purge, "nothing is deleted unless it was asked for");
}

#[test]
fn a_settings_choice_to_remember_survives_the_wire() {
    let asked = Request::Remember {
        model: "a-model.gguf".to_owned(),
        settings: mcf_record::json::Value::map([(
            "context",
            mcf_record::json::Value::Integer(32_768),
        )]),
    };
    let back = Request::read(&asked.to_line()).expect("a choice is readable");
    assert_eq!(back, asked);
}

#[test]
fn remembering_and_hosting_are_different_asks() {
    let model = "a-model.gguf".to_owned();
    let settings = mcf_record::json::Value::Null;
    let remember = Request::Remember {
        model: model.clone(),
        settings: settings.clone(),
    };
    let host = Request::Host { model, settings };
    assert_ne!(remember.to_line(), host.to_line());
    assert!(remember.to_line().contains("\"remember\""));
}

#[test]
fn a_choice_to_remember_naming_no_model_is_refused() {
    let line = mcf_record::json::Value::map([
        ("protocol", mcf_record::json::Value::Integer(1)),
        ("ask", mcf_record::json::Value::text("remember")),
    ])
    .to_line();
    assert!(Request::read(&line).is_err());
}
