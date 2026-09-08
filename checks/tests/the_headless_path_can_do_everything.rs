#![allow(clippy::panic, clippy::expect_used)]

use std::path::PathBuf;

fn read(relative: &str) -> String {
    let path = mcf_checks::workspace::root().join(relative);
    std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("{} is readable: {error}", path.display()))
}

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

fn operations() -> Vec<String> {
    let source = read("crates/mcf-serve/src/control.rs");
    let (_, after) = source
        .split_once("pub enum Request {")
        .expect("the control plane's requests are an enum");
    let body = after.split_once("\n}\n").map_or(after, |(held, _)| held);
    let mut found = Vec::new();
    for line in body.lines() {
        let trimmed = line.trim();
        let Some(name) = trimmed
            .strip_suffix(',')
            .or_else(|| trimmed.strip_suffix(" {"))
        else {
            continue;
        };
        if name.starts_with(char::is_uppercase)
            && name.chars().all(|held| held.is_ascii_alphanumeric())
            && !found.contains(&name.to_owned())
        {
            found.push(name.to_owned());
        }
    }
    found
}

#[test]
fn every_control_operation_has_a_command() {
    let operations = operations();
    assert!(
        operations.len() >= 4,
        "only {} control operation(s) were found, so this check is reading the protocol wrong: \
         {operations:#?}",
        operations.len()
    );

    let surface: String = sources("crates/mcf-cli/src")
        .iter()
        .filter_map(|path| std::fs::read_to_string(path).ok())
        .collect();

    let mut orphans = Vec::new();
    for operation in &operations {
        if !surface.contains(&format!("Request::{operation}")) {
            orphans.push(operation.clone());
        }
    }
    assert!(
        orphans.is_empty(),
        "the control plane answers {orphans:#?}, and no command asks. A capability reachable \
         only through a client is what A22 forbids, and one reachable through neither is a \
         capability the laboratory cannot test (A19, B-072)"
    );
}

#[test]
fn the_operations_a_person_can_ask_for_are_commands() {
    let main = read("crates/mcf-cli/src/main.rs");
    for (operation, command) in [("Status", "status"), ("Stop", "stop"), ("Generate", "run")] {
        assert!(
            main.contains(&format!("[\"{command}\"")),
            "`{operation}` is answered by the daemon and `mcf {command}` is not a command a \
             person can type (A22)"
        );
    }
    assert!(
        read("crates/mcf-cli/src/serve.rs").contains("Request::Holding"),
        "nothing headless asks the daemon what it is holding (A22, B-072)"
    );
}

struct Surface {
    krate: &'static str,
    table: Option<(&'static str, &'static str)>,
}

const SURFACES: &[Surface] = &[
    Surface {
        krate: "mcf-cli",
        table: None,
    },
    Surface {
        krate: "mcf-desk",
        table: Some((
            "crates/mcf-desk/src/lib.rs",
            "pub const ACTIONS: &[Action] = &[",
        )),
    },
    Surface {
        krate: "mcf-tui",
        table: Some((
            "crates/mcf-tui/src/lib.rs",
            "pub const ACTIONS: &[Action] = &[",
        )),
    },
];

fn surfaces_in_the_tree() -> Vec<String> {
    let crates = mcf_checks::workspace::root().join("crates");
    let mut found = Vec::new();
    for entry in std::fs::read_dir(&crates)
        .expect("the crates directory is in the tree")
        .flatten()
    {
        let Some(name) = entry.file_name().to_str().map(str::to_owned) else {
            continue;
        };
        if name == "mcf-serve" {
            continue;
        }
        let reaches = sources(&format!("crates/{name}/src"))
            .into_iter()
            .any(|path| {
                let source = std::fs::read_to_string(path).unwrap_or_default();
                source.contains("mcf_serve::control")
                    && operations()
                        .iter()
                        .any(|operation| source.contains(&format!("Request::{operation}")))
            });
        if reaches {
            found.push(name);
        }
    }
    found.sort();
    found
}

#[test]
fn every_surface_in_the_tree_is_declared() {
    let found = surfaces_in_the_tree();
    let undeclared: Vec<&String> = found
        .iter()
        .filter(|name| !SURFACES.iter().any(|known| known.krate == name.as_str()))
        .collect();
    assert!(
        undeclared.is_empty(),
        "a crate reaches the control plane and is not declared a surface: {undeclared:#?}. \
         A22 says the interface may not be the only way to do anything, so its actions have \
         to be enumerated and matched against the control plane here"
    );
    assert!(
        found.len() >= SURFACES.len(),
        "a surface is declared that no longer reaches the control plane: declared {:#?}, \
         found {found:#?}",
        SURFACES.iter().map(|s| s.krate).collect::<Vec<_>>()
    );
}

#[test]
fn every_surface_action_has_a_command() {
    let operations = operations();
    let headless = sources("crates/mcf-cli")
        .into_iter()
        .map(|path| std::fs::read_to_string(path).unwrap_or_default())
        .collect::<String>();

    for surface in SURFACES {
        let Some((file, marker)) = surface.table else {
            continue;
        };
        let source = read(file);
        let (_, after) = source
            .split_once(marker)
            .unwrap_or_else(|| panic!("{} enumerates its actions in {file}", surface.krate));
        let body = after.split_once("];").map_or(after, |(held, _)| held);

        let mut reached = Vec::new();
        for line in body.lines() {
            let Some((_, rest)) = line.split_once("reaches:") else {
                continue;
            };
            let rest = rest.trim();
            if rest.starts_with("None") {
                continue;
            }
            let name = rest
                .trim_start_matches("Some(")
                .trim_start_matches('"')
                .split('"')
                .next()
                .unwrap_or_default()
                .to_owned();
            assert!(
                !name.is_empty(),
                "{}: an action reaches something unreadable: {line}",
                surface.krate
            );
            reached.push(name);
        }
        assert!(
            !reached.is_empty(),
            "{} declares a table that reaches nothing; A22's second half depends on it \
             being legible here",
            surface.krate
        );

        for action in &reached {
            assert!(
                operations.contains(action),
                "{} reaches `{action}`, which the control plane does not have. An action \
                 only a surface can take is the shape A22 forbids",
                surface.krate
            );
            assert!(
                headless.contains(&format!("Request::{action}")),
                "{} reaches `{action}` and no command does; A22 says the interface may not \
                 be the only way to do anything",
                surface.krate
            );
        }
    }
}

#[test]
fn every_surface_is_opened_by_a_command() {
    let cli = sources("crates/mcf-cli")
        .into_iter()
        .map(|path| std::fs::read_to_string(path).unwrap_or_default())
        .collect::<String>();
    for surface in SURFACES {
        if surface.table.is_none() {
            continue;
        }
        let module = surface.krate.replace('-', "_");
        assert!(
            cli.contains(&format!("{module}::run")),
            "no command starts {}",
            surface.krate
        );
    }
}
