use super::{prefix_for, script_for};
use mcf_core::component::{COMPONENTS, Component, Packaging};

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

#[test]
fn the_script_is_the_recipe_and_only_the_recipe() {
    for component in COMPONENTS {
        let script = script_for(component);
        assert!(script.contains(component.commit), "{}", component.name);
        assert!(script.contains(component.source), "{}", component.name);
        for needed in ["toolchain.txt", "commit.txt", "errexit"] {
            assert!(script.contains(needed), "{}: no {needed}", component.name);
        }
        for flag in component.configure {
            let tuned = flag.contains("-march=native")
                || flag.contains("-mtune=native")
                || flag.to_ascii_uppercase().ends_with("NATIVE=ON");
            assert!(!tuned, "{}: {flag} tunes to this machine", component.name);
        }
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

#[test]
fn a_prefix_names_the_component_and_the_commit() {
    let component = &COMPONENTS[0];
    let prefix = prefix_for(component, std::path::Path::new("/somewhere"));
    let name = prefix.file_name().and_then(|n| n.to_str()).unwrap_or("");
    assert!(name.starts_with(component.name), "{name}");
    assert!(name.contains('@'), "{name}");
    assert!(name.ends_with(&component.commit[..12]), "{name}");
}

#[test]
fn component_names_are_unique() {
    let mut names: Vec<&str> = COMPONENTS.iter().map(|c| c.name).collect();
    names.sort_unstable();
    names.dedup();
    assert_eq!(names.len(), COMPONENTS.len());
}

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
    let after = configure
        .split_once("2>&1 | tee /work/configure.log")
        .expect("the configure output is copied to the log")
        .0;
    assert!(
        !after
            .replace("'-DCMAKE_CUDA_ARCHITECTURES=89;120'", "")
            .contains(';'),
        "no bare separator may survive quoting: {configure}"
    );
}

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

#[test]
fn the_script_announces_its_stages_and_keeps_its_logs() {
    for component in COMPONENTS {
        let script = script_for(component);
        for stage in [
            "echo 'installing the toolchain'",
            "echo 'fetching the source at",
            "echo 'configuring'",
            "echo 'building'",
        ] {
            assert!(script.contains(stage), "{}: no {stage}", component.name);
        }
        assert!(
            script.contains("| tee /work/build.log"),
            "{}: the build log must be copied, not diverted",
            component.name
        );
        assert!(
            script.contains("set -o errexit -o nounset -o pipefail"),
            "{}: a failed cmake behind a tee must still fail the script",
            component.name
        );
    }
}
