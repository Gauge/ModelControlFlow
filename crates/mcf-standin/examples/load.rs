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
