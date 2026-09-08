#![allow(clippy::panic)]

#[test]
fn the_delta_is_optional_in_the_type() {
    let source = code_only(&bench_source("compare.rs"));
    assert!(
        source.contains("pub const fn verdict(&self) -> Option<&Verdict>"),
        "a confounded comparison must have no delta to hand out, which means the accessor is an \
         `Option` (A8, B-085)"
    );
    assert!(
        source.contains("isolation.is_confounded() && self.declared.is_none()"),
        "the refusal must be reached: a confound nobody declared is an error, and a confound the \
         operator declared is science (A8)"
    );
    assert!(
        source.contains("pub fn declaring("),
        "the operator must be able to declare a confound, or A8's science half is unreachable"
    );
}

#[test]
fn isolation_reads_the_floor_itself() {
    let source = code_only(&core_source("measurement/isolation.rs"));
    assert!(
        source.contains(".floor().entries()"),
        "isolation must enumerate the floor rather than name its fields (§3.3, B16)"
    );
    for written_out in ["thermal_state", "quantization", "context_length"] {
        assert!(
            !source.contains(written_out),
            "`{written_out}` is written out in the isolation check, which is a second copy of a \
             list the floor already holds (§3.3)"
        );
    }
}

#[test]
fn an_unread_condition_is_not_treated_as_a_match() {
    let source = code_only(&core_source("measurement/isolation.rs"));
    assert!(
        source.contains("Undetermined {"),
        "a condition MCF could not read must have its own answer (A7)"
    );
    assert!(
        !source.contains("mine == theirs && "),
        "an unknown must not be compared as if it were a value"
    );
}

fn bench_source(file: &str) -> String {
    read(&format!("crates/mcf-bench/src/{file}"))
}

fn core_source(file: &str) -> String {
    read(&format!("crates/mcf-core/src/{file}"))
}

fn read(relative: &str) -> String {
    let path = mcf_checks::workspace::root().join(relative);
    std::fs::read_to_string(&path).unwrap_or_else(|error| {
        panic!("{} is readable: {error}", path.display());
    })
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
