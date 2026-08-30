//! The companion site is a different program, and stays one (B-406, §6.12).
//!
//! **MCF cannot send and cannot listen.** `mcf share` writes a file and there
//! is no destination in the tree; `the_five_gates` holds the absence of network
//! exposure against `crates/`. The site does nothing but listen and receive,
//! which is the opposite of both — so it lives outside that workspace, and the
//! separation is what lets both statements be true at once.
//!
//! **This is the check that keeps them apart.** Adding `site` to MCF's
//! workspace members would import a TCP listener into the tree the gate
//! protects, and it would do it quietly: the site would simply start being
//! scanned, and the gate would start failing for a reason nobody expected. The
//! failure here says what actually happened instead.

use std::path::PathBuf;

fn root() -> PathBuf {
    mcf_checks::workspace::root()
}

fn read(relative: &str) -> String {
    std::fs::read_to_string(root().join(relative)).unwrap_or_default()
}

/// The site is not a member of MCF's workspace.
#[test]
fn the_site_is_its_own_workspace() {
    let manifest = read("Cargo.toml");
    let (_, members) = manifest
        .split_once("members = [")
        .expect("the workspace names its members");
    let members = members.split_once(']').map_or(members, |(held, _)| held);
    assert!(
        !members.contains("site"),
        "the site has been made a member of MCF's workspace, which puts a TCP listener \
         inside the tree §6.12's gate protects:\n{members}"
    );
    assert!(
        root().join("site/Cargo.toml").is_file(),
        "the site is gone, and this check is now guarding nothing"
    );
    assert!(
        read("site/Cargo.toml").contains("[workspace]"),
        "the site has stopped being its own workspace, so its dependencies now reach MCF's \
         lockfile — which P3 makes a condition of every measurement"
    );
}

/// Nothing MCF ships depends on the site.
///
/// The edge runs one way: the site reads the contribution format from
/// `mcf-core`, and nothing in MCF knows the site exists. An edge the other way
/// would make a measuring instrument depend on a web server.
#[test]
fn nothing_mcf_ships_depends_on_the_site() {
    let crates = root().join("crates");
    let entries = std::fs::read_dir(&crates).expect("the crates directory is in the tree");
    let mut offenders = Vec::new();
    for entry in entries.flatten() {
        let manifest = entry.path().join("Cargo.toml");
        let Ok(text) = std::fs::read_to_string(&manifest) else {
            continue;
        };
        if text.contains("mcf-site") || text.contains("../site") {
            offenders.push(entry.file_name().to_string_lossy().into_owned());
        }
    }
    assert!(
        offenders.is_empty(),
        "MCF depends on the companion site: {offenders:#?}"
    );
}

/// And the site depends on nothing of MCF's but the format.
///
/// `mcf-core` has no dependencies of its own, so this edge carries nothing with
/// it. An edge to the daemon or the hub would give a public server a way into
/// somebody's machine, which is exactly what the arrangement exists to prevent.
#[test]
fn the_site_reaches_only_the_format() {
    let manifest = read("site/Cargo.toml");
    let (_, dependencies) = manifest
        .split_once("[dependencies]")
        .expect("the site names its dependencies");
    for reaching in [
        "mcf-serve",
        "mcf-hub",
        "mcf-cli",
        "mcf-standin",
        "mcf-helper",
        "mcf-lab",
        "mcf-record",
    ] {
        assert!(
            !dependencies.contains(reaching),
            "the site depends on {reaching}; the only edge it may have is the contribution \
             format in mcf-core"
        );
    }
    assert!(
        dependencies.contains("mcf-core"),
        "the site no longer shares the format with MCF, so the two can drift"
    );
}
