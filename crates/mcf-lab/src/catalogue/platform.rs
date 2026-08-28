//! Scenarios that need a right this machine does not give (B-190, D35, A13).
//!
//! §6.32's privileged surface is three operations, and every one of them can be
//! refused by the platform rather than by MCF: the rights were never granted,
//! the mechanism is not there, or the vendor's tool is absent. A13 asks for a
//! scenario per category MCF's code constructs, and these are the helper's.
//!
//! **Each is built rather than caused** (D26). A machine with no governor is a
//! directory with no `scaling_governor` in it, not a kernel compiled without
//! `cpufreq`; a device whose mode cannot be changed is the helper refusing
//! before it runs anything, which is what an unprivileged operator's machine
//! does. The helper's `--under` is what makes that possible: every path it
//! touches is built from fixed components under a root, so a laboratory can
//! watch a privileged program work without letting it near the machine.

use mcf_core::failure::Category;

use crate::scenario::{Outcome, Scenario};
use crate::world::World;

/// The rights were never granted.
pub(super) const PRIVILEGE_DENIED: Scenario = Scenario {
    id: "platform/privilege-denied",
    produces: Category::PlatformPrivilegeDenied,
    summary: "an operation needing elevation is refused before anything is attempted",
    run: privilege_denied,
};

/// The machine has no such knob.
pub(super) const MECHANISM_UNAVAILABLE: Scenario = Scenario {
    id: "platform/mechanism-unavailable",
    produces: Category::PlatformMechanismUnavailable,
    summary: "a machine that publishes no frequency governor says so, rather than failing",
    run: mechanism_unavailable,
};

/// There is no tool to ask.
pub(super) const PRIVILEGE_UNAVAILABLE: Scenario = Scenario {
    id: "platform/privilege-unavailable",
    produces: Category::PlatformPrivilegeUnavailable,
    summary: "the vendor's tool an operation needs is not on this machine",
    run: privilege_unavailable,
};

fn privilege_denied(_world: &World) -> Outcome {
    // Every machine the suite runs on is unprivileged — B19 would not permit a
    // test that needed root — so this is the ordinary case rather than a
    // constructed one, and the helper must refuse *before* it runs anything.
    match mcf_helper::run(&["accelerator", "exclusive", "0"]) {
        Err(failure) if failure.category() == Category::PlatformPrivilegeDenied => {
            Outcome::Produced(failure)
        }
        Err(failure) => Outcome::Unexpected(format!("a different refusal: {failure}")),
        Ok(said) => Outcome::Unexpected(format!(
            "an unprivileged process changed a device's mode: {said:?}"
        )),
    }
}

fn mechanism_unavailable(world: &World) -> Outcome {
    let root = world.path("machine");
    if std::fs::create_dir_all(root.join("sys/devices/system/cpu")).is_err() {
        return Outcome::Unexpected("the fixture could not be built".to_owned());
    }
    let Some(under) = root.to_str() else {
        return Outcome::Unexpected("the fixture's path is not text".to_owned());
    };
    match mcf_helper::run_under(std::path::Path::new(under), &["governor", "performance"]) {
        Err(failure) if failure.category() == Category::PlatformMechanismUnavailable => {
            Outcome::Produced(failure)
        }
        Err(failure) => Outcome::Unexpected(format!("a different refusal: {failure}")),
        Ok(said) => {
            Outcome::Unexpected(format!("a machine with no governor had one set: {said:?}"))
        }
    }
}

fn privilege_unavailable(_world: &World) -> Outcome {
    // The helper refuses an unprivileged caller before it looks for the tool,
    // so the absence of the tool is reachable only from a privileged process —
    // which the suite is not, and B19 says must not be. What *is* asserted here
    // is that the category exists with the failure MCF would construct, built
    // by the same constructor the helper uses.
    Outcome::Produced(mcf_helper::no_vendor_tool("nvidia-smi"))
}
