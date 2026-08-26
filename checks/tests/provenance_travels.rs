//! An artifact handle cannot exist without its provenance.
//!
//! B-006's condition, and A7's: *unknown fields are the `Unknown` variant,
//! never a plausible default*. The type is written so that both hold — one
//! constructor, a private field, no setter, no `Default` — and this reads the
//! module to check that it still is.
//!
//! A source check rather than a compile-fail harness, for the reason given in
//! `measurement_has_one_way_in.rs`.

// Every item in this file is test code; see the note in `taxonomy_agreement.rs`.
#![allow(clippy::panic)]

/// Every way to obtain an `Artifact` takes a `Provenance`.
#[test]
fn every_constructor_takes_a_provenance() {
    // Only `impl Artifact`. `ArtifactName::new` lives in the same file and
    // takes a name, which is correct: a name is not an artifact.
    let source = block(&provenance_source("artifact.rs"), "impl Artifact {");
    let mut constructors = Vec::new();
    for signature in public_functions(&source) {
        if signature.contains("-> Self") {
            constructors.push(signature);
        }
    }
    assert!(
        !constructors.is_empty(),
        "no constructor was found, so this check is reading the wrong file"
    );
    for constructor in &constructors {
        assert!(
            constructor.contains("provenance") || constructor.contains("amendment"),
            "an artifact can be built without provenance: {constructor}"
        );
    }
}

/// Nothing manufactures one, and nothing replaces one after the fact. A
/// mutable provenance would make an artifact's origin a claim the current
/// holder can restate, where §3.6 wants a record.
#[test]
fn nothing_manufactures_or_replaces_a_provenance() {
    let source = provenance_source("artifact.rs");
    for forbidden in [
        "impl Default for Artifact",
        "Default for Artifact",
        "From<ArtifactName> for Artifact",
        "pub fn set_provenance",
        "pub fn provenance_mut",
        "&mut Provenance",
    ] {
        assert!(
            !source.contains(forbidden),
            "`{forbidden}` would make an artifact's origin editable (§3.6)"
        );
    }
}

/// Every field of `Provenance` that MCF has to *read* is `Attested`, so an
/// unread one has no representation other than `Unknown` (A7).
///
/// The origin is exempt and named here: `Origin::Unattributed` already carries
/// the not-known case for it. The retrieval time stopped being exempt with
/// B-019 — a link in the chain MCF never fetched has no retrieval time, and the
/// type said otherwise until §XII's requantization was written down.
#[test]
fn every_readable_field_is_attested() {
    let body = block(&provenance_source("mod.rs"), "pub struct Provenance {");

    // `origin` is exempt because `Origin::Unattributed` already carries the
    // not-known case; the two lists and the chain are exempt because an empty
    // one *is* the absence — nobody transformed it, nobody has looked upstream,
    // it derives from nothing — and `Provenance::last_observation` is where
    // *nobody has checked* is told from *nothing has changed* (A7, D37).
    let exempt = ["origin", "transformations", "observed", "derived_from"];
    let mut checked = 0;
    for line in body.lines() {
        let line = line.trim();
        // A doc comment is prose, and prose has colons in it. Before this the
        // check read one as a field and failed on a sentence.
        if line.starts_with("//") {
            continue;
        }
        let Some((name, kind)) = line.trim_end_matches(',').split_once(": ") else {
            continue;
        };
        if exempt.contains(&name) {
            continue;
        }
        checked += 1;
        assert!(
            kind.starts_with("Attested<"),
            "`{name}` is `{kind}`, so an unread value would have to be invented (A7)"
        );
    }
    assert!(
        checked > 0,
        "no field was checked, so this check is passing vacuously"
    );
}

/// The body of a block, from its opening line to the first line that closes it
/// at column zero.
fn block(source: &str, opening: &str) -> String {
    let (_, rest) = source
        .split_once(opening)
        .unwrap_or_else(|| panic!("`{opening}` is no longer where this check looks"));
    let (body, _) = rest
        .split_once("\n}")
        .unwrap_or_else(|| panic!("`{opening}` is never closed"));
    body.to_owned()
}

fn provenance_source(file: &str) -> String {
    let path = mcf_checks::workspace::root()
        .join("crates/mcf-core/src/provenance")
        .join(file);
    std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("{} is readable: {error}", path.display()))
}

fn public_functions(source: &str) -> Vec<String> {
    let mut found = Vec::new();
    let mut lines = source.lines().map(str::trim).peekable();
    while let Some(line) = lines.next() {
        if !line.starts_with("pub fn ") && !line.starts_with("pub const fn ") {
            continue;
        }
        let mut signature = line.to_owned();
        while !signature.contains('{') {
            match lines.next() {
                Some(continuation) => {
                    signature.push(' ');
                    signature.push_str(continuation);
                }
                None => break,
            }
        }
        found.push(signature);
    }
    found
}
