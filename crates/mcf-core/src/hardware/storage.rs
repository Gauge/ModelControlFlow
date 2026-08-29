//! Where a file's bytes come from, and what that costs a measurement.
//!
//! **This module exists because of a specific run.** The budget tier failed its
//! cold-start figure at 252 ms against a 100 ms ceiling, and the cause was not
//! MCF: the artifact was on a FUSE mount whose p99 page-fault service time is a
//! thousand times its median. The same binary on tmpfs cleared the ceiling by
//! two orders of magnitude, and a later run of the same command on the same
//! machine passed at 0.58 ms. `doc/findings.md` F5 has the readings.
//!
//! Two things were missing and B-193 adds both. The **storage a measured
//! artifact was read from** is a condition of the measurement (§3.4, A6) and
//! this module reads it. And a measurement whose cost is in *another process*
//! cannot be judged by the measuring thread's scheduling (D30), because that
//! thread is blocked in `wait` while the child faults its pages in — so
//! [`children_major_faults`] is the second reading, and a measurement that took
//! any is a measurement of the device.
//!
//! **A major fault is the right signal, and the threshold is zero.** It is not
//! a judgement call like [`TOLERATED_DELAY_PPM`]: a warm, local artifact takes
//! none, and every one that happens is the kernel going to a device for bytes
//! the measurement then waited on. `scripts/check-fault-signal.sh` is the
//! experiment that established it — thirty spawns of one binary, page cache
//! evicted before each: zero major faults warm, exactly thirty evicted.
//!
//! [`TOLERATED_DELAY_PPM`]: super::TOLERATED_DELAY_PPM
//!
//! **Cross-check owed (B-390):** the filesystem and device behind a path are
//! read from one place.

use core::fmt;
use std::path::Path;

use crate::attested::Attested;

/// What a path's bytes are stored on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Storage {
    /// The filesystem type the kernel reports, such as `btrfs` or `fuseblk`.
    pub filesystem: String,
    /// Where it is mounted.
    pub mount_point: String,
    /// What is mounted there — a device, or whatever the platform names.
    pub source: String,
}

impl fmt::Display for Storage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} on {} ({})",
            self.filesystem, self.mount_point, self.source
        )
    }
}

/// The storage a path lives on.
///
/// `Unknown` where the platform does not publish its mounts, or where no
/// mount point is a prefix of the path — never a guess (A7). The answer is the
/// **longest** matching mount point, because mounts nest: `/home/x/y` under
/// `/home` under `/` is three candidates and only the innermost is the one
/// serving the bytes.
#[must_use]
pub fn of(path: &Path) -> Attested<Storage> {
    let Ok(mounts) = std::fs::read_to_string("/proc/self/mounts") else {
        return Attested::Unknown;
    };
    // A relative path cannot be matched against mount points; resolving it
    // needs a current directory, which is not a property of the file.
    let Ok(absolute) = std::fs::canonicalize(path) else {
        return Attested::Unknown;
    };
    match of_mounts(&mounts, &absolute.to_string_lossy()) {
        Some(storage) => Attested::Known(storage),
        None => Attested::Unknown,
    }
}

/// The judgement itself, separated from the reading so a test can supply the
/// mount table rather than needing one (D26's habit).
#[must_use]
pub(super) fn of_mounts(mounts: &str, path: &str) -> Option<Storage> {
    let mut best: Option<Storage> = None;
    let mut best_length = 0;
    for line in mounts.lines() {
        let mut fields = line.split_whitespace();
        let (Some(source), Some(mount_point), Some(filesystem)) =
            (fields.next(), fields.next(), fields.next())
        else {
            continue;
        };
        // The kernel escapes spaces and a few other characters as octal; a
        // mount point MCF cannot read literally is one it does not match
        // against rather than one it guesses at.
        if mount_point.contains('\\') {
            continue;
        }
        if !covers(mount_point, path) || mount_point.len() < best_length {
            continue;
        }
        best_length = mount_point.len();
        best = Some(Storage {
            filesystem: filesystem.to_owned(),
            mount_point: mount_point.to_owned(),
            source: source.to_owned(),
        });
    }
    best
}

/// Whether a mount point is an ancestor of a path, by path components rather
/// than by string prefix — `/home` covers `/home/x` and does not cover
/// `/homework`.
fn covers(mount_point: &str, path: &str) -> bool {
    if mount_point == "/" {
        return path.starts_with('/');
    }
    path == mount_point
        || path
            .strip_prefix(mount_point)
            .is_some_and(|rest| rest.starts_with('/'))
}

/// How many major page faults this process's **reaped children** have taken.
///
/// Major, not minor: a minor fault is a page already in memory being mapped,
/// which every process does thousands of times and which costs nothing worth
/// measuring. A major fault is the kernel going to a device. Read before and
/// after a measurement that spawns children, the difference is how much of that
/// measurement was the storage rather than the work.
///
/// The children's counter rather than this process's own, because the work a
/// cold-start measurement times happens in the child — which is exactly the
/// blind spot D30's thread-scheduling signal has (F5).
///
/// `Unknown` where the platform does not publish it (A7).
#[must_use]
pub fn children_major_faults() -> Attested<u64> {
    let Ok(stat) = std::fs::read_to_string("/proc/self/stat") else {
        return Attested::Unknown;
    };
    // The second field is the executable's name in parentheses and may itself
    // contain parentheses and spaces, so the fields after it are found from the
    // *last* closing parenthesis rather than by splitting the whole line.
    let Some(rest) = stat.rsplit_once(')').map(|(_, rest)| rest) else {
        return Attested::Unknown;
    };
    // Counting from `state`, which is field 3: `cmajflt` is field 17 in the
    // kernel's documented order, so it is the eleventh here.
    match rest.split_whitespace().nth(10).map(str::parse) {
        Some(Ok(faults)) => Attested::Known(faults),
        Some(Err(_)) | None => Attested::Unknown,
    }
}

#[cfg(test)]
mod tests;
