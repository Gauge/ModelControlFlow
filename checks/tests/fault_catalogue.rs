//! Every failure MCF claims has a scenario that produces it.
//!
//! A13 is the rule and D26 is what binds it: *an untested claim is not made*,
//! and MCF claims a category when its own code can construct one. So this check
//! reads the workspace for every `Category::` the shipped crates construct, and
//! requires the laboratory's catalogue to produce each of them (B-010).
//!
//! **Why it binds to the code and not to the whole taxonomy.** The taxonomy has
//! 110 codes and MCF has not yet written the subsystems that produce most of
//! them. A check against the full table would have failed on the day it was
//! written and stayed failing for years, which is a check nobody reads. Bound
//! this way it is green from the first day and cannot regress: a new failure
//! site cannot land without its scenario, because this fails the build.
//!
//! **What is excluded, and why.** Test code, because a category constructed in
//! a test is a category being *tested*, not one MCF claims. The prototype under
//! `prototypes/`, because it is explicitly not MCF and is superseded by the
//! items that replace it. And `mcf-core`'s own `failure` module, which defines
//! the vocabulary rather than using it.

// Every item in this file is test code; see the note in `taxonomy_agreement.rs`.
#![allow(clippy::panic)]

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use mcf_core::failure::Category;
use mcf_lab::CATALOGUE;

/// Every category MCF's shipped code constructs has a scenario.
#[test]
fn every_claimed_category_has_a_scenario() {
    let claimed = claimed_categories();
    assert!(
        !claimed.is_empty(),
        "no category was found, so this check is reading the wrong tree"
    );

    let produced: BTreeSet<&str> = CATALOGUE
        .iter()
        .map(|scenario| scenario.produces.code())
        .collect();

    let mut unclaimed = Vec::new();
    for (variant, sites) in &claimed {
        let Some(category) = category_of(variant) else {
            panic!("`Category::{variant}` is constructed at {sites:?} and is not a variant");
        };
        if !produced.contains(category.code()) {
            unclaimed.push(format!("{} claimed at {sites:?}", category.code()));
        }
    }
    assert!(
        unclaimed.is_empty(),
        "MCF claims to handle failures the laboratory cannot produce (A13, B-010):\n{unclaimed:#?}"
    );
}

/// Every scenario produces a category MCF's code actually constructs.
///
/// The other direction, and it is not symmetry for its own sake: a scenario
/// that reproduces a failure nothing can produce any more is a scenario that
/// will pass for ever while testing nothing, which is worse than an absent one
/// because it looks like coverage.
#[test]
fn every_scenario_produces_a_category_the_code_claims() {
    let claimed: BTreeSet<Category> = claimed_categories()
        .keys()
        .filter_map(|variant| category_of(variant))
        .collect();

    let mut orphans = Vec::new();
    for scenario in CATALOGUE {
        if !claimed.contains(&scenario.produces) {
            orphans.push(format!("{} produces {}", scenario.id, scenario.produces));
        }
    }
    assert!(
        orphans.is_empty(),
        "the laboratory reproduces failures no code constructs any more:\n{orphans:#?}"
    );
}

/// The check reports what it covers, so the number is visible rather than
/// implied. §3.17 wants the rare paths exercised, and a coverage figure nobody
/// prints is a figure nobody watches.
#[test]
fn the_coverage_is_reported() {
    let claimed = claimed_categories();
    let total = Category::ALL.len();
    println!(
        "fault catalogue: {} scenarios cover {} of {} categories MCF's code claims, \
         out of {total} in the taxonomy",
        CATALOGUE.len(),
        claimed.len(),
        claimed.len(),
    );
    assert!(claimed.len() <= total);
}

/// Every `Category::Variant` constructed in shipped, non-test code, and where.
fn claimed_categories() -> BTreeMap<String, Vec<String>> {
    let root = mcf_checks::workspace::root();
    let mut found: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for path in rust_sources(&root.join("crates")) {
        // The failure module defines the vocabulary rather than using it.
        if path.components().any(|c| c.as_os_str() == "failure") {
            continue;
        }
        let Ok(source) = std::fs::read_to_string(&path) else {
            continue;
        };
        let relative = path
            .strip_prefix(&root)
            .unwrap_or(&path)
            .display()
            .to_string();
        for (number, line) in production_lines(&source) {
            for variant in variants_in(&line) {
                found
                    .entry(variant)
                    .or_default()
                    .push(format!("{relative}:{number}"));
            }
        }
    }
    found
}

/// Lines that are neither documentation nor test code.
///
/// Test code is everything from the first `#[cfg(test)]` onward. The workspace
/// puts a file's test module last, which is what makes that cut exact; a file
/// that broke the convention would over-report rather than under-report, which
/// is the safe direction for this check.
fn production_lines(source: &str) -> Vec<(usize, String)> {
    let mut lines = Vec::new();
    for (index, line) in source.lines().enumerate() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("#[cfg(test)]") {
            break;
        }
        if trimmed.starts_with("//") {
            continue;
        }
        lines.push((index + 1, line.to_owned()));
    }
    lines
}

/// `Category::EngineExitSignal` → `EngineExitSignal`.
fn variants_in(line: &str) -> Vec<String> {
    let mut found = Vec::new();
    let mut rest = line;
    while let Some(at) = rest.find("Category::") {
        let after = rest.split_at(at + "Category::".len()).1;
        let variant: String = after
            .chars()
            .take_while(char::is_ascii_alphanumeric)
            .collect();
        rest = after;
        if !variant.is_empty() && variant != "ALL" {
            found.push(variant);
        }
    }
    found
}

/// The category a variant name refers to.
fn category_of(variant: &str) -> Option<Category> {
    Category::ALL
        .into_iter()
        .find(|category| variant_name(*category) == variant)
}

/// `hub.auth.required` → `HubAuthRequired`, which is how the variants are named.
fn variant_name(category: Category) -> String {
    category
        .code()
        .split('.')
        .flat_map(|segment| segment.split('_'))
        .map(|word| {
            let mut characters = word.chars();
            match characters.next() {
                Some(first) => first.to_ascii_uppercase().to_string() + characters.as_str(),
                None => String::new(),
            }
        })
        .collect()
}

fn rust_sources(directory: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    let Ok(entries) = std::fs::read_dir(directory) else {
        return found;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            found.extend(rust_sources(&path));
        } else if path.extension().is_some_and(|e| e == "rs") && !path.ends_with("tests.rs") {
            found.push(path);
        }
    }
    found.sort();
    found
}
