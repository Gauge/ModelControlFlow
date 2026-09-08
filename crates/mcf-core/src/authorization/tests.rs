use super::{Asking, GATED, Gated};

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
        Asking::ByCommand {
            command: "mcf host",
            ..
        }
    ));
}

#[test]
fn a_gate_renders_with_its_clause() {
    let rendered = Gated::Destruction.to_string();
    assert!(rendered.contains("destruction"), "{rendered}");
    assert!(rendered.contains("§3.11"), "{rendered}");
}
