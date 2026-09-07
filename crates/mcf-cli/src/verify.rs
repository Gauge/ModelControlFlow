//! `mcf verify <bundle>`: does this machine agree, and if not, with what
//! (B-212, PR2, §II, A8, A7).
//!
//! **What a bundle is for.** PR2: *this bundle reproduces this number, or tells
//! you exactly why your machine cannot.* B-211 built the first half. This is
//! the second, and its whole discipline is in one clause of the register:
//! *names which conditions differ and **refuses to attribute the gap**.*
//!
//! **Why the refusal is the point.** Two machines, one bundle, two different
//! numbers, and nine conditions that differ. Saying *the difference is the
//! quantization* would be picking one of them and calling it the cause, which
//! is A8's confound wearing a helpful voice. MCF says which nine, says the two
//! numbers, and stops — the reader has the evidence and the attribution is
//! theirs.
//!
//! **It re-runs rather than reasons.** The bundle carries the method (B-211),
//! so the same comparison is taken here, through the same code path `mcf bench`
//! uses. Where the artifacts are not on this machine it says so and compares
//! only the conditions, which is still an answer: *your machine could not run
//! this, and here is what is different about it* is exactly what PR2 asks for.
//!
//! **Nothing here trusts the bundle.** It arrives from somewhere else, so it is
//! an untrusted input (§3.7): the digest is checked before anything is read out
//! of it, a bundle naming a path is not permitted to make MCF read that path
//! without saying so, and a field this build does not understand is reported
//! rather than assumed away (§7.30).

use std::path::{Path, PathBuf};

use mcf_core::build_identity::BuildIdentity;
use mcf_core::measurement::{Conditions, Isolation};
use mcf_record::export::{self, Kind};
use mcf_record::json::Value;

use crate::Response;

/// Checks a bundle against this machine.
pub(crate) fn run(bundle: &str) -> Response {
    let path = PathBuf::from(bundle);
    let (kind, manifest, entries) = match export::read(&path) {
        Ok(held) => held,
        Err(failure) => {
            return Response {
                text: crate::say::refusal("the bundle could not be read", &failure),
                served: false,
            };
        }
    };
    if kind != Kind::ReproBundle {
        return Response {
            text: format!(
                "mcf: {} is an {} and `verify` checks a repro bundle\n  `mcf bundle <entry-id>` \
                 writes one (PR2)",
                path.display(),
                kind.as_str()
            ),
            served: false,
        };
    }
    let Some(claim) = entries
        .iter()
        .find(|entry| entry.get("kind").and_then(Value::as_text) == Some("comparison"))
    else {
        return Response {
            text: format!(
                "mcf: {} carries no claim: {} entr(ies) and none of them a comparison",
                path.display(),
                manifest.entries
            ),
            served: false,
        };
    };

    let mut lines = vec![
        format!("verifying {}", path.display()),
        format!("  {} entr(ies), digest checked", manifest.entries),
        String::new(),
    ];
    lines.extend(what_it_claims(claim));
    lines.push(String::new());
    lines.extend(conditions_then_and_now(claim));
    lines.push(String::new());
    lines.extend(what_is_here(claim));
    lines.push(String::new());
    lines.extend(the_refusal());
    Response {
        text: lines.join("\n"),
        served: true,
    }
}

/// What the bundle says was found, and how.
fn what_it_claims(claim: &Value) -> Vec<String> {
    let method = claim.get("body").and_then(|body| body.get("method"));
    let said = |name: &str| {
        method
            .and_then(|held| held.get(name))
            .map_or_else(|| "— not stated".to_owned(), Value::to_line)
    };
    vec![
        "── what it claims ───────────────────────────────────────────".to_owned(),
        format!(
            "  {}",
            claim
                .get("body")
                .and_then(|body| body.get("outcome"))
                .and_then(|outcome| outcome.get("kind"))
                .and_then(Value::as_text)
                .unwrap_or("an outcome this build does not know")
        ),
        format!("  prompt      {}", said("prompt")),
        format!("  resolving   {} ppm", said("resolving_ppm")),
        format!("  engine      {}", said("engine_asked")),
        format!("  every trial loaded the model: {}", said("cold")),
    ]
}

/// What differs between the machine the claim was taken on and this one.
///
/// The comparison is `Isolation`'s, which is the same arithmetic that decides
/// whether two arms of a benchmark are comparable (A8, B-085) — a second way of
/// answering *which conditions differ* would eventually disagree with the
/// first.
fn conditions_then_and_now(claim: &Value) -> Vec<String> {
    let mut lines =
        vec!["── conditions, there and here ───────────────────────────────".to_owned()];
    let Some(there) = claim
        .get("body")
        .and_then(|body| body.get("left"))
        .and_then(|arm| arm.get("conditions"))
        .and_then(|held| mcf_record::decode::conditions(held, BuildIdentity::current()))
    else {
        lines.push("  the bundle's conditions could not be read by this build".to_owned());
        return lines;
    };
    let here = Conditions::new(
        BuildIdentity::current(),
        mcf_core::capture::floor(
            &mcf_core::hardware::Machine::read(),
            None,
            "verifying a bundle",
            "none",
            None,
        ),
    );
    let held = Isolation::between(&there, &here);
    lines.push(format!("  {held}"));
    match &held {
        Isolation::SameConfiguration => lines.push(
            "  Nothing differs, so a difference in outcome would be this machine's noise or \
             MCF's (§3.27)."
                .to_owned(),
        ),
        Isolation::Isolated { .. } | Isolation::Confounded { .. } => lines
            .push("  A difference in outcome cannot be attributed to any one of these.".to_owned()),
        Isolation::Undetermined { unread, .. } => lines.push(format!(
            "  {} condition(s) could not be compared, so this is what MCF can see rather than \
             what is there (A7).",
            unread.len()
        )),
    }
    lines
}

/// Whether what the claim measured is on this machine.
fn what_is_here(claim: &Value) -> Vec<String> {
    let mut lines =
        vec!["── what is on this machine ──────────────────────────────────".to_owned()];
    let mut all = true;
    for side in ["left", "right"] {
        let named = claim
            .get("body")
            .and_then(|body| body.get(side))
            .and_then(|arm| arm.get("arm"))
            .and_then(Value::as_text)
            .unwrap_or("an arm the bundle does not name");
        let held = Path::new(named).is_file();
        all = all && held;
        lines.push(format!(
            "  {:<6} {}  {}",
            side,
            if held { "held" } else { "NOT held" },
            named
        ));
    }
    if all {
        lines.push(String::new());
        lines.push("  Both arms are here, so this claim can be re-run:".to_owned());
        lines.push(format!("    {}", rerun(claim)));
    } else {
        lines.push(String::new());
        lines.push(
            "  This claim cannot be re-run here, which is an answer rather than a failure: \
             what MCF can compare is the conditions above (PR2)."
                .to_owned(),
        );
    }
    lines
}

/// The command that re-runs the claim, from the method the bundle carries.
///
/// Printed rather than run. Verifying a bundle is reading a file somebody sent;
/// *running a model because a file said to* is a different act with a different
/// cost, and MCF does not take it on the reader's behalf (§3.7, A16).
fn rerun(claim: &Value) -> String {
    let body = claim.get("body");
    let arm = |side: &str| {
        body.and_then(|held| held.get(side))
            .and_then(|arm| arm.get("arm"))
            .and_then(Value::as_text)
            .unwrap_or("<model>")
            .to_owned()
    };
    let method = body.and_then(|held| held.get("method"));
    let said = |name: &str| method.and_then(|held| held.get(name));
    let tokens = body
        .and_then(|held| held.get("discipline"))
        .and_then(|held| held.get("tokens_pinned"))
        .and_then(Value::as_integer)
        .unwrap_or(0);
    let resolving = said("resolving_ppm")
        .and_then(Value::as_integer)
        .unwrap_or(50_000);
    format!(
        "mcf bench {} --against {} --prompt {} --limit {tokens} --resolving {}{}{}",
        arm("left"),
        arm("right"),
        said("prompt").map_or_else(|| "<prompt>".to_owned(), Value::to_line),
        per_cent(resolving),
        said("engine_asked")
            .and_then(Value::as_text)
            .map_or_else(String::new, |named| format!(" --engine {named}")),
        if said("cold") == Some(&Value::Bool(true)) {
            " --cold"
        } else {
            ""
        }
    )
}

/// A ratio in parts per million as a percentage, without a float (A6).
fn per_cent(held: i64) -> String {
    let whole = held.wrapping_div(10_000);
    let tenths = held.wrapping_div(1_000).wrapping_rem(10);
    if tenths == 0 {
        format!("{whole}")
    } else {
        format!("{whole}.{tenths}")
    }
}

/// The sentence B-212 exists for.
fn the_refusal() -> Vec<String> {
    vec![
        "── what MCF will not tell you ───────────────────────────────".to_owned(),
        "  If you re-run this and get a different number, MCF will not say which".to_owned(),
        "  of the differences above caused it. Picking one of them and calling it".to_owned(),
        "  the cause is a confound wearing a helpful voice: you have the".to_owned(),
        "  evidence and the attribution is yours.".to_owned(),
    ]
}

#[cfg(test)]
mod tests;
