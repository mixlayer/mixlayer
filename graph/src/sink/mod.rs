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
