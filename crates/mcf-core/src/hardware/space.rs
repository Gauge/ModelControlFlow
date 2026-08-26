//! How much room a filesystem has left (B-026, §3.11).
//!
//! **Why this is here rather than inferred.** §3.11 asks that a download which
//! would exhaust the disk be *a decision, not a surprise*, and a decision needs
//! a number before the download rather than an error at ninety per cent. The
//! platform has that number and the standard library does not expose it, so
//! this is the second module in the workspace to take the `unsafe_code`
//! opt-out — the first being [`super::nvml`], and the rule being a `deny`
//! rather than a `forbid` for exactly this reason (D4, build.md §4).
//!
//! **What is admitted is one call.** `statvfs`, with a NUL-terminated path, a
//! zeroed struct, and a status checked before any field is read. Everything
//! above this module sees safe Rust and an [`Attested`] reading, which is
//! `Unknown` wherever the call fails rather than a zero — a filesystem MCF
//! cannot measure has *unknown* room, and A7 forbids the plausible substitute
//! (which here would read as "there is none").
//!
//! **What it does not do.** Predict. The number is what the kernel said at the
//! moment it was asked; another process can take the space a moment later, and
//! [findings.md](../../../../doc/findings.md) F11 records what that looks like
//! when it happens — `StorageFull`, at the flush rather than at the write.
//! Checking beforehand turns the common case from a surprise into a refusal; it
//! does not make the failure impossible, which is why the transfer classifies
//! it too.

// The second module in the workspace that opts in; the reason is above.
#![allow(unsafe_code)]

use core::ffi::{c_char, c_int, c_ulong};
use std::path::Path;

use crate::attested::Attested;
use crate::measurement::Bytes;

/// What a filesystem has and what is left.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Space {
    /// How large the filesystem is.
    pub total: Bytes,
    /// How much of it an ordinary process could still use.
    ///
    /// The *available* figure rather than the free one: a filesystem reserves
    /// blocks for the superuser, and a plan made against free space would
    /// promise room MCF cannot have.
    pub available: Bytes,
}

impl core::fmt::Display for Space {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{} available of {}", self.available, self.total)
    }
}

/// The `statvfs` structure, as Linux lays it out.
///
/// Declared here rather than taken from a binding crate: it is eleven integers
/// and a reserved tail, and B15 admits weight against a stated cost — a
/// dependency for one struct would be weight admitted for familiarity.
///
/// The field order is the kernel's ABI and must not be rearranged. Only two
/// fields are read; the rest are named so that the layout is checkable against
/// the manual page rather than trusted.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
// The names are the kernel's, and a reader checking this against `man 3
// statvfs` should find the same words in the same order. Renaming them to
// please a lint would make the one thing that must be verifiable — that this
// layout is that layout — harder to verify.
#[allow(clippy::struct_field_names)]
struct StatVfs {
    /// Filesystem block size.
    f_bsize: c_ulong,
    /// Fragment size — the unit the block counts below are in.
    f_frsize: c_ulong,
    /// Blocks in total.
    f_blocks: u64,
    /// Blocks free.
    f_bfree: u64,
    /// Blocks available to an unprivileged process.
    f_bavail: u64,
    /// Inodes in total.
    f_files: u64,
    /// Inodes free.
    f_ffree: u64,
    /// Inodes available to an unprivileged process.
    f_favail: u64,
    /// Filesystem identifier.
    f_fsid: c_ulong,
    /// Mount flags.
    f_flag: c_ulong,
    /// Maximum filename length.
    f_namemax: c_ulong,
    /// Reserved by the ABI.
    f_spare: [c_int; 6],
}

unsafe extern "C" {
    /// `int statvfs(const char *path, struct statvfs *buf)`.
    fn statvfs(path: *const c_char, buf: *mut StatVfs) -> c_int;
}

/// What the filesystem holding this path has left.
///
/// `Unknown` when the path cannot be turned into a C string, when the call
/// fails, or when the arithmetic overflows — three different ways of not
/// knowing, and none of them is a number (A7).
#[must_use]
pub fn on(path: &Path) -> Attested<Space> {
    // A NUL anywhere in the path makes it something the kernel cannot be
    // asked about, which is a refusal rather than a truncation (§3.7).
    let Some(text) = path.to_str() else {
        return Attested::Unknown;
    };
    if text.contains('\0') {
        return Attested::Unknown;
    }
    let mut c_path = text.as_bytes().to_vec();
    c_path.push(0);

    let mut reading = StatVfs {
        f_bsize: 0,
        f_frsize: 0,
        f_blocks: 0,
        f_bfree: 0,
        f_bavail: 0,
        f_files: 0,
        f_ffree: 0,
        f_favail: 0,
        f_fsid: 0,
        f_flag: 0,
        f_namemax: 0,
        f_spare: [0; 6],
    };

    // SAFETY: `c_path` is a NUL-terminated byte string that outlives the call,
    // and `reading` is a fully initialized `StatVfs` this call may write to.
    // The status is checked before any field of it is read.
    let status = unsafe { statvfs(c_path.as_ptr().cast::<c_char>(), &raw mut reading) };
    if status != 0 {
        return Attested::Unknown;
    }

    // The fragment size is what the block counts are in; some filesystems
    // report zero for it, in which case the block size is the unit.
    let unit = if reading.f_frsize == 0 {
        reading.f_bsize
    } else {
        reading.f_frsize
    };
    if unit == 0 {
        return Attested::Unknown;
    }

    let Some(total) = reading.f_blocks.checked_mul(unit) else {
        return Attested::Unknown;
    };
    let Some(available) = reading.f_bavail.checked_mul(unit) else {
        return Attested::Unknown;
    };
    Attested::Known(Space {
        total: Bytes(total),
        available: Bytes(available),
    })
}

#[cfg(test)]
mod tests;
