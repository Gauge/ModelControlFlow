//! What a share shows, and what it refuses to show as a summary (B-160, A24).

// Every item in this file is test code; see the note in
// checks/tests/taxonomy_agreement.rs.
#![allow(clippy::panic, clippy::expect_used)]

use mcf_record::json::Value;

use super::{destination, read_comparison, workload_of};

/// A comparison whose arms separated, as the record writes one.
fn a_recorded_comparison(kind: &str, workload: Option<&str>) -> mcf_record::journal::Entry {
    let conditions = mcf_record::encode::conditions(&mcf_core::measurement::Conditions::new(
        mcf_core::build_identity::BuildIdentity::current(),
        mcf_core::measurement::Floor::nothing_known(),
    ));
    let mut method = vec![("resolving_ppm", Value::Integer(50_000))];
    if let Some(workload) = workload {
        method.push(("workload", Value::text(workload)));
    }
    mcf_record::journal::Entry::new(
        mcf_record::journal::EntryKind::Comparison,
        mcf_core::time::Timestamp::from_utc_nanos(0, mcf_core::attested::Attested::Unknown),
        Value::map([
            (
                "left",
                Value::map([
                    ("arm", Value::text("/models/a.gguf")),
                    ("conditions", conditions.clone()),
                ]),
            ),
            (
                "right",
                Value::map([
                    ("arm", Value::text("/models/b.gguf")),
                    ("conditions", conditions),
                ]),
            ),
            ("method", Value::map(method)),
            (
                "outcome",
                Value::map([
                    ("kind", Value::text(kind)),
                    ("quicker", Value::text("left")),
                    ("difference", Value::Integer(120_000)),
                    ("pairs", Value::Integer(40)),
                ]),
            ),
        ]),
    )
}

/// A recorded comparison with an established size becomes a row.
#[test]
fn a_comparison_that_separated_is_a_row() {
    let compared = read_comparison(&a_recorded_comparison("differ", Some("declared")))
        .expect("a differ outcome is a row");
    assert_eq!(compared.pairs, 40);
    assert!(compared.left_quicker);
    assert_eq!(compared.left.as_str(), "/models/a.gguf");
}

/// One that did not separate is not a row, and is not an error either.
///
/// *No difference* is a real result (A9) and it is not a size, so there is
/// nothing to contribute about it — which is different from a failure and is
/// reported as such by the surface.
#[test]
fn a_comparison_without_an_established_size_is_not_a_row() {
    for kind in ["same", "not_comparable", "ordered", "not_yet"] {
        assert!(
            read_comparison(&a_recorded_comparison(kind, Some("declared"))).is_none(),
            "{kind} became a contributable row"
        );
    }
}

/// A row that does not say where its workload came from is the operator's.
///
/// The safe direction, and the one B42 requires: a row nobody else can
/// interpret must not travel, and a record that does not say is not evidence
/// that it may (A7).
#[test]
fn a_workload_that_is_not_stated_is_the_operators_own() {
    let unstated = a_recorded_comparison("differ", None);
    assert_eq!(
        workload_of(unstated.body()),
        mcf_core::contribution::Workload::Custom
    );
    let stated = a_recorded_comparison("differ", Some("declared"));
    assert_eq!(
        workload_of(stated.body()),
        mcf_core::contribution::Workload::Declared
    );
}

/// The file goes where it was told, and has a name when it was not.
#[test]
fn the_destination_is_named_or_defaulted_visibly() {
    assert_eq!(
        destination(Some("/tmp/x.mcf")),
        std::path::Path::new("/tmp/x.mcf")
    );
    assert_eq!(destination(None), std::path::Path::new("contribution.mcf"));
}
