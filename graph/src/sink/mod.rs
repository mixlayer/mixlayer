use crate::graph::{MxlNode, MxlNodeCtx};
use crate::{Frame, MxlData};

pub trait MxlSink: MxlNode {
    type Input: MxlData;

    fn recv(&self, ctx: &mut MxlNodeCtx) -> Option<Frame<Self::Input>> {
        if let Some(data) = ctx.recv(0) {
            Some(Self::Input::from_buffer_frame(data))
        } else {
            None
        }
    }
}
