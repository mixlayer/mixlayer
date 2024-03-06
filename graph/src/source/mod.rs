use std::collections::VecDeque;

use crate::graph::{MxlNode, MxlNodeCtx};
use crate::{Frame, Result, MxlData};

pub trait MxlSource: MxlNode {
    type Output: MxlData;

    fn send(&mut self, ctx: &mut MxlNodeCtx, data: Frame<Self::Output>) -> Result<()> {
        //FIXME remove unwrap
        let data = data.flat_map(|d| d.into_buffer_frame().unwrap());
        ctx.send(0, data);
        Ok(())
    }
}

pub struct VecSource<V: MxlData> {
    data: VecDeque<V>,
    finished: bool,
}

impl<V: MxlData> MxlSource for VecSource<V> {
    type Output = V;
}

impl<V: MxlData> MxlNode for VecSource<V> {
    fn tick(&mut self, ctx: &mut MxlNodeCtx) -> Result<()> {
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

impl<V: MxlData> VecSource<V> {
    pub fn new(data: Vec<V>) -> Self {
        Self {
            data: data.into(),
            finished: false,
        }
    }
}

pub fn vec_source<V: MxlData>(data: Vec<V>) -> VecSource<V> {
    VecSource::new(data)
}
