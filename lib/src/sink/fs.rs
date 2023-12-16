use crate::graph::{VNode, VNodeCtx, VSink};
use crate::io::VFile;
use crate::Frame;
use anyhow::Result;
use log::error;
use std::io::Write;
use std::path::Path;

pub struct FsLineSink {
    file: VFile,
}

impl FsLineSink {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        Ok(Self {
            file: VFile::open(path, crate::io::VFileMode::Write)?,
        })
    }
}

impl VNode for FsLineSink {
    fn tick(&mut self, ctx: &mut VNodeCtx) -> () {
        let next = self.recv(ctx);

        match next {
            Some(Frame::Data(data)) => {
                match self
                    .file
                    .write_all(data.as_bytes())
                    .and_then(|_| self.file.write_all("\n".as_bytes()))
                {
                    Ok(_) => (),
                    Err(err) => error!("error writing: {}", err),
                }
            }
            _ => (),
        }
    }
}

impl VSink for FsLineSink {
    type Input = String;
}
