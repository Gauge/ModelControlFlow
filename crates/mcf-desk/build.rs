//! Finds the provisioned SDL3, or arranges for the window to refuse politely.
//!
//! **A crate that will not build without a provisioned component is a crate
//! that breaks the build for everybody who has not provisioned it.** So this
//! looks for the library MCF built, links it where it is there, and sets a
//! `cfg` where it is not — and the window then refuses at run time with a
//! sentence saying what to run. The check suite, the from-scratch tier and a
//! fresh clone all keep working on a machine with no window library at all.

use std::path::PathBuf;

fn provisioned() -> Option<PathBuf> {
    let home = std::env::var_os("MCF_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var_os("XDG_DATA_HOME")
                .map(PathBuf::from)
                .map(|data| data.join("mcf"))
        })
        .or_else(|| {
            std::env::var_os("HOME")
                .map(PathBuf::from)
                .map(|home| home.join(".local").join("share").join("mcf"))
        })?;
    let root = home.join("provisioned");
    let entries = std::fs::read_dir(root).ok()?;
    for entry in entries.flatten() {
        let name = entry.file_name();
        let name = name.to_str().unwrap_or_default();
        if !name.starts_with("SDL3@") {
            continue;
        }
        let archive = entry.path().join("build").join("libSDL3.a");
        if archive.is_file() {
            return Some(entry.path().join("build"));
        }
    }
    None
}

fn main() {
    println!("cargo:rerun-if-env-changed=MCF_DATA_HOME");
    println!("cargo:rustc-check-cfg=cfg(have_sdl)");
    // Watch the directory itself, not only the variable that names it.
    // Without this, provisioning the library after a build leaves the
    // decision cached, and the crate goes on believing what was true when it
    // last looked.
    if let Some(home) = std::env::var_os("HOME") {
        let root = PathBuf::from(home).join(".local/share/mcf/provisioned");
        println!("cargo:rerun-if-changed={}", root.display());
    }
    let Some(directory) = provisioned() else {
        return;
    };
    println!(
        "cargo:rerun-if-changed={}",
        directory.join("libSDL3.a").display()
    );
    println!("cargo:rustc-link-search=native={}", directory.display());
    println!("cargo:rustc-link-lib=static=SDL3");
    // What the archive leaves undefined, from its own `sdl3.pc`: threads and
    // the maths library, plus `dl` because SDL opens the windowing system at
    // run time rather than linking it. That is why one artifact runs under
    // both X11 and Wayland.
    println!("cargo:rustc-link-lib=dylib=m");
    println!("cargo:rustc-link-lib=dylib=dl");
    println!("cargo:rustc-link-lib=dylib=pthread");
    println!("cargo:rustc-cfg=have_sdl");
}
