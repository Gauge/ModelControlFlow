use std::path::{Path, PathBuf};
use std::process::Command;

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

fn compiler() -> Option<String> {
    let named = std::env::var("CC").ok();
    let candidates = named
        .iter()
        .map(String::as_str)
        .chain(["cc", "gcc", "clang"]);
    for candidate in candidates {
        let ran = Command::new(candidate).arg("--version").output();
        if ran.is_ok_and(|out| out.status.success()) {
            return Some(candidate.to_owned());
        }
    }
    None
}

fn build_the_font_stack(out: &Path, csrc: &Path) -> bool {
    let Some(cc) = compiler() else {
        println!("cargo:warning=no C compiler found: mcf-desk will have no text");
        return false;
    };
    let object = out.join("font.o");
    let compiled = Command::new(&cc)
        .arg("-c")
        .arg(csrc.join("font.c"))
        .arg("-o")
        .arg(&object)
        .args(["-O2", "-fPIC", "-std=c99", "-Wall"])
        .arg("-I")
        .arg(csrc)
        .output();
    match compiled {
        Ok(out) if out.status.success() => {}
        Ok(out) => {
            println!(
                "cargo:warning=font.c did not compile: {}",
                String::from_utf8_lossy(&out.stderr)
                    .lines()
                    .last()
                    .unwrap_or("?")
            );
            return false;
        }
        Err(error) => {
            println!("cargo:warning=font.c did not compile: {error}");
            return false;
        }
    }
    let archive = out.join("libmcffont.a");
    let _cleared = std::fs::remove_file(&archive);
    let archiver = std::env::var("AR").unwrap_or_else(|_| "ar".to_owned());
    let bundled = Command::new(archiver)
        .arg("rcs")
        .arg(&archive)
        .arg(&object)
        .output();
    if !bundled.is_ok_and(|done| done.status.success()) {
        println!("cargo:warning=font.o could not be archived: mcf-desk will have no text");
        return false;
    }
    println!("cargo:rustc-link-search=native={}", out.display());
    println!("cargo:rustc-link-lib=static=mcffont");
    true
}

fn main() {
    println!("cargo:rerun-if-env-changed=MCF_DATA_HOME");
    println!("cargo:rerun-if-env-changed=CC");
    println!("cargo:rustc-check-cfg=cfg(have_sdl)");
    println!("cargo:rustc-check-cfg=cfg(have_font)");

    let csrc = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("csrc");
    println!("cargo:rerun-if-changed={}", csrc.join("font.c").display());
    println!(
        "cargo:rerun-if-changed={}",
        csrc.join("stb_truetype.h").display()
    );
    if let Some(out) = std::env::var_os("OUT_DIR")
        && build_the_font_stack(&PathBuf::from(out), &csrc)
    {
        println!("cargo:rustc-cfg=have_font");
        println!("cargo:rustc-link-lib=dylib=m");
    }

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
    println!("cargo:rustc-link-lib=dylib=m");
    println!("cargo:rustc-link-lib=dylib=dl");
    println!("cargo:rustc-link-lib=dylib=pthread");
    println!("cargo:rustc-cfg=have_sdl");
}
