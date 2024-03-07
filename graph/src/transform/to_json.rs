use std::marker::PhantomData;

use serde::Serialize;
use mixlayer_data::JsonObject;

use super::MxlTransform;
use crate::{graph::MxlNode, Frame, Result, MxlData};

pub struct ToJsonXform<I>
where
    I: MxlData + Serialize,
{
    _i: PhantomData<I>,
}

impl<I> ToJsonXform<I>
where
    I: MxlData + Serialize,
{
    pub fn new() -> Self {
        ToJsonXform {
            _i: Default::default(),
        }
    }
}

impl<I> MxlTransform for ToJsonXform<I>
where
    I: MxlData + Serialize,
{
    type Input = I;
    type Output = JsonObject;
}

impl<I> MxlNode for ToJsonXform<I>
where
    I: MxlData + Serialize,
{
    fn tick(&mut self, ctx: &mut crate::graph::MxlNodeCtx) -> Result<()> {
        if let Some(next) = self.recv(ctx) {
            match next {
                crate::Frame::Data(data) => {
                    let json = serde_json::to_value(data)?;
                    let json_obj = json.try_into()?;
                    self.send(ctx, Frame::Data(json_obj))?
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
        Some("ToJson".to_owned())
    }
}
