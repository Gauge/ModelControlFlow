//! Three states that do not collapse, and the fourth that is worth having.

use super::{Capability, State};

/// The four situations, each reachable and each distinct.
#[test]
fn the_four_situations_are_distinguishable() {
    let unknown: Capability<u32> = Capability::unknown();
    assert_eq!(unknown.state(), State::Unknown);

    let declared = Capability::declared(4096_u32);
    assert_eq!(declared.state(), State::Declared);

    let verified = Capability::verified(4096_u32);
    assert_eq!(verified.state(), State::Verified);

    let agreed = Capability::declared(4096_u32).and_verified(4096);
    assert_eq!(agreed.state(), State::Verified);

    let diverged = Capability::declared(8192_u32).and_verified(4096);
    assert_eq!(diverged.state(), State::Diverged);
}

/// A declaration is never enough to act on, which is §3.18 in one method.
#[test]
fn only_an_observation_may_be_acted_on() {
    assert!(!Capability::declared(4096_u32).is_established());
    assert!(!Capability::<u32>::unknown().is_established());
    assert!(Capability::verified(4096_u32).is_established());
    assert!(
        !Capability::declared(8192_u32)
            .and_verified(4096)
            .is_established(),
        "a divergence is not something to act on: it is something to look at"
    );
}

/// The two halves stay apart. There is no operation that turns what somebody
/// said into what MCF found, which is the whole reason this is a type.
#[test]
fn a_declaration_does_not_become_an_observation() {
    let declared = Capability::declared("llama".to_owned());
    assert_eq!(declared.declaration().map(String::as_str), Some("llama"));
    assert_eq!(declared.observation(), None);

    let verified = Capability::verified("mamba".to_owned());
    assert_eq!(verified.declaration(), None);
    assert_eq!(verified.observation().map(String::as_str), Some("mamba"));
}

/// The divergence carries both halves, because *which* disagreement it is is
/// the finding rather than the fact that there is one (B-058).
#[test]
fn a_divergence_names_both_sides() {
    let diverged = Capability::declared("llama".to_owned()).and_verified("mamba".to_owned());
    let (declared, verified) = diverged.divergence().expect("they disagree");
    assert_eq!(declared, "llama");
    assert_eq!(verified, "mamba");

    assert_eq!(Capability::verified(1_u8).divergence(), None);
    assert_eq!(Capability::declared(1_u8).divergence(), None);
    assert_eq!(
        Capability::declared(1_u8).and_verified(1).divergence(),
        None,
        "agreement is not a divergence"
    );
}

/// A value never renders without which kind of knowing it is: a number shown
/// alone is a number a reader will take for a fact (§3.18, A21).
#[test]
fn nothing_renders_without_saying_what_kind_of_knowing_it_is() {
    assert_eq!(Capability::<u32>::unknown().to_string(), "unknown");
    assert!(
        Capability::declared(4096_u32)
            .to_string()
            .contains("declared, unverified")
    );
    assert!(
        Capability::verified(4096_u32)
            .to_string()
            .contains("nothing declared it")
    );
    assert!(
        Capability::declared(4096_u32)
            .and_verified(4096)
            .to_string()
            .contains("the declaration agrees")
    );
    let diverged = Capability::declared(8192_u32)
        .and_verified(4096)
        .to_string();
    assert!(diverged.contains("DIVERGES"), "{diverged}");
    assert!(
        diverged.contains("8192") && diverged.contains("4096"),
        "{diverged}"
    );
}

/// The state's name is what a record writes, and each is its own.
#[test]
fn every_state_has_its_own_name() {
    let names = [
        State::Unknown.as_str(),
        State::Declared.as_str(),
        State::Verified.as_str(),
        State::Diverged.as_str(),
    ];
    let mut sorted = names;
    sorted.sort_unstable();
    let mut deduped = sorted.to_vec();
    deduped.dedup();
    assert_eq!(deduped.len(), names.len(), "two states share a name");
}

/// Default is unknown, because the alternative is a default value pretending to
/// be knowledge (A7).
#[test]
fn the_default_is_knowing_nothing() {
    assert_eq!(Capability::<u32>::default().state(), State::Unknown);
}
