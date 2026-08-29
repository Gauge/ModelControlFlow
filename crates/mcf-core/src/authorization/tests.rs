//! The five, and what each says about itself.
//!
//! Four for most of this project's life, because §6.14 names four and the fifth
//! comes from §3.20 — which A16 absorbs and which nothing enumerated (F115).

use super::{Asking, GATED, Gated};

/// Five, and the five A16 names.
///
/// A16 says *the five categories are enumerable in code*, and for a long time
/// four were: publication is the one §6.14 does not name, A16 absorbs from
/// §3.20, and A24 exists as its own rule because it is the only one of the five
/// that cannot be undone (F115).
#[test]
fn the_five_are_the_five() {
    assert_eq!(GATED.len(), 5);
    for gate in [
        Gated::UntrustedExecution,
        Gated::LargeIrrecoverableUse,
        Gated::NetworkExposure,
        Gated::Destruction,
        Gated::Publication,
    ] {
        assert!(GATED.contains(&gate), "{gate} is not in the list");
    }
}

/// Each names itself once, so a record cannot confuse two of them.
#[test]
fn each_has_its_own_name_and_clause() {
    let mut names: Vec<&str> = GATED.iter().map(|gate| gate.as_str()).collect();
    let count = names.len();
    names.sort_unstable();
    names.dedup();
    assert_eq!(names.len(), count, "two gates share a name");

    for gate in GATED {
        assert!(gate.clause().starts_with('§'), "{gate} cites no clause");
        assert!(
            !gate.as_str().is_empty() && !gate.as_str().contains(' '),
            "{gate} has a name a record cannot hold"
        );
    }
}

/// Every gate says where MCF asks, or that there is nothing to ask about — and
/// the second is a claim with a reason rather than a blank.
#[test]
fn every_gate_says_where_it_is() {
    for gate in GATED {
        match gate.asking() {
            Asking::ByCommand { command, and } => {
                assert!(command.starts_with("mcf "), "{gate}: {command}");
                assert!(
                    and.len() > 40,
                    "{gate} names a command and does not say what makes it an authorization"
                );
            }
            Asking::NoPathExists { why } => assert!(
                why.len() > 40,
                "{gate} claims no path exists and does not say why"
            ),
        }
    }
}

/// The two that have a gate are the two MCF can do; the two that do not are the
/// two it cannot. That correspondence is the whole claim, and it is what
/// `checks/tests/the_four_gates.rs` holds against the tree.
#[test]
fn what_mcf_can_do_is_what_it_gates() {
    assert!(matches!(
        Gated::Destruction.asking(),
        Asking::ByCommand { .. }
    ));
    assert!(matches!(
        Gated::LargeIrrecoverableUse.asking(),
        Asking::ByCommand { .. }
    ));
    assert!(matches!(
        Gated::UntrustedExecution.asking(),
        Asking::NoPathExists { .. }
    ));
    assert!(matches!(
        Gated::NetworkExposure.asking(),
        Asking::NoPathExists { .. }
    ));
}

/// A gate renders as itself with the clause that put it there, because a
/// message naming a gate without its clause is a message nobody can check.
#[test]
fn a_gate_renders_with_its_clause() {
    let rendered = Gated::Destruction.to_string();
    assert!(rendered.contains("destruction"), "{rendered}");
    assert!(rendered.contains("§3.11"), "{rendered}");
}
