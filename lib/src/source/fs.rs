use crate::graph::VSource;
use crate::graph::{VNode, VNodeCtx};
use crate::io::{VFile, VFileMode};
use crate::Frame;
use anyhow::Result;
use std::io::{self, BufRead};
use std::path::Path;

/// Reads lines from a file on the local filesystem
pub struct FsLineSource {
    lines: io::Lines<io::BufReader<VFile>>,

    //TODO should be unnecessary beacuse node should not tick if edge is finished writing.
    done: bool,
}

impl FsLineSource {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        let file = VFile::open(path, VFileMode::Read)?;
        let reader = io::BufReader::new(file);
        let lines = reader.lines();

        Ok(Self { lines, done: false })
    }
}

impl VSource for FsLineSource {
    type Output = String;
}

impl VNode for FsLineSource {
    fn tick(&mut self, ctx: &mut VNodeCtx) -> () {
        if !self.done {
            let next_line = self.lines.next();

            match next_line {
                Some(Ok(line)) => self.send(ctx, Frame::Data(line)),
                Some(Err(_err)) => self.send(ctx, Frame::Error),
                None => {
                    self.done = true;
                    self.send(ctx, Frame::End)
                }
            }
        }
    }
}
