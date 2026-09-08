use std::path::{Path, PathBuf};

use mcf_core::failure::Category;

use super::{OPERATIONS, run, run_under};

struct Machine(PathBuf);

impl Machine {
    fn new(name: &str, count: usize, governors: &str, current: &str) -> Self {
        let root = std::env::temp_dir().join(format!("mcf-helper-{name}-{}", std::process::id()));
        drop(std::fs::remove_dir_all(&root));
        for index in 0..count {
            let cpufreq = root
                .join("sys/devices/system/cpu")
                .join(format!("cpu{index}"))
                .join("cpufreq");
            std::fs::create_dir_all(&cpufreq).expect("a fixture");
            std::fs::write(cpufreq.join("scaling_governor"), format!("{current}\n"))
                .expect("a governor");
            std::fs::write(
                cpufreq.join("scaling_available_governors"),
                format!("{governors}\n"),
            )
            .expect("the offered governors");
        }
        Self(root)
    }

    fn with_energy(self, domains: &[(&str, u64)]) -> Self {
        for (index, (name, microjoules)) in domains.iter().enumerate() {
            let domain = self
                .0
                .join("sys/class/powercap")
                .join(format!("rapl:{index}"));
            std::fs::create_dir_all(&domain).expect("a fixture");
            std::fs::write(domain.join("name"), format!("{name}\n")).expect("a name");
            std::fs::write(domain.join("energy_uj"), format!("{microjoules}\n"))
                .expect("a counter");
        }
        self
    }

    fn under(&self) -> &str {
        self.0.to_str().unwrap_or_default()
    }

    fn governor_of(&self, index: usize) -> String {
        std::fs::read_to_string(
            self.0
                .join("sys/devices/system/cpu")
                .join(format!("cpu{index}"))
                .join("cpufreq/scaling_governor"),
        )
        .expect("a governor")
        .trim()
        .to_owned()
    }
}

impl Drop for Machine {
    fn drop(&mut self) {
        drop(std::fs::remove_dir_all(&self.0));
    }
}

#[test]
fn it_performs_three_operations_and_refuses_every_other_name() {
    assert_eq!(OPERATIONS.len(), 3);
    let refused = run(&["cpuset", "0-7"]).expect_err("an operation that is not on the list");
    assert_eq!(refused.category(), Category::ConfigInvalid);
    let said = refused.to_string();
    assert!(said.contains("not an operation"), "{said}");
    for (name, _) in OPERATIONS {
        assert!(
            refused
                .context()
                .iter()
                .any(|entry| entry.value.contains(name)),
            "the refusal does not name {name}"
        );
    }
}

#[test]
fn setting_a_governor_says_what_each_processor_was() {
    let machine = Machine::new("set", 4, "performance powersave", "powersave");
    let said = run_under(Path::new(machine.under()), &["governor", "performance"])
        .expect("the fixture permits it");

    for index in 0..4 {
        assert_eq!(machine.governor_of(index), "performance");
        assert!(
            said.iter()
                .any(|line| line.starts_with(&format!("cpu{index}: was powersave"))),
            "cpu{index} did not say what it was: {said:?}"
        );
    }
    assert!(
        said.contains(&"governor: performance".to_owned()),
        "{said:?}"
    );
}

#[test]
fn putting_it_back_is_the_same_operation() {
    let machine = Machine::new("restore", 2, "performance powersave", "powersave");
    run_under(Path::new(machine.under()), &["governor", "performance"]).expect("it sets");
    let back =
        run_under(Path::new(machine.under()), &["governor", "powersave"]).expect("it restores");
    assert_eq!(machine.governor_of(0), "powersave");
    assert!(
        back.iter().any(|line| line.contains("was performance")),
        "{back:?}"
    );

    let again = run_under(Path::new(machine.under()), &["governor", "powersave"])
        .expect("it is asked again");
    assert!(
        again.iter().any(|line| line.contains("already powersave")),
        "a processor that was already there was written to anyway: {again:?}"
    );
}

#[test]
fn a_governor_the_machine_does_not_offer_is_refused_before_anything_is_written() {
    let machine = Machine::new("unknown", 2, "performance powersave", "powersave");
    let refused = run_under(Path::new(machine.under()), &["governor", "conservative"])
        .expect_err("a governor this machine does not offer");
    assert_eq!(refused.category(), Category::ConfigInvalid);
    assert!(
        refused
            .context()
            .iter()
            .any(|entry| entry.key == "offers" && entry.value.contains("powersave")),
        "the refusal does not say what is offered"
    );
    assert_eq!(machine.governor_of(0), "powersave", "it wrote anyway");
}

#[test]
fn a_machine_with_no_governor_says_so() {
    let root = std::env::temp_dir().join(format!("mcf-helper-bare-{}", std::process::id()));
    drop(std::fs::remove_dir_all(&root));
    std::fs::create_dir_all(root.join("sys/devices/system/cpu")).expect("a fixture");
    let refused = run_under(&root, &["governor", "performance"]).expect_err("nothing to set");
    assert_eq!(refused.category(), Category::PlatformMechanismUnavailable);
    drop(std::fs::remove_dir_all(&root));
}

#[test]
fn a_refused_write_partway_through_says_what_was_already_changed() {
    let machine = Machine::new("partial", 3, "performance powersave", "powersave");
    let second = machine
        .0
        .join("sys/devices/system/cpu/cpu1/cpufreq/scaling_governor");
    std::fs::remove_file(&second).expect("it goes");
    std::fs::create_dir(&second).expect("something that cannot be written as a file");

    let refused = run_under(Path::new(machine.under()), &["governor", "performance"])
        .expect_err("cpu1 refuses");
    assert!(
        refused
            .context()
            .iter()
            .any(|entry| entry.key == "already_changed" && entry.value.contains("cpu0=powersave")),
        "the partial outcome does not say what was already changed: {refused}"
    );
}

#[test]
fn energy_is_read_per_domain_and_its_absence_is_said() {
    let machine = Machine::new("energy", 1, "performance", "performance")
        .with_energy(&[("package-0", 123_456), ("dram", 654_321)]);
    let said = run_under(Path::new(machine.under()), &["energy"]).expect("counters");
    assert!(said.contains(&"package-0: 123456".to_owned()), "{said:?}");
    assert!(said.contains(&"dram: 654321".to_owned()), "{said:?}");

    let bare = Machine::new("no-energy", 1, "performance", "performance");
    let refused = run_under(Path::new(bare.under()), &["energy"]).expect_err("no counters");
    assert!(
        matches!(
            refused.category(),
            Category::PlatformMechanismUnavailable | Category::ResourceDiskReadonly
        ),
        "{refused}"
    );
}

#[test]
fn the_accelerator_operation_refuses_without_rights_before_running_anything() {
    let refused = run(&["accelerator", "exclusive", "0"]).expect_err("this test is not root");
    assert_eq!(refused.category(), Category::PlatformPrivilegeDenied);
    assert!(
        refused
            .context()
            .iter()
            .any(|entry| entry.key == "what_to_do"),
        "a refusal with nothing to do about it: {refused}"
    );
}

#[test]
fn arguments_are_refused_rather_than_guessed() {
    for arguments in [
        vec![],
        vec!["governor"],
        vec!["governor", "performance", "and-another"],
        vec!["energy", "please"],
        vec!["accelerator", "exclusive"],
        vec!["accelerator", "sideways", "0"],
        vec!["accelerator", "exclusive", "0; rm -rf /"],
        vec!["governor", "performance", "--under"],
    ] {
        let refused = run(&arguments).expect_err("arguments that name no operation");
        assert_eq!(
            refused.category(),
            Category::ConfigInvalid,
            "{arguments:?} was not refused as invalid"
        );
    }
}

#[test]
fn under_cannot_name_a_file_to_write() {
    let elsewhere =
        std::env::temp_dir().join(format!("mcf-helper-elsewhere-{}", std::process::id()));
    drop(std::fs::remove_file(&elsewhere));
    std::fs::write(&elsewhere, "untouched").expect("a file the helper must not write");

    let refused = run(&[
        "governor",
        "performance",
        "--under",
        elsewhere.to_str().unwrap_or_default(),
    ])
    .expect_err("the shipped program takes no root");
    assert_eq!(refused.category(), Category::ConfigInvalid, "{refused}");
    let refused = run_under(&elsewhere, &["governor", "performance"])
        .expect_err("a root that is a file has no processors under it");
    assert!(
        matches!(
            refused.category(),
            Category::PlatformMechanismUnavailable | Category::ResourceDiskReadonly
        ),
        "{refused}"
    );
    assert_eq!(
        std::fs::read_to_string(&elsewhere).expect("it is still there"),
        "untouched"
    );
    drop(std::fs::remove_file(&elsewhere));
}

#[test]
fn the_helper_reads_no_environment() {
    let source = concat!(include_str!("lib.rs"), include_str!("main.rs"));
    for reader in ["env::var", "env::var_os", "env::vars"] {
        assert!(
            !source.contains(reader),
            "the helper reads the environment through {reader}"
        );
    }
    assert!(source.contains("std::env::args"));
}

fn _unused(_: &Path) {}

#[test]
fn the_shipped_program_refuses_to_be_pointed_elsewhere() {
    let machine = Machine::new("not-the-machine", 1, "performance powersave", "powersave");
    for arguments in [
        vec!["energy", "--under", machine.under()],
        vec!["governor", "performance", "--under", machine.under()],
        vec!["accelerator", "exclusive", "0", "--under", machine.under()],
    ] {
        let refused = run(&arguments).expect_err("a root from an argument is refused");
        assert_eq!(refused.category(), Category::ConfigInvalid);
        assert!(
            refused.to_string().contains("cannot be pointed elsewhere"),
            "the refusal has to say why: {refused}"
        );
    }
}

#[test]
fn being_pointed_elsewhere_is_refused_rather_than_ignored() {
    let machine = Machine::new("ignored-root", 1, "performance powersave", "powersave");
    assert!(
        run(&["energy", "--under", machine.under()]).is_err(),
        "a rebased read must not quietly become a read of this machine"
    );
    let through = run_under(Path::new(machine.under()), &["governor", "performance"]);
    assert!(
        through.is_ok(),
        "the laboratory still drives it against a fixture: {through:?}"
    );
}
