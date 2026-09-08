use super::{FromCorpus, LocallyMeasured};
use crate::measurement::Count;

#[test]
fn a_corpus_value_says_what_it_is_and_what_it_rests_on() {
    let advice = FromCorpus::new(Count(41), 400);
    let rendered = advice.to_string();
    assert!(rendered.contains("from the corpus"), "{rendered}");
    assert!(rendered.contains("400 reports"), "{rendered}");
    assert_eq!(advice.reports(), 400);
}

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

#[test]
fn a_local_value_and_a_corpus_value_never_read_alike() {
    let here = LocallyMeasured::new(Count(41));
    let elsewhere = FromCorpus::new(Count(41), 400);
    assert_ne!(here.to_string(), elsewhere.to_string());
    assert!(here.to_string().contains("measured here"));
    assert!(!here.to_string().contains("corpus"));
}

#[test]
fn transforming_keeps_the_origin() {
    let doubled = LocallyMeasured::new(Count(21)).map(|Count(n)| Count(n * 2));
    assert_eq!(doubled.value(), &Count(42));
    assert!(doubled.to_string().contains("measured here"));

    let advice = FromCorpus::new(Count(21), 7).map(|Count(n)| Count(n * 2));
    assert_eq!(advice.advises(), &Count(42));
    assert_eq!(advice.reports(), 7);
}

#[test]
fn the_corpus_accessor_is_named_for_what_it_permits() {
    let advice = FromCorpus::new(Count(3), 5);
    assert_eq!(advice.advises(), &Count(3));
}
