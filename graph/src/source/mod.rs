use std::collections::VecDeque;

use crate::graph::{VNode, VNodeCtx};
use crate::{Frame, Result, VData};

pub trait VSource: VNode {
    type Output: VData;

    fn send(&mut self, ctx: &mut VNodeCtx, data: Frame<Self::Output>) -> Result<()> {
        //FIXME remove unwrap
        let data = data.flat_map(|d| d.into_buffer_frame().unwrap());
        ctx.send(0, data);
        Ok(())
    }
}

pub struct VecSource<V: VData> {
    data: VecDeque<V>,
    finished: bool,
}

impl<V: VData> VSource for VecSource<V> {
    type Output = V;
}

impl<V: VData> VNode for VecSource<V> {
    fn tick(&mut self, ctx: &mut VNodeCtx) -> Result<()> {
        if !self.finished {
            if let Some(next) = self.data.pop_back() {
                self.send(ctx, Frame::Data(next))?;
            } else {
                self.finished = true;
                self.send(ctx, Frame::End)?;
            }
        }

        Ok(())
    }
}

impl<V: VData> VecSource<V> {
    pub fn new(data: Vec<V>) -> Self {
        Self {
            data: data.into(),
            finished: false,
        }
    }
}

pub fn vec_source<V: VData>(data: Vec<V>) -> VecSource<V> {
    VecSource::new(data)
}
