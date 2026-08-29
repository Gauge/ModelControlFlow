//! Environment failures are a distinct branch from model failures
//! (B-233, B49, §7.10, §3.1, §3.4).
//!
//! **The failure this prevents.** An out-of-memory caused by another process
//! competing for the machine is a condition of the run. Recorded as the
//! model's, it becomes *this model gave up* — a claim about a model arrived at
//! by measuring a busy afternoon. Nothing downstream can undo it: the
//! attribution is what a reader sees, and a wrong one reads exactly like a
//! right one.
//!
//! **Which axis answers.** The first design here read the branch from the
//! *category*, and that is wrong in a way worth recording, because it is the
//! obvious design. `probe.inconclusive` is MCF's when its own logic could not
//! decide and the machine's when the machine misbehaved; `engine.unavailable`
//! is MCF's when the stand-in does not implement a format and the machine's
//! when nothing is installed; `config.invalid` is the operator's. A category
//! says *what went wrong*. Only the attribution says *whose* — which is why
//! `Failure::new` demands one and has no default, and why B-233 was already
//! structurally satisfied before this file existed.
//!
//! So what is checked here is the part that is not structural: that the
//! grouping exists and is unambiguous, that *MCF cannot tell* stays its own
//! answer rather than being folded into a branch, and that no failure in this
//! workspace attributes a model's behaviour to anything but the model or a
//! non-behavioural category to the model.

// Every item in this file is test code; see the note in `taxonomy_agreement.rs`.
#![allow(clippy::panic, clippy::expect_used, clippy::unwrap_used)]

use std::path::{Path, PathBuf};

use mcf_core::failure::{Attribution, Branch, Category, Disposition, Domain, Failure, Subsystem};

const EVERY: [Attribution; 8] = [
    Attribution::Mcf,
    Attribution::Managed,
    Attribution::Machine,
    Attribution::Hub,
    Attribution::Artifact,
    Attribution::User,
    Attribution::ModelUnderTest,
    Attribution::Unattributable,
];

/// Only the model under test reaches the model's branch.
#[test]
fn nothing_but_the_model_is_evidence_about_the_model() {
    for attribution in EVERY {
        let branch = attribution.branch();
        assert_eq!(
            branch == Some(Branch::TheModel),
            attribution == Attribution::ModelUnderTest,
            "{attribution:?} reaches the model's branch without being the model: the only \
             evidence about a model is what the model did (B-233)"
        );
    }
}

/// The machine's conditions are the environment's, not anybody's fault.
#[test]
fn the_machine_and_the_hub_are_the_environment() {
    for attribution in [Attribution::Machine, Attribution::Hub] {
        assert_eq!(
            attribution.branch(),
            Some(Branch::TheEnvironment),
            "an out-of-memory from competition is a condition of the run (B-233, §3.4)"
        );
    }
}

/// *MCF cannot tell* is its own answer, not a fifth branch and not a fourth.
#[test]
fn unattributable_is_not_folded_into_a_branch() {
    assert_eq!(
        Attribution::Unattributable.branch(),
        None,
        "B24 makes *MCF cannot tell* a real result, and folding it into a branch would be the \
         attribution MCF refused to make, made anyway"
    );
}

/// Every attribution answers, and answers once.
#[test]
fn the_grouping_is_total_and_unambiguous() {
    for attribution in EVERY {
        assert_eq!(
            attribution.branch(),
            attribution.branch(),
            "{attribution:?} must answer deterministically"
        );
    }
    let reached: Vec<Branch> = EVERY.into_iter().filter_map(Attribution::branch).collect();
    for branch in [
        Branch::TheModel,
        Branch::TheEnvironment,
        Branch::TheArtifact,
        Branch::McfItself,
    ] {
        assert!(
            reached.contains(&branch),
            "no attribution reaches {branch}, so the branch is unreachable and the taxonomy \
             claims a distinction it cannot make"
        );
    }
}

/// A failure reports its branch from its attribution.
#[test]
fn a_failure_carries_the_branch_of_its_attribution() {
    let competed = Failure::new(
        Category::ResourceMemoryExhausted,
        Attribution::Machine,
        Disposition::Refused,
        Subsystem::new("checks"),
        "the machine ran out of memory while another process held it",
    );
    assert_eq!(competed.branch(), Some(Branch::TheEnvironment));
    assert!(
        competed.branch() != Some(Branch::TheModel),
        "B-233's own example: an out-of-memory from competition is never the model giving up"
    );
}

/// Nothing in this workspace records a model's behaviour as anybody else's,
/// or anybody else's as the model's.
#[test]
fn no_construction_crosses_the_two_branches() {
    let mut wrong = Vec::new();
    for path in sources() {
        let Ok(source) = std::fs::read_to_string(&path) else {
            continue;
        };
        let mut rest = source.as_str();
        while let Some(at) = rest.find("Failure::new(") {
            let after = rest.get(at..).unwrap_or_default();
            let call = after.get(..after.len().min(400)).unwrap_or_default();
            if let Some((category, attribution)) = pair(call)
                && let (Some(category), Some(attribution)) =
                    (named(&category), attributed(&attribution))
            {
                let behaviour = category.domain() == Domain::Model;
                let blamed = attribution == Attribution::ModelUnderTest;
                if behaviour != blamed && attribution != Attribution::Unattributable {
                    wrong.push(format!(
                        "{}: {category} attributed to {attribution:?}",
                        path.display()
                    ));
                }
            }
            rest = after.get("Failure::new(".len()..).unwrap_or_default();
        }
    }
    assert!(
        wrong.is_empty(),
        "a failure of the model's behaviour attributed elsewhere, or a condition of the run \
         attributed to the model, is B-233's failure in one direction or the other: {wrong:#?}"
    );
}

/// The first two arguments of a `Failure::new` call, as written.
fn pair(call: &str) -> Option<(String, String)> {
    let inner = call.split_once("Failure::new(")?.1;
    let mut arguments = inner.split(',');
    let category = arguments.next()?.trim().to_owned();
    let attribution = arguments.next()?.trim().to_owned();
    Some((category, attribution))
}

fn named(written: &str) -> Option<Category> {
    let wanted = written.trim().strip_prefix("Category::")?;
    Category::ALL
        .into_iter()
        .find(|category| format!("{category:?}") == wanted)
}

fn attributed(written: &str) -> Option<Attribution> {
    let wanted = written.trim().strip_prefix("Attribution::")?;
    EVERY.into_iter().find(|held| format!("{held:?}") == wanted)
}

/// Every Rust source in the workspace's crates.
fn sources() -> Vec<PathBuf> {
    let mut found = Vec::new();
    walk(&mcf_checks::workspace::root().join("crates"), &mut found);
    found
}

fn walk(directory: &Path, into: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(directory) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            walk(&path, into);
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            into.push(path);
        }
    }
}
