#![allow(clippy::panic)]

#[test]
fn only_the_monotonic_clock_may_be_published() {
    let source = code_only(&read("crates/mcf-core/src/time/clock.rs"));
    let implementations: Vec<&str> = source
        .lines()
        .filter(|line| line.trim_start().starts_with("impl Measurable for "))
        .collect();
    assert_eq!(
        implementations,
        ["impl Measurable for Monotonic {}"],
        "a clock other than the monotonic one has been declared publishable (A11, B-082)"
    );
    assert!(
        source.contains("pub trait Measurable: ClockKind {}"),
        "this check is reading the wrong file"
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
