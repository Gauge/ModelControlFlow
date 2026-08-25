//! What the storage reader claims, checked against what the machine says.

use super::{Storage, children_major_faults, covers, of, of_mounts};
use crate::attested::Attested;

/// A table with three mounts, one of them nested. The reader must choose the
/// innermost, because that is the one serving the bytes.
const MOUNTS: &str = "\
/dev/nvme1n1p6 / btrfs rw,relatime 0 0
tmpfs /tmp tmpfs rw,nosuid,nodev 0 0
/dev/sda1 /home/gauge/Content fuseblk rw,nosuid,nodev 0 0
/dev/nvme1n1p6 /home btrfs rw,relatime 0 0
";

#[test]
fn the_innermost_mount_is_the_one_that_answers() {
    let found = of_mounts(MOUNTS, "/home/gauge/Content/git/mcf/target/release/mcf")
        .expect("a mount covers it");
    assert_eq!(found.filesystem, "fuseblk");
    assert_eq!(found.mount_point, "/home/gauge/Content");
    assert_eq!(found.source, "/dev/sda1");
}

#[test]
fn a_path_under_a_plain_mount_gets_that_mount() {
    let found = of_mounts(MOUNTS, "/tmp/mcf-lab-1/record.jsonl").expect("a mount covers it");
    assert_eq!(found.filesystem, "tmpfs");
    assert_eq!(found.mount_point, "/tmp");
}

#[test]
fn a_path_under_nothing_but_the_root_gets_the_root() {
    let found = of_mounts(MOUNTS, "/usr/bin/mcf").expect("the root covers everything");
    assert_eq!(found.mount_point, "/");
    assert_eq!(found.filesystem, "btrfs");
}

/// A mount point is an ancestor by *components*. `/home` does not cover
/// `/homework`, and a prefix comparison would say it does — which would
/// attribute a measurement to the wrong device.
#[test]
fn a_mount_point_is_an_ancestor_and_not_a_string_prefix() {
    assert!(covers("/home", "/home/gauge"));
    assert!(covers("/home", "/home"));
    assert!(!covers("/home", "/homework/x"));
    assert!(covers("/", "/anything"));
    assert_eq!(
        of_mounts(MOUNTS, "/homework/x").map(|storage| storage.mount_point),
        Some("/".to_owned())
    );
}

/// A mount table this reader cannot read literally is one it declines rather
/// than guesses at (A7): the kernel escapes a space in a mount point as `\040`,
/// and unescaping it is work this module does not do.
#[test]
fn an_escaped_mount_point_is_declined_rather_than_matched() {
    let escaped = "/dev/sdb1 /mnt/my\\040disk ext4 rw 0 0\n/dev/sda1 / btrfs rw 0 0\n";
    let found = of_mounts(escaped, "/mnt/my disk/file").expect("the root still covers it");
    assert_eq!(found.mount_point, "/");
}

/// The live machine answers for a path that certainly exists, and the answer is
/// a filesystem the kernel names rather than an invention (A19: checked against
/// something independently true — `/proc/self/mounts` itself).
#[test]
fn the_live_reading_names_a_filesystem_the_kernel_lists() {
    let Attested::Known(Storage { filesystem, .. }) = of(std::path::Path::new("/")) else {
        println!("  this platform does not publish its mounts, so nothing is checked");
        return;
    };
    let mounts = std::fs::read_to_string("/proc/self/mounts").expect("mounts are readable");
    assert!(
        mounts
            .lines()
            .filter_map(|line| line.split_whitespace().nth(2))
            .any(|listed| listed == filesystem),
        "the reader answered {filesystem}, which is not a filesystem the kernel lists"
    );
}

/// A path that does not exist has no storage, rather than the storage of
/// whatever directory it would have been in.
#[test]
fn a_path_that_is_not_there_is_unknown() {
    assert_eq!(
        of(std::path::Path::new("/nonexistent/mcf-not-here")),
        Attested::Unknown
    );
}

/// The counter is a counter: it does not go backwards.
#[test]
fn the_fault_counter_only_ever_grows() {
    let Attested::Known(first) = children_major_faults() else {
        println!("  this platform does not account for children's faults");
        return;
    };
    let Attested::Known(second) = children_major_faults() else {
        panic!("the counter was readable a moment ago and is not now");
    };
    assert!(
        second >= first,
        "{second} is below the {first} read before it"
    );
}

/// It counts *children*, and a warm, tiny child usually costs nothing — but
/// this is a **process-wide** counter, so what it reports here includes every
/// child every other test on every other thread is spawning at the same time.
///
/// The observation is printed and not asserted, and that is F4.3's lesson
/// arriving for the third time: a reading that belongs to the process cannot be
/// taken beside another test. The assertion this test looks like it should make
/// lives in `scripts/check-fault-signal.sh`, which owns its process and watches
/// the counter go from zero to exactly one per spawn.
#[test]
fn what_a_warm_child_costs_is_observed_here_and_asserted_elsewhere() {
    let Attested::Known(before) = children_major_faults() else {
        println!("  this platform does not account for children's faults");
        return;
    };
    for _ in 0..20 {
        let _outcome = std::process::Command::new("/bin/true").status();
    }
    let Attested::Known(after) = children_major_faults() else {
        return;
    };
    println!(
        "  twenty warm, tiny children and whatever else this process was running: \
         {} major faults",
        after.saturating_sub(before)
    );
}
