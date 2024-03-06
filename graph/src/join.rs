use crate::graph::{MxlNode, MxlNodeCtx};
use crate::Result;
use crate::{Frame, MxlData, KV};
use std::marker::PhantomData;

pub const LEFT_INPUT: u32 = 0;
pub const RIGHT_INPUT: u32 = 1;

pub trait VJoin: MxlNode {
    type K: MxlData + PartialEq;
    type LV: MxlData;
    type RV: MxlData;
    type Output: MxlData;

    fn recv_left(&self, ctx: &mut MxlNodeCtx) -> Option<Frame<KV<Self::K, Self::LV>>> {
        if let Some(data) = ctx.recv(LEFT_INPUT) {
            Some(KV::from_buffer_frame(data))
        } else {
            None
        }
    }

    fn recv_right(&self, ctx: &mut MxlNodeCtx) -> Option<Frame<KV<Self::K, Self::RV>>> {
        if let Some(data) = ctx.recv(RIGHT_INPUT) {
            Some(KV::from_buffer_frame(data))
        } else {
            None
        }
    }

    fn send(&self, ctx: &mut MxlNodeCtx, data: Frame<Self::Output>) {
        let data = data.flat_map(|d| d.into_buffer_frame().unwrap());
        ctx.send(0, data);
    }
}

pub struct MxlLeftJoin<K: MxlData, L: MxlData, R: MxlData> {
    _left: PhantomData<L>,
    right_buffer: Vec<KV<K, R>>,
    buffering: bool,
}

impl<K, L, R> VJoin for MxlLeftJoin<K, L, R>
where
    K: MxlData + PartialEq,
    L: MxlData,
    R: MxlData,
{
    type K = K;
    type LV = L;
    type RV = R;
    //TODO should be Option<Self::RV>
    type Output = KV<Self::K, KV<Self::LV, Self::RV>>;
}

impl<K, L, R> MxlNode for MxlLeftJoin<K, L, R>
where
    K: MxlData + PartialEq,
    L: MxlData,
    R: MxlData,
{
    fn tick(&mut self, ctx: &mut MxlNodeCtx) -> Result<()> {
        if self.buffering {
            match self.recv_right(ctx) {
                Some(frame) => match frame {
                    Frame::Error => panic!("left join error"),
                    Frame::Data(right_kv) => self.right_buffer.push(right_kv),
                    Frame::End => self.buffering = false,
                },
                None => (),
            }
        }

        if !self.buffering {
            match self.recv_left(ctx) {
                Some(Frame::Data(left)) => {
                    for right in self.match_left_in_buffer(&left) {
                        let out_kv =
                            KV(left.key().clone(), KV(left.value().clone(), right.clone()));

                        let out_kv = Frame::Data(out_kv);
                        self.send(ctx, out_kv)
                    } //FIXME emit a None frame if no matches
                }
                Some(Frame::End) => self.send(ctx, Frame::End),
                _ => (),
            }
        }

        Ok(())
    }

    fn default_label(&self) -> Option<String> {
        None
    }
}

impl<K, L, R> MxlLeftJoin<K, L, R>
where
    K: MxlData + PartialEq,
    L: MxlData,
    R: MxlData,
{
    pub fn new() -> Self {
        Self {
            _left: Default::default(),
            right_buffer: Vec::new(),
            buffering: true,
        }
    }

    fn match_left_in_buffer<'a>(&'a self, left: &'a KV<K, L>) -> impl Iterator<Item = &'a R> + 'a {
        self.right_buffer
            .iter()
            .filter(|kv| kv.key() == left.key())
            .map(|kv| kv.value())
    }
}
