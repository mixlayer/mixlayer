mod fs;

pub use fs::FsLineSink;
use log::debug;

use anyhow::Result;
use std::fmt::Debug;
use std::marker::PhantomData;
use mixlayer_data::Frame;
use mixlayer_graph::{MxlData, MxlNode, MxlNodeCtx, MxlSink};

pub struct DebugSink<V: Debug + MxlData> {
    _v: PhantomData<V>,
}

impl<V: Debug + MxlData> MxlSink for DebugSink<V> {
    type Input = V;
}

impl<V: Debug + MxlData> MxlNode for DebugSink<V> {
    fn tick(&mut self, ctx: &mut MxlNodeCtx) -> Result<()> {
        if let Some(next) = self.recv(ctx) {
            match next {
                Frame::Error => debug!("sink error"),
                Frame::Data(d) => debug!("frame: {:#?}", d),
                Frame::End => debug!("single input finished"),
            }

            if ctx.recv_finished() {
                debug!("all inputs finished");
            }
        }

        Ok(())
    }

    fn default_label(&self) -> Option<String> {
        Some("Debug".to_owned())
    }
}

impl<V: Debug + MxlData> DebugSink<V> {
    pub fn new() -> Self {
        Self {
            _v: Default::default(),
        }
    }
}
