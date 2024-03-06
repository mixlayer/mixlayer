mod batch;
mod collect;
mod filter;
mod flatten;
mod groupby;
mod map;
mod to_json;

use std::{fmt::Display, marker::PhantomData};

use crate::graph::{MxlNode, MxlNodeCtx};
use crate::{Frame, Result, MxlData};

use anyhow::anyhow;
use serde::Serialize;

pub use self::filter::FilterXform;
pub use self::groupby::GroupByKey;
pub use self::map::{MapXform, TryMapXform};

pub trait MxlTransform: MxlNode {
    type Input: MxlData;
    type Output: MxlData;

    fn recv(&self, ctx: &mut MxlNodeCtx) -> Option<Frame<Self::Input>> {
        if let Some(data) = ctx.recv(0) {
            Some(Self::Input::from_buffer_frame(data))
        } else {
            None
        }
    }

    fn send(&self, ctx: &mut MxlNodeCtx, data: Frame<Self::Output>) -> Result<()> {
        match data {
            Frame::Data(d) => {
                let byte_frame = d
                    .into_buffer_frame()
                    .map_err(|_| anyhow!("error serializing frame"))?;

                ctx.send(0, byte_frame);
            }
            Frame::End => ctx.send(0, Frame::End),
            Frame::Error => ctx.send(0, Frame::Error),
        };

        Ok(())
    }
}

pub struct LowercaseXform;

impl MxlNode for UppercaseXform {
    fn tick(&mut self, ctx: &mut MxlNodeCtx) -> Result<()> {
        match self.recv(ctx) {
            Some(Frame::Data(data)) => self.send(ctx, Frame::Data(data.to_uppercase()))?,
            Some(Frame::End) => self.send(ctx, Frame::End)?,
            _ => (),
        }

        Ok(())
    }
}

impl MxlTransform for UppercaseXform {
    type Input = String;
    type Output = String;
}

pub struct UppercaseXform;

impl MxlNode for LowercaseXform {
    fn tick(&mut self, ctx: &mut MxlNodeCtx) -> Result<()> {
        match self.recv(ctx) {
            Some(Frame::Data(data)) => self.send(ctx, Frame::Data(data.to_lowercase()))?,
            Some(Frame::End) => self.send(ctx, Frame::End)?,
            _ => (),
        }

        Ok(())
    }
}

impl MxlTransform for LowercaseXform {
    type Input = String;
    type Output = String;
}

pub struct CountXform {
    state: u32,
}

impl MxlTransform for CountXform {
    type Input = String; //TODO how can we abstract over any type?
    type Output = u32;
}

impl MxlNode for CountXform {
    fn tick(&mut self, ctx: &mut MxlNodeCtx) -> Result<()> {
        match self.recv(ctx) {
            Some(Frame::Data(_data)) => self.state = self.state + 1,
            Some(Frame::End) => {
                self.send(ctx, Frame::Data(self.state))?;
                self.send(ctx, Frame::End)?;
            }
            _ => (),
        }

        Ok(())
    }
}

pub struct ToStringXform<I: Display + MxlData> {
    _in: PhantomData<I>,
}

impl<I: Display + MxlData> MxlTransform for ToStringXform<I> {
    type Input = I;
    type Output = String;
}

impl<I: Display + MxlData> MxlNode for ToStringXform<I> {
    fn tick(&mut self, ctx: &mut MxlNodeCtx) -> Result<()> {
        let frame = self.recv(ctx);
        match frame {
            Some(Frame::Data(data)) => self.send(ctx, Frame::Data(format!("{}", data)))?,
            Some(Frame::End) => self.send(ctx, Frame::End)?,
            _ => (),
        }

        Ok(())
    }
}

pub struct ToDebugStringXform<I: std::fmt::Debug + MxlData> {
    _in: PhantomData<I>,
}

impl<I: std::fmt::Debug + MxlData> MxlTransform for ToDebugStringXform<I> {
    type Input = I;
    type Output = String;
}

impl<I: std::fmt::Debug + MxlData> MxlNode for ToDebugStringXform<I> {
    fn tick(&mut self, ctx: &mut MxlNodeCtx) -> Result<()> {
        let frame = self.recv(ctx);
        match frame {
            Some(Frame::Data(data)) => self.send(ctx, Frame::Data(format!("{:?}", data)))?,
            Some(Frame::End) => self.send(ctx, Frame::End)?,
            _ => (),
        }

        Ok(())
    }
}

pub fn to_debug<I: std::fmt::Debug + MxlData>() -> ToDebugStringXform<I> {
    ToDebugStringXform {
        _in: Default::default(),
    }
}

pub fn to_string<I: Display + MxlData>() -> ToStringXform<I> {
    ToStringXform {
        _in: Default::default(),
    }
}

pub fn count() -> CountXform {
    CountXform { state: 0 }
}

pub fn group_by_key<K, V>() -> GroupByKey<K, V>
where
    K: MxlData + Eq + std::hash::Hash,
    V: MxlData,
{
    GroupByKey::new()
}

pub fn try_map<I, O, F>(f: F) -> TryMapXform<I, O, F>
where
    I: MxlData,
    O: MxlData,
    F: Fn(I) -> Result<O>,
{
    map::TryMapXform::new(f)
}

pub fn map<I, O, F>(f: F) -> MapXform<I, O, F>
where
    I: MxlData,
    O: MxlData,
    F: Fn(I) -> O,
{
    map::MapXform::new(f)
}

pub fn filter<I, F>(f: F) -> FilterXform<I, F>
where
    I: MxlData,
    F: Fn(&I) -> bool,
{
    filter::FilterXform::new(f)
}

pub fn flatten<I>() -> flatten::FlattenXform<I>
where
    I: MxlData,
{
    flatten::FlattenXform::new()
}

pub fn collect<I>() -> collect::CollectXform<I>
where
    I: MxlData,
{
    collect::CollectXform::new()
}

pub fn to_json<I>() -> to_json::ToJsonXform<I>
where
    I: MxlData + Serialize,
{
    to_json::ToJsonXform::new()
}

pub fn batch<I>(size: usize) -> batch::BatchXform<I>
where
    I: MxlData,
{
    batch::BatchXform::new(size)
}
