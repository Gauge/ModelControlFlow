#![allow(clippy::panic, clippy::expect_used)]

use std::path::PathBuf;

const SURFACE: &str = "crates/mcf-cli/src";

const BARE: [&str; 5] = [
    ".value()",
    ".at(Percentile::",
    ".maximum()",
    ".minimum()",
    ".spread().median",
];

const RENDERING: [&str; 4] = ["format!(", "write!(", "writeln!(", "push_str("];

fn read(relative: &str) -> String {
    let path = mcf_checks::workspace::root().join(relative);
    std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("{} is readable: {error}", path.display()))
}

fn sources(relative: &str) -> Vec<PathBuf> {
    let root = mcf_checks::workspace::root().join(relative);
    let mut found = Vec::new();
    let mut stack = vec![root];
    while let Some(directory) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&directory) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().is_some_and(|kind| kind == "rs") {
                found.push(path);
            }
        }
    }
    found.sort();
    found
}

fn statements(source: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut held = String::new();
    let mut depth = 0_i32;
    for line in source.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("//") && depth == 0 {
            continue;
        }
        held.push(' ');
        held.push_str(trimmed);
        depth += i32::try_from(trimmed.matches('(').count()).unwrap_or(0);
        depth -= i32::try_from(trimmed.matches(')').count()).unwrap_or(0);
        if depth <= 0 && (trimmed.ends_with(';') || trimmed.ends_with('{')) {
            out.push(std::mem::take(&mut held));
            depth = 0;
        }
    }
    if !held.is_empty() {
        out.push(held);
    }
    out
}

#[test]
fn no_surface_renders_a_statistic_without_its_evidence() {
    let mut offenders = Vec::new();
    for path in sources(SURFACE) {
        let Ok(source) = std::fs::read_to_string(&path) else {
            continue;
        };
        let name = path
            .strip_prefix(mcf_checks::workspace::root())
            .unwrap_or(&path)
            .display()
            .to_string();
        for statement in statements(&source) {
            if !RENDERING.iter().any(|held| statement.contains(held)) {
                continue;
            }
            if !statement.contains(".statistic(") {
                continue;
            }
            if let Some(bare) = BARE.iter().find(|held| statement.contains(**held)) {
                offenders.push(format!("{name}: renders `{bare}` — {}", statement.trim()));
            }
        }
    }
    assert!(
        offenders.is_empty(),
        "a surface renders a number taken out of a measurement, without the sample count and \
         spread that make it a measurement (A6, B-073). `Stated` renders all three and has no \
         rendering that does not; `value()` is for arithmetic:\n{offenders:#?}"
    );
}

#[test]
fn a_budgets_statistic_cannot_be_obtained_bare() {
    let source = read("crates/mcf-core/src/self_cost.rs");
    assert!(
        source.contains("pub fn statistic(&self, measured: &Measurement<Q>) -> Stated<Q>"),
        "a budget's statistic must come back as a `Stated`, or a surface can print the number \
         and drop the rest — which is what `mcf doctor` did (A6, B-073)"
    );
}

#[test]
fn a_stated_statistic_has_no_rendering_that_drops_its_evidence() {
    let source = read("crates/mcf-core/src/measurement/mod.rs");
    let (_, after) = source
        .split_once("impl<Q: Quantity> fmt::Display for Stated<Q>")
        .expect("`Stated` renders itself");
    let body = after.split_once("\n}\n").map_or(after, |(held, _)| held);
    for required in [
        "self.n",
        "self.spread.median",
        "self.spread.p5",
        "self.spread.p95",
        "self.what",
    ] {
        assert!(
            body.contains(required),
            "`Stated`'s rendering drops {required}, so a statistic can reach a reader without \
             the evidence A6 requires beside it"
        );
    }
    assert_eq!(
        source
            .matches("impl<Q: Quantity> fmt::Display for Stated<Q>")
            .count(),
        1,
        "a second rendering of a statistic is a rendering that can drop something"
    );
}
