#![allow(clippy::panic)]

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

#[test]
fn it_never_extrapolates() {
    let source = code_only(&read("crates/mcf-bench/src/project.rs"));
    assert!(
        source.contains("if bytes < smallest.bytes || bytes > largest.bytes"),
        "a file outside the measured range must get no band (B46, B-214)"
    );
}

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

#[test]
fn a_mixed_run_is_not_projected_from() {
    let source = code_only(&read("crates/mcf-cli/src/history.rs"));
    assert!(
        source.contains(r#".is_some_and(|held| held.starts_with("MIXED"))"#),
        "a mixed comparison must not become a point (§6.13, B-081)"
    );
}

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
