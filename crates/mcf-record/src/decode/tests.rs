use mcf_core::attested::Attested;
use mcf_core::failure::{Attribution, Category, Disposition, Failure, Subsystem};
use mcf_core::provenance::{
    Checksum, Licence, Origin, Provenance, Repository, Revision, ToolIdentity, Transformation,
    TransformationKind,
};
use mcf_core::time::{Timestamp, UtcOffset};

use crate::encode;
use crate::json::{Value, parse};

use super::{failure_said, provenance};

fn at(nanos: i128) -> Timestamp {
    Timestamp::from_utc_nanos(nanos, Attested::Unknown)
}

fn a_chain() -> Provenance {
    let upstream = Provenance::acquired(
        Origin::hub(
            Repository::new("owner/original"),
            Some(Revision::new("abc123")),
        ),
        at(1_700_000_000_000_000_000),
    )
    .with_licence(Licence::spdx("apache-2.0"))
    .with_integrity(Checksum::sha256(&"a".repeat(64)).expect("a digest"));

    Provenance::acquired(
        Origin::hub(Repository::new("somebody/original-GGUF"), None),
        at(1_700_000_100_000_000_000),
    )
    .with_licence(Licence::Stated)
    .transformed(Transformation::new(
        TransformationKind::Requantization,
        Attested::Known("Q4_K_M from Q8_0".to_owned()),
        Attested::Known(ToolIdentity::new(
            "a-conversion-tool",
            Some("b1234".to_owned()),
        )),
        Attested::Known(at(1_699_000_000_000_000_000)),
    ))
    .derived_from(upstream)
}

#[test]
fn a_chain_survives_the_record() {
    let original = a_chain();
    let written = encode::provenance(&original);
    let read = provenance(&written).expect("it reads back");
    assert_eq!(read, original);
}

#[test]
fn it_survives_the_bytes_as_well_as_the_structure() {
    let original = a_chain();
    let line = encode::provenance(&original).to_line();
    let parsed = parse(&line).expect("MCF's own JSON parses");
    assert_eq!(provenance(&parsed).expect("it reads back"), original);
}

#[test]
fn unknown_stays_unknown() {
    let bare = Provenance::acquired(Origin::Unattributed, at(0));
    let written = encode::provenance(&bare);
    assert_eq!(written.get("integrity"), Some(&Value::Null));
    assert_eq!(written.get("licence"), Some(&Value::Null));
    assert_eq!(written.get("derived_from"), Some(&Value::Null));

    let read = provenance(&written).expect("it reads back");
    assert_eq!(read, bare);
    assert!(read.integrity().known().is_none());
    assert!(read.licence().known().is_none());
    assert!(read.source().is_none());
}

#[test]
fn every_origin_comes_back_as_itself() {
    for origin in [
        Origin::hub(Repository::new("owner/model"), None),
        Origin::hub(
            Repository::new("owner/model"),
            Some(Revision::new("deadbeef")),
        ),
        Origin::LocalFile {
            path: std::path::PathBuf::from("/home/somebody/model.gguf"),
        },
        Origin::Unattributed,
    ] {
        let written = encode::provenance(&Provenance::acquired(origin.clone(), at(1)));
        let read = provenance(&written).expect("it reads back");
        assert_eq!(read.origin(), &origin);
    }
}

#[test]
fn the_licence_states_stay_apart() {
    let identified = Provenance::acquired(Origin::Unattributed, at(1))
        .with_licence(Licence::spdx("cc-by-nc-4.0"));
    let unmatched = Provenance::acquired(Origin::Unattributed, at(1)).with_licence(Licence::Stated);
    let absent = Provenance::acquired(Origin::Unattributed, at(1));

    for original in [&identified, &unmatched, &absent] {
        let read = provenance(&encode::provenance(original)).expect("it reads back");
        assert_eq!(&read, original);
    }
    assert_ne!(
        encode::provenance(&unmatched).get("licence"),
        encode::provenance(&absent).get("licence"),
        "terms nobody could name were written the same way as no terms at all"
    );
}

#[test]
fn a_moment_comes_back_to_the_nanosecond() {
    let east = UtcOffset::from_seconds_east(3600).expect("an hour east");
    let moment = Timestamp::from_utc_nanos(1_700_000_000_123_456_789, Attested::Known(east));
    let original = Provenance::acquired(Origin::Unattributed, moment);
    let read = provenance(&encode::provenance(&original)).expect("it reads back");
    let read_at = read.retrieved_at().known().copied().expect("a moment");
    assert_eq!(read_at.utc_nanos(), moment.utc_nanos());
    assert_eq!(read_at.offset(), Attested::Known(east));
}

#[test]
fn a_record_missing_a_field_is_refused_and_says_which() {
    let field = "origin";
    let Value::Map(mut entries) = encode::provenance(&a_chain()) else {
        panic!("the encoder writes an object");
    };
    entries.remove(field);
    let failure = provenance(&Value::Map(entries)).expect_err("a field is missing");
    assert_eq!(failure.category(), Category::ArtifactProvenanceIncomplete);
    assert!(
        failure
            .context()
            .iter()
            .any(|entry| entry.value.contains(field)),
        "the refusal does not name {field}"
    );
}

#[test]
fn a_link_nobody_fetched_reads_back_as_one() {
    let never_fetched = Provenance::known_of(Origin::hub(
        Repository::new("owner/original"),
        Some(Revision::new("abc123")),
    ));
    let written = encode::provenance(&never_fetched);
    assert_eq!(written.get("retrieved_at"), Some(&Value::Null));

    let read = provenance(&written).expect("it reads back");
    assert_eq!(read, never_fetched);
    assert!(read.retrieved_at().known().is_none());

    let chain = Provenance::acquired(Origin::Unattributed, at(5)).derived_from(never_fetched);
    assert_eq!(
        provenance(&encode::provenance(&chain)).expect("it reads back"),
        chain
    );
}

#[test]
fn an_unreadable_link_refuses_the_whole_chain() {
    let Value::Map(mut entries) = encode::provenance(&a_chain()) else {
        panic!("the encoder writes an object");
    };
    entries.insert(
        "derived_from".to_owned(),
        Value::map([("origin", Value::map([("kind", Value::text("a rumour"))]))]),
    );
    let failure = provenance(&Value::Map(entries)).expect_err("the upstream is not readable");
    assert_eq!(failure.category(), Category::ArtifactProvenanceIncomplete);
}

#[test]
fn a_checksum_mcf_cannot_compute_is_not_kept() {
    let Value::Map(mut entries) = encode::provenance(&a_chain()) else {
        panic!("the encoder writes an object");
    };
    entries.insert(
        "integrity".to_owned(),
        Value::map([
            ("algorithm", Value::text("md5")),
            ("hex", Value::text("d41d8cd98f00b204e9800998ecf8427e")),
        ]),
    );
    let failure = provenance(&Value::Map(entries)).expect_err("md5 is not one MCF computes");
    assert_eq!(failure.category(), Category::ArtifactProvenanceIncomplete);
}

#[test]
fn a_transformation_keeps_the_words_it_was_given() {
    let original =
        Provenance::acquired(Origin::Unattributed, at(1)).transformed(Transformation::new(
            TransformationKind::Other("pruned by hand, badly".to_owned()),
            Attested::Unknown,
            Attested::Unknown,
            Attested::Unknown,
        ));
    let read = provenance(&encode::provenance(&original)).expect("it reads back");
    assert_eq!(read, original);
}

#[test]
fn an_origin_this_build_does_not_know_is_refused() {
    let value = Value::map([
        (
            "origin",
            Value::map([("kind", Value::text("something_later"))]),
        ),
        ("retrieved_at", encode::timestamp(at(1))),
    ]);
    let failure = provenance(&value).expect_err("this build does not know that kind");
    assert_eq!(failure.category(), Category::ArtifactProvenanceIncomplete);
    assert!(
        failure
            .context()
            .iter()
            .any(|entry| entry.value.contains("something_later")),
        "the refusal does not say what it saw"
    );
}

#[test]
fn what_a_failure_said_is_read_back_whole() {
    let inner = Failure::new(
        Category::ArtifactMissing,
        Attribution::Machine,
        Disposition::Refused,
        Subsystem::new("mcf-hub::store"),
        "this model could not be measured",
    );
    let outer = Failure::new(
        Category::ConfigInvalid,
        Attribution::User,
        Disposition::Refused,
        Subsystem::new("mcf-serve::control"),
        "a client sent something MCF cannot read",
    )
    .with_context("wanted", "a model this machine is holding".to_owned())
    .caused_by(inner);
    let said = failure_said(&encode::failure(&outer)).expect("a failure reads back");
    assert!(
        said.starts_with("a client sent something MCF cannot read"),
        "{said}"
    );
    assert!(
        said.contains("  wanted: a model this machine is holding"),
        "{said}"
    );
    assert!(
        said.contains("caused by: this model could not be measured"),
        "{said}"
    );
    assert_eq!(
        failure_said(&Value::map([("served", Value::Bool(false))])),
        None
    );
}
