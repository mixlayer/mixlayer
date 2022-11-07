//! Run subcommand

use anyhow::{Context, Result};
use clap::Args;
use std::path::PathBuf;
use valence_runtime::VSession;

use std::fs;

#[derive(Debug, Args)]
pub struct RunCommand {
    app_path: PathBuf,
}

pub fn handle_run(run: RunCommand) -> Result<()> {
    let app_wasm_binary = fs::read(&run.app_path)
        .with_context(|| format!("App path {:?} is invalid", &run.app_path))?;

    let mut session = VSession::new(&app_wasm_binary)?;

    // let graph = session.export_graph()?;
    // dbg!(graph);

    session.run()?;

    Ok(())
}
