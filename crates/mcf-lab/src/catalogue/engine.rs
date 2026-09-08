use mcf_core::engine::{Run, StandIn};
use mcf_core::failure::Category;

use crate::scenario::{Outcome, Scenario};
use crate::world::World;

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

    match marked.degradation().causes().first() {
        Some(failure) => Outcome::Produced(failure.clone()),
        None => Outcome::Unexpected("a stand-in result carried no mark".to_owned()),
    }
}

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

pub(super) const ENGINE_SPAWN_REFUSED: Scenario = Scenario {
    id: "engine/spawn-refused",
    produces: Category::EngineSpawnRefused,
    summary: "an engine the platform will not start — a directory where a program should be \
              — is refused with the platform's reason, and told apart from one that is absent",
    run: engine_spawn_refused,
};

fn engine_spawn_refused(world: &World) -> Outcome {
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

pub(super) const SERVER_NEVER_LISTENS: Scenario = Scenario {
    id: "engine/server-never-listens",
    produces: Category::EngineHangNoOutput,
    summary: "a provisioned server that starts and never begins answering is given up on with \
              how long it was waited for, and is told apart from one that died",
    run: server_never_listens,
};

fn server_never_listens(world: &World) -> Outcome {
    let prefix = world.scratch().join("llama.cpp@cccccccccccc");
    let bin = prefix.join("build").join("bin");
    if std::fs::create_dir_all(&bin).is_err() {
        return Outcome::Unexpected("the fixture prefix could not be made".to_owned());
    }
    let server = bin.join("llama-server");
    if std::fs::write(&server, b"#!/bin/sh\nexec sleep 5\n").is_err() {
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
        component: "llama.cpp".to_owned(),
    };
    for attempt in 0..20 {
        let waited = mcf_serve::served::Served::start_within(
            &llama,
            &world.scratch().join("no-such-model.gguf"),
            world.scratch(),
            3,
            0,
            4096,
            None,
            mcf_serve::declared::Started::default(),
        );
        match waited {
            Ok(_) => {
                return Outcome::Unexpected(
                    "a server that binds nothing was called ready".to_owned(),
                );
            }
            Err(failure) if failure.category() == Category::EngineSpawnRefused && attempt < 19 => {}
            Err(failure) => return Outcome::Produced(failure),
        }
    }
    Outcome::Unexpected("the fixture server could not be started at all".to_owned())
}

pub(super) const SERVER_ANSWER_UNREADABLE: Scenario = Scenario {
    id: "engine/server-answer-unreadable",
    produces: Category::EngineProtocolMalformed,
    summary: "an answer from the provisioned server that is not a generation — not JSON, or an \
              error wearing a completion's shape — is refused rather than read as a model that \
              said nothing",
    run: server_answer_unreadable,
};

fn server_answer_unreadable(_world: &World) -> Outcome {
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

pub(super) const NOTHING_TO_CROSS_CHECK: Scenario = Scenario {
    id: "engine/nothing-to-cross-check",
    produces: Category::ProbeInconclusive,
    summary: "an engine that produced no tokens gives a cross-check nothing to read, which is \
              *could not tell* and never *the engines disagree*",
    run: nothing_to_cross_check,
};

fn nothing_to_cross_check(_world: &World) -> Outcome {
    match mcf_serve::crosscheck::against(b"not a model either", &[1], &[], None) {
        Err(failure) => Outcome::Produced(failure),
        Ok(_) => {
            Outcome::Unexpected("a comparison against no tokens was called an agreement".to_owned())
        }
    }
}

pub(super) const CLIENT_LEFT_MIDSTREAM: Scenario = Scenario {
    id: "engine/client-left-midstream",
    produces: Category::LabInterrupted,
    summary: "a request whose client has gone is closed rather than run to its end for nobody, \
              and the closing says who left",
    run: client_left_midstream,
};

fn client_left_midstream(world: &World) -> Outcome {
    let socket = world.path("engine.sock");
    let Ok(listener) = std::os::unix::net::UnixListener::bind(&socket) else {
        return Outcome::Unexpected("the fixture engine could not listen".to_owned());
    };
    let Ok((near, far)) = std::os::unix::net::UnixStream::pair() else {
        return Outcome::Unexpected("the client pair could not be made".to_owned());
    };
    drop(far);
    std::thread::scope(|scope| {
        let engine = scope.spawn(move || {
            if let Ok((mut connection, _)) = listener.accept() {
                let mut sink = Vec::new();
                let _read = std::io::Read::read_to_end(&mut connection, &mut sink);
            }
        });
        let waiting = mcf_serve::served::Waiting {
            client: Some(&near),
            ..mcf_serve::served::Waiting::NOBODY
        };
        let outcome = match mcf_serve::served::asked_while(
            &socket,
            std::path::Path::new("a-model.gguf"),
            "POST",
            "/completion",
            Some("{}"),
            waiting,
        ) {
            Err(failure) if failure.category() == Category::LabInterrupted => {
                Outcome::Produced(failure)
            }
            Err(failure) => Outcome::Unexpected(format!(
                "the closed request was reported as {} rather than as the client leaving",
                failure.category().code()
            )),
            Ok(_) => Outcome::Unexpected(
                "an engine that never answered was read as answering".to_owned(),
            ),
        };
        let _joined = engine.join();
        outcome
    })
}
