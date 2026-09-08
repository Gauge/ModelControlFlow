use mcf_core::failure::Category;

use crate::scenario::{Outcome, Scenario};
use crate::world::World;

pub(super) const PRIVILEGE_DENIED: Scenario = Scenario {
    id: "platform/privilege-denied",
    produces: Category::PlatformPrivilegeDenied,
    summary: "an operation needing elevation is refused before anything is attempted",
    run: privilege_denied,
};

pub(super) const MECHANISM_UNAVAILABLE: Scenario = Scenario {
    id: "platform/mechanism-unavailable",
    produces: Category::PlatformMechanismUnavailable,
    summary: "a machine that publishes no frequency governor says so, rather than failing",
    run: mechanism_unavailable,
};

pub(super) const PRIVILEGE_UNAVAILABLE: Scenario = Scenario {
    id: "platform/privilege-unavailable",
    produces: Category::PlatformPrivilegeUnavailable,
    summary: "the vendor's tool an operation needs is not on this machine",
    run: privilege_unavailable,
};

fn privilege_denied(_world: &World) -> Outcome {
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
    Outcome::Produced(mcf_helper::no_vendor_tool("nvidia-smi"))
}
