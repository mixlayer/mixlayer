use std::mem;

use crate::{graph::VNode, Frame, Result, VData, VNodeCtx, VTransform};

/// Transforms that accumulates inputs into a batch and
/// then sends the batch to downstream nodes for processing
pub struct BatchXform<I>
where
    I: VData,
{
    batch_size: usize,
    cur_batch: Vec<I>,
}

impl<I> BatchXform<I>
where
    I: VData,
{
    pub fn new(batch_size: usize) -> Self {
        Self {
            batch_size,
            cur_batch: Vec::new(),
        }
    }

    fn send_batch(&mut self, ctx: &mut VNodeCtx) -> Result<()> {
        if !self.cur_batch.is_empty() {
            let batch_to_send =
                mem::replace(&mut self.cur_batch, Vec::with_capacity(self.batch_size));
            self.send(ctx, Frame::Data(batch_to_send))?;
        }

        Ok(())
    }
}

impl<I> VNode for BatchXform<I>
where
    I: VData,
{
    fn tick(&mut self, ctx: &mut VNodeCtx) -> Result<()> {
        if ctx.recv_finished() {
            self.send_batch(ctx)?;
            self.send(ctx, Frame::End)?;
        } else {
            while let Some(next) = self.recv(ctx) {
                if let Frame::Data(data) = next {
                    self.cur_batch.push(data);
                    if self.cur_batch.len() >= self.batch_size {
                        self.send_batch(ctx)?;
                        break; //send at most one batch per tick
                    }
                }
            }
        }

        Ok(())
    }

    fn default_label(&self) -> Option<String> {
        Some(format!("Batch[{}]", self.batch_size))
    }
}

impl<I> VTransform for BatchXform<I>
where
    I: VData,
{
    type Input = I;
    type Output = Vec<I>;
}
