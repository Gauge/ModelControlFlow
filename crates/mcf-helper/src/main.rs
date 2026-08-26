//! The privileged helper, as a program (B-190, D35, §6.32).
//!
//! Everything this binary does is in the library beside it, so that the
//! laboratory can drive the same code the operator's machine runs (D26, A13).
//! What is here is what a program is and a library is not: arguments in, lines
//! out, an exit status.

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
