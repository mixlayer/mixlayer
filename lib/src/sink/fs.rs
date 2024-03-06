use crate::graph::{MxlNode, MxlNodeCtx, MxlSink};
use crate::io::MxlFile;
use crate::Frame;
use anyhow::Result;
use std::io::Write;
use std::path::Path;

pub struct FsLineSink {
    file: MxlFile,
}

impl FsLineSink {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        Ok(Self {
            file: MxlFile::open(path, crate::io::MxlFileMode::Write)?,
        })
    }
}

impl MxlNode for FsLineSink {
    fn tick(&mut self, ctx: &mut MxlNodeCtx) -> Result<()> {
        let next = self.recv(ctx);

        match next {
            Some(Frame::Data(data)) => {
                self.file
                    .write_all(data.as_bytes())
                    .and_then(|_| self.file.write_all("\n".as_bytes()))?;
            }
            _ => (),
        }

        Ok(())
    }
}

impl MxlSink for FsLineSink {
    type Input = String;
}
