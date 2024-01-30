use std::marker::PhantomData;

use super::VTransform;
use crate::{graph::VNode, Frame, Result, VData};

pub struct FilterXform<I, F>
where
    I: VData,
    F: Fn(&I) -> bool,
{
    func: F,
    _i: PhantomData<I>,
}

impl<I, F> FilterXform<I, F>
where
    I: VData,
    F: Fn(&I) -> bool,
{
    pub fn new(func: F) -> Self {
        FilterXform {
            func,
            _i: Default::default(),
        }
    }
}

impl<I, F> VTransform for FilterXform<I, F>
where
    I: VData,
    F: Fn(&I) -> bool,
{
    type Input = I;
    type Output = I;
}

impl<I, F> VNode for FilterXform<I, F>
where
    I: VData,
    F: Fn(&I) -> bool,
{
    fn tick(&mut self, ctx: &mut crate::graph::VNodeCtx) -> Result<()> {
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
