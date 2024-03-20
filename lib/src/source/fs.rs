use crate::graph::MxlSource;
use crate::graph::{MxlNode, MxlNodeCtx};
use crate::io::{MxlFile, MxlFileMode};
use crate::Frame;
use crate::Result;
use std::io::{self, BufRead};
use std::path::{Path, PathBuf};

/// Reads lines from a file on the local filesystem
pub struct FsLineSource {
    lines: Option<io::Lines<io::BufReader<MxlFile>>>,

    path: PathBuf,

    //TODO should be unnecessary because node should not tick if edge is finished writing.
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

impl MxlSource for FsLineSource {
    type Output = String;
}

impl MxlNode for FsLineSource {
    fn tick(&mut self, ctx: &mut MxlNodeCtx) -> Result<()> {
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
                let file = MxlFile::open(&self.path, MxlFileMode::Read)?;
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

/// Reads an entire file as a utf8 string and emits it as a frame
pub struct FsStringSource {
    path: Option<PathBuf>,
}

impl FsStringSource {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        Ok(Self {
            path: Some(path.as_ref().to_owned()),
        })
    }
}

impl MxlSource for FsStringSource {
    type Output = String;
}

impl MxlNode for FsStringSource {
    fn tick(&mut self, ctx: &mut MxlNodeCtx) -> Result<()> {
        if let Some(path) = self.path.take() {
            let mut file = MxlFile::open(&path, MxlFileMode::Read)?;
            let contents: String = std::io::read_to_string(&mut file)?;

            self.send(ctx, Frame::Data(contents))?;
            self.send(ctx, Frame::End)?;
        }

        Ok(())
    }

    fn default_label(&self) -> Option<String> {
        self.path.as_ref().map(|path| format!("{}", path.display()))
    }
}
