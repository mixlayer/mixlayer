use std::marker::PhantomData;

use super::MxlTransform;
use crate::{graph::MxlNode, Frame, Result, MxlData};

pub struct MapXform<I, O, F>
where
    I: MxlData,
    O: MxlData,
    F: Fn(I) -> O,
{
    func: F,
    _i: PhantomData<I>,
    _o: PhantomData<O>,
}

impl<I, O, F> MapXform<I, O, F>
where
    I: MxlData,
    O: MxlData,
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

impl<I, O, F> MxlTransform for MapXform<I, O, F>
where
    I: MxlData,
    O: MxlData,
    F: Fn(I) -> O,
{
    type Input = I;
    type Output = O;
}

impl<I, O, F> MxlNode for MapXform<I, O, F>
where
    I: MxlData,
    O: MxlData,
    F: Fn(I) -> O,
{
    fn tick(&mut self, ctx: &mut crate::graph::MxlNodeCtx) -> Result<()> {
        if let Some(next) = self.recv(ctx) {
            match next {
                Frame::Data(data) => self.send(ctx, Frame::Data((self.func)(data)))?,
                _ => (),
            }
        }

        if ctx.recv_finished() {
            self.send(ctx, Frame::End)?;
        }

        Ok(())
    }

    fn default_label(&self) -> Option<String> {
        Some("Map".to_owned())
    }
}

pub struct TryMapXform<I, O, F>
where
    I: MxlData,
    O: MxlData,
    F: Fn(I) -> Result<O>,
{
    func: F,
    _i: PhantomData<I>,
    _o: PhantomData<O>,
}

impl<I, O, F> TryMapXform<I, O, F>
where
    I: MxlData,
    O: MxlData,
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

impl<I, O, F> MxlTransform for TryMapXform<I, O, F>
where
    I: MxlData,
    O: MxlData,
    F: Fn(I) -> Result<O>,
{
    type Input = I;
    type Output = O;
}

impl<I, O, F> MxlNode for TryMapXform<I, O, F>
where
    I: MxlData,
    O: MxlData,
    F: Fn(I) -> Result<O>,
{
    fn tick(&mut self, ctx: &mut crate::graph::MxlNodeCtx) -> Result<()> {
        if let Some(next) = self.recv(ctx) {
            match next {
                crate::Frame::Data(data) => self.send(ctx, Frame::Data((self.func)(data)?))?,
                _ => (),
            }
        }

        if ctx.recv_finished() {
            self.send(ctx, Frame::End)?;
        }

        Ok(())
    }

    fn default_label(&self) -> Option<String> {
        Some("TryMap".to_owned())
    }
}
