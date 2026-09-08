#![allow(clippy::panic, clippy::expect_used, clippy::indexing_slicing)]

const CATALOGUE: &str = "scripts/check-mutants.sh";

#[test]
fn every_declared_mutation_can_still_be_placed() {
    let root = mcf_checks::workspace::root();
    let script = std::fs::read_to_string(root.join(CATALOGUE))
        .unwrap_or_else(|error| panic!("{CATALOGUE} is readable: {error}"));

    let files = array(&script, "files");
    let finds = array(&script, "finds");
    let replaces = array(&script, "replaces");
    assert!(!files.is_empty(), "the catalogue is empty");
    assert_eq!(
        files.len(),
        finds.len(),
        "the catalogue names {} files and {} things to find",
        files.len(),
        finds.len()
    );
    assert_eq!(
        finds.len(),
        replaces.len(),
        "the catalogue has {} things to find and {} replacements",
        finds.len(),
        replaces.len()
    );

    let control = scalar(&script, "CONTROL_REPLACE");

    let mut lost = Vec::new();
    for ((file, find), replace) in files.iter().zip(&finds).zip(&replaces) {
        let source = match std::fs::read_to_string(root.join(file)) {
            Ok(source) => source,
            Err(error) => {
                lost.push(format!("{file}: cannot be read ({error})"));
                continue;
            }
        };
        if source.contains(control.as_str()) {
            continue;
        }
        if let Err(why) = placeable(&source, find, replace) {
            lost.push(format!("{file}: {why}"));
        }
    }
    assert_eq!(
        lost,
        Vec::<String>::new(),
        "the mutation catalogue no longer describes the code it is about. A mutation that \
         cannot be placed unambiguously stops the whole scheduled tier with `cannot check` \
         rather than producing a score (B-186), and the person who can fix it is whoever \
         moved the line"
    );
}

#[test]
fn the_equivalent_mutant_control_can_still_be_placed() {
    let root = mcf_checks::workspace::root();
    let script = std::fs::read_to_string(root.join(CATALOGUE)).expect("the catalogue is readable");
    let file = scalar(&script, "CONTROL_FILE");
    let find = scalar(&script, "CONTROL_FIND");
    let replace = scalar(&script, "CONTROL_REPLACE");
    let source = std::fs::read_to_string(root.join(&file))
        .unwrap_or_else(|error| panic!("{file} is readable: {error}"));
    if let Err(why) = placeable(&source, &find, &replace) {
        panic!("the control mutation cannot be placed in {file}: {why}");
    }
}

fn placeable(source: &str, find: &str, replace: &str) -> Result<(), String> {
    let as_written = source.matches(find).count();
    let as_mutated = source.matches(replace).count();
    match (as_written, as_mutated) {
        (1, _) | (0, 1) => Ok(()),
        (0, 0) => Err(format!(
            "no line matches `{find}`, and none is mutated to `{replace}`"
        )),
        (many, _) => Err(format!("{many} lines match `{find}`")),
    }
}

fn array(script: &str, name: &str) -> Vec<String> {
    let opening = format!("declare -a {name}=(");
    let start = script
        .find(&opening)
        .unwrap_or_else(|| panic!("{CATALOGUE} declares no array called {name}"))
        + opening.len();
    let rest = &script[start..];
    let end = rest
        .find("\n)")
        .unwrap_or_else(|| panic!("the {name} array does not end"));
    rest[..end].lines().filter_map(quoted).collect()
}

fn scalar(script: &str, name: &str) -> String {
    let opening = format!("readonly {name}=");
    let start = script
        .find(&opening)
        .unwrap_or_else(|| panic!("{CATALOGUE} declares no {name}"))
        + opening.len();
    quoted(script[start..].lines().next().unwrap_or_default())
        .unwrap_or_else(|| panic!("{name} is not a quoted string"))
}

fn quoted(line: &str) -> Option<String> {
    let trimmed = line.trim();
    if trimmed.is_empty() || trimmed.starts_with('#') {
        return None;
    }
    let inner = trimmed.strip_prefix('"')?.strip_suffix('"')?;
    let mut out = String::with_capacity(inner.len());
    let mut characters = inner.chars();
    while let Some(character) = characters.next() {
        if character != '\\' {
            out.push(character);
            continue;
        }
        match characters.next() {
            Some(next @ ('"' | '\\' | '$' | '`')) => out.push(next),
            Some(next) => {
                out.push('\\');
                out.push(next);
            }
            None => out.push('\\'),
        }
    }
    Some(out)
}
