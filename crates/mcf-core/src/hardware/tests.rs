//! Tests for the hardware profiler.
//!
//! B19 requires the suite pass on a laptop with no accelerator, so every test
//! here has to hold on a machine with one and on a machine without. What is
//! asserted is therefore never *what* the machine is — that varies — but that
//! whatever MCF says about it is well-formed, honest about what it could not
//! read, and consistent between the routes that answered.

use super::{Characterization, Machine, Missing, Reading, Route as _, routes};
use crate::attested::Attested;
use crate::measurement::Bytes;

/// A19: the profiler is checked against an independently known value — the
/// kernel's own accounting, read a second way.
#[test]
fn the_thread_count_matches_the_kernel() {
    let machine = Machine::read();
    let Attested::Known(threads) = machine.processor.threads else {
        // No `/proc`. The suite must still pass, and B19 is why.
        return;
    };
    let available = std::thread::available_parallelism().map_or(0, std::num::NonZeroUsize::get);
    assert!(
        u64::from(threads) >= available as u64,
        "the profile reports {threads} threads and the runtime sees {available}"
    );
}

/// A7: what MCF cannot read stays unknown. Reading a machine never fails and
/// never invents — a machine with no `/proc`, no governor and no accelerator
/// produces a complete profile in which almost everything is `unknown`.
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

/// Available memory never exceeds total. A profile that said otherwise would be
/// a reading nobody could act on, and the two come from different lines of the
/// same file — which is exactly where a parsing error would show.
#[test]
fn available_memory_does_not_exceed_total() {
    let memory = Machine::read().memory;
    if let (Attested::Known(total), Attested::Known(available)) = (memory.total, memory.available) {
        assert!(available <= total, "{available} available of {total} total");
    }
}

/// D25: a device is characterized when all four readings are present, and
/// otherwise names which are missing. Both outcomes are correct; what is
/// asserted is that the verdict and the reading agree.
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

/// A8: two routes that both claim to know a field and disagree is a finding
/// about a route, and it is surfaced rather than resolved by preferring one.
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

/// A9: no accelerator is a result. The profile says so and the machine is
/// still fully described.
#[test]
fn a_machine_with_no_accelerator_says_so() {
    let machine = Machine::read();
    if machine.accelerators.is_empty() {
        assert!(machine.to_string().contains("accelerators: none present"));
        assert!(machine.every_accelerator_is_characterized());
    }
}

/// Every route declares what it can supply, and the declaration is not empty.
/// A21's shape: a route that *should* have supplied a reading and did not is a
/// different thing from one that never claimed to.
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
            // A route may fail to supply something it covers — the driver may
            // be loaded and refuse a query — but it may never supply something
            // it does not claim to.
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

/// Merging fills gaps and never overwrites. A route that ran second must not
/// be able to replace a reading the first one took, because that would hide a
/// disagreement instead of reporting it.
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

/// A reading that knows nothing is missing all four, which is what makes a
/// file-only machine report *attempted, uncharacterized* rather than silently
/// passing D25's bar.
#[test]
fn a_reading_that_knows_nothing_is_missing_everything() {
    assert_eq!(Reading::nothing_known().missing(), Missing::ALL.to_vec());
}

/// An unknown field is never a disagreement. Two routes, one of which did not
/// look, have not contradicted each other.
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

// ---------------------------------------------------------------------------
// B-015 — the seam, and what it lets the suite check on the wrong machine.
// ---------------------------------------------------------------------------

/// B19: *the suite runs on a laptop, offline, with no accelerator.* On a
/// machine that has one, that is otherwise unverifiable — so the seam is used
/// to produce the no-accelerator machine and the profile is checked for
/// honesty rather than for silence.
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
    // Everything else is still reported: a machine with no accelerator is a
    // machine MCF describes fully, not a degraded case it says less about.
    assert!(rendered.contains("processor:"), "{rendered}");
    assert!(rendered.contains("memory:"), "{rendered}");
    assert!(rendered.contains("load, one minute:"), "{rendered}");
}

/// The seam takes routes rather than reading ambient state, so a test can ask
/// a question about a different machine without changing this one. B2: nothing
/// that could change a result is undeclared.
#[test]
fn the_seam_is_a_parameter_and_changes_nothing_ambient() {
    let without = Machine::read_through(&[]);
    let with = Machine::read();
    assert!(without.accelerators.is_empty());
    // Reading through no routes did not disturb the real reading.
    assert_eq!(
        with.accelerators.len(),
        Machine::read().accelerators.len(),
        "asking about a different machine changed this one"
    );
}

/// A Radeon is read from the files its driver writes, laid out here the way
/// the kernel lays them out, so the route is checked on a machine with no
/// such card (B-015).
///
/// The chip carves its memory out of the system's — the driver's `uma` group
/// says so — and the pool a model lands in is then the system memory the
/// card may address, not the carve-out.
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

    // Without the table, the id stands rather than a name MCF made up.
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

/// Two routes each numbering their first device nought are not describing
/// one device when they name different vendors: the first Radeon and the
/// first NVIDIA card are two accelerators, not one with a disagreement on it.
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
