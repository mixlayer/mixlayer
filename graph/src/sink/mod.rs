use std::{fmt::Debug, marker::PhantomData};

use crate::graph::{VNode, VNodeCtx};
use crate::{Frame, VData};

pub trait VSink: VNode {
    type Input: VData;

    fn recv(&self, ctx: &mut VNodeCtx) -> Option<Frame<Self::Input>> {
        if let Some(data) = ctx.recv(0) {
            Some(Self::Input::from_buffer_frame(data))
        } else {
            None
        }
    }
}

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
                Frame::Error => println!("sink error"),
                Frame::Data(d) => println!("frame: {:#?}", d),
                Frame::End => println!("stream ended"),
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

// pub fn file_sink<P: AsRef<Path>>(f: P) -> Result<fs::FsLineSink, std::io::Error> {
//     Ok(fs::FsLineSink::new(f)?)
// }
