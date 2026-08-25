//! Tests for where a number came from.
//!
//! The separation itself is the compiler's to enforce; what is tested is that
//! neither type hides what it is on a surface, and that the sample count B44
//! requires is not something a caller can forget.

use super::{FromCorpus, LocallyMeasured};
use crate::measurement::Count;

/// B43: a corpus statement is labelled, carries its sample count, and is
/// visibly distinguishable from a local measurement. All three in one line,
/// because a label that lives only in the type is a label nobody sees.
#[test]
fn a_corpus_value_says_what_it_is_and_what_it_rests_on() {
    let advice = FromCorpus::new(Count(41), 400);
    let rendered = advice.to_string();
    assert!(rendered.contains("from the corpus"), "{rendered}");
    assert!(rendered.contains("400 reports"), "{rendered}");
    assert_eq!(advice.reports(), 400);
}

/// B44: a claim resting on two reports reads differently from one resting on
/// four hundred, and one resting on nothing says so.
#[test]
fn a_thin_claim_reads_differently_from_a_thick_one() {
    assert!(
        FromCorpus::new(Count(1), 2)
            .to_string()
            .contains("2 reports")
    );
    assert!(
        FromCorpus::new(Count(1), 1)
            .to_string()
            .contains("1 report,")
            || FromCorpus::new(Count(1), 1)
                .to_string()
                .contains("1 report)")
    );
    assert!(
        FromCorpus::new(Count(1), 0)
            .to_string()
            .contains("resting on nothing")
    );
}

/// A local value says it was measured here, so the two never read alike on a
/// surface even when they hold the same number.
#[test]
fn a_local_value_and_a_corpus_value_never_read_alike() {
    let here = LocallyMeasured::new(Count(41));
    let elsewhere = FromCorpus::new(Count(41), 400);
    assert_ne!(here.to_string(), elsewhere.to_string());
    assert!(here.to_string().contains("measured here"));
    assert!(!here.to_string().contains("corpus"));
}

/// Transforming a value keeps what it is. A `map` that dropped the origin
/// would be the silent promotion B43 exists to prevent, wearing a combinator —
/// the same shape A5's degradation mark has.
#[test]
fn transforming_keeps_the_origin() {
    let doubled = LocallyMeasured::new(Count(21)).map(|Count(n)| Count(n * 2));
    assert_eq!(doubled.value(), &Count(42));
    assert!(doubled.to_string().contains("measured here"));

    let advice = FromCorpus::new(Count(21), 7).map(|Count(n)| Count(n * 2));
    assert_eq!(advice.advises(), &Count(42));
    assert_eq!(advice.reports(), 7);
}

/// The corpus accessor is named for what the corpus may do. B43: arithmetic
/// may refuse; the corpus may only advise.
#[test]
fn the_corpus_accessor_is_named_for_what_it_permits() {
    let advice = FromCorpus::new(Count(3), 5);
    assert_eq!(advice.advises(), &Count(3));
}
