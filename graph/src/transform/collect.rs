use std::marker::PhantomData;

use super::MxlTransform;
use crate::{graph::MxlNode, Frame, Result, MxlData, MxlNodeCtx};

pub struct CollectXform<I>
where
    I: MxlData,
{
    buf: Vec<I>,
    _i: PhantomData<I>,
}

impl<I> CollectXform<I>
where
    I: MxlData,
{
    pub fn new() -> Self {
        CollectXform {
            buf: Vec::new(),
            _i: Default::default(),
        }
    }
}

impl<I> MxlTransform for CollectXform<I>
where
    I: MxlData,
{
    type Input = I;
    type Output = Vec<I>;
}

impl<I> MxlNode for CollectXform<I>
where
    I: MxlData,
{
    fn tick(&mut self, ctx: &mut MxlNodeCtx) -> Result<()> {
        if let Some(next) = self.recv(ctx) {
            match next {
                crate::Frame::Data(data) => {
                    self.buf.push(data);
                }
                _ => (), //TODO
            }
        }

        if ctx.recv_finished() {
            //TODO take from Option<_> so we don't have to clone
            self.send(ctx, Frame::Data(self.buf.clone()))?;
            self.send(ctx, Frame::End)?;
        }

        Ok(())
    }

    fn default_label(&self) -> Option<String> {
        Some("Collect".to_owned())
    }
}
