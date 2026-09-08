use super::{Characterization, Machine, Missing, Reading, Route as _, routes};
use crate::attested::Attested;
use crate::measurement::Bytes;

#[test]
fn the_thread_count_matches_the_kernel() {
    let machine = Machine::read();
    let Attested::Known(threads) = machine.processor.threads else {
        return;
    };
    let available = std::thread::available_parallelism().map_or(0, std::num::NonZeroUsize::get);
    assert!(
        u64::from(threads) >= available as u64,
        "the profile reports {threads} threads and the runtime sees {available}"
    );
}

#[test]
fn reading_a_machine_never_fails_and_never_invents() {
    let machine = Machine::read();
    let rendered = machine.to_string();
    assert!(rendered.contains("processor:"), "{rendered}");
    assert!(rendered.contains("memory:"), "{rendered}");
    assert!(rendered.contains("power profile:"), "{rendered}");
    if let Attested::Known(total) = machine.memory.total {
        assert!(total > Bytes(0), "the machine reports no memory at all");
    }
}

#[test]
fn available_memory_does_not_exceed_total() {
    let memory = Machine::read().memory;
    if let (Attested::Known(total), Attested::Known(available)) = (memory.total, memory.available) {
        assert!(available <= total, "{available} available of {total} total");
    }
}

#[test]
fn the_verdict_agrees_with_the_reading() {
    for device in Machine::read().accelerators {
        let reading = device.reading();
        match device.characterization() {
            Characterization::Characterized => {
                assert!(reading.model.is_known(), "{device}");
                assert!(reading.driver.is_known(), "{device}");
                assert!(reading.memory_available.is_known(), "{device}");
                assert!(reading.temperature_c.is_known(), "{device}");
                assert!(reading.missing().is_empty(), "{device}");
            }
            Characterization::AttemptedUncharacterized { missing } => {
                assert!(!missing.is_empty(), "uncharacterized and missing nothing");
                assert_eq!(missing, reading.missing());
            }
        }
        assert!(!device.routes().is_empty(), "a device nothing reported");
    }
}

#[test]
fn routes_that_disagree_are_reported() {
    for device in Machine::read().accelerators {
        assert!(
            device.disagreements().is_empty(),
            "two routes disagree about {:?} on {device}",
            device.disagreements()
        );
    }
}

#[test]
fn a_machine_with_no_accelerator_says_so() {
    let machine = Machine::read();
    if machine.accelerators.is_empty() {
        assert!(machine.to_string().contains("accelerators: none present"));
        assert!(machine.every_accelerator_is_characterized());
    }
}

#[test]
fn every_route_declares_its_coverage() {
    let routes = routes();
    assert!(!routes.is_empty(), "no route is compiled in");
    for route in &routes {
        assert!(!route.name().is_empty());
        assert!(
            !route.covers().is_empty(),
            "{} claims to supply nothing",
            route.name()
        );
        for reading in route.probe() {
            let supplied = supplied_by(&reading);
            for what in supplied {
                assert!(
                    route.covers().contains(&what),
                    "{} supplied {what}, which it does not claim to cover",
                    route.name()
                );
            }
        }
    }
}

fn supplied_by(reading: &Reading) -> Vec<Missing> {
    Missing::ALL
        .into_iter()
        .filter(|what| !reading.missing().contains(what))
        .collect()
}

#[test]
fn merging_fills_gaps_and_never_overwrites() {
    let first = Reading {
        model: Attested::Known("first".to_owned()),
        ..Reading::nothing_known()
    };
    let second = Reading {
        model: Attested::Known("second".to_owned()),
        temperature_c: Attested::Known(41),
        ..Reading::nothing_known()
    };
    let merged = first.clone().filled_from(&second);
    assert_eq!(merged.model, Attested::Known("first".to_owned()));
    assert_eq!(merged.temperature_c, Attested::Known(41));
    assert_eq!(first.disagreements_with(&second), ["model"]);
}

#[test]
fn a_reading_that_knows_nothing_is_missing_everything() {
    assert_eq!(Reading::nothing_known().missing(), Missing::ALL.to_vec());
}

#[test]
fn silence_is_not_disagreement() {
    let known = Reading {
        model: Attested::Known("a device".to_owned()),
        ..Reading::nothing_known()
    };
    assert!(
        known
            .disagreements_with(&Reading::nothing_known())
            .is_empty()
    );
}

#[test]
fn a_machine_with_no_route_reports_no_accelerator_and_stays_complete() {
    let machine = Machine::read_through(&[]);
    assert!(machine.accelerators.is_empty());
    assert!(machine.every_accelerator_is_characterized());

    let rendered = machine.to_string();
    assert!(
        rendered.contains("accelerators: none present"),
        "{rendered}"
    );
    assert!(rendered.contains("processor:"), "{rendered}");
    assert!(rendered.contains("memory:"), "{rendered}");
    assert!(rendered.contains("load, one minute:"), "{rendered}");
}

#[test]
fn the_seam_is_a_parameter_and_changes_nothing_ambient() {
    let without = Machine::read_through(&[]);
    let with = Machine::read();
    assert!(without.accelerators.is_empty());
    assert_eq!(
        with.accelerators.len(),
        Machine::read().accelerators.len(),
        "asking about a different machine changed this one"
    );
}

#[test]
fn a_radeon_is_read_from_the_files_its_driver_writes() {
    let root = std::env::temp_dir().join(format!("mcf-amdgpu-{}", std::process::id()));
    let _cleared = std::fs::remove_dir_all(&root);
    let drm = root.join("drm");
    let device = drm.join("card0").join("device");
    std::fs::create_dir_all(device.join("uma")).expect("a fake card");
    std::fs::create_dir_all(device.join("hwmon").join("hwmon3")).expect("its sensor");
    std::fs::create_dir_all(drm.join("card0-DP-1")).expect("a connector");
    std::fs::create_dir_all(drm.join("renderD128")).expect("a render node");
    let driver = root.join("drivers").join("amdgpu");
    std::fs::create_dir_all(&driver).expect("a fake driver");
    std::os::unix::fs::symlink(&driver, device.join("driver")).expect("the driver link");
    for (name, text) in [
        ("vendor", "0x1002\n"),
        ("device", "0x1586\n"),
        ("mem_info_vram_total", "536870912\n"),
        ("mem_info_vram_used", "440705024\n"),
        ("mem_info_gtt_total", "101139496960\n"),
        ("mem_info_gtt_used", "450560000\n"),
    ] {
        std::fs::write(device.join(name), text).expect(name);
    }
    std::fs::write(
        device.join("hwmon").join("hwmon3").join("temp1_input"),
        "72000\n",
    )
    .expect("a temperature");
    std::fs::write(root.join("osrelease"), "6.18.35-test\n").expect("a kernel");
    std::fs::write(
        root.join("pci.ids"),
        "# a table\n1002  Advanced Micro Devices, Inc. [AMD/ATI]\n\t1586  Strix Halo [Radeon 8060S Graphics]\n\t\t1022 1099  A board\n10de  NVIDIA\n\t1586  Not this one\n",
    )
    .expect("the table");
    std::fs::create_dir_all(root.join("icd.d")).expect("a loader table");
    std::fs::write(root.join("icd.d").join("radeon_icd.json"), "{}").expect("the driver entry");

    let route = super::route_amdgpu::Amdgpu {
        drm,
        pci_ids: root.join("pci.ids"),
        icds: root.join("icd.d"),
        kernel: root.join("osrelease"),
    };
    let readings = route.probe();
    assert_eq!(
        readings.len(),
        1,
        "one card, not its connectors: {readings:?}"
    );
    let reading = readings.first().expect("the card");
    assert_eq!(reading.vendor, Attested::Known("AMD".to_owned()));
    assert_eq!(
        reading.model,
        Attested::Known("Strix Halo [Radeon 8060S Graphics]".to_owned())
    );
    assert_eq!(
        reading.driver,
        Attested::Known("amdgpu, kernel 6.18.35-test".to_owned())
    );
    assert!(reading.runtime.is_known(), "the loader lists its driver");
    assert_eq!(
        reading.memory_total,
        Attested::Known(Bytes(101_139_496_960)),
        "a chip that carves out reports the pool it may address"
    );
    assert_eq!(
        reading.memory_available,
        Attested::Known(Bytes(101_139_496_960 - 450_560_000))
    );
    assert_eq!(reading.temperature_c, Attested::Known(72));
    assert!(reading.missing().is_empty(), "characterized: {reading:?}");

    let unnamed = super::route_amdgpu::Amdgpu {
        pci_ids: root.join("no-table"),
        ..route
    };
    let reading = unnamed.probe();
    assert_eq!(
        reading.first().map(|held| held.model.clone()),
        Some(Attested::Known("PCI device 0x1002:0x1586".to_owned()))
    );
    let _cleared = std::fs::remove_dir_all(&root);
}

#[test]
fn two_vendors_at_index_nought_are_two_devices() {
    struct Fixed(&'static str, Vec<Reading>);
    impl super::Route for Fixed {
        fn name(&self) -> &'static str {
            self.0
        }
        fn covers(&self) -> &'static [Missing] {
            &[Missing::Identity]
        }
        fn probe(&self) -> Vec<Reading> {
            self.1.clone()
        }
    }
    let of = |vendor: &str| Reading {
        vendor: Attested::Known(vendor.to_owned()),
        model: Attested::Known(format!("{vendor} card")),
        ..Reading::nothing_known()
    };
    let machine = Machine::read_through(&[
        Box::new(Fixed("nvidia-files", vec![of("NVIDIA")])),
        Box::new(Fixed("nvidia-library", vec![of("NVIDIA")])),
        Box::new(Fixed("amdgpu-files", vec![of("AMD")])),
    ]);
    assert_eq!(machine.accelerators.len(), 2, "{:?}", machine.accelerators);
    let first = machine.accelerators.first().expect("the NVIDIA card");
    assert_eq!(first.routes(), ["nvidia-files", "nvidia-library"]);
    assert!(first.disagreements().is_empty(), "{first}");
    let second = machine.accelerators.get(1).expect("the Radeon");
    assert_eq!(second.index(), 1);
    assert_eq!(second.routes(), ["amdgpu-files"]);
}
