//! A model's recommended settings are a declaration, and MCF's own choice says
//! it is MCF's (B60, D18, A21, B-281).
//!
//! **B60's violation is *a global default temperature applied to every model,
//! which measures each of them under settings some were never designed for*.**
//! The rule against it is not *have no defaults* — a sampler has to be told
//! something — it is that the choice is **attributed**: the artifact's own
//! recommendation is adopted where there is one and marked declared and
//! unverified; a laboratory may pin its own only as declared method, with its
//! results kept apart; and MCF's own choice, where nothing else spoke, is named
//! as MCF's with the reason it had to make one.
//!
//! Three things hold that, and each is one edit from not holding it:
//!
//! 1. `Calibrated` has no constructor that omits the source, so a sampling
//!    configuration cannot exist without saying whose it is;
//! 2. `Sampling` has no `Default`, so there is no value MCF falls back to
//!    without anybody choosing it;
//! 3. MCF **looks** before it chooses — the file's own metadata is read, and
//!    the surface that reports the choice says which of the four states it is.
//!
//! A source check rather than a compile-fail harness, for the reason given in
//! `measurement_has_one_way_in.rs`.

// Every item in this file is test code; see the note in `taxonomy_agreement.rs`.
#![allow(clippy::panic)]

/// Every way to hold a sampling configuration names its source.
#[test]
fn a_sampling_configuration_cannot_exist_without_a_source() {
    /// The four states B60 admits, and no fifth.
    const SOURCES: [&str; 4] = [
        "DeclaredByArtifact",
        "MeasuredHere",
        "PinnedByLaboratory",
        "McfsOwn",
    ];

    let source = code_only(&read("crates/mcf-core/src/configuration/calibrated.rs"));
    for state in SOURCES {
        assert!(
            source.contains(&format!("{state} {{")),
            "the `{state}` state must exist: B60 names four and a missing one is a choice nobody \
             can attribute"
        );
    }
    for forbidden in [
        "Default for Calibrated",
        "chosen: Option<Chosen>",
        "pub fn unattributed",
    ] {
        assert!(
            !source.contains(forbidden),
            "`{forbidden}` would let a sampling configuration exist without saying whose it is"
        );
    }
    // Every constructor sets `chosen`. The list is read from the source so a
    // fifth added later fails here rather than slipping past.
    for signature in constructors(&source) {
        let body = source
            .split_once(&signature)
            .and_then(|(_, rest)| rest.split_once("\n    }"))
            .map(|(body, _)| body)
            .unwrap_or_default();
        assert!(
            body.contains("chosen:"),
            "a constructor that does not attribute the choice: {signature}"
        );
    }
}

/// `Sampling` has no `Default`.
///
/// A `Default` is a global default sampling with the serial numbers filed off:
/// it is a set of values nobody chose, applied wherever a caller did not think
/// to choose, which is precisely B60's violation.
#[test]
fn sampling_has_no_default() {
    let source = code_only(&read("crates/mcf-core/src/configuration/sampling.rs"));
    for forbidden in ["Default for Sampling", "Default,", "Default)]"] {
        assert!(
            !source.contains(forbidden),
            "`{forbidden}` is a global default sampling (B60)"
        );
    }
    assert!(
        source.contains("pub const fn nothing_set()"),
        "the honest empty state must remain, and must not be called a default"
    );
}

/// MCF looks at the artifact before choosing for it.
///
/// B60 has MCF *adopt what the artifact recommends rather than imposing a house
/// style*. A house style is only forgivable where there was nothing to adopt,
/// and that is a question MCF has to actually ask.
#[test]
fn mcf_reads_the_artifacts_recommendation_before_choosing() {
    let reader = code_only(&read("crates/mcf-standin/src/recommended.rs"));
    assert!(
        reader.contains("pub fn read(file: &Model) -> Recommendation"),
        "MCF must read a model file's own sampling recommendation (B60)"
    );
    assert!(
        reader.contains("NoneDeclared"),
        "and *the file recommends nothing* must be a state rather than an empty set: only the \
         first justifies MCF choosing for itself"
    );

    let hub = code_only(&read("crates/mcf-hub/src/recommendation.rs"));
    assert!(
        hub.contains("pub const WHERE: &str = \"generation_config.json\";"),
        "and the repository's recommendation must be read from a named place, so a report can \
         say where MCF looked"
    );
    assert!(
        hub.contains("NothingStated") && hub.contains("NoneDeclared"),
        "a publisher who looked and said nothing and one who did not look are different facts"
    );
}

/// The surface that reports the choice says whose it is.
///
/// §3.15's *no hidden choices*, and the half of B60 a reader actually meets: a
/// house choice that says it is a house choice is a condition somebody can
/// weigh, and one that does not is a hidden default.
#[test]
fn the_surface_says_whose_choice_the_sampler_is() {
    let source = code_only(&read("crates/mcf-cli/src/explain.rs"));
    assert!(
        source.contains("mcf_standin::recommended::read(file)"),
        "`mcf explain` must ask the file before reporting MCF's own choice (B60)"
    );
    assert!(
        source.contains("MCF's own, because this file recommends none"),
        "and must say the choice is MCF's, with the reason it had to make one"
    );
    assert!(
        source.contains("unverified here"),
        "and must mark an adopted recommendation as declared rather than verified (A21)"
    );
}

/// Every `pub fn` in a source that returns `Self`.
fn constructors(source: &str) -> Vec<String> {
    let mut found = Vec::new();
    let mut lines = source.lines().peekable();
    while let Some(line) = lines.next() {
        let trimmed = line.trim();
        if !trimmed.starts_with("pub fn ") && !trimmed.starts_with("pub const fn ") {
            continue;
        }
        let mut signature = trimmed.to_owned();
        while !signature.contains('{') {
            match lines.next() {
                Some(more) => {
                    signature.push(' ');
                    signature.push_str(more.trim());
                }
                None => break,
            }
        }
        if signature.contains("-> Self") {
            found.push(signature);
        }
    }
    assert_eq!(
        found.len(),
        4,
        "B60 names four sources and this file has {} constructors returning one: {found:#?}",
        found.len()
    );
    found
}

/// The source with its documentation comments removed, so that a sentence
/// quoting a forbidden shape is not read as the shape itself.
fn code_only(source: &str) -> String {
    source
        .lines()
        .filter(|line| {
            let trimmed = line.trim_start();
            !trimmed.starts_with("//!") && !trimmed.starts_with("///") && !trimmed.starts_with("//")
        })
        .collect::<Vec<&str>>()
        .join("\n")
}

fn read(relative: &str) -> String {
    let path = mcf_checks::workspace::root().join(relative);
    std::fs::read_to_string(&path).unwrap_or_else(|error| {
        panic!("{} is readable: {error}", path.display());
    })
}
