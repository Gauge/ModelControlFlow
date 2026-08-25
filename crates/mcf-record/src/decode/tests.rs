//! What survives the trip to the disk and back.

use mcf_core::attested::Attested;
use mcf_core::failure::Category;
use mcf_core::provenance::{
    Checksum, Licence, Origin, Provenance, Repository, Revision, ToolIdentity, Transformation,
    TransformationKind,
};
use mcf_core::time::{Timestamp, UtcOffset};

use crate::encode;
use crate::json::{Value, parse};

use super::provenance;

fn at(nanos: i128) -> Timestamp {
    Timestamp::from_utc_nanos(nanos, Attested::Unknown)
}

/// The hard case from §XII: a requantization of somebody else's weights, with
/// the upstream chain kept whole.
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

/// The whole point: what is written is what is read.
#[test]
fn a_chain_survives_the_record() {
    let original = a_chain();
    let written = encode::provenance(&original);
    let read = provenance(&written).expect("it reads back");
    assert_eq!(read, original);
}

/// Including through the text of the file, which is what actually goes to the
/// disk — an encoder and a decoder that agree in memory and disagree about
/// JSON would be two halves of nothing.
#[test]
fn it_survives_the_bytes_as_well_as_the_structure() {
    let original = a_chain();
    let line = encode::provenance(&original).to_line();
    let parsed = parse(&line).expect("MCF's own JSON parses");
    assert_eq!(provenance(&parsed).expect("it reads back"), original);
}

/// A7's round trip: unknown goes out as `null` and comes back as unknown, never
/// as a plausible value.
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

/// The three origins are distinct on the way back, because *a local file* and
/// *nobody can say* are different answers (A9).
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

/// And so do the licence's three states, which B-023 exists to keep apart.
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

/// A timestamp comes back to the nanosecond, offset included — the offset is
/// what makes a record from another machine legible (B-352).
#[test]
fn a_moment_comes_back_to_the_nanosecond() {
    let east = UtcOffset::from_seconds_east(3600).expect("an hour east");
    let moment = Timestamp::from_utc_nanos(1_700_000_000_123_456_789, Attested::Known(east));
    let original = Provenance::acquired(Origin::Unattributed, moment);
    let read = provenance(&encode::provenance(&original)).expect("it reads back");
    assert_eq!(read.retrieved_at().utc_nanos(), moment.utc_nanos());
    assert_eq!(read.retrieved_at().offset(), Attested::Known(east));
}

/// A record missing something it must state is refused, naming the field.
/// Nothing is filled in: a provenance MCF partly invented is indistinguishable
/// from one it read (A7, A21).
#[test]
fn a_record_missing_a_field_is_refused_and_says_which() {
    let complete = encode::provenance(&a_chain());
    for field in ["origin", "retrieved_at"] {
        let Value::Map(mut entries) = complete.clone() else {
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
}

/// A chain whose upstream link is unreadable refuses the derivative too: a
/// chain with an invented link is worse than no chain (A1, §XII).
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

/// An algorithm this build does not compute is refused rather than recorded: a
/// checksum MCF cannot check is not a checksum, and keeping it would make an
/// unverifiable artifact look verified (A21).
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

/// A transformation nobody has a name for keeps the words it was given (A7).
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

/// An origin from a later MCF is refused rather than guessed at (§7.30): a
/// build that invented a meaning for a kind it does not know would be a build
/// that reads newer records wrongly and says nothing.
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
