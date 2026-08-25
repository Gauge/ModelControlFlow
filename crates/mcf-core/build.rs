//! Captures what produced this binary, for `mcf_core::build_identity`.
//!
//! §3.12 makes the compiler and the target part of the conditions under which
//! an artifact was produced, and neither is knowable from the source tree —
//! only cargo and rustc know them, and only at build time.
//!
//! Three constraints shape this file:
//!
//! * **B36** — no build-time errand. `git` is not invoked: a source revision
//!   arrives through `MCF_BUILD_COMMIT` when the build environment knows one,
//!   and is absent otherwise (A7).
//! * **§3.12** — the output is a function of the toolchain and the target
//!   alone, so two builds of one revision agree byte for byte.
//! * **A2** — a capture that fails is not papered over with a plausible
//!   default. There is exactly one thing this script can fail at, running
//!   rustc, and it fails the build loudly rather than recording a guess.
//!
//! This is the one file in the workspace permitted to panic: a build script
//! has no MCF failure type available to it and no record to write to, and a
//! failed build is the loudest, most honest outcome available here.

// The workspace denies these everywhere else (A2, B-003). A build script has
// no MCF failure type available to it, no record to write to and no caller to
// return an outcome to, so the loudest available failure is the honest one —
// and it stops the build rather than shipping a binary that misreports what
// compiled it.
#![allow(clippy::panic)]

use std::process::Command;

fn main() {
    // Re-run only when something that changes the answer changes. Without
    // these, cargo reruns the script on every source edit, which costs a rustc
    // invocation per build for no new information (§3.13).
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

/// Reads a variable cargo documents itself as always setting for a build
/// script. Its absence is a broken build environment, not a missing value, so
/// it is not an `Unknown` (A7 governs what MCF does not know, not what its own
/// build system failed to tell it).
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
