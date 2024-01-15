use std::marker::PhantomData;

use super::VTransform;
use crate::{graph::VNode, Frame, VData, VNodeCtx};

pub struct CollectXform<I>
where
    I: VData,
{
    buf: Vec<I>,
    _i: PhantomData<I>,
}

impl<I> CollectXform<I>
where
    I: VData,
{
    pub fn new() -> Self {
        CollectXform {
            buf: Vec::new(),
            _i: Default::default(),
        }
    }
}

impl<I> VTransform for CollectXform<I>
where
    I: VData,
{
    type Input = I;
    type Output = Vec<I>;
}

impl<I> VNode for CollectXform<I>
where
    I: VData,
{
    fn tick(&mut self, ctx: &mut VNodeCtx) -> () {
        if let Some(next) = self.recv(ctx) {
            match next {
                crate::Frame::Error => (), //TODO
                crate::Frame::Data(data) => {
                    self.buf.push(data);
                }
                crate::Frame::End => {
                    //TODO take from Option<_> so we don't have to clone
                    self.send(ctx, Frame::Data(self.buf.clone()));
                    self.send(ctx, Frame::End)
                }
            }
        }
    }

    fn default_label(&self) -> Option<String> {
        Some("Collect".to_owned())
    }
}
