use std::path::PathBuf;

use mcf_serve::adapters::{Found, ProvisionedLlama, only_one};

fn build(component: &str, commit: &str) -> ProvisionedLlama {
    ProvisionedLlama {
        prefix: PathBuf::from(format!("/nowhere/{component}@{commit}")),
        commit: commit.to_owned(),
        component: component.to_owned(),
    }
}

#[test]
fn several_provisioned_builds_are_told_apart_by_name() {
    let held = Found::Several(vec![
        build("llama.cpp-vulkan", "aaa"),
        build("llama.cpp-rocm", "bbb"),
    ]);
    assert_eq!(
        held.components(),
        vec!["llama.cpp-vulkan".to_owned(), "llama.cpp-rocm".to_owned()]
    );
    let Some(picked) = held.clone().named("llama.cpp-rocm") else {
        panic!("a build that is here can be named");
    };
    assert_eq!(picked.commit, "bbb");
    assert!(
        held.named("llama.cpp-cuda").is_none(),
        "a build that is not here is absent rather than substituted"
    );
}

#[test]
fn naming_none_of_several_refuses_and_lists_what_is_here() {
    let held = Found::Several(vec![
        build("llama.cpp-vulkan", "aaa"),
        build("llama.cpp-rocm", "bbb"),
    ]);
    let Err(refused) = only_one(held) else {
        panic!("two builds and no name is a refusal");
    };
    let said = format!("{refused:?}");
    assert!(said.contains("llama.cpp-vulkan"), "{said}");
    assert!(said.contains("llama.cpp-rocm"), "{said}");
    assert!(
        said.contains("--engine"),
        "the refusal says how to proceed: {said}"
    );
}

#[test]
fn one_provisioned_build_still_needs_no_name() {
    let Ok(Some(picked)) = only_one(Found::One(build("llama.cpp-vulkan", "ccc"))) else {
        panic!("one build is chosen without being named");
    };
    assert_eq!(picked.component, "llama.cpp-vulkan");
}

#[test]
fn nothing_provisioned_is_absent_rather_than_a_failure() {
    assert!(
        matches!(only_one(Found::None), Ok(None)),
        "no build is an absence the caller decides about"
    );
}

#[test]
fn every_back_end_in_the_table_can_be_named() {
    let built: Vec<&str> = mcf_core::component::COMPONENTS
        .iter()
        .map(|held| held.name)
        .filter(|name| name.starts_with("llama.cpp"))
        .collect();
    assert!(
        built.contains(&"llama.cpp-vulkan") && built.contains(&"llama.cpp-rocm"),
        "an AMD card has two back ends and both are provisionable: {built:?}"
    );
    let here = Found::Several(built.iter().map(|name| build(name, "aaa")).collect());
    for name in built {
        assert!(
            here.clone().named(name).is_some(),
            "{name} is in the table and can be named"
        );
    }
}
