//! A hosted model listens where this machine can reach it and nowhere else
//! (B-036, §6.12, §3.10, A16).
//!
//! **The claim this holds used to be simpler.** B-036 said MCF was local *by
//! construction rather than by configuration*: there was no bind address, no
//! port and no flag, so exposure was not something a mistake could do because
//! it was not something MCF could do. B-416 gave it a port — hosting a model
//! is being reachable, and there is no way to be reachable without listening.
//!
//! So the claim narrowed rather than went away, and this is the narrower one:
//! **the port is the operator's and the address is not MCF's to offer.** A
//! person can move a hosted model to another port; nobody can move it to
//! another interface, because there is no setting for one and the constant it
//! binds is the loopback address.
//!
//! Three things are checked, and each is one way the claim could stop being
//! true: that the constant is what it says it is, that nothing in the settings
//! can carry a different one, and that the control plane itself is still a
//! Unix socket with no address at all.

// Every item in this file is test code; see the note in `taxonomy_agreement.rs`.
#![allow(clippy::panic, clippy::expect_used, clippy::unwrap_used)]

use std::path::Path;

fn read(relative: &str) -> String {
    let path = mcf_checks::workspace::root().join(relative);
    std::fs::read_to_string(&path).unwrap_or_else(|error| {
        panic!("{} is readable: {error}", path.display());
    })
}

/// The address a hosted model binds is the loopback address.
///
/// A name is not evidence (A21): what the constant *is* decides this, not what
/// it is called.
#[test]
fn the_address_is_the_loopback_address() {
    let source = read("crates/mcf-serve/src/hosting.rs");
    let declared: Vec<&str> = source
        .lines()
        .filter(|line| line.trim_start().starts_with("pub const LOOPBACK"))
        .collect();
    assert_eq!(
        declared.len(),
        1,
        "there is not exactly one address constant: {declared:#?}"
    );
    assert!(
        declared
            .first()
            .is_some_and(|line| line.contains("\"127.0.0.1\"")),
        "the address a hosted model binds is not the loopback address: {declared:#?}"
    );
}

/// Nothing in the hosting settings can carry an address.
///
/// **This is the whole of *by construction*.** A port is a number somebody may
/// choose and an interface is not, so there is no field for one — and a field
/// that appeared would make exposure something a mistake could do, which is
/// what §6.12 forbids.
#[test]
fn no_setting_carries_an_address() {
    let source = read("crates/mcf-serve/src/hosting.rs");
    let (_, body) = source
        .split_once("pub struct Hosting {")
        .expect("the hosting settings are a struct");
    let body = body.split_once("\n}\n").map_or(body, |(held, _)| held);

    let mut carrying = Vec::new();
    for line in body.lines() {
        let trimmed = line.trim();
        // A field, not documentation: `name: Type,`.
        if trimmed.starts_with("//") || !trimmed.contains(':') {
            continue;
        }
        let Some((name, _)) = trimmed.split_once(':') else {
            continue;
        };
        let name = name.trim().trim_start_matches("pub ").to_ascii_lowercase();
        for shape in ["host", "address", "bind", "interface", "listen"] {
            if name.contains(shape) {
                carrying.push(trimmed.to_owned());
            }
        }
    }
    assert!(
        carrying.is_empty(),
        "a hosting setting names an address or an interface, so where a model is reachable \
         from has become something a client can ask for. §6.12 asks that network exposure be \
         an explicit, informed, revocable act — a field is none of those (B-036): {carrying:#?}"
    );
}

/// The control plane is still a socket with no address at all.
///
/// The port belongs to a *hosted model*. MCF's own control plane never grew
/// one, and a person reaching MCF from another machine is still not something
/// MCF can do.
#[test]
fn the_control_plane_has_no_port() {
    let source = read("crates/mcf-serve/src/daemon.rs");
    assert!(
        source.contains("UnixListener"),
        "the control plane is no longer a Unix socket"
    );
    let mut binding = Vec::new();
    for line in source.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("//") {
            continue;
        }
        if trimmed.contains("TcpListener") && trimmed.contains("bind") {
            binding.push(trimmed.to_owned());
        }
    }
    assert!(
        binding.is_empty(),
        "the control plane binds a port, so MCF is reachable from another machine by \
         configuration rather than unreachable by construction (B-036, §6.12): {binding:#?}"
    );
}

/// Hosting a model is written down, and letting it go is too.
///
/// §6.12 asks that exposure be an explicit act. An act nobody wrote down is
/// indistinguishable from a side effect, and after a stop the entry is the
/// only thing that says anything was ever listening (A26).
#[test]
fn hosting_and_letting_go_are_both_recorded() {
    let source = read("crates/mcf-serve/src/daemon.rs");
    for kind in ["EntryKind::ModelHosted", "EntryKind::ModelUnhosted"] {
        assert!(
            source.contains(kind),
            "{kind} is never written, so {} leaves no account",
            if kind.contains("Unhosted") {
                "letting a model go"
            } else {
                "holding a model where something can reach it"
            }
        );
    }
    // And the key is never in it. A record is something MCF publishes, and a
    // secret in a published record is one nobody meant to publish (A25).
    let hosting = read("crates/mcf-serve/src/hosting.rs");
    let (_, written) = hosting
        .split_once("pub fn to_value")
        .expect("the settings say how they are recorded");
    let written = written
        .split_once("\n    }\n")
        .map_or(written, |(held, _)| held);
    assert!(
        !written.contains("self.api_key.clone()"),
        "the key itself reaches the record rather than the fact that one is set (A25)"
    );
    assert!(
        written.contains("api_key_set"),
        "the record does not say whether a key was set, which is a condition of the hosting"
    );
}

/// The workspace root is where this check thinks it is.
#[test]
fn the_files_this_watches_are_there() {
    for named in [
        "crates/mcf-serve/src/hosting.rs",
        "crates/mcf-serve/src/daemon.rs",
    ] {
        let path: &Path = &mcf_checks::workspace::root().join(named);
        assert!(path.is_file(), "{named} is not where this check looks");
    }
}
