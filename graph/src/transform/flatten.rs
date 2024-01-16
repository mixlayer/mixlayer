use std::marker::PhantomData;

use super::VTransform;
use crate::{graph::VNode, Frame, Result, VData};

pub struct FlattenXform<I>
where
    I: VData,
{
    _i: PhantomData<I>,
}

impl<I> FlattenXform<I>
where
    I: VData,
{
    pub fn new() -> Self {
        FlattenXform {
            _i: Default::default(),
        }
    }
}

impl<I> VTransform for FlattenXform<I>
where
    I: VData,
{
    type Input = Vec<I>;
    type Output = I;
}

impl<I> VNode for FlattenXform<I>
where
    I: VData,
{
    fn tick(&mut self, ctx: &mut crate::graph::VNodeCtx) -> Result<()> {
        if let Some(next) = self.recv(ctx) {
            match next {
                crate::Frame::Error => (), //TODO
                crate::Frame::Data(data) => {
                    for d in data {
                        self.send(ctx, Frame::Data(d))?
                    }
                }
                crate::Frame::End => self.send(ctx, Frame::End)?,
            }
        }

        Ok(())
    }

    fn default_label(&self) -> Option<String> {
        Some("Flatten".to_owned())
    }
}
