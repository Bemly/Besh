// Bemly_ Shell - A Unix shell written in Rust with libc

mod builtin;
mod common_shell;
mod environment;
mod error;
mod history;
mod job_control;
mod parser;
mod process;
mod shell;
mod signal;
mod terminal;

use shell::run_shell;
use common_shell::main as common_shell;

fn main() {
    // Parse command line arguments
    let args: Vec<String> = std::env::args().skip(1).collect();

    match args.len() {
        0 => {
            // Interactive mode - use libc shell
            if let Err(e) = run_shell(args) {
                eprintln!("besh: {}", e);
                std::process::exit(1);
            }
        }
        _ => match args.get(0).unwrap().to_lowercase().trim() {
            // Help
            "-h" | "--help" => {
                println!("BESH - Bemly's Shell");
                println!();
                println!("Usage:");
                println!("  besh                    Enter interactive shell");
                println!("  besh <command [args]>  Execute a single command");
                println!("  besh <script.besh>      Execute a script file");
                println!();
                println!("Options:");
                println!("  -h, --help     Show this help message");
                println!("  -v, --version  Show version information");
                println!("  -u, --use-libc Use libc-based shell (default)");
            }
            // Version
            "-v" | "--version" => {
                println!("BESH version 26.4.14");
                println!("Written by Bemly_. 2026.04.14");
            }
            // Execute single command
            _ => {
                if let Err(e) = run_shell(args) {
                    eprintln!("besh: {}", e);
                    std::process::exit(1);
                }
            }
        },
    }
}