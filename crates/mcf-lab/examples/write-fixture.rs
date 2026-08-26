//! Writes the laboratory's smallest runnable model to a file (B-183, D31).
//!
//! **Why this exists outside the test suite.** `scripts/check-from-scratch.sh`
//! runs MCF in a container holding the binary and nothing else, and B-183's
//! condition is that it *reaches a first token* there. A first token needs a
//! model file, and the container has no toolchain to build one with — so the
//! machine that has a toolchain writes it out, and the container is handed the
//! bytes.
//!
//! **What it is not.** Not a way to ship a model, and not the reference model
//! (B-019): four tokens, one block, an identity embedding — a file that
//! exercises the whole path from bytes to text and says nothing whatever about
//! what a real model does. An example rather than a command, because nothing
//! about it belongs in the artifact an operator installs.
//!
//!   cargo run -p mcf-lab --example write-fixture -- <path>

fn main() -> std::process::ExitCode {
    let Some(path) = std::env::args().nth(1) else {
        eprintln!("usage: write-fixture <path>");
        return std::process::ExitCode::FAILURE;
    };
    match std::fs::write(&path, mcf_lab::fixture::a_model_that_runs()) {
        Ok(()) => {
            println!("{path}");
            std::process::ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("write-fixture: {path} could not be written: {error}");
            std::process::ExitCode::FAILURE
        }
    }
}
