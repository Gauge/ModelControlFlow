//! Cold start: the first token with the file not yet in memory, against
//! the same with it there, and the read bandwidth the difference implies
//! (B-501, D52).
//!
//! The ladder's start-up figure is warm and says so: the file is in the
//! page cache after the first load and nothing evicts it. This evicts
//! it — the kernel lets a process drop the clean pages of a file it can
//! open — confirms the eviction by asking which of the file's pages are
//! resident before and after, and times a start twice. A file that stays
//! resident after being told to go is mapped by another process, and the
//! reading says so rather than timing a warm start under a cold name.
#![allow(
    unsafe_code,
    reason = "the page cache is asked and advised through the C interface, which has no safe form"
)]

use std::ffi::{c_int, c_void};
use std::os::fd::AsRawFd as _;
use std::path::Path;

use mcf_record::json::Value;

use super::{Found, Site, as_ms, filler, gigabytes, per_second, timed, whole};
use crate::generation::Draw;
use crate::served::Prompt;

/// The measurement's name.
pub const NAME: &str = "cold-start";

/// How far apart two loads have to be before a read rate is read off
/// their difference: a start is noticed ready at the tenth of a second
/// it is looked for, so a difference under half a second is inside the
/// looking (A7).
const RESOLVED_NS: u64 = 500_000_000;

/// The share of pages that may remain resident for an eviction to count,
/// in parts per million: a page or two the kernel keeps for its own
/// reasons is not another process holding the file.
const STILL_RESIDENT_ALLOWED_PPM: i64 = 50_000;

unsafe extern "C" {
    fn posix_fadvise(fd: c_int, offset: i64, len: i64, advice: c_int) -> c_int;
    fn mmap(
        addr: *mut c_void,
        len: usize,
        prot: c_int,
        flags: c_int,
        fd: c_int,
        offset: i64,
    ) -> *mut c_void;
    fn munmap(addr: *mut c_void, len: usize) -> c_int;
    fn mincore(addr: *mut c_void, len: usize, vec: *mut u8) -> c_int;
}

const POSIX_FADV_DONTNEED: c_int = 4;
const PROT_NONE: c_int = 0;
const MAP_SHARED: c_int = 1;

/// How many of a file's pages are resident, of how many.
fn resident_pages(path: &Path) -> Option<(u64, u64)> {
    let file = std::fs::File::open(path).ok()?;
    let len = usize::try_from(file.metadata().ok()?.len()).ok()?;
    if len == 0 {
        return Some((0, 0));
    }
    let page = 4096_usize;
    let pages = len.div_ceil(page);
    let mut vector = vec![0_u8; pages];
    // SAFETY: a mapping of the file with no access rights, asked only
    // which of its pages are resident, and unmapped before the file is
    // closed. `vector` is as long as the mapping has pages.
    let counted = unsafe {
        let mapped = mmap(
            std::ptr::null_mut(),
            len,
            PROT_NONE,
            MAP_SHARED,
            file.as_raw_fd(),
            0,
        );
        if mapped as isize == -1 {
            return None;
        }
        let asked = mincore(mapped, len, vector.as_mut_ptr());
        let _unmapped = munmap(mapped, len);
        asked == 0
    };
    if !counted {
        return None;
    }
    let resident = vector.iter().filter(|held| **held & 1 == 1).count();
    Some((
        u64::try_from(resident).unwrap_or(u64::MAX),
        u64::try_from(pages).unwrap_or(u64::MAX),
    ))
}

/// Tells the kernel this process has no further use for the file's pages.
fn evict(path: &Path) -> bool {
    let Ok(file) = std::fs::File::open(path) else {
        return false;
    };
    // SAFETY: advice on an open descriptor over its whole length; the
    // call reads nothing and writes nothing of ours.
    unsafe { posix_fadvise(file.as_raw_fd(), 0, 0, POSIX_FADV_DONTNEED) == 0 }
}

/// Runs it.
#[must_use]
pub fn measure(site: &Site<'_>) -> Found {
    let Some(bytes) = std::fs::metadata(site.model).ok().map(|about| about.len()) else {
        return Found::could_not_tell("the file's size could not be read");
    };
    let Some((before, pages)) = resident_pages(site.model) else {
        return Found::could_not_tell(
            "the kernel would not say which of the file's pages are resident",
        );
    };
    if !evict(site.model) {
        return Found::could_not_tell("the kernel refused to drop the file's pages");
    }
    let Some((after, _)) = resident_pages(site.model) else {
        return Found::could_not_tell(
            "the kernel would not say which of the file's pages are resident",
        );
    };
    let still = super::ppm(after, pages);
    if still > STILL_RESIDENT_ALLOWED_PPM {
        return Found::could_not_tell(&format!(
            "{} of the file's pages are still resident after they were dropped, so another \
             process holds it mapped — a server hosting this model, most likely; a cold start \
             cannot be timed while it is up",
            super::per_cent(still)
        ));
    }
    let cold = match start_and_first_token(site) {
        Ok(cold) => cold,
        Err(why) => return Found::could_not_tell(&why),
    };
    let warm = match start_and_first_token(site) {
        Ok(warm) => warm,
        Err(why) => return Found::could_not_tell(&why),
    };
    let bandwidth = cold
        .0
        .checked_sub(warm.0)
        .filter(|difference| *difference >= RESOLVED_NS)
        .map(|difference| per_second(bytes, difference));
    Found {
        lines: vec![
            format!(
                "  {} of the file's pages were resident; {} after they were dropped",
                super::per_cent(super::ppm(before, pages)),
                super::per_cent(still)
            ),
            format!(
                "  cold   load {:>9} ms   first token {:>9} ms after it",
                as_ms(cold.0),
                as_ms(cold.1)
            ),
            format!(
                "  warm   load {:>9} ms   first token {:>9} ms after it",
                as_ms(warm.0),
                as_ms(warm.1)
            ),
            bandwidth.map_or_else(
                || {
                    format!(
                        "  the cold load was within {} ms of the warm one, which is inside the \
                         tenth of a second a start is looked for at, so no read rate is implied",
                        as_ms(RESOLVED_NS)
                    )
                },
                |rate| {
                    format!(
                        "  {} read in the difference: {} a second from wherever the file lives",
                        gigabytes(bytes),
                        gigabytes(rate)
                    )
                },
            ),
        ],
        fields: vec![
            ("file_bytes", whole(bytes)),
            (
                "resident_before_ppm",
                Value::Integer(super::ppm(before, pages)),
            ),
            ("resident_after_ppm", Value::Integer(still)),
            ("cold_load_ns", whole(cold.0)),
            ("cold_first_token_ns", whole(cold.1)),
            (
                "cold_first_token_ms",
                whole(to_ms(cold.0.saturating_add(cold.1))),
            ),
            ("warm_load_ns", whole(warm.0)),
            ("warm_first_token_ns", whole(warm.1)),
            (
                "warm_first_token_ms",
                whole(to_ms(warm.0.saturating_add(warm.1))),
            ),
            ("bytes_per_second", bandwidth.map_or(Value::Null, whole)),
        ],
    }
}

/// Nanoseconds as whole milliseconds.
#[expect(clippy::integer_division, reason = "nanoseconds to whole milliseconds")]
fn to_ms(ns: u64) -> u64 {
    ns / 1_000_000
}

/// How long a start takes until the engine says it is ready, and how
/// long the first token takes after that.
fn start_and_first_token(site: &Site<'_>) -> Result<(u64, u64), String> {
    let (engine, load) = timed(|| site.server(&site.startup()));
    let engine = engine?;
    let prompt = filler(16);
    let (done, first) = timed(|| {
        engine.complete(
            Prompt::Identifiers(&prompt),
            1,
            Draw::greedy(0),
            true,
            site.timed(),
        )
    });
    let _read = done.map_err(|failure| failure.detail().to_owned())?;
    Ok((load, first))
}
