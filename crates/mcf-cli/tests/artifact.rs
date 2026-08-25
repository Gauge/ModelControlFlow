//! What the shipped artifact needs from the machine it lands on (B-192, B36,
//! §XVI).
//!
//! B36: *the user obtains MCF and runs it — no runtime, interpreter, toolchain,
//! framework or separately-fetched engine.* B-192's condition is the checkable
//! half of that: **the artifact has no dynamic dependency a stock machine
//! lacks.** A binary that needs one is a binary whose first honest message is an
//! installation instruction, which B36 calls an unpinned dependency wearing a
//! helpful face.
//!
//! **The check reads the binary rather than asking a tool.** `ldd` is the
//! obvious route and it is a program that may not be installed, that runs the
//! loader, and that answers a slightly different question — what resolves *on
//! this machine* rather than what the file *requires*. The requirement is in
//! the file: `DT_NEEDED` entries in the dynamic section. Eighty lines of ELF
//! reading is the same trade `mcf_record::json` made, for the same reason
//! (B15).
//!
//! **It examines whichever binary cargo built for this run**, and says which
//! profile that was. The gating tier therefore checks the test-profile binary
//! and the scheduled `--with-budget` run — which is `cargo test --release` —
//! checks the artifact D24's ceilings are about. The set of libraries does not
//! differ between the two today, and if it ever does, the release run is the
//! one that is about the shipped thing (§3.4).
//!
//! **What it cannot check here it says rather than skips.** The vendored
//! inference stack is B-320 and needs an engine, which needs DEC-004; a
//! from-scratch container with no toolchain is B-183. This test is about what
//! the artifact requires of a machine, which is checkable now and stays true as
//! those arrive.

// Every item in this file is test code; see the note in
// checks/tests/taxonomy_agreement.rs.
#![allow(clippy::expect_used, clippy::panic, clippy::indexing_slicing)]

use std::path::{Path, PathBuf};

use mcf_core::build_identity::BuildIdentity;

/// The binary under test: the one cargo built for this test run.
fn binary() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_mcf"))
}

/// The libraries a stock Linux machine has, and MCF may therefore need.
///
/// Short, and each entry is here because it is part of what "a Linux machine"
/// means rather than because MCF happens to link it: the C library, the
/// compiler's unwinding support, the maths library and the dynamic loader
/// itself. Anything else — a vendor runtime, a maths kernel, a compression
/// library — is a prerequisite the user would have to obtain, which is exactly
/// what B36 refuses.
///
/// Adding an entry is a decision about what MCF requires of a machine, and it
/// belongs in `doc/vendored.md` with its reasoning (B-321) rather than here
/// alone.
const STOCK: &[&str] = &[
    "libc.so.6",
    "libm.so.6",
    "libgcc_s.so.1",
    "ld-linux-x86-64.so.2",
    "ld-linux-aarch64.so.1",
];

/// The artifact requires nothing a stock machine lacks.
#[test]
fn the_artifact_needs_nothing_a_stock_machine_lacks() {
    let path = binary();
    let bytes = std::fs::read(&path).expect("the binary cargo built is readable");
    let Some(elf) = Elf::read(&bytes) else {
        println!(
            "  {} is not ELF, so this platform's dependency check is not this one \
             (D29: every platform, Linux first)",
            path.display()
        );
        return;
    };

    let needed = elf.needed();
    println!(
        "  {} profile, {} dynamic dependencies: {}",
        BuildIdentity::current().profile,
        needed.len(),
        if needed.is_empty() {
            "none".to_owned()
        } else {
            needed.join(", ")
        }
    );

    let strangers: Vec<&String> = needed
        .iter()
        .filter(|library| !STOCK.contains(&library.as_str()))
        .collect();
    assert!(
        strangers.is_empty(),
        "the artifact requires {strangers:?}, which a stock machine may not have (B36). \
         A dependency admitted deliberately is recorded in doc/vendored.md and added to \
         STOCK with its reasoning; one that arrived by accident is a prerequisite MCF \
         would be asking the user to obtain."
    );
}

/// The artifact does not carry a search path of its own.
///
/// `DT_RUNPATH` and `DT_RPATH` say *look for libraries over there*, and "over
/// there" is a directory on the machine that built it. A binary that needs one
/// is a binary that works where it was made, which is the opposite of what
/// §XVI asks for; it is also how a vendored stack gets shipped by accident
/// rather than by decision (B-320).
#[test]
fn the_artifact_carries_no_library_search_path() {
    let bytes = std::fs::read(binary()).expect("the binary is readable");
    let Some(elf) = Elf::read(&bytes) else {
        return;
    };
    let paths = elf.search_paths();
    assert!(
        paths.is_empty(),
        "the artifact carries a library search path: {paths:?}"
    );
}

/// The one thing the loader is asked for is the loader every Linux has.
#[test]
fn the_interpreter_is_the_platforms_own() {
    let bytes = std::fs::read(binary()).expect("the binary is readable");
    let Some(elf) = Elf::read(&bytes) else {
        return;
    };
    match elf.interpreter() {
        None => println!("  statically linked: no interpreter at all"),
        Some(interpreter) => {
            println!("  interpreter: {interpreter}");
            let name = interpreter.rsplit('/').next().unwrap_or(&interpreter);
            assert!(
                STOCK.contains(&name),
                "the artifact asks for the interpreter {interpreter}, which is not the \
                 platform's own"
            );
            assert!(
                interpreter.starts_with("/lib"),
                "the interpreter is outside the platform's library directories: {interpreter}"
            );
        }
    }
}

/// Just enough ELF to answer "what does this file require".
///
/// Sixty-four-bit, little-endian, which is every platform D29 calls
/// characterized today. Anything else reads as *not this format* rather than as
/// a guess (A7).
struct Elf<'a> {
    bytes: &'a [u8],
    program_headers: Vec<ProgramHeader>,
}

struct ProgramHeader {
    kind: u32,
    offset: u64,
    virtual_address: u64,
    file_size: u64,
}

/// Segment kinds, from the ELF specification.
const PT_LOAD: u32 = 1;
const PT_DYNAMIC: u32 = 2;
const PT_INTERP: u32 = 3;

/// Dynamic-section tags, from the ELF specification.
const DT_NULL: u64 = 0;
const DT_NEEDED: u64 = 1;
const DT_STRTAB: u64 = 5;
const DT_RPATH: u64 = 15;
const DT_RUNPATH: u64 = 29;

impl<'a> Elf<'a> {
    fn read(bytes: &'a [u8]) -> Option<Self> {
        // \x7fELF, 64-bit, little-endian.
        if bytes.get(..4)? != b"\x7fELF" || *bytes.get(4)? != 2 || *bytes.get(5)? != 1 {
            return None;
        }
        let program_header_offset = u64_at(bytes, 0x20)?;
        let entry_size = usize::from(u16_at(bytes, 0x36)?);
        let count = usize::from(u16_at(bytes, 0x38)?);

        let mut program_headers = Vec::with_capacity(count);
        for index in 0..count {
            let at = usize::try_from(program_header_offset).ok()? + index * entry_size;
            program_headers.push(ProgramHeader {
                kind: u32_at(bytes, at)?,
                offset: u64_at(bytes, at + 0x08)?,
                virtual_address: u64_at(bytes, at + 0x10)?,
                file_size: u64_at(bytes, at + 0x20)?,
            });
        }
        Some(Self {
            bytes,
            program_headers,
        })
    }

    /// The file offset a virtual address lives at, if a loadable segment covers
    /// it.
    fn offset_of(&self, address: u64) -> Option<usize> {
        self.program_headers
            .iter()
            .filter(|header| header.kind == PT_LOAD)
            .find(|header| {
                address >= header.virtual_address
                    && address < header.virtual_address + header.file_size
            })
            .and_then(|header| {
                usize::try_from(header.offset + (address - header.virtual_address)).ok()
            })
    }

    /// Every `(tag, value)` in the dynamic section.
    fn dynamic(&self) -> Vec<(u64, u64)> {
        let Some(section) = self
            .program_headers
            .iter()
            .find(|header| header.kind == PT_DYNAMIC)
        else {
            return Vec::new();
        };
        let Ok(start) = usize::try_from(section.offset) else {
            return Vec::new();
        };
        let Ok(length) = usize::try_from(section.file_size) else {
            return Vec::new();
        };

        let mut entries = Vec::new();
        let mut at = start;
        while at + 16 <= start + length {
            let (Some(tag), Some(value)) = (u64_at(self.bytes, at), u64_at(self.bytes, at + 8))
            else {
                break;
            };
            if tag == DT_NULL {
                break;
            }
            entries.push((tag, value));
            at += 16;
        }
        entries
    }

    /// The string table the dynamic section names, as a file offset.
    fn string_table(&self) -> Option<usize> {
        self.dynamic()
            .into_iter()
            .find(|(tag, _)| *tag == DT_STRTAB)
            .and_then(|(_, address)| self.offset_of(address))
    }

    fn string_at(&self, table: usize, index: u64) -> Option<String> {
        let start = table + usize::try_from(index).ok()?;
        let rest = self.bytes.get(start..)?;
        let end = rest.iter().position(|byte| *byte == 0)?;
        Some(String::from_utf8_lossy(&rest[..end]).into_owned())
    }

    /// The libraries this file requires by name.
    fn needed(&self) -> Vec<String> {
        let Some(table) = self.string_table() else {
            return Vec::new();
        };
        self.dynamic()
            .into_iter()
            .filter(|(tag, _)| *tag == DT_NEEDED)
            .filter_map(|(_, index)| self.string_at(table, index))
            .collect()
    }

    /// Any library search path baked into the file.
    fn search_paths(&self) -> Vec<String> {
        let Some(table) = self.string_table() else {
            return Vec::new();
        };
        self.dynamic()
            .into_iter()
            .filter(|(tag, _)| *tag == DT_RPATH || *tag == DT_RUNPATH)
            .filter_map(|(_, index)| self.string_at(table, index))
            .collect()
    }

    /// The dynamic loader this file asks for, if it asks for one.
    fn interpreter(&self) -> Option<String> {
        let segment = self
            .program_headers
            .iter()
            .find(|header| header.kind == PT_INTERP)?;
        let start = usize::try_from(segment.offset).ok()?;
        let length = usize::try_from(segment.file_size).ok()?;
        let raw = self.bytes.get(start..start + length)?;
        let end = raw.iter().position(|byte| *byte == 0).unwrap_or(raw.len());
        Some(String::from_utf8_lossy(&raw[..end]).into_owned())
    }
}

fn u16_at(bytes: &[u8], at: usize) -> Option<u16> {
    Some(u16::from_le_bytes(bytes.get(at..at + 2)?.try_into().ok()?))
}

fn u32_at(bytes: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(bytes.get(at..at + 4)?.try_into().ok()?))
}

fn u64_at(bytes: &[u8], at: usize) -> Option<u64> {
    Some(u64::from_le_bytes(bytes.get(at..at + 8)?.try_into().ok()?))
}

/// The reader is right about a file whose answer is known independently.
///
/// A19, and the negative control this check needs: an ELF reader that returned
/// nothing would report every binary as depending on nothing at all, and the
/// three tests above would pass on an artifact that required a vendor runtime.
/// `/bin/sh` is on every machine this runs on and is dynamically linked against
/// the C library.
#[test]
fn the_reader_agrees_with_a_file_whose_dependencies_are_known() {
    let known = Path::new("/bin/sh");
    let Ok(bytes) = std::fs::read(known) else {
        println!("  /bin/sh is not readable here, so the reader has no control to check against");
        return;
    };
    let Some(elf) = Elf::read(&bytes) else {
        println!("  /bin/sh is not ELF here");
        return;
    };
    let needed = elf.needed();
    println!("  control: /bin/sh needs {}", needed.join(", "));
    assert!(
        needed.iter().any(|library| library.starts_with("libc.so")),
        "the reader found {needed:?} for /bin/sh, which every machine links against the C \
         library — so the reader, not the artifact, is what this check is measuring"
    );
    assert!(
        elf.interpreter().is_some_and(|path| path.contains("ld-")),
        "the reader found no interpreter for a dynamically linked /bin/sh"
    );

    // And the predicate, not only the reader. `/bin/sh` on this machine needs
    // a terminal library that is not on the stock list, so the filter the first
    // test applies has something it must reject — without this, a STOCK list
    // that accidentally matched everything would look identical to a clean
    // artifact.
    let strangers: Vec<&String> = needed
        .iter()
        .filter(|library| !STOCK.contains(&library.as_str()))
        .collect();
    if strangers.is_empty() {
        println!(
            "  control: every library /bin/sh needs happens to be on the stock list here, \
             so the predicate has nothing to reject on this machine"
        );
    } else {
        println!("  control: the predicate rejects {strangers:?} for /bin/sh");
    }
}
