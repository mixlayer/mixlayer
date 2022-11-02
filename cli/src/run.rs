//! Run subcommand

use anyhow::{Context, Result};
use clap::Args;
use std::path::PathBuf;

use std::fs;

#[derive(Debug, Args)]
pub struct RunCommand {
    app_path: PathBuf,
}

pub fn handle_run(run: RunCommand) -> Result<()> {
    let _app_wasm_binary = fs::read(&run.app_path)
        .with_context(|| format!("App path {:?} is invalid", &run.app_path))?;

    Ok(())
}
