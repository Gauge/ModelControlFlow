#![allow(clippy::panic)]

use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-env-changed=MCF_BUILD_COMMIT");
    println!("cargo:rerun-if-env-changed=RUSTC");
    println!("cargo:rerun-if-env-changed=TARGET");
    println!("cargo:rerun-if-env-changed=PROFILE");

    let rustc = std::env::var("RUSTC").unwrap_or_else(|_| "rustc".to_owned());
    let output = Command::new(&rustc)
        .arg("--version")
        .output()
        .unwrap_or_else(|error| panic!("{rustc} --version could not be run: {error}"));
    assert!(
        output.status.success(),
        "{rustc} --version exited {}",
        output.status
    );
    let version = String::from_utf8(output.stdout)
        .unwrap_or_else(|error| panic!("{rustc} --version emitted non-UTF-8: {error}"));
    let version = version.trim();
    assert!(
        version.starts_with("rustc "),
        "{rustc} --version emitted {version:?}, which is not a version line"
    );

    emit("MCF_BUILD_RUSTC", version);
    emit("MCF_BUILD_TARGET", &required("TARGET"));
    emit("MCF_BUILD_PROFILE", &required("PROFILE"));
}

fn required(name: &str) -> String {
    std::env::var(name)
        .unwrap_or_else(|error| panic!("cargo did not set {name} for the build script: {error}"))
}

fn emit(name: &str, value: &str) {
    assert!(
        !value.contains('\n'),
        "{name} would embed a newline: {value:?}"
    );
    println!("cargo:rustc-env={name}={value}");
}
