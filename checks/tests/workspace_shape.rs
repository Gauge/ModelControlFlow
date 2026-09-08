use mcf_checks::workspace::{MEMBERS, Member, declared_mcf_dependencies, read, read_member, root};

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

#[test]
fn the_lockfile_is_committed() {
    assert!(
        root().join("Cargo.lock").is_file(),
        "Cargo.lock is not in the repository",
    );
}

#[test]
fn every_declared_member_directory_exists() {
    for Member { name, path, .. } in MEMBERS {
        assert!(
            root().join(path).join("Cargo.toml").is_file(),
            "{name} declares {path}, which holds no manifest",
        );
    }
}

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

#[test]
fn the_workspace_takes_exactly_what_the_register_admits() {
    let workspace = read("Cargo.toml").expect("the workspace manifest is readable");
    assert!(
        workspace.keys("workspace.dependencies").is_empty(),
        "a shared dependency was admitted without this check being updated",
    );

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
    for lint in ["unsafe_code", "unused_must_use"] {
        assert_eq!(
            manifest.get("workspace.lints.rust", lint),
            Some("\"deny\""),
            "{lint} is not denied",
        );
    }
}

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
