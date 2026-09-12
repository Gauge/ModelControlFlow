
fn main() -> std::process::ExitCode {
    let Some(path) = std::env::args().nth(1) else {
        eprintln!("usage: write-fixture <path>");
        return std::process::ExitCode::FAILURE;
    };
    match std::fs::write(&path, mcf_standin::fixture::a_model_that_runs()) {
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
