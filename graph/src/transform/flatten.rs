use std::marker::PhantomData;

use super::MxlTransform;
use crate::{graph::MxlNode, Frame, Result, MxlData};

pub struct FlattenXform<I>
where
    I: MxlData,
{
    _i: PhantomData<I>,
}

impl<I> FlattenXform<I>
where
    I: MxlData,
{
    pub fn new() -> Self {
        FlattenXform {
            _i: Default::default(),
        }
    }
}

impl<I> MxlTransform for FlattenXform<I>
where
    I: MxlData,
{
    type Input = Vec<I>;
    type Output = I;
}

impl<I> MxlNode for FlattenXform<I>
where
    I: MxlData,
{
    fn tick(&mut self, ctx: &mut crate::graph::MxlNodeCtx) -> Result<()> {
        if let Some(next) = self.recv(ctx) {
            match next {
                crate::Frame::Data(data) => {
                    for d in data {
                        self.send(ctx, Frame::Data(d))?
                    }
                }
                _ => (), //TODO
            }

            if ctx.recv_finished() {
                self.send(ctx, Frame::End)?;
            }
        }

        Ok(())
    }

    fn default_label(&self) -> Option<String> {
        Some("Flatten".to_owned())
    }
}
