#![allow(clippy::panic, clippy::expect_used, clippy::unwrap_used)]

use std::path::Path;

fn read(relative: &str) -> String {
    let path = mcf_checks::workspace::root().join(relative);
    std::fs::read_to_string(&path).unwrap_or_else(|error| {
        panic!("{} is readable: {error}", path.display());
    })
}

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
