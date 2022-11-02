// use clap::Parser;
use anyhow::Result;
use clap::{Parser, Subcommand};

mod run;

#[derive(Debug, Parser)]
#[command(name = "valence")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    Run(run::RunCommand),
}

fn main() -> Result<()> {
    let args = Cli::parse();

    match args.command {
        Commands::Run(run) => run::handle_run(run)?,
    }

    Ok(())
}
