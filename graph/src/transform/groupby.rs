use std::collections::HashMap;
use std::hash::Hash;

use crate::{
    graph::{VNode, VNodeCtx},
    Frame, Result, VData, KV,
};

use super::VTransform;

pub struct GroupByKey<K, V>
where
    K: VData + Eq + Hash,
    V: VData,
{
    buffer: Option<HashMap<K, Vec<V>>>, //FIXME introduce a state enum to get rid of panics in tick logic
    buffering: bool,
}

impl<K, V> GroupByKey<K, V>
where
    K: VData + Eq + Hash,
    V: VData,
{
    pub(crate) fn new() -> Self {
        Self {
            buffer: Some(HashMap::new()),
            buffering: true,
        }
    }
}

impl<K, V> VTransform for GroupByKey<K, V>
where
    K: VData + Eq + Hash,
    V: VData,
{
    type Input = KV<K, V>;
    type Output = KV<K, Vec<V>>;
}

impl<K, V> VNode for GroupByKey<K, V>
where
    K: VData + Eq + Hash,
    V: VData,
{
    fn tick(&mut self, ctx: &mut VNodeCtx) -> Result<()> {
        if self.buffering {
            if let Some(frame) = self.recv(ctx) {
                match frame {
                    crate::Frame::Data(data) => {
                        if let Some(buffer) = self.buffer.as_mut() {
                            let (key, value) = data.into_parts();
                            if !buffer.contains_key(&key) {
                                buffer.insert(key.clone(), Vec::new());
                            }

                            let key_buffer = buffer.get_mut(&key).unwrap();
                            key_buffer.push(value)
                        } else {
                            panic!("invalid buffer state")
                        }
                    }
                    crate::Frame::Error => (),
                    crate::Frame::End => self.buffering = false,
                }
            }
        } else {
            if let Some(mut buffer) = self.buffer.take() {
                for (k, v) in buffer.drain() {
                    self.send(ctx, Frame::Data(KV(k, v)))?;
                }

                self.send(ctx, Frame::End)?;
            }
        }

        Ok(())
    }

    fn default_label(&self) -> Option<String> {
        Some("GroupBy".to_owned())
    }
}
