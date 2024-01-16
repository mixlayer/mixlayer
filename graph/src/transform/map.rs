use std::marker::PhantomData;

use super::VTransform;
use crate::{graph::VNode, Frame, Result, VData};

pub struct MapXform<I, O, F>
where
    I: VData,
    O: VData,
    F: Fn(I) -> O,
{
    func: F,
    _i: PhantomData<I>,
    _o: PhantomData<O>,
}

impl<I, O, F> MapXform<I, O, F>
where
    I: VData,
    O: VData,
    F: Fn(I) -> O,
{
    pub fn new(func: F) -> Self {
        MapXform {
            func,
            _i: Default::default(),
            _o: Default::default(),
        }
    }
}

impl<I, O, F> VTransform for MapXform<I, O, F>
where
    I: VData,
    O: VData,
    F: Fn(I) -> O,
{
    type Input = I;
    type Output = O;
}

impl<I, O, F> VNode for MapXform<I, O, F>
where
    I: VData,
    O: VData,
    F: Fn(I) -> O,
{
    fn tick(&mut self, ctx: &mut crate::graph::VNodeCtx) -> Result<()> {
        if let Some(next) = self.recv(ctx) {
            match next {
                crate::Frame::Data(data) => self.send(ctx, Frame::Data((self.func)(data)))?,
                crate::Frame::Error => (),
                crate::Frame::End => self.send(ctx, Frame::End)?,
            }
        }

        Ok(())
    }

    fn default_label(&self) -> Option<String> {
        Some("Map".to_owned())
    }
}

pub struct TryMapXform<I, O, F>
where
    I: VData,
    O: VData,
    F: Fn(I) -> Result<O>,
{
    func: F,
    _i: PhantomData<I>,
    _o: PhantomData<O>,
}

impl<I, O, F> TryMapXform<I, O, F>
where
    I: VData,
    O: VData,
    F: Fn(I) -> Result<O>,
{
    pub fn new(func: F) -> Self {
        TryMapXform {
            func,
            _i: Default::default(),
            _o: Default::default(),
        }
    }
}

impl<I, O, F> VTransform for TryMapXform<I, O, F>
where
    I: VData,
    O: VData,
    F: Fn(I) -> Result<O>,
{
    type Input = I;
    type Output = O;
}

impl<I, O, F> VNode for TryMapXform<I, O, F>
where
    I: VData,
    O: VData,
    F: Fn(I) -> Result<O>,
{
    fn tick(&mut self, ctx: &mut crate::graph::VNodeCtx) -> Result<()> {
        if let Some(next) = self.recv(ctx) {
            match next {
                crate::Frame::Data(data) => self.send(ctx, Frame::Data((self.func)(data)?))?,
                crate::Frame::Error => (),
                crate::Frame::End => self.send(ctx, Frame::End)?,
            }
        }

        Ok(())
    }

    fn default_label(&self) -> Option<String> {
        Some("TryMap".to_owned())
    }
}
