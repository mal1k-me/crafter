// Crafter - Local CodeCrafters CLI
// Main entry point

use clap::Parser;

mod cli;

use cli::args::Cli;

fn main() {
    let cli = Cli::parse();
    std::process::exit(cli::runtime::run(cli));
}


