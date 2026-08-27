//! What loading a model costs, apart from running it (DEC-018, D41).
//!
//! Residency is a decision about whether to pay a load on every request, so the
//! load has to be measured on its own: `mcf run` pays it inside a figure that
//! also holds every forward pass. This reads the file, parses the directory,
//! and dequantizes every tensor — and prints how long each took, on this
//! machine, once. It is a diagnostic and not a benchmark: one reading, no
//! window, no conditions recorded (B65 does not reach it because nothing here
//! is a model's speed, but B20 does, and this claims nothing beyond the line it
//! prints).
//!
//!   cargo run --release -p mcf-standin --example load -- <model.gguf>

fn main() -> std::process::ExitCode {
    let Some(path) = std::env::args().nth(1) else {
        eprintln!("usage: load <model.gguf>");
        return std::process::ExitCode::FAILURE;
    };
    let started = std::time::Instant::now();
    let bytes = match std::fs::read(&path) {
        Ok(bytes) => bytes,
        Err(error) => {
            eprintln!("{path}: {error}");
            return std::process::ExitCode::FAILURE;
        }
    };
    let read = started.elapsed();
    let file = match mcf_standin::gguf::parse(&bytes) {
        Ok(file) => file,
        Err(failure) => {
            eprintln!("{path}: {failure}");
            return std::process::ExitCode::FAILURE;
        }
    };
    let parsed = started.elapsed();
    let model = match mcf_standin::llama::load(&file, &bytes) {
        Ok(model) => model,
        Err(failure) => {
            eprintln!("{path}: {failure}");
            return std::process::ExitCode::FAILURE;
        }
    };
    let loaded = started.elapsed();
    println!(
        "read {:.3} s, parse {:.3} s, dequantize {:.3} s — {} bytes on disk, {} blocks",
        read.as_secs_f64(),
        parsed.saturating_sub(read).as_secs_f64(),
        loaded.saturating_sub(parsed).as_secs_f64(),
        bytes.len(),
        model.shape.blocks
    );
    std::process::ExitCode::SUCCESS
}
