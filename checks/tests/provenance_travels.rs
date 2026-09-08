#![allow(clippy::panic)]

#[test]
fn every_constructor_takes_a_provenance() {
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

#[test]
fn every_readable_field_is_attested() {
    let body = block(&provenance_source("mod.rs"), "pub struct Provenance {");

    let exempt = ["origin", "transformations", "observed", "derived_from"];
    let mut checked = 0;
    for line in body.lines() {
        let line = line.trim();
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
