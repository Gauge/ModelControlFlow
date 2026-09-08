use core::fmt;
use std::path::Path;

use crate::attested::Attested;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Storage {
    pub filesystem: String,
    pub mount_point: String,
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

#[must_use]
pub fn of(path: &Path) -> Attested<Storage> {
    let Ok(mounts) = std::fs::read_to_string("/proc/self/mounts") else {
        return Attested::Unknown;
    };
    let Ok(absolute) = std::fs::canonicalize(path) else {
        return Attested::Unknown;
    };
    match of_mounts(&mounts, &absolute.to_string_lossy()) {
        Some(storage) => Attested::Known(storage),
        None => Attested::Unknown,
    }
}

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

#[must_use]
pub fn children_major_faults() -> Attested<u64> {
    let Ok(stat) = std::fs::read_to_string("/proc/self/stat") else {
        return Attested::Unknown;
    };
    let Some(rest) = stat.rsplit_once(')').map(|(_, rest)| rest) else {
        return Attested::Unknown;
    };
    match rest.split_whitespace().nth(10).map(str::parse) {
        Some(Ok(faults)) => Attested::Known(faults),
        Some(Err(_)) | None => Attested::Unknown,
    }
}

#[cfg(test)]
mod tests;
