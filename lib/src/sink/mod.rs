mod fs;

pub use fs::FsLineSink;
use log::debug;

use anyhow::Result;
use std::fmt::Debug;
use std::marker::PhantomData;
use valence_data::Frame;
use valence_graph::{VData, VNode, VNodeCtx, VSink};

pub struct DebugSink<V: Debug + VData> {
    _v: PhantomData<V>,
}

impl<V: Debug + VData> VSink for DebugSink<V> {
    type Input = V;
}

impl<V: Debug + VData> VNode for DebugSink<V> {
    fn tick(&mut self, ctx: &mut VNodeCtx) -> Result<()> {
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

impl<V: Debug + VData> DebugSink<V> {
    pub fn new() -> Self {
        Self {
            _v: Default::default(),
        }
    }
}
