use crate::graph::VSource;
use crate::graph::{VNode, VNodeCtx};
use crate::io::{VFile, VFileMode};
use crate::Frame;
use crate::Result;
use std::io::{self, BufRead};
use std::path::{Path, PathBuf};

/// Reads lines from a file on the local filesystem
pub struct FsLineSource {
    lines: Option<io::Lines<io::BufReader<VFile>>>,

    path: PathBuf,

    //TODO should be unnecessary beacuse node should not tick if edge is finished writing.
    done: bool,
}

impl FsLineSource {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        Ok(Self {
            lines: None,
            path: path.as_ref().to_owned(),
            done: false,
        })
    }
}

impl VSource for FsLineSource {
    type Output = String;
}

impl VNode for FsLineSource {
    fn tick(&mut self, ctx: &mut VNodeCtx) -> Result<()> {
        if !self.done {
            if let Some(lines) = self.lines.as_mut() {
                let next_line = lines.next();

                match next_line {
                    Some(Ok(line)) => self.send(ctx, Frame::Data(line))?,
                    Some(Err(_err)) => self.send(ctx, Frame::Error)?,
                    None => {
                        self.done = true;
                        self.send(ctx, Frame::End)?;
                    }
                }
            } else {
                let file = VFile::open(&self.path, VFileMode::Read)?;
                let reader = io::BufReader::new(file);
                let lines = reader.lines();
                self.lines = Some(lines);
            }
        }

        Ok(())
    }

    fn default_label(&self) -> Option<String> {
        Some(format!("{}", self.path.display()))
    }
}
