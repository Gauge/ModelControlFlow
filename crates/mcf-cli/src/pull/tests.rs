use mcf_hub::reference;
use mcf_hub::source::{Entry, Listing};

fn no_plan() -> std::result::Result<super::Plan, String> {
    Err("this repository publishes no configuration, and a plan needs one".to_owned())
}

use mcf_hub::offer::PLANNING_CONTEXT;
use mcf_hub::wire::for_url as wire_for;

use super::{DEFAULT_HUB, Offered, credential, licence_of, offer, run};
use mcf_hub::http::Url;

fn a_listing() -> Listing {
    Listing {
        reference: reference::parse("owner/model").expect("a reference"),
        revision: Some("50968a4468ef4233ed78cd7c3de230dd1d61a56b".to_owned()),
        entries: vec![
            Entry::new("Q4_K_M.gguf", 396_705_472).declaring("a".repeat(64)),
            Entry::new("Q8_0.gguf", 700_000_000),
        ],
        gated: None,
        declared_licence: Some("apache-2.0".to_owned()),
        lineage: None,
    }
}

#[test]
fn without_a_file_it_offers_the_choice_and_acquires_nothing() {
    let offered = offer(&a_listing(), &no_plan());
    assert!(offered.contains("Q4_K_M.gguf"), "{offered}");
    assert!(offered.contains("396705472"), "{offered}");
    assert!(offered.contains("50968a44"), "{offered}");
    assert!(offered.contains("nothing was acquired"), "{offered}");
    assert!(offered.contains("mcf pull owner/model:<file>"), "{offered}");
}

#[test]
fn a_file_with_no_declared_digest_is_pointed_out() {
    let offered = offer(&a_listing(), &no_plan());
    let undeclared = offered
        .lines()
        .find(|line| line.contains("Q8_0.gguf"))
        .unwrap_or_default();
    assert!(undeclared.contains("no digest"), "{undeclared}");
    let declared = offered
        .lines()
        .find(|line| line.contains("Q4_K_M.gguf"))
        .unwrap_or_default();
    assert!(!declared.contains("no digest"), "{declared}");
}

#[test]
fn the_terms_are_offered_with_the_files() {
    let offered = offer(&a_listing(), &no_plan());
    assert!(offered.contains("apache-2.0"), "{offered}");
    assert!(offered.contains("permissive"), "{offered}");
    assert_eq!(
        licence_of(&a_listing()),
        Some(mcf_core::provenance::Licence::spdx("apache-2.0"))
    );
}

#[test]
fn a_reference_that_is_not_one_is_refused() {
    let response = run(
        "not a reference at all",
        None,
        None,
        Offered::Nothing,
        false,
    );
    assert!(!response.served);
    assert!(
        response.text.contains("not a reference"),
        "{}",
        response.text
    );
}

#[test]
fn the_wire_is_chosen_by_the_scheme() {
    assert!(DEFAULT_HUB.starts_with("https://"), "{DEFAULT_HUB}");
    let encrypted = wire_for(&Url::parse(DEFAULT_HUB).expect("a URL")).expect("a wire");
    assert!(encrypted.carries_secrets(), "{}", encrypted.describe());
    assert!(
        encrypted.describe().contains("TLS"),
        "{}",
        encrypted.describe()
    );

    let plain = wire_for(&Url::parse("http://127.0.0.1:8080/").expect("a URL")).expect("a wire");
    assert!(
        !plain.carries_secrets(),
        "a plain socket claimed it could keep a secret"
    );
}

#[test]
fn a_hub_that_is_not_a_url_is_refused() {
    let response = run(
        "owner/model",
        Some("not-a-hub"),
        None,
        Offered::Nothing,
        false,
    );
    assert!(!response.served);
    assert!(response.text.contains("not a hub"), "{}", response.text);
}

#[test]
fn a_plan_is_offered_at_a_stated_context() {
    let plan = mcf_hub::offer::Plan {
        context: PLANNING_CONTEXT,
        available: mcf_core::measurement::Bytes(2),
        verdicts: vec![(
            "Q4_K_M.gguf".to_owned(),
            mcf_hub::fitment::Verdict::Fits {
                needs: mcf_core::measurement::Bytes(1),
                headroom: mcf_core::measurement::Bytes(1),
            },
        )],
    };
    let offered = offer(&a_listing(), &Ok(plan));
    assert!(
        offered.contains(&format!("at {PLANNING_CONTEXT} tokens of context")),
        "{offered}"
    );
    assert!(offered.contains("fits: needs"), "{offered}");
}

#[test]
fn no_plan_is_said_rather_than_shown_empty() {
    let offered = offer(&a_listing(), &no_plan());
    assert!(
        offered.contains("cannot say which of these would run here"),
        "{offered}"
    );
    assert!(offered.contains("configuration"), "{offered}");
}

fn nothing_set(_variable: &str) -> Option<String> {
    None
}

#[test]
fn nothing_offered_is_nothing_held() {
    assert_eq!(
        credential(Offered::Nothing, &nothing_set).expect("no credential"),
        None
    );
}

#[test]
fn a_credential_from_a_named_file_carries_its_origin() {
    let path = std::env::temp_dir().join(format!("mcf-pull-token-{}", std::process::id()));
    std::fs::write(&path, "hf_from_a_file\n").expect("a token file");

    let held = credential(
        Offered::File(path.to_str().unwrap_or_default()),
        &nothing_set,
    )
    .expect("it reads")
    .expect("one is there");
    assert_eq!(
        held.secret().reveal(),
        "hf_from_a_file",
        "the newline came with it"
    );
    assert!(held.describe().contains(&path.display().to_string()));
    assert!(!held.describe().contains("hf_from_a_file"));

    let _cleared = std::fs::remove_file(&path);
}

#[test]
fn a_credential_file_that_is_not_there_is_reported() {
    let missing =
        credential(Offered::File("/nowhere/at/all/token"), &nothing_set).expect_err("not there");
    assert!(missing.contains("could not be read"), "{missing}");
}

#[test]
fn an_empty_credential_is_refused_before_it_is_offered() {
    let path = std::env::temp_dir().join(format!("mcf-pull-blank-{}", std::process::id()));
    std::fs::write(&path, "   \n").expect("a blank file");
    let blank = credential(
        Offered::File(path.to_str().unwrap_or_default()),
        &nothing_set,
    )
    .expect_err("nothing in it");
    assert!(blank.contains("holds nothing"), "{blank}");
    let _cleared = std::fs::remove_file(&path);
}

#[test]
fn an_environment_variable_is_read_only_when_it_is_named() {
    let named = "A_VARIABLE_THE_OPERATOR_NAMED";
    let asked: std::cell::RefCell<Vec<String>> = std::cell::RefCell::new(Vec::new());
    let look_up = |variable: &str| {
        asked.borrow_mut().push(variable.to_owned());
        (variable == named).then(|| "hf_from_the_environment".to_owned())
    };

    let held = credential(Offered::Variable(named), &look_up)
        .expect("it reads")
        .expect("one is there");
    assert_eq!(held.secret().reveal(), "hf_from_the_environment");
    assert!(held.describe().contains(named));
    assert!(!held.describe().contains("hf_from_the_environment"));
    assert_eq!(
        asked.borrow().as_slice(),
        [named.to_owned()],
        "MCF looked at something nobody named"
    );

    let absent = credential(Offered::Variable("SOMETHING_ELSE"), &look_up).expect_err("not set");
    assert!(absent.contains("is not set"), "{absent}");
}
