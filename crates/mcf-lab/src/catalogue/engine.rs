//! Scenarios in which no vendored engine will run the artifact.
//!
//! D31: MCF ships a stand-in so that a model no vendored engine runs still
//! runs — marked, behaviour-class only, and never reporting a speed (B65). The
//! failure MCF claims to handle here is `engine.unavailable`, and A13 requires
//! it be reproducible.
//!
//! **What is simulated is the observation** (D26). MCF cannot conjure an
//! artifact no engine supports, and does not need to: what it observes in that
//! case is a run that fell through to the stand-in and a result that carries
//! the mark saying so. That is what this constructs.

use mcf_core::engine::{Run, StandIn};
use mcf_core::failure::Category;

use crate::scenario::{Outcome, Scenario};
use crate::world::World;

/// The vendored engine will not run it, so the stand-in did.
pub(super) const NO_VENDORED_ENGINE: Scenario = Scenario {
    id: "engine/no-vendored-engine",
    produces: Category::EngineUnavailable,
    summary: "an artifact no vendored engine runs falls to the stand-in, and every \
              result taken there is marked",
    run: no_vendored_engine,
};

fn no_vendored_engine(_world: &World) -> Outcome {
    let stand_in: Run<StandIn> = Run::at_build("mcf-0.1.0-m0");
    let marked = stand_in.mark(stand_in.behaviour("the tool call parsed"));

    // A5: the mark is what makes the result usable. A stand-in result with no
    // mark would be a corrupted result, so the scenario produces the mark's own
    // cause — which is the classified failure MCF is claiming to handle.
    match marked.degradation().causes().first() {
        Some(failure) => Outcome::Produced(failure.clone()),
        None => Outcome::Unexpected("a stand-in result carried no mark".to_owned()),
    }
}

/// The supervision contract, at every stage a process can die in (B-033, §3.1).
///
/// Each scenario hands `supervise` a command that dies a particular way, and
/// what comes back is the classified failure MCF claims to handle. The engine
/// is a shell rather than a model because the contract is about the process,
/// not the weights (D26: the observable is the exit, and that is what is
/// built).
pub(super) const ENGINE_NOT_FOUND: Scenario = Scenario {
    id: "engine/spawn-not-found",
    produces: Category::EngineSpawnNotFound,
    summary: "an engine whose program is not there is refused by name, not searched for",
    run: engine_not_found,
};

fn engine_not_found(_world: &World) -> Outcome {
    let mut command = std::process::Command::new("/nonexistent/mcf-engine-that-is-not-there");
    match mcf_serve::adapters::supervise(&mut command, &mut |_chunk| {}) {
        Err(failure) => Outcome::Produced(failure),
        Ok(_) => Outcome::Unexpected("a program that does not exist ran".to_owned()),
    }
}

/// Dies before saying anything.
pub(super) const ENGINE_EXIT_IMMEDIATE: Scenario = Scenario {
    id: "engine/exit-immediate",
    produces: Category::EngineExitImmediate,
    summary: "an engine that exits before producing anything is an aborted answer with its own \
              words attached",
    run: engine_exit_immediate,
};

fn engine_exit_immediate(_world: &World) -> Outcome {
    let mut command = std::process::Command::new("sh");
    command.args(["-c", "echo 'the model would not load' >&2; exit 3"]);
    match mcf_serve::adapters::supervise(&mut command, &mut |_chunk| {}) {
        Err(failure) => Outcome::Produced(failure),
        Ok(_) => Outcome::Unexpected("an engine that exited 3 was read as success".to_owned()),
    }
}

/// Dies after part of an answer.
pub(super) const ENGINE_EXIT_MIDSTREAM: Scenario = Scenario {
    id: "engine/exit-midstream",
    produces: Category::EngineExitMidstream,
    summary: "an engine that dies after part of an answer leaves a partial answer, kept, and a \
              failure that says how much",
    run: engine_exit_midstream,
};

fn engine_exit_midstream(_world: &World) -> Outcome {
    let mut command = std::process::Command::new("sh");
    command.args(["-c", "printf 'Paris is'; exit 4"]);
    let mut received = Vec::new();
    match mcf_serve::adapters::supervise(&mut command, &mut |chunk| {
        received.extend_from_slice(chunk);
    }) {
        Err(failure) if received == b"Paris is" => Outcome::Produced(failure),
        Err(_) => {
            Outcome::Unexpected("the partial answer was not delivered before the death".to_owned())
        }
        Ok(_) => Outcome::Unexpected("an engine that exited 4 was read as success".to_owned()),
    }
}

/// Killed by a signal.
pub(super) const ENGINE_EXIT_SIGNAL: Scenario = Scenario {
    id: "engine/exit-signal",
    produces: Category::EngineExitSignal,
    summary: "an engine the platform kills is told apart from one that chose to exit",
    run: engine_exit_signal,
};

fn engine_exit_signal(_world: &World) -> Outcome {
    let mut command = std::process::Command::new("sh");
    command.args(["-c", "kill -9 $$"]);
    match mcf_serve::adapters::supervise(&mut command, &mut |_chunk| {}) {
        Err(failure) => Outcome::Produced(failure),
        Ok(_) => Outcome::Unexpected("a killed engine was read as success".to_owned()),
    }
}

/// A program that is there and cannot be run: refused, not "not found".
pub(super) const ENGINE_SPAWN_REFUSED: Scenario = Scenario {
    id: "engine/spawn-refused",
    produces: Category::EngineSpawnRefused,
    summary: "an engine the platform will not start — a directory where a program should be \
              — is refused with the platform's reason, and told apart from one that is absent",
    run: engine_spawn_refused,
};

fn engine_spawn_refused(world: &World) -> Outcome {
    // A directory is there and is not executable: `spawn` fails with something
    // other than not-found, which is the case this category is for.
    let directory = world.scratch().join("an-engine-that-is-a-directory");
    if std::fs::create_dir_all(&directory).is_err() {
        return Outcome::Unexpected("the scratch directory could not be made".to_owned());
    }
    let mut command = std::process::Command::new(&directory);
    match mcf_serve::adapters::supervise(&mut command, &mut |_chunk| {}) {
        Err(failure) => Outcome::Produced(failure),
        Ok(_) => Outcome::Unexpected("a directory ran as a program".to_owned()),
    }
}

/// Two provisioned pins of one component: the operator's choice, not MCF's.
pub(super) const TWO_ENGINES_PROVISIONED: Scenario = Scenario {
    id: "engine/two-provisioned",
    produces: Category::ConfigConflict,
    summary: "two provisioned builds of the same engine are refused by name rather than chosen \
              between",
    run: two_engines_provisioned,
};

fn two_engines_provisioned(world: &World) -> Outcome {
    let home = world.scratch().join("mcf");
    for pin in ["llama.cpp@aaaaaaaaaaaa", "llama.cpp@bbbbbbbbbbbb"] {
        let bin = home.join("provisioned").join(pin).join("build").join("bin");
        if std::fs::create_dir_all(&bin).is_err()
            || std::fs::write(bin.join("llama-completion"), b"#!/bin/sh\n").is_err()
            || std::fs::write(
                home.join("provisioned")
                    .join(pin)
                    .join("mcf-provenance.json"),
                b"{\"component\":\"llama.cpp\",\"commit\":\"x\"}\n",
            )
            .is_err()
        {
            return Outcome::Unexpected("the fixture prefixes could not be written".to_owned());
        }
    }
    match mcf_serve::adapters::only_one(mcf_serve::adapters::provisioned_llama(&home)) {
        Err(failure) => Outcome::Produced(failure),
        Ok(_) => Outcome::Unexpected("two pins were chosen between".to_owned()),
    }
}

/// A server that binds nothing: started, alive, never ready.
pub(super) const SERVER_NEVER_LISTENS: Scenario = Scenario {
    id: "engine/server-never-listens",
    produces: Category::EngineHangNoOutput,
    summary: "a provisioned server that starts and never begins answering is given up on with \
              how long it was waited for, and is told apart from one that died",
    run: server_never_listens,
};

fn server_never_listens(world: &World) -> Outcome {
    // A "server" that lives and does nothing: the case the readiness wait is
    // for. It is not enough for the process to be absent — that is
    // `engine.spawn.not_found` — nor for it to die, which is
    // `engine.exit.immediate`. It has to be alive and silent.
    let prefix = world.scratch().join("llama.cpp@cccccccccccc");
    let bin = prefix.join("build").join("bin");
    if std::fs::create_dir_all(&bin).is_err() {
        return Outcome::Unexpected("the fixture prefix could not be made".to_owned());
    }
    let server = bin.join("llama-server");
    if std::fs::write(&server, b"#!/bin/sh\nexec sleep 600\n").is_err() {
        return Outcome::Unexpected("the fixture server could not be written".to_owned());
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        if std::fs::set_permissions(&server, std::fs::Permissions::from_mode(0o755)).is_err() {
            return Outcome::Unexpected("the fixture server could not be made runnable".to_owned());
        }
    }

    let llama = mcf_serve::adapters::ProvisionedLlama {
        prefix,
        commit: "cccccccccccc".to_owned(),
    };
    // Three attempts rather than the six hundred a real start is given: the
    // bound is a parameter so that this scenario can exist at all (A13).
    let waited = mcf_serve::served::Served::start_within(
        &llama,
        &world.scratch().join("no-such-model.gguf"),
        world.scratch(),
        3,
    );
    match waited {
        Err(failure) => Outcome::Produced(failure),
        Ok(_) => Outcome::Unexpected("a server that binds nothing was called ready".to_owned()),
    }
}

/// A server whose answer MCF cannot read.
pub(super) const SERVER_ANSWER_UNREADABLE: Scenario = Scenario {
    id: "engine/server-answer-unreadable",
    produces: Category::EngineProtocolMalformed,
    summary: "an answer from the provisioned server that is not a generation — not JSON, or an \
              error wearing a completion's shape — is refused rather than read as a model that \
              said nothing",
    run: server_answer_unreadable,
};

fn server_answer_unreadable(_world: &World) -> Outcome {
    // Two shapes, because the second is the dangerous one: an error carries no
    // `stop_type` and no content, which reads exactly like a model that
    // emitted its end-of-turn token and nothing else — the observation F38
    // turns on. Reading it as that would blame the model for the server.
    let error_shaped =
        mcf_serve::served::interpret("{\"error\":{\"code\":503,\"message\":\"Loading model\"}}");
    if error_shaped.is_ok() {
        return Outcome::Unexpected(
            "an error was read as a generation that said nothing".to_owned(),
        );
    }
    match mcf_serve::served::interpret("this is not JSON at all") {
        Err(failure) => Outcome::Produced(failure),
        Ok(_) => Outcome::Unexpected("text that is not JSON was read as a completion".to_owned()),
    }
}

/// A cross-check with nothing to check against.
pub(super) const NOTHING_TO_CROSS_CHECK: Scenario = Scenario {
    id: "engine/nothing-to-cross-check",
    produces: Category::ProbeInconclusive,
    summary: "an engine that produced no tokens gives a cross-check nothing to read, which is \
              *could not tell* and never *the engines disagree*",
    run: nothing_to_cross_check,
};

fn nothing_to_cross_check(_world: &World) -> Outcome {
    // The bytes are deliberately not a model, and are never reached: the
    // emptiness is noticed first. That ordering is the thing under test as
    // much as the category — a comparison against nothing must not be reported
    // as a comparison that found nothing wrong (A7, D42), and it must not cost
    // a model load to say so (F44).
    match mcf_serve::crosscheck::against(b"not a model either", &[1], &[]) {
        Err(failure) => Outcome::Produced(failure),
        Ok(_) => {
            Outcome::Unexpected("a comparison against no tokens was called an agreement".to_owned())
        }
    }
}
