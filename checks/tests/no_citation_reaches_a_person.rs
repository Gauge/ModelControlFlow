use std::path::PathBuf;

const SURFACES: &[&str] = &[
    "crates/mcf-cli",
    "crates/mcf-tui",
    "crates/mcf-serve",
    "crates/mcf-desk",
    "crates/mcf-core",
    "crates/mcf-bench",
    "crates/mcf-hub",
    "crates/mcf-record",
    "crates/mcf-standin",
    "crates/mcf-helper",
];

fn is_exempt(line: &str) -> bool {
    let trimmed = line.trim_start();
    trimmed.starts_with("//")
        || trimmed.starts_with("#[")
        || trimmed.starts_with("reason =")
        || trimmed.contains("assert")
}

fn sources(relative: &str) -> Vec<PathBuf> {
    let mut found = Vec::new();
    let mut stack = vec![mcf_checks::workspace::root().join(relative)];
    while let Some(path) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&path) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                if path.file_name().is_some_and(|name| name == "tests") {
                    continue;
                }
                stack.push(path);
            } else if path.extension().is_some_and(|kind| kind == "rs")
                && path.file_name().is_none_or(|name| name != "tests.rs")
            {
                found.push(path);
            }
        }
    }
    found.sort();
    found
}

fn literals(line: &str) -> Vec<String> {
    let mut found = Vec::new();
    let mut current: Option<String> = None;
    let mut escaped = false;
    for character in line.chars() {
        match current.as_mut() {
            Some(held) => {
                if escaped {
                    escaped = false;
                    held.push(character);
                } else if character == '\\' {
                    escaped = true;
                } else if character == '"' {
                    found.push(current.take().unwrap_or_default());
                } else {
                    held.push(character);
                }
            }
            None if character == '"' => current = Some(String::new()),
            None => {}
        }
    }
    found
}

fn citation_in(text: &str) -> Option<String> {
    if let Some(at) = text.find('§') {
        return Some(text.get(at..).unwrap_or("§").chars().take(6).collect());
    }
    let words = text.split(|c: char| !(c.is_ascii_alphanumeric() || c == '-'));
    for word in words {
        if let Some((head, tail)) = word.split_once('-')
            && !head.is_empty()
            && head.len() <= 2
            && head.chars().all(|c| c.is_ascii_uppercase())
            && !tail.is_empty()
            && tail.chars().all(|c| c.is_ascii_digit())
        {
            return Some(word.to_owned());
        }
        let letters = word.chars().take_while(char::is_ascii_uppercase).count();
        let digits = word.chars().skip(letters).collect::<String>();
        if (1..=2).contains(&letters)
            && !digits.is_empty()
            && digits.len() <= 3
            && digits.chars().all(|c| c.is_ascii_digit())
            && word.len() == letters + digits.len()
        {
            return Some(word.to_owned());
        }
    }
    None
}

#[test]
fn nothing_a_person_reads_cites_a_document() {
    let mut found = Vec::new();
    for surface in SURFACES {
        for path in sources(surface) {
            let Ok(text) = std::fs::read_to_string(&path) else {
                continue;
            };
            for (number, line) in text.lines().enumerate() {
                if is_exempt(line) {
                    continue;
                }
                for literal in literals(line) {
                    if literal.split_whitespace().count() < 4 {
                        continue;
                    }
                    if let Some(cited) = citation_in(&literal) {
                        let relative = path
                            .strip_prefix(mcf_checks::workspace::root())
                            .unwrap_or(&path)
                            .display();
                        found.push(format!("{relative}:{} → {cited}", number + 1));
                    }
                }
            }
        }
    }
    assert!(
        found.is_empty(),
        "{} sentence(s) a person reads cite a document. The rule belongs in the code and in \
         the record; the screen gets what to do about it:\n{found:#?}",
        found.len()
    );
}

#[test]
fn the_check_knows_a_citation_when_it_sees_one() {
    for cited in [
        "no inference engine is vendored yet (B-320, D32)",
        "the interface may not be the only way to do anything (A22)",
        "idle costs nothing because of §3.13 and nothing else",
        "this was found by F128 while building",
    ] {
        assert!(
            citation_in(cited).is_some(),
            "missed a citation in {cited:?}"
        );
    }
    for clean in [
        "no engine is installed yet — MCF can build one for you",
        "this model needs about 20.00 GB and the largest device here has 2.00 GB free",
        "nothing is listening; mcf serve starts a daemon",
        "recovered 5232 record entries and 34 model files",
    ] {
        assert_eq!(citation_in(clean), None, "false alarm on {clean:?}");
    }
}
