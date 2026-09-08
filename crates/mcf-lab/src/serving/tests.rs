use std::collections::BTreeMap;

use super::{Serving, answer, status};
use mcf_hub::http::{Request, Url};
use mcf_hub::wire::{Deadlines, Tcp, fetch};

fn quick() -> Tcp {
    Tcp {
        deadlines: Deadlines {
            connect: std::time::Duration::from_secs(5),
            idle: std::time::Duration::from_millis(300),
        },
    }
}

#[test]
fn it_answers_what_it_was_scripted_and_refuses_the_rest() {
    let serving = Serving::answering(BTreeMap::from([(
        "/api/models/owner/model".to_owned(),
        answer(r#"{"sha":"abc"}"#),
    )]))
    .expect("a loopback port");

    let url = Url::parse(&format!("{}api/models/owner/model", serving.base())).expect("a URL");
    let mut body = Vec::new();
    let exchanged = fetch(&quick(), &Request::get(url), &mut body).expect("an answer");
    assert_eq!(exchanged.response.status(), 200);
    assert_eq!(body, br#"{"sha":"abc"}"#);

    let missing = Url::parse(&format!("{}nothing", serving.base())).expect("a URL");
    let mut nothing = Vec::new();
    let refused = fetch(&quick(), &Request::get(missing), &mut nothing).expect("an answer");
    assert_eq!(refused.response.status(), 404);
}

#[test]
fn it_remembers_what_it_was_asked() {
    let serving = Serving::answering(BTreeMap::from([("/x".to_owned(), status(200, "OK"))]))
        .expect("a loopback port");

    let url = Url::parse(&format!("{}x", serving.base())).expect("a URL");
    let mut body = Vec::new();
    fetch(&quick(), &Request::get(url).resuming(64), &mut body).expect("an answer");

    let asked = serving.asked();
    assert_eq!(asked.len(), 1, "{asked:?}");
    assert!(asked[0].contains("Range: bytes=64-"), "{asked:?}");
}

#[test]
fn a_silent_server_ends_at_the_clients_deadline() {
    let serving = Serving::holding_open().expect("a loopback port");
    let url = Url::parse(&format!("{}anything", serving.base())).expect("a URL");
    let mut body = Vec::new();
    let failure = fetch(&quick(), &Request::get(url), &mut body).expect_err("stalled");
    assert_eq!(
        failure.category(),
        mcf_core::failure::Category::TransferStalled
    );
}
