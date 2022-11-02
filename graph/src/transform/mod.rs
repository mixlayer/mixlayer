mod groupby;
mod map;

use std::{fmt::Display, marker::PhantomData};

use crate::graph::{VNode, VNodeCtx};
use crate::{Frame, VData};

use self::groupby::GroupByKey;
use self::map::MapXform;

pub trait VTransform: VNode {
    type Input: VData;
    type Output: VData;

    fn recv(&mut self, ctx: &mut VNodeCtx) -> Option<Frame<Self::Input>> {
        if let Some(data) = ctx.recv(0) {
            Some(Self::Input::from_buffer_frame(data))
        } else {
            None
        }
    }

    fn send(&mut self, ctx: &mut VNodeCtx, data: Frame<Self::Output>) -> () {
        let data = data.flat_map(|d| d.into_buffer_frame().unwrap());
        ctx.send(0, data);
    }
}

pub struct LowercaseXform;

impl VNode for UppercaseXform {
    fn tick(&mut self, ctx: &mut VNodeCtx) -> () {
        match self.recv(ctx) {
            Some(Frame::Data(data)) => self.send(ctx, Frame::Data(data.to_uppercase())),
            Some(Frame::End) => self.send(ctx, Frame::End),
            _ => (),
        }
    }
}

impl VTransform for UppercaseXform {
    type Input = String;
    type Output = String;
}

pub struct UppercaseXform;

impl VNode for LowercaseXform {
    fn tick(&mut self, ctx: &mut VNodeCtx) -> () {
        match self.recv(ctx) {
            Some(Frame::Data(data)) => self.send(ctx, Frame::Data(data.to_lowercase())),
            Some(Frame::End) => self.send(ctx, Frame::End),
            _ => (),
        }
    }
}

impl VTransform for LowercaseXform {
    type Input = String;
    type Output = String;
}

pub struct CountXform {
    state: u32,
}

impl VTransform for CountXform {
    type Input = String; //TODO how can we abstract over any type?
    type Output = u32;
}

impl VNode for CountXform {
    fn tick(&mut self, ctx: &mut VNodeCtx) -> () {
        match self.recv(ctx) {
            Some(Frame::Data(_data)) => self.state = self.state + 1,
            Some(Frame::End) => {
                self.send(ctx, Frame::Data(self.state));
                self.send(ctx, Frame::End)
            }
            _ => (),
        }
    }
}

pub struct ToStringXform<I: Display + VData> {
    _in: PhantomData<I>,
}

impl<I: Display + VData> VTransform for ToStringXform<I> {
    type Input = I;
    type Output = String;
}

impl<I: Display + VData> VNode for ToStringXform<I> {
    fn tick(&mut self, ctx: &mut VNodeCtx) -> () {
        let frame = self.recv(ctx);
        match frame {
            Some(Frame::Data(data)) => self.send(ctx, Frame::Data(format!("{}", data))),
            Some(Frame::End) => self.send(ctx, Frame::End),
            _ => (),
        }
    }
}

pub struct ToDebugStringXform<I: std::fmt::Debug + VData> {
    _in: PhantomData<I>,
}

impl<I: std::fmt::Debug + VData> VTransform for ToDebugStringXform<I> {
    type Input = I;
    type Output = String;
}

impl<I: std::fmt::Debug + VData> VNode for ToDebugStringXform<I> {
    fn tick(&mut self, ctx: &mut VNodeCtx) -> () {
        let frame = self.recv(ctx);
        match frame {
            Some(Frame::Data(data)) => self.send(ctx, Frame::Data(format!("{:?}", data))),
            Some(Frame::End) => self.send(ctx, Frame::End),
            _ => (),
        }
    }
}

pub fn to_debug<I: std::fmt::Debug + VData>() -> ToDebugStringXform<I> {
    ToDebugStringXform {
        _in: Default::default(),
    }
}

pub fn to_string<I: Display + VData>() -> ToStringXform<I> {
    ToStringXform {
        _in: Default::default(),
    }
}

pub fn count() -> CountXform {
    CountXform { state: 0 }
}

pub fn group_by_key<K, V>() -> GroupByKey<K, V>
where
    K: VData + Eq + std::hash::Hash,
    V: VData,
{
    GroupByKey::new()
}

pub fn map<I, O, F>(f: F) -> MapXform<I, O, F>
where
    I: VData,
    O: VData,
    F: Fn(I) -> O,
{
    map::MapXform::new(f)
}
