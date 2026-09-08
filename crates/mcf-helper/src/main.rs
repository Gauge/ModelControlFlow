use mcf_helper::run;

fn main() -> std::process::ExitCode {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    let borrowed: Vec<&str> = arguments.iter().map(String::as_str).collect();
    match run(&borrowed) {
        Ok(said) => {
            for line in said {
                println!("{line}");
            }
            std::process::ExitCode::SUCCESS
        }
        Err(failure) => {
            eprintln!("mcf-helper: {failure}");
            for entry in failure.context() {
                eprintln!("  {}: {}", entry.key, entry.value);
            }
            std::process::ExitCode::FAILURE
        }
    }
}
