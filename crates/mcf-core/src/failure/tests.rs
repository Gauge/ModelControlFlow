//! Tests for the failure type.
//!
//! A19 holds failure paths to the same standard as success paths, because A2
//! makes them a feature: untested error handling is decorative.

use super::{Attribution, Category, Disposition, Domain, Failure, Subsystem};

const WHERE: Subsystem = Subsystem::new("mcf-core::failure::tests");

fn a_failure() -> Failure {
    Failure::new(
        Category::EngineExitMidstream,
        Attribution::Managed,
        Disposition::Partial,
        WHERE,
        "the engine died after two of five chunks",
    )
}

/// The three axes are what construction requires, and what a failure reports.
#[test]
fn a_failure_carries_all_three_axes() {
    let failure = a_failure();
    assert_eq!(failure.category(), Category::EngineExitMidstream);
    assert_eq!(failure.attribution(), Attribution::Managed);
    assert_eq!(failure.disposition(), Disposition::Partial);
    assert_eq!(failure.subsystem(), WHERE);
}

/// A19: the codes are checked against the taxonomy's own spelling, which is
/// what travels between machines (§XIV) and may never be renamed (C5).
#[test]
fn codes_are_the_taxonomy_spelling() {
    assert_eq!(Category::HubAuthRequired.code(), "hub.auth.required");
    assert_eq!(
        Category::AccelDriverQueryFailed.code(),
        "accel.driver.query_failed"
    );
    assert_eq!(
        Category::InternalUnclassified.code(),
        "internal.unclassified"
    );
    assert_eq!(
        Category::ModelToolMalformedCall.code(),
        "model.tool.malformed_call"
    );
}

/// Every code is unique. A duplicate would make two different failures
/// indistinguishable in the record, which is A1's loss of information.
#[test]
fn every_code_is_unique() {
    let mut codes: Vec<&str> = Category::ALL.iter().map(|c| c.code()).collect();
    let count = codes.len();
    codes.sort_unstable();
    codes.dedup();
    assert_eq!(codes.len(), count, "two categories share a code");
}

/// Every code sits in the domain its prefix names. A code filed under the
/// wrong domain is a code consumers switch on incorrectly.
#[test]
fn every_code_belongs_to_the_domain_its_prefix_names() {
    for category in Category::ALL {
        let prefix = category
            .code()
            .split('.')
            .next()
            .expect("a code always has a first segment");
        assert_eq!(
            category.domain().prefix(),
            prefix,
            "{} is filed under {}",
            category.code(),
            category.domain()
        );
    }
}

/// No code is deeper than three segments — the shallowness the taxonomy's
/// three-axis split exists to buy.
#[test]
fn no_code_is_deeper_than_three_segments() {
    for category in Category::ALL {
        let depth = category.code().split('.').count();
        assert!(
            (2..=3).contains(&depth),
            "{} has {depth} segments",
            category.code()
        );
    }
}

/// Every category has a meaning, and no meaning is a placeholder.
#[test]
fn every_category_states_what_it_means() {
    for category in Category::ALL {
        let meaning = category.meaning();
        assert!(!meaning.is_empty(), "{} has no meaning", category.code());
        assert!(
            !meaning.contains("TODO") && !meaning.contains("TBD"),
            "{} has a placeholder meaning",
            category.code()
        );
    }
}

/// Round-tripping a code is exact. This is the property the record depends on
/// when it reads back what an earlier version wrote.
#[test]
fn a_code_round_trips() {
    for category in Category::ALL {
        assert_eq!(Category::from_code(category.code()), Some(category));
    }
}

/// A7: a code this version does not know is `None`, never a fallback to
/// `internal.unclassified`. Deciding which it is belongs to the caller with
/// the context — a record written by a newer schema is
/// `record.schema.unknown`, not an MCF invariant violation.
#[test]
fn an_unknown_code_does_not_become_unclassified() {
    assert_eq!(Category::from_code("hub.invented"), None);
    assert_eq!(Category::from_code(""), None);
    assert_eq!(Category::from_code("internal"), None);
}

/// Both other axes round-trip too, for the same reason.
#[test]
fn the_other_axes_round_trip() {
    for attribution in Attribution::ALL {
        assert_eq!(Attribution::parse(attribution.as_str()), Some(attribution));
    }
    for disposition in Disposition::ALL {
        assert_eq!(Disposition::parse(disposition.as_str()), Some(disposition));
    }
    assert_eq!(Attribution::parse("nobody"), None);
    assert_eq!(Disposition::parse("succeeded"), None);
}

/// Every domain is reachable from at least one category. A domain with no
/// codes is a domain that was added and never used, which the taxonomy's
/// extension policy treats as a decision to revisit.
#[test]
fn every_domain_has_at_least_one_category() {
    for domain in Domain::ALL {
        assert!(
            Category::ALL.iter().any(|c| c.domain() == domain),
            "{domain} has no categories"
        );
    }
}

/// B21: context is what the laboratory rebuilds a failure from, and it is kept
/// in the order it was added, because order is information about what the
/// failing code knew and when.
#[test]
fn context_is_kept_in_order_and_queryable() {
    let failure = a_failure()
        .with_context("chunks_expected", "5")
        .with_context("chunks_received", "2");
    assert_eq!(failure.context_value("chunks_expected"), Some("5"));
    assert_eq!(failure.context_value("chunks_received"), Some("2"));
    assert_eq!(failure.context_value("absent"), None);
    let keys: Vec<&str> = failure.context().iter().map(|e| e.key).collect();
    assert_eq!(keys, ["chunks_expected", "chunks_received"]);
}

/// A1: a wrapping failure keeps what it wrapped rather than replacing it with
/// a summary.
#[test]
fn a_cause_is_kept_whole() {
    let underlying = Failure::new(
        Category::ResourceMemoryExhausted,
        Attribution::Machine,
        Disposition::Aborted,
        WHERE,
        "the host refused the allocation",
    );
    let failure = a_failure().caused_by(underlying.clone());
    assert_eq!(failure.cause(), Some(&underlying));
    let chain: Vec<&str> = failure.chain().map(|f| f.category().code()).collect();
    assert_eq!(
        chain,
        ["engine.exit.midstream", "resource.memory.exhausted"]
    );
}

/// The chain of a failure with no cause is the failure itself, so a caller
/// never has to special-case the single-link case.
#[test]
fn a_chain_of_one_is_still_a_chain() {
    assert_eq!(a_failure().chain().count(), 1);
}

/// `std::error::Error::source` exposes the same chain, so a failure composes
/// with anything that expects an error.
#[test]
fn the_error_source_is_the_cause() {
    use std::error::Error as _;
    let failure = a_failure().caused_by(Failure::new(
        Category::TimeJumpBackward,
        Attribution::Machine,
        Disposition::Invalidated,
        WHERE,
        "the wall clock stepped back",
    ));
    let source = failure.source().expect("the failure has a cause");
    assert!(
        source.to_string().contains("time.jump.backward"),
        "{source}"
    );
}

/// The rendering drops no axis. A6's habit: a surface that shows a result and
/// drops its conditions is doing damage, and the axes are a failure's
/// conditions.
#[test]
fn the_rendering_names_all_three_axes_and_the_subsystem() {
    let rendered = a_failure().to_string();
    for expected in [
        "engine.exit.midstream",
        "managed",
        "partial",
        "mcf-core::failure::tests",
        "the engine died after two of five chunks",
    ] {
        assert!(
            rendered.contains(expected),
            "{rendered:?} omits {expected:?}"
        );
    }
}

/// The unclassified category is detectable, which is what lets its count be
/// reported against a target of zero rather than discovered in a log.
#[test]
fn the_unclassified_category_is_detectable() {
    assert!(!a_failure().is_unclassified());
    let unclassified = Failure::new(
        Category::InternalUnclassified,
        Attribution::Mcf,
        Disposition::Aborted,
        WHERE,
        "a failure that fits nothing in the taxonomy",
    );
    assert!(unclassified.is_unclassified());
}

/// The counts the taxonomy states, checked against the types. A19: the
/// document is the independently known value.
#[test]
fn the_taxonomy_counts_hold() {
    assert_eq!(Domain::ALL.len(), 16);
    assert_eq!(Category::ALL.len(), 110);
    assert_eq!(Attribution::ALL.len(), 8);
    assert_eq!(Disposition::ALL.len(), 6);
}
