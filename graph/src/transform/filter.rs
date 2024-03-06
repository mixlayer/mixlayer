use std::marker::PhantomData;

use super::MxlTransform;
use crate::{graph::MxlNode, Frame, Result, MxlData};

pub struct FilterXform<I, F>
where
    I: MxlData,
    F: Fn(&I) -> bool,
{
    func: F,
    _i: PhantomData<I>,
}

impl<I, F> FilterXform<I, F>
where
    I: MxlData,
    F: Fn(&I) -> bool,
{
    pub fn new(func: F) -> Self {
        FilterXform {
            func,
            _i: Default::default(),
        }
    }
}

impl<I, F> MxlTransform for FilterXform<I, F>
where
    I: MxlData,
    F: Fn(&I) -> bool,
{
    type Input = I;
    type Output = I;
}

impl<I, F> MxlNode for FilterXform<I, F>
where
    I: MxlData,
    F: Fn(&I) -> bool,
{
    fn tick(&mut self, ctx: &mut crate::graph::MxlNodeCtx) -> Result<()> {
        if let Some(next) = self.recv(ctx) {
            match next {
                crate::Frame::Data(data) => {
                    if (self.func)(&data) {
                        self.send(ctx, Frame::Data(data))?
                    }
                }
                _ => (),
            }
        }

        if ctx.recv_finished() {
            self.send(ctx, Frame::End)?;
        }

        Ok(())
    }

    fn default_label(&self) -> Option<String> {
        Some("Filter".to_owned())
    }
}
