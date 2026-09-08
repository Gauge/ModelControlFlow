#![allow(clippy::expect_used, clippy::panic, clippy::indexing_slicing)]

use std::path::{Path, PathBuf};

use mcf_core::build_identity::BuildIdentity;

fn binary() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_mcf"))
}

const STOCK: &[&str] = &[
    "libc.so.6",
    "libm.so.6",
    "libgcc_s.so.1",
    "ld-linux-x86-64.so.2",
    "ld-linux-aarch64.so.1",
];

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

const PT_LOAD: u32 = 1;
const PT_DYNAMIC: u32 = 2;
const PT_INTERP: u32 = 3;

const DT_NULL: u64 = 0;
const DT_NEEDED: u64 = 1;
const DT_STRTAB: u64 = 5;
const DT_RPATH: u64 = 15;
const DT_RUNPATH: u64 = 29;

impl<'a> Elf<'a> {
    fn read(bytes: &'a [u8]) -> Option<Self> {
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
