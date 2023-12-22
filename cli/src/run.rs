//! Run subcommand

use anyhow::{Context, Result};
use clap::Args;
use std::path::PathBuf;

use std::fs;

// pub fn create_output_dir(base_path: &PathBuf) -> Result<PathBuf> {
//     use std::time::{SystemTime, UNIX_EPOCH};

//     let unix_secs = SystemTime::now()
//         .duration_since(UNIX_EPOCH)
//         .unwrap()
//         .as_secs();

//     let path: PathBuf = ["out", format!("{}", unix_secs).as_str()].iter().collect();
//     let path = base_path.join(path);

//     std::fs::create_dir_all(&path).with_context(|| "error creating output directory")?;

//     Ok(path.canonicalize()?)
// }

#[derive(Debug, Args)]
pub struct RunCommand {
    app_path: PathBuf,
    base_path: PathBuf,
}

pub fn handle_run(run: RunCommand) -> Result<()> {
    let app_wasm_binary = fs::read(&run.app_path)
        .with_context(|| format!("App path {:?} is invalid", &run.app_path))?;

    // create_output_dir(&run.base_path)?;
    let fs = valence_runtime::LocalFilesystem::new(run.base_path)?;
    // let mut session = VSession::new(&app_wasm_binary, fs, |_l| Ok(()))?;

    // let graph = session.export_graph()?;
    // dbg!(graph);

    // session.run()?;

    Ok(())
}
