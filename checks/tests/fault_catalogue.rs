#![allow(clippy::panic)]

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use mcf_core::failure::Category;
use mcf_lab::CATALOGUE;

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

fn claimed_categories() -> BTreeMap<String, Vec<String>> {
    let root = mcf_checks::workspace::root();
    let mut found: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for path in rust_sources(&root.join("crates")) {
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

fn category_of(variant: &str) -> Option<Category> {
    Category::ALL
        .into_iter()
        .find(|category| variant_name(*category) == variant)
}

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
