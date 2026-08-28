//! A projection is band-shaped, marked, local, and absent where there is no
//! history (A20, B46, B34, B-214, PR3).
//!
//! A20 permits the middle answer and then draws the line absolutely: *an
//! estimate can never be mistaken for a measurement, never be promoted into
//! one, and never be compared with one. It can only be replaced by one.* The
//! type wall is `Estimate`'s and `measurement_has_one_way_in.rs` guards it.
//! What this file guards is the four conditions B-214 adds, each of which is a
//! sentence rather than a type and therefore easy to lose:
//!
//! 1. **band-shaped** — a duration predicted from a rate is a range, and one
//!    number is *the smallest possible version of a confident wrong number*
//!    (B46);
//! 2. **marked** — in the sentence a reader meets, not only in a type name
//!    nobody sees;
//! 3. **from local history only** — the corpus advises and never decides, and
//!    there is no corpus (B34);
//! 4. **absent where there is no history**, and absent past the ends of it,
//!    because a straight line beyond the data is the thing B46 forbids.
//!
//! A source check rather than a compile-fail harness, for the reason given in
//! `measurement_has_one_way_in.rs`.

// Every item in this file is test code; see the note in `taxonomy_agreement.rs`.
#![allow(clippy::panic)]

/// A projection is a band, and its basis is local history.
#[test]
fn a_projection_is_a_band_from_local_history() {
    let source = code_only(&read("crates/mcf-bench/src/project.rs"));
    assert!(
        source.contains("-> Result<Projection, NoBand>")
            && source.contains("band: Estimate<Duration<Monotonic>>"),
        "a projection must carry an `Estimate`, which is the type A20's wall is built from — \
         and only an `Estimate`, whatever else travels beside it (B-385)"
    );
    assert!(
        source.contains("Estimate::band("),
        "and a band rather than a point: one number is the smallest possible confident wrong \
         number (B46)"
    );
    assert!(
        source.contains("Basis::LocalHistory"),
        "from this machine's own history (B34)"
    );
    for forbidden in ["Basis::Corpus", "Basis::VendorModel", "Estimate::point("] {
        assert!(
            !source.contains(forbidden),
            "`{forbidden}` is not what B-214 projects from"
        );
    }
}

/// Every way there is no projection is a named state, not an empty band.
#[test]
fn an_absent_projection_says_why() {
    let source = code_only(&read("crates/mcf-bench/src/project.rs"));
    for state in [
        "NoHistoryAtThatBudget",
        "OutsideWhatWasMeasured",
        "TooLittleHistory",
    ] {
        assert!(
            source.contains(&format!("{state} {{")),
            "`{state}` must exist: *MCF has never measured anything like this here* is a useful \
             answer and a band with nothing under it is not (A7)"
        );
        assert!(
            source.contains(&format!("Err(NoBand::{state}")),
            "and be reached, not merely declared"
        );
    }
}

/// It reads *between* measured points and never past them.
///
/// F67 measured the relationship this rests on and also measured where it stops
/// being straight: a line through the extremes predicted the largest point
/// eleven percent low. Extrapolating would be the confident wrong number.
#[test]
fn it_never_extrapolates() {
    let source = code_only(&read("crates/mcf-bench/src/project.rs"));
    assert!(
        source.contains("if bytes < smallest.bytes || bytes > largest.bytes"),
        "a file outside the measured range must get no band (B46, B-214)"
    );
}

/// The surface a reader meets says the word.
///
/// A20's *clearly-labelled* is not satisfied by a type name nobody sees. The
/// sentence has to contain it, and it has to say what the reader may not do
/// with it.
#[test]
fn the_surface_calls_it_an_estimate() {
    let source = read("crates/mcf-cli/src/explain.rs");
    assert!(
        source.contains("which is an ESTIMATE"),
        "the sentence a reader meets must say so (A20)"
    );
    assert!(
        source.contains("forbids it standing beside a measurement"),
        "and say what A20 forbids being done with it"
    );
    assert!(
        source.contains("There is no projection either:"),
        "and where there is none, say why rather than falling silent (A7)"
    );
}

/// A mixed run is not a point.
///
/// §6.13: a run whose trials were not alike is not one measurement, so it is
/// not one point either — projecting from it would carry the mixture into the
/// projection without anybody seeing it.
#[test]
fn a_mixed_run_is_not_projected_from() {
    let source = code_only(&read("crates/mcf-cli/src/history.rs"));
    assert!(
        source.contains(r#".is_some_and(|held| held.starts_with("MIXED"))"#),
        "a mixed comparison must not become a point (§6.13, B-081)"
    );
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
