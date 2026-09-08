#![allow(unsafe_code)]

use core::ffi::{c_char, c_int, c_ulong};
use std::path::Path;

use crate::attested::Attested;
use crate::measurement::Bytes;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Space {
    pub total: Bytes,
    pub available: Bytes,
}

impl core::fmt::Display for Space {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{} available of {}", self.available, self.total)
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
#[allow(clippy::struct_field_names)]
struct StatVfs {
    f_bsize: c_ulong,
    f_frsize: c_ulong,
    f_blocks: u64,
    f_bfree: u64,
    f_bavail: u64,
    f_files: u64,
    f_ffree: u64,
    f_favail: u64,
    f_fsid: c_ulong,
    f_flag: c_ulong,
    f_namemax: c_ulong,
    f_spare: [c_int; 6],
}

unsafe extern "C" {
    fn statvfs(path: *const c_char, buf: *mut StatVfs) -> c_int;
}

#[must_use]
pub fn on(path: &Path) -> Attested<Space> {
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

    let status = unsafe { statvfs(c_path.as_ptr().cast::<c_char>(), &raw mut reading) };
    if status != 0 {
        return Attested::Unknown;
    }

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
