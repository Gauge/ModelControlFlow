#![allow(
    unsafe_code,
    reason = "the page cache is asked and advised through the C interface, which has no safe form"
)]

use std::ffi::{c_int, c_void};
use std::os::fd::AsRawFd as _;
use std::path::Path;

use mcf_record::json::Value;

use super::{Found, Reading, Site, as_ms, filler, gigabytes, per_second, timed, whole};
use crate::generation::Draw;
use crate::served::Prompt;

pub const NAME: &str = "cold-start";

const RESOLVED_NS: u64 = 500_000_000;

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

fn resident_pages(path: &Path) -> Option<(u64, u64)> {
    let file = std::fs::File::open(path).ok()?;
    let len = usize::try_from(file.metadata().ok()?.len()).ok()?;
    if len == 0 {
        return Some((0, 0));
    }
    let page = 4096_usize;
    let pages = len.div_ceil(page);
    let mut vector = vec![0_u8; pages];
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

fn evict(path: &Path) -> bool {
    let Ok(file) = std::fs::File::open(path) else {
        return false;
    };
    unsafe { posix_fadvise(file.as_raw_fd(), 0, 0, POSIX_FADV_DONTNEED) == 0 }
}

#[must_use]
#[allow(
    clippy::too_many_lines,
    reason = "one measurement read straight through: what it started, what it read, the rows it kept"
)]
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
    let bytes_of = |held: u64| i64::try_from(held).unwrap_or(i64::MAX);
    let mut rows = vec![
        Reading::new(&[], "resident_before_ppm", super::ppm(before, pages), "ppm"),
        Reading::new(&[], "resident_after_ppm", still, "ppm"),
    ];
    for (start, (load, first)) in [("cold", cold), ("warm", warm)] {
        let at = [("start", Value::text(start))];
        rows.push(Reading::new(&at, "load_ns", bytes_of(load), "ns"));
        rows.push(Reading::new(&at, "first_token_ns", bytes_of(first), "ns"));
    }
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
        rows,
    }
}

#[expect(clippy::integer_division, reason = "nanoseconds to whole milliseconds")]
fn to_ms(ns: u64) -> u64 {
    ns / 1_000_000
}

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
