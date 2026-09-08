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

#[test]
fn a_failure_carries_all_three_axes() {
    let failure = a_failure();
    assert_eq!(failure.category(), Category::EngineExitMidstream);
    assert_eq!(failure.attribution(), Attribution::Managed);
    assert_eq!(failure.disposition(), Disposition::Partial);
    assert_eq!(failure.subsystem(), WHERE);
}

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

#[test]
fn every_code_is_unique() {
    let mut codes: Vec<&str> = Category::ALL.iter().map(|c| c.code()).collect();
    let count = codes.len();
    codes.sort_unstable();
    codes.dedup();
    assert_eq!(codes.len(), count, "two categories share a code");
}

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

#[test]
fn a_code_round_trips() {
    for category in Category::ALL {
        assert_eq!(Category::from_code(category.code()), Some(category));
    }
}

#[test]
fn an_unknown_code_does_not_become_unclassified() {
    assert_eq!(Category::from_code("hub.invented"), None);
    assert_eq!(Category::from_code(""), None);
    assert_eq!(Category::from_code("internal"), None);
}

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

#[test]
fn every_domain_has_at_least_one_category() {
    for domain in Domain::ALL {
        assert!(
            Category::ALL.iter().any(|c| c.domain() == domain),
            "{domain} has no categories"
        );
    }
}

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

#[test]
fn a_chain_of_one_is_still_a_chain() {
    assert_eq!(a_failure().chain().count(), 1);
}

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

#[test]
fn the_taxonomy_counts_hold() {
    assert_eq!(Domain::ALL.len(), 16);
    assert_eq!(Category::ALL.len(), 112);
    assert_eq!(Attribution::ALL.len(), 8);
    assert_eq!(Disposition::ALL.len(), 6);
}
