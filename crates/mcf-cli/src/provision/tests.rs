//! What the recipe table has to keep true without a container in sight.

use super::{COMPONENTS, Component, Packaging, prefix_for, script_for};

/// Every component pins everything a rerun needs: an image digest, a full
/// commit, at least one package and one target.
#[test]
fn every_component_is_fully_pinned() {
    assert!(!COMPONENTS.is_empty());
    for component in COMPONENTS {
        assert!(
            component.image_digest.starts_with("sha256:") && component.image_digest.len() == 71,
            "{}: an image digest pins or it does not: {}",
            component.name,
            component.image_digest
        );
        assert!(
            component.commit.len() == 40 && component.commit.chars().all(|c| c.is_ascii_hexdigit()),
            "{}: a commit pin is the whole hash: {}",
            component.name,
            component.commit
        );
        assert!(!component.packages.is_empty(), "{}", component.name);
        assert!(!component.targets.is_empty(), "{}", component.name);
        assert!(
            component.source.starts_with("https://"),
            "{}: source over an unencrypted wire: {}",
            component.name,
            component.source
        );
    }
}

/// The script quotes nothing from outside the table, checks out the pinned
/// commit, and writes the three files the recording step reads back.
#[test]
fn the_script_is_the_recipe_and_only_the_recipe() {
    for component in COMPONENTS {
        let script = script_for(component);
        assert!(script.contains(component.commit), "{}", component.name);
        assert!(script.contains(component.source), "{}", component.name);
        for needed in ["toolchain.txt", "commit.txt", "errexit"] {
            assert!(script.contains(needed), "{}: no {needed}", component.name);
        }
        // Portability is part of the recipe: a tuned build is a condition
        // nobody can restate elsewhere.
        //
        // Checked by the property rather than by one project's flag. This used
        // to assert `-DGGML_NATIVE=OFF` on every component, which was llama.cpp
        // spelled as a rule — and the second component to arrive failed it for
        // not being llama.cpp rather than for tuning to anything.
        for flag in component.configure {
            let tuned = flag.contains("-march=native")
                || flag.contains("-mtune=native")
                || flag.to_ascii_uppercase().ends_with("NATIVE=ON");
            assert!(!tuned, "{}: {flag} tunes to this machine", component.name);
        }
        // And where a component HAS such a switch, it is turned off rather
        // than left to whatever the project defaults to.
        let has_native_switch = component
            .configure
            .iter()
            .any(|flag| flag.to_ascii_uppercase().contains("NATIVE"));
        if has_native_switch {
            assert!(
                component
                    .configure
                    .iter()
                    .any(|flag| flag.to_ascii_uppercase().ends_with("NATIVE=OFF")),
                "{}: a native switch that is not off",
                component.name
            );
        }
        // And self-containment: a shared build carries the container's own
        // library path, which exists nowhere on the host (F31).
        //
        // By the property again, not by one project's spelling: CMake calls it
        // BUILD_SHARED_LIBS, SDL calls it SDL_SHARED and SDL_STATIC, and a
        // check that knows only the first refuses the second for the wrong
        // reason.
        let asks_for_shared = component
            .configure
            .iter()
            .any(|flag| flag.to_ascii_uppercase().ends_with("SHARED=ON"));
        assert!(
            !asks_for_shared,
            "{}: a shared build carries the container's library path, which exists nowhere \
             on the host (F31)",
            component.name
        );
        let self_contained = component.configure.iter().any(|flag| {
            let flag = flag.to_ascii_uppercase();
            flag.ends_with("SHARED_LIBS=OFF") || flag.ends_with("SHARED=OFF")
        });
        assert!(
            self_contained,
            "{}: the artifact must run where it lands, without the container",
            component.name
        );
    }
}

/// Two pins of one component are two prefixes.
#[test]
fn a_prefix_names_the_component_and_the_commit() {
    let component = &COMPONENTS[0];
    let prefix = prefix_for(component, std::path::Path::new("/somewhere"));
    let name = prefix.file_name().and_then(|n| n.to_str()).unwrap_or("");
    assert!(name.starts_with(component.name), "{name}");
    assert!(name.contains('@'), "{name}");
    assert!(name.ends_with(&component.commit[..12]), "{name}");
}

/// A second component could not silently collide with the first.
#[test]
fn component_names_are_unique() {
    let mut names: Vec<&str> = COMPONENTS.iter().map(|c| c.name).collect();
    names.sort_unstable();
    names.dedup();
    assert_eq!(names.len(), COMPONENTS.len());
}

/// The struct stays constructible in tests without a container: this is the
/// compile-time shape the table depends on.
#[test]
fn a_component_is_data() {
    let component = Component {
        name: "example",
        role: "a test",
        image: "example:1",
        image_digest: "sha256:0000000000000000000000000000000000000000000000000000000000000000",
        source: "https://example.invalid/repo.git",
        commit: "0000000000000000000000000000000000000000",
        packages: &["nothing"],
        packaging: Packaging::Dnf,
        configure: &[],
        targets: &["something"],
    };
    assert_eq!(component.name, "example");
}

/// A configure flag holding a shell metacharacter is an argument, not a
/// command.
///
/// `CMAKE_CUDA_ARCHITECTURES` takes a semicolon-separated list. Unquoted, bash
/// ended the cmake line at the semicolon and ran `120` as the next command —
/// exit 127, after a configure that had reported success while silently
/// dropping the flag (F128). The failure was invisible in the configure log,
/// which is why it is pinned here rather than left to be noticed again.
#[test]
fn a_configure_flag_cannot_become_a_command() {
    let component = Component {
        name: "example",
        role: "a test",
        image: "example:1",
        image_digest: "sha256:0000000000000000000000000000000000000000000000000000000000000000",
        source: "https://example.invalid/repo.git",
        commit: "0000000000000000000000000000000000000000",
        packages: &["nothing"],
        packaging: Packaging::Apt,
        configure: &["-DCMAKE_CUDA_ARCHITECTURES=89;120", "-DWITH SPACE=a b"],
        targets: &["something"],
    };
    let script = script_for(&component);
    let configure = script
        .lines()
        .find(|line| line.starts_with("cmake -S"))
        .expect("the script configures");

    assert!(
        configure.contains("'-DCMAKE_CUDA_ARCHITECTURES=89;120'"),
        "the semicolon must sit inside quotes, not end the command: {configure}"
    );
    assert!(
        configure.contains("'-DWITH SPACE=a b'"),
        "a space must not split one argument into two: {configure}"
    );
    // Nothing outside the redirect may follow the quoted arguments: an
    // unquoted `;` would leave a second command on this line.
    let after = configure
        .split_once("> /work/configure.log")
        .expect("the configure output is redirected")
        .0;
    assert!(
        !after
            .replace("'-DCMAKE_CUDA_ARCHITECTURES=89;120'", "")
            .contains(';'),
        "no bare separator may survive quoting: {configure}"
    );
}

/// Each image installs the way its own distribution does.
///
/// The recipe said `dnf` and `rpm` outright, which was true of the one image
/// there was and false the moment a CUDA toolkit image arrived — it failed on
/// `dnf: command not found` before compiling anything (F128).
#[test]
fn packaging_follows_the_image() {
    for component in COMPONENTS {
        let script = script_for(component);
        match component.packaging {
            Packaging::Dnf => {
                assert!(script.contains("dnf -q install"), "{}", component.name);
                assert!(script.contains("rpm -q"), "{}", component.name);
            }
            Packaging::Apt => {
                assert!(script.contains("apt-get -qq install"), "{}", component.name);
                assert!(script.contains("dpkg-query -W"), "{}", component.name);
                assert!(
                    script.contains("apt-get -qq update"),
                    "a Debian image ships no package lists: {}",
                    component.name
                );
            }
        }
    }
}
