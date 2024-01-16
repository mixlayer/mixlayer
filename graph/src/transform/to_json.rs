use std::marker::PhantomData;

use serde::Serialize;
use valence_data::JsonObject;

use super::VTransform;
use crate::{graph::VNode, Frame, Result, VData};

pub struct ToJsonXform<I>
where
    I: VData + Serialize,
{
    _i: PhantomData<I>,
}

impl<I> ToJsonXform<I>
where
    I: VData + Serialize,
{
    pub fn new() -> Self {
        ToJsonXform {
            _i: Default::default(),
        }
    }
}

impl<I> VTransform for ToJsonXform<I>
where
    I: VData + Serialize,
{
    type Input = I;
    type Output = JsonObject;
}

impl<I> VNode for ToJsonXform<I>
where
    I: VData + Serialize,
{
    fn tick(&mut self, ctx: &mut crate::graph::VNodeCtx) -> Result<()> {
        if let Some(next) = self.recv(ctx) {
            match next {
                crate::Frame::Data(data) => {
                    let json = serde_json::to_value(data)?;
                    let json_obj = json.try_into()?;
                    self.send(ctx, Frame::Data(json_obj))?
                }
                crate::Frame::Error => (),
                crate::Frame::End => self.send(ctx, Frame::End)?,
            }
        }

        Ok(())
    }

    fn default_label(&self) -> Option<String> {
        Some("ToJson".to_owned())
    }
}
