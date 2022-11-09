mod fs;

pub use fs::FsLineSink;

use std::fmt::Debug;
use std::marker::PhantomData;
use valence_data::Frame;
use valence_graph::{VData, VNode, VNodeCtx, VSink};

use crate::vlog;

pub struct DebugSink<V: Debug + VData> {
    _v: PhantomData<V>,
}

impl<V: Debug + VData> VSink for DebugSink<V> {
    type Input = V;
}

impl<V: Debug + VData> VNode for DebugSink<V> {
    fn tick(&mut self, ctx: &mut VNodeCtx) -> () {
        if let Some(next) = self.recv(ctx) {
            match next {
                Frame::Error => vlog!("sink error"),
                Frame::Data(d) => vlog!("frame: {:#?}", d),
                Frame::End => vlog!("stream ended"),
            }
        }
    }
}

impl<V: Debug + VData> DebugSink<V> {
    pub fn new() -> Self {
        Self {
            _v: Default::default(),
        }
    }
}
