//! What `pull` says, and what it refuses to do.
//!
//! The acquisition path itself is exercised against a real server in
//! `mcf_hub::client`'s tests and end to end in `tests/whole_system.rs`; what is
//! here is the surface's own decisions — what it offers when nobody has chosen,
//! and what it will not attempt.

use mcf_hub::reference;
use mcf_hub::source::{Entry, Listing};

/// A listing with no plan behind it, and the reason MCF gives when there is
/// none: an offer still has to say why it is not planning (A2).
fn no_plan() -> std::result::Result<Vec<String>, String> {
    Err("this repository publishes no configuration, and a plan needs one".to_owned())
}

use super::{DEFAULT_HUB, Offered, PLANNING_CONTEXT, credential, licence_of, offer, run, wire_for};
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

/// Nobody's quantization is chosen for them: what a repository publishes is put
/// in front of the operator with the sizes, which is the question they were
/// actually asking (§3.13, A7).
#[test]
fn without_a_file_it_offers_the_choice_and_acquires_nothing() {
    let offered = offer(&a_listing(), &no_plan());
    assert!(offered.contains("Q4_K_M.gguf"), "{offered}");
    assert!(offered.contains("396705472"), "{offered}");
    assert!(offered.contains("50968a44"), "{offered}");
    assert!(offered.contains("nothing was acquired"), "{offered}");
    assert!(offered.contains("mcf pull owner/model:<file>"), "{offered}");
}

/// A file the hub declares no digest for is named as such, because an artifact
/// nobody can check is a condition of every measurement taken on it (A21).
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

/// The terms are in front of the operator before they choose, which is what
/// §III asks and B-023 built.
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

/// A string that is not a reference is refused by name rather than attempted.
#[test]
fn a_reference_that_is_not_one_is_refused() {
    let response = run("not a reference at all", None, None, Offered::Nothing);
    assert!(!response.served);
    assert!(
        response.text.contains("not a reference"),
        "{}",
        response.text
    );
}

/// The default hub is the real one and MCF reaches it over TLS; a plain socket
/// is what an `http` mirror gets. Chosen from the URL rather than configured,
/// because a wire that cannot keep a secret must not be handed one (B-024,
/// B-322).
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

/// And a hub that is not a URL is refused before anything is opened.
#[test]
fn a_hub_that_is_not_a_url_is_refused() {
    let response = run("owner/model", Some("not-a-hub"), None, Offered::Nothing);
    assert!(!response.served);
    assert!(response.text.contains("not a hub"), "{}", response.text);
}

/// A plan is offered at a stated context, because *this fits* means nothing
/// without the length it fits at (A6, §3.4).
#[test]
fn a_plan_is_offered_at_a_stated_context() {
    let plan = vec!["  Q4_K_M.gguf — fits: needs 1 of 2 usable, 1 left".to_owned()];
    let offered = offer(&a_listing(), &Ok(plan.clone()));
    assert!(
        offered.contains(&format!("at {PLANNING_CONTEXT} tokens of context")),
        "{offered}"
    );
    assert!(offered.contains("fits: needs"), "{offered}");
}

/// And where no plan can be made, the surface says so rather than showing an
/// empty one — a missing plan and a plan that found nothing are different
/// answers (A7).
#[test]
fn no_plan_is_said_rather_than_shown_empty() {
    let offered = offer(&a_listing(), &no_plan());
    assert!(
        offered.contains("cannot say which of these would run here"),
        "{offered}"
    );
    assert!(offered.contains("configuration"), "{offered}");
}

/// Nothing is ever read from the environment unless a test says what is there:
/// these look through a lookup of their own, which is the same discipline the
/// surface itself keeps.
fn nothing_set(_variable: &str) -> Option<String> {
    None
}

/// Nothing offered is nothing held: the ordinary case, and the one B-024 makes
/// the default.
#[test]
fn nothing_offered_is_nothing_held() {
    assert_eq!(
        credential(Offered::Nothing, &nothing_set).expect("no credential"),
        None
    );
}

/// A credential read from a file the operator named carries where it came
/// from, and never the token itself into a message (§3.4, B-024).
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

/// A file that is not there is said rather than treated as no credential: an
/// operator who names one means it, and silently proceeding anonymously would
/// produce a refusal they cannot explain.
#[test]
fn a_credential_file_that_is_not_there_is_reported() {
    let missing =
        credential(Offered::File("/nowhere/at/all/token"), &nothing_set).expect_err("not there");
    assert!(missing.contains("could not be read"), "{missing}");
}

/// An empty one is not a credential.
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

/// A variable MCF was told to read is read, and one it was not told about is
/// not looked at — which is the whole of B-024 at this surface.
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
