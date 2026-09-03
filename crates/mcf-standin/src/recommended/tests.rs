//! What reading a recommendation has to get right.

use std::collections::BTreeMap;

use mcf_core::attested::Attested;
use mcf_core::configuration::Thousandths;

use crate::gguf::{Model, Value};

use super::{Recommendation, read};

/// A model file carrying exactly the metadata a test names.
fn file(entries: &[(&str, Value)]) -> Model {
    Model {
        version: 3,
        metadata: entries
            .iter()
            .map(|(key, value)| ((*key).to_owned(), value.clone()))
            .collect::<BTreeMap<String, Value>>(),
        tensors: Vec::new(),
        data_offset: 0,
        alignment: 32,
    }
}

/// **The ordinary case, and the one six of six real files gave** (F63): the
/// file recommends nothing, which is a state to report rather than a hole to
/// fill.
#[test]
fn a_file_that_says_nothing_recommends_nothing() {
    let held = read(&file(&[
        ("general.architecture", Value::Text("llama".to_owned())),
        ("llama.block_count", Value::Integer(6)),
    ]));
    assert_eq!(held, Recommendation::NoneDeclared);
    assert!(held.sampling().is_none());
}

/// A file that states a temperature has recommended a temperature, and the
/// parameters it did not state stay unknown — inventing the rest is the
/// substitution A7 forbids.
#[test]
fn what_the_file_states_is_read_and_no_more() {
    let held = read(&file(&[
        ("general.architecture", Value::Text("llama".to_owned())),
        ("llama.temperature", Value::Float(0.7)),
    ]));
    let Recommendation::Declared { sampling, keys } = &held else {
        panic!("a stated temperature is a recommendation: {held:?}")
    };
    assert_eq!(sampling.temperature, Attested::Known(Thousandths(700)));
    assert_eq!(sampling.top_p, Attested::Unknown);
    assert_eq!(sampling.top_k, Attested::Unknown);
    assert_eq!(
        keys,
        &["llama.temperature".to_owned()],
        "which key was read travels with the claim, because *the artifact says so* is not \
         checkable and *this key says so* is (A21)"
    );
}

/// Every parameter the type carries can be read, under the architecture the
/// file declares — which is how GGUF namespaces everything else it says.
///
/// The architecture here is invented rather than named from a real family:
/// what is under test is the namespacing, and B28 keeps a particular model out
/// of the code either way.
#[test]
fn every_parameter_is_read_under_the_declared_architecture() {
    let held = read(&file(&[
        (
            "general.architecture",
            Value::Text("an-architecture".to_owned()),
        ),
        ("an-architecture.temperature", Value::Float(0.6)),
        ("an-architecture.top_p", Value::Float(0.95)),
        ("an-architecture.top_k", Value::Integer(20)),
        ("an-architecture.min_p", Value::Float(0.05)),
        ("an-architecture.repetition_penalty", Value::Float(1.05)),
        ("an-architecture.max_output_tokens", Value::Integer(2048)),
    ]));
    let Recommendation::Declared { sampling, keys } = &held else {
        panic!("six stated parameters are a recommendation: {held:?}")
    };
    assert_eq!(sampling.temperature, Attested::Known(Thousandths(600)));
    assert_eq!(sampling.top_p, Attested::Known(Thousandths(950)));
    assert_eq!(sampling.top_k, Attested::Known(20));
    assert_eq!(sampling.min_p, Attested::Known(Thousandths(50)));
    assert_eq!(
        sampling.repetition_penalty,
        Attested::Known(Thousandths(1_050))
    );
    assert_eq!(sampling.max_output_tokens, Attested::Known(2_048));
    assert_eq!(keys.len(), 6);
}

/// **The namespace the converter writes and the engine reads** (F157). Three
/// of ten files on the machine that found this state their recommendation as
/// `general.sampling.temp`, `general.sampling.top_p` and
/// `general.sampling.top_k` — the engine's own spellings, under no
/// architecture — and were reported as recommending nothing.
#[test]
fn the_engines_namespace_is_read_with_its_own_spellings() {
    let held = read(&file(&[
        ("general.architecture", Value::Text("llama".to_owned())),
        ("general.sampling.temp", Value::Float(0.7)),
        ("general.sampling.top_p", Value::Float(0.8)),
        ("general.sampling.top_k", Value::Integer(20)),
        ("general.sampling.min_p", Value::Float(0.0)),
        ("general.sampling.penalty_repeat", Value::Float(1.05)),
    ]));
    let Recommendation::Declared { sampling, keys } = &held else {
        panic!("the engine's namespace is a recommendation: {held:?}")
    };
    assert_eq!(sampling.temperature, Attested::Known(Thousandths(700)));
    assert_eq!(sampling.top_p, Attested::Known(Thousandths(800)));
    assert_eq!(sampling.top_k, Attested::Known(20));
    assert_eq!(sampling.min_p, Attested::Known(Thousandths(0)));
    assert_eq!(
        sampling.repetition_penalty,
        Attested::Known(Thousandths(1_050))
    );
    assert_eq!(sampling.max_output_tokens, Attested::Unknown);
    assert_eq!(
        keys,
        &[
            "general.sampling.temp".to_owned(),
            "general.sampling.top_p".to_owned(),
            "general.sampling.min_p".to_owned(),
            "general.sampling.penalty_repeat".to_owned(),
            "general.sampling.top_k".to_owned(),
        ],
        "the keys read are the engine's spellings, so the claim is checkable against the file"
    );
}

/// The engine's namespace needs no architecture to be looked in: it is where
/// the engine looks, and the engine does not ask first.
#[test]
fn the_engines_namespace_is_read_without_an_architecture() {
    let held = read(&file(&[("general.sampling.temp", Value::Float(0.6))]));
    assert!(
        matches!(held.sampling(), Some(sampling) if sampling.temperature == Attested::Known(Thousandths(600))),
        "{held:?}"
    );
}

/// Where a file states one parameter in both namespaces, the engine's governs
/// — it is the value the provisioned engine applies when it runs the file —
/// and the key recorded is the one whose value was taken.
#[test]
fn where_both_namespaces_state_a_parameter_the_engines_governs() {
    let held = read(&file(&[
        ("general.architecture", Value::Text("llama".to_owned())),
        ("general.sampling.temp", Value::Float(0.6)),
        ("llama.temperature", Value::Float(1.0)),
    ]));
    let Recommendation::Declared { sampling, keys } = &held else {
        panic!("{held:?}")
    };
    assert_eq!(sampling.temperature, Attested::Known(Thousandths(600)));
    assert_eq!(keys, &["general.sampling.temp".to_owned()]);
}

/// A file that declares no architecture has no namespace to look in, so it
/// recommends nothing — the same answer as a file that declares one and says
/// nothing in it, and right for the same reason.
#[test]
fn a_file_with_no_architecture_recommends_nothing() {
    assert_eq!(
        read(&file(&[("llama.temperature", Value::Float(0.7))])),
        Recommendation::NoneDeclared,
        "a key under an architecture the file does not claim is not this file's recommendation"
    );
}

/// **Every input is untrusted** (§3.7). A value outside what a sampler
/// parameter can be is not read rather than clamped: a clamped value is a
/// number MCF chose, and a file stating one is a file MCF should not adopt
/// from.
#[test]
fn a_value_outside_what_a_parameter_can_be_is_not_read() {
    for hostile in [
        Value::Float(-1.0),
        Value::Float(f64::NAN),
        Value::Float(f64::INFINITY),
        Value::Float(1e9),
        Value::Text("0.7".to_owned()),
        Value::Bool(true),
    ] {
        let held = read(&file(&[
            ("general.architecture", Value::Text("llama".to_owned())),
            ("llama.temperature", hostile.clone()),
        ]));
        assert_eq!(
            held,
            Recommendation::NoneDeclared,
            "{hostile:?} is not a temperature, and is refused rather than repaired"
        );
    }
}

/// A top-k of a size no vocabulary has is still an integer and is read: what
/// is checkable here is the *type*, and whether the number is sensible for a
/// given model is a question about that model (A21 reads, it does not judge).
#[test]
fn an_implausible_but_well_typed_value_is_read_and_marked_rather_than_refused() {
    let held = read(&file(&[
        ("general.architecture", Value::Text("llama".to_owned())),
        ("llama.top_k", Value::Integer(4_000_000)),
    ]));
    assert!(
        matches!(held.sampling(), Some(sampling) if sampling.top_k == Attested::Known(4_000_000)),
        "a well-typed value is what the file says, and saying so is all this does: {held:?}"
    );
}
