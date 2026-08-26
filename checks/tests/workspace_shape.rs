//! The workspace holds the shape B-001 declares.
//!
//! B16: prefer the machine-checked form of every rule. The crate split and the
//! direction of its dependency edges are a structural application of §3.16 —
//! an edge that does not exist cannot be taken by accident — and a structure
//! nothing checks is a structure that erodes one convenient import at a time.

use mcf_checks::workspace::{MEMBERS, Member, declared_mcf_dependencies, read, read_member, root};

/// Every declared member is a member of the workspace, and no member is
/// declared that the workspace does not have. B-001 named seven crates plus the
/// checks, and the register has admitted two more since — `mcf-standin` (D31)
/// and `mcf-helper` (D35); a manifest that grows another silently has changed
/// the architecture without saying so.
#[test]
fn the_workspace_members_are_exactly_the_declared_ones() {
    let manifest = read("Cargo.toml").expect("the workspace manifest is readable");
    let mut declared: Vec<String> = MEMBERS.iter().map(|m| m.path.to_owned()).collect();
    declared.sort();
    let mut actual = manifest
        .list("workspace", "members")
        .expect("the workspace manifest declares members");
    actual.sort();
    assert_eq!(actual, declared);
}

/// The dependency edges are exactly the declared ones. Both directions of the
/// comparison matter: an added edge inverts the layering, and a missing one
/// means the declaration in `workspace.rs` has drifted from the code and can
/// no longer be read as the architecture.
#[test]
fn every_crate_depends_on_exactly_what_it_declares() {
    for member in MEMBERS {
        let manifest = read_member(member).expect("a member manifest is readable");
        let actual = declared_mcf_dependencies(&manifest);
        let mut expected: Vec<String> = member.depends_on.iter().map(|d| (*d).to_owned()).collect();
        expected.sort();
        assert_eq!(actual, expected, "{} has the wrong edges", member.name);
    }
}

/// The layering is acyclic, and the declaration is written in topological
/// order. This is the property the split exists for: `mcf-core` cannot acquire
/// a dependency on the record store, so no convenience in the record store can
/// weaken a type in `mcf-core`.
#[test]
fn the_layering_is_acyclic() {
    let mut seen: Vec<&str> = Vec::new();
    for member in MEMBERS {
        for dependency in member.depends_on {
            assert!(
                seen.contains(dependency),
                "{} depends on {dependency}, which is not above it in the layering",
                member.name,
            );
        }
        seen.push(member.name);
    }
}

/// `mcf-core` is the bottom. Stated separately from the layering check because
/// it is the load-bearing half: §3.16's types live there, and an edge out of
/// it is how they would acquire a reason to be constructible without their
/// conditions.
#[test]
fn mcf_core_depends_on_nothing() {
    let core = MEMBERS
        .iter()
        .find(|m| m.name == "mcf-core")
        .expect("mcf-core is a member");
    assert_eq!(core.depends_on, &[] as &[&str]);
    let manifest = read_member(core).expect("mcf-core's manifest is readable");
    assert_eq!(declared_mcf_dependencies(&manifest), Vec::<String>::new());
}

/// Nothing depends on the checks crate, so the gating suite's own machinery
/// cannot reach a shipped binary (B15 — weight is admitted only against a
/// stated cost, and this crate states that it ships nothing).
#[test]
fn nothing_depends_on_the_checks_crate() {
    for member in MEMBERS {
        assert!(
            !member.depends_on.contains(&"mcf-checks"),
            "{} depends on the checks crate",
            member.name,
        );
    }
    let manifest = read("checks/Cargo.toml").expect("the checks manifest is readable");
    assert_eq!(manifest.get("package", "publish"), Some("false"));
}

/// Every crate inherits the workspace lints. A crate that does not is a crate
/// where `unsafe_code`, `missing_docs` and the panicking constructs are merely
/// discouraged — which is the review-only enforcement B16 exists to eliminate.
#[test]
fn every_crate_inherits_the_workspace_lints() {
    for member in MEMBERS {
        let manifest = read_member(member).expect("a member manifest is readable");
        assert_eq!(
            manifest.get("lints", "workspace"),
            Some("true"),
            "{} does not inherit the workspace lints",
            member.name,
        );
    }
}

/// Every crate inherits the workspace's version, edition, toolchain floor and
/// licence rather than restating them. A restated value is a value that will
/// eventually disagree, and the licence in particular is a claim MCF makes to
/// a redistributor (D22, B-330).
#[test]
fn every_crate_inherits_the_shared_package_fields() {
    for member in MEMBERS {
        let manifest = read_member(member).expect("a member manifest is readable");
        for field in ["version", "edition", "rust-version", "license"] {
            assert_eq!(
                manifest.get("package", &format!("{field}.workspace")),
                Some("true"),
                "{} restates {field} instead of inheriting it",
                member.name,
            );
        }
    }
}

/// The pinned toolchain and the declared minimum are the same version.
///
/// §3.12: the compiler is a condition of every artifact. A `rust-version`
/// below the pin would mean the workspace claims to build on a toolchain
/// nothing has ever built it with, which is an untested claim (A19).
#[test]
fn the_toolchain_pin_and_the_declared_minimum_agree() {
    let workspace = read("Cargo.toml").expect("the workspace manifest is readable");
    let toolchain = read("rust-toolchain.toml").expect("the toolchain file is readable");
    let pinned = toolchain
        .get("toolchain", "channel")
        .expect("the toolchain file pins a channel");
    let minimum = workspace
        .get("workspace.package", "rust-version")
        .expect("the workspace declares a minimum rust version");
    assert_eq!(pinned, minimum);
}

/// The pin is an exact version, not a channel. `stable` builds a different
/// instrument every six weeks (§3.12, P3).
#[test]
fn the_toolchain_pin_is_an_exact_version() {
    let toolchain = read("rust-toolchain.toml").expect("the toolchain file is readable");
    let pinned = toolchain
        .get("toolchain", "channel")
        .expect("the toolchain file pins a channel")
        .trim_matches('"')
        .to_owned();
    assert_eq!(
        pinned.split('.').count(),
        3,
        "{pinned:?} is a channel, not a version",
    );
    assert!(
        pinned.split('.').all(|part| part.parse::<u32>().is_ok()),
        "{pinned:?} is not three numbers",
    );
}

/// The lockfile is committed. `cargo build --locked` — B-001's own condition —
/// cannot hold without one, and an uncommitted lockfile means every checkout
/// resolves its own set of conditions.
#[test]
fn the_lockfile_is_committed() {
    assert!(
        root().join("Cargo.lock").is_file(),
        "Cargo.lock is not in the repository",
    );
}

/// Every member directory named in the declaration exists and holds a
/// manifest. Guards against a member declared and never created, which cargo
/// would refuse but the declaration in `workspace.rs` would not.
#[test]
fn every_declared_member_directory_exists() {
    for Member { name, path, .. } in MEMBERS {
        assert!(
            root().join(path).join("Cargo.toml").is_file(),
            "{name} declares {path}, which holds no manifest",
        );
    }
}

/// A member's package name matches the directory the declaration gives it.
/// A mismatch makes `cargo build -p <name>` and the layering check disagree
/// about which crate is which.
#[test]
fn package_names_match_the_declaration() {
    for member in MEMBERS {
        let manifest = read_member(member).expect("a member manifest is readable");
        assert_eq!(
            manifest.get("package", "name").map(|n| n.trim_matches('"')),
            Some(member.name),
            "the crate at {} is not named {}",
            member.path,
            member.name,
        );
    }
}

/// The workspace takes exactly the dependencies the register admits, in exactly
/// the crate that needs them.
///
/// B15 admits weight only against a stated cost, and the cost of the one
/// admission is in [findings.md](../../doc/findings.md) F9 with the register
/// row in `doc/vendored.md`. What this holds is that the list does not grow
/// quietly: a crate added to any manifest, in any table, fails here until
/// somebody puts it in both places.
#[test]
fn the_workspace_takes_exactly_what_the_register_admits() {
    let workspace = read("Cargo.toml").expect("the workspace manifest is readable");
    assert!(
        workspace.keys("workspace.dependencies").is_empty(),
        "a shared dependency was admitted without this check being updated",
    );

    // The TLS stack, and only in the crate that reaches a hub. Written out
    // rather than pattern-matched: B15 admits weight *for a stated reason*, and
    // a check that accepted anything with a row somewhere would be a check that
    // accepts the next thing too (B-322, doc/vendored.md §2).
    let admitted: &[(&str, &[&str])] =
        &[("mcf-hub", &["rustls", "rustls-graviola", "webpki-roots"])];
    let register = std::fs::read_to_string(root().join("doc/vendored.md"))
        .expect("doc/vendored.md is readable");

    for member in MEMBERS {
        let manifest = read_member(member).expect("a member manifest is readable");
        let allowed = admitted
            .iter()
            .find(|(name, _)| *name == member.name)
            .map_or(&[] as &[&str], |(_, crates)| *crates);
        for table in ["dependencies", "dev-dependencies", "build-dependencies"] {
            let foreign: Vec<&str> = manifest
                .keys(table)
                .into_iter()
                .filter(|name| !name.starts_with("mcf-"))
                .collect();
            for name in &foreign {
                assert!(
                    allowed.contains(name),
                    "{} takes {name} in {table}, which nothing has admitted: a dependency is \
                     admitted for a stated reason (B15) and recorded in doc/vendored.md (B-330)",
                    member.name,
                );
                assert!(
                    register.contains(name),
                    "{name} is a dependency and has no row in doc/vendored.md (B-330)"
                );
            }
        }
    }
}

/// The workspace denies the constructs A2 forbids.
///
/// `scripts/check-lints-bite.sh` proves each of these actually refuses code;
/// this test proves the table still *claims* them. The two are different
/// failures: a lint quietly deleted from the manifest would leave that script
/// checking a shorter list and still reporting every entry refused.
#[test]
fn the_workspace_denies_the_constructs_a2_forbids() {
    let manifest = read("Cargo.toml").expect("the workspace manifest is readable");
    for lint in [
        "unwrap_used",
        "expect_used",
        "panic",
        "todo",
        "unimplemented",
        "indexing_slicing",
        "let_underscore_must_use",
        "exit",
    ] {
        assert_eq!(
            manifest.get("workspace.lints.clippy", lint),
            Some("\"deny\""),
            "clippy::{lint} is not denied",
        );
    }
    for lint in ["unsafe_code", "missing_docs", "unused_must_use"] {
        assert_eq!(
            manifest.get("workspace.lints.rust", lint),
            Some("\"deny\""),
            "{lint} is not denied",
        );
    }
}

/// Tests are exempt from the panicking constructs, and the exemption is
/// declared rather than incidental. B-003 draws the line at non-test code: an
/// assertion that fails loudly is the honest outcome in a test, and denying it
/// there would push tests toward returning early instead — A2's silent failure,
/// aimed at the suite.
#[test]
fn the_test_exemption_is_declared() {
    let clippy = read("clippy.toml").expect("clippy.toml is readable");
    for setting in [
        "allow-unwrap-in-tests",
        "allow-expect-in-tests",
        "allow-panic-in-tests",
    ] {
        assert_eq!(
            clippy.get("", setting),
            Some("true"),
            "{setting} is not set"
        );
    }
}
