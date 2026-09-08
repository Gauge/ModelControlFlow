use super::{Storage, children_major_faults, covers, of, of_mounts};
use crate::attested::Attested;

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

#[test]
fn an_escaped_mount_point_is_declined_rather_than_matched() {
    let escaped = "/dev/sdb1 /mnt/my\\040disk ext4 rw 0 0\n/dev/sda1 / btrfs rw 0 0\n";
    let found = of_mounts(escaped, "/mnt/my disk/file").expect("the root still covers it");
    assert_eq!(found.mount_point, "/");
}

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

#[test]
fn a_path_that_is_not_there_is_unknown() {
    assert_eq!(
        of(std::path::Path::new("/nonexistent/mcf-not-here")),
        Attested::Unknown
    );
}

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
