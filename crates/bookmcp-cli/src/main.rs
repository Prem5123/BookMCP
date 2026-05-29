#![forbid(unsafe_code)]

use clap::Parser;

fn main() {
    if let Err(error) = bookmcp_cli::run(bookmcp_cli::Cli::parse()) {
        eprintln!("error: {error:#}");
        std::process::exit(1);
    }
}
