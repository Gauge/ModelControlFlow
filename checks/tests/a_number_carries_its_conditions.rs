//! A statistic does not leave a measurement alone (A6, B-073, §3.4, §3.14).
//!
//! **A6's half that was held by nobody.** *No number without its conditions, its
//! sample count and its spread.* `Measurement<Q>` has no constructor that omits
//! a condition set and no rendering that drops one — B-005 made that structural
//! and `measurement_has_one_way_in.rs` holds it. But a surface never had to use
//! that rendering. It could ask for a percentile, get a bare `Q` back, and print
//! it: `mcf doctor` did, reporting `p99 {} over n={}` assembled from two
//! separate asks with **no spread at all**.
//!
//! So the statistic leaves in a [`mcf_core::measurement::Stated`], which renders
//! with its sample count and its spread and has no rendering that does not. The
//! bare number is reachable — comparing a statistic with a ceiling needs it —
//! through `Stated::value()`, named for what calling it does, the way
//! `Content::disclose` and `Touchstone::bare` are. This file is what makes that
//! name load-bearing: a **surface** that calls it is formatting a number without
//! its evidence, and that is the violation.
//!
//! **A test may call it.** The budget tier compares a statistic with a baseline
//! and writes the number to a file; that is arithmetic and a record, not a view.
//! The distinction is the one A22 draws between a surface and everything else,
//! and it is why this check reads `src/` and not `tests/`.

// Every item in this file is test code; see the note in `taxonomy_agreement.rs`.
#![allow(clippy::panic, clippy::expect_used)]

use std::path::PathBuf;

/// The surface crate: what a person reads.
const SURFACE: &str = "crates/mcf-cli/src";

/// Ways to get a bare number out of a measurement.
///
/// Each is legitimate somewhere — arithmetic, a record, a comparison — and none
/// of them is legitimate as the argument of a formatting macro on a surface,
/// which is what A6 forbids and what this file looks for.
const BARE: [&str; 5] = [
    ".value()",
    ".at(Percentile::",
    ".maximum()",
    ".minimum()",
    ".spread().median",
];

/// Where a number becomes something a person reads.
const RENDERING: [&str; 4] = ["format!(", "write!(", "writeln!(", "push_str("];

fn read(relative: &str) -> String {
    let path = mcf_checks::workspace::root().join(relative);
    std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("{} is readable: {error}", path.display()))
}

/// Every `.rs` under a directory.
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

/// The statements of a file, joined across the lines a formatting macro spans.
///
/// A `write!` argument list is written over five lines as often as one, and a
/// check reading line by line would see `COLD_START.statistic(measured)` on its
/// own line and conclude nothing — which is how this check would pass while the
/// surface it watches printed bare numbers.
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

/// No surface formats a number it took out of a measurement.
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
            // A statement that renders *and* takes a statistic out of a
            // measurement in the same breath is one that prints a bare number.
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

/// The statistic a budget is about comes back with its evidence attached.
#[test]
fn a_budgets_statistic_cannot_be_obtained_bare() {
    let source = read("crates/mcf-core/src/self_cost.rs");
    assert!(
        source.contains("pub fn statistic(&self, measured: &Measurement<Q>) -> Stated<Q>"),
        "a budget's statistic must come back as a `Stated`, or a surface can print the number \
         and drop the rest — which is what `mcf doctor` did (A6, B-073)"
    );
}

/// And `Stated` has exactly one rendering, which carries everything.
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
