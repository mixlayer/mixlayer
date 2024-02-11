#[derive(Debug, Clone, PartialEq)]
pub enum EdgeChannelState {
    FinishedWriting,
    FinishedReading,
    Running,
}

use std::{
    collections::VecDeque,
    ops::{Deref, DerefMut},
    sync::{Arc, RwLock},
};

use bytes::Bytes;

use crate::Frame;

use super::{InputChannel, OutputChannel};

/// Naive temporary implementation that just pushes/pops from a Vec.
/// ideally for a node with many outputs they share the same Vec and the
/// read maintains a cursor on the shared Vec
#[derive(Clone)]
pub struct InMemoryEdgeChannel {
    edge_id: String,
    buffer: Arc<RwLock<(EdgeChannelState, VecDeque<Frame<Bytes>>)>>,
}

impl InMemoryEdgeChannel {
    pub fn new(edge_id: String) -> Self {
        Self {
            edge_id,
            buffer: Arc::new(RwLock::new((EdgeChannelState::Running, VecDeque::new()))),
        }
    }

    pub fn state(&self) -> EdgeChannelState {
        let guard = self.buffer.read().unwrap();
        let state = &guard.0;
        state.clone()
    }

    pub fn size(&self) -> usize {
        let guard = self.buffer.read().unwrap();
        let (_state, buffer) = guard.deref();

        buffer.len()
    }
}

impl OutputChannel for InMemoryEdgeChannel {
    fn send(&self, data: Frame<Bytes>) -> () {
        let mut guard = self.buffer.write().unwrap();
        let (state, buffer) = guard.deref_mut();

        match state {
            EdgeChannelState::FinishedWriting => {
                warn!(
                    "output ch[{}]: tried to write {:?} after finished writing",
                    self.edge_id, data
                )
            }
            EdgeChannelState::FinishedReading => {
                warn!(
                    "output ch[{}]: tried to write {:?} after finished reading",
                    self.edge_id, data
                )
            }
            EdgeChannelState::Running => match data {
                Frame::Error => (),
                frame @ Frame::Data(_) => buffer.push_back(frame),
                frame @ Frame::End => {
                    buffer.push_back(frame);
                    // println!("input ch[{}]: transition to finish writing", self.edge_id);
                    *state = EdgeChannelState::FinishedWriting
                }
            },
        }
    }
}

impl InputChannel for InMemoryEdgeChannel {
    fn recv(&self) -> Option<Frame<Bytes>> {
        let mut guard = self.buffer.write().unwrap();
        let (state, buffer) = guard.deref_mut();

        let next = match state {
            EdgeChannelState::FinishedWriting | EdgeChannelState::Running => buffer.pop_front(),
            EdgeChannelState::FinishedReading => {
                // println!(
                //     "input ch[{}]: tried to read after finished reading",
                //     self.edge_id
                // );
                None
            }
        };

        if let Some(Frame::End) = next {
            // println!("input ch[{}]: transition to finish reading", self.edge_id);
            *state = EdgeChannelState::FinishedReading;
        }

        next
    }

    //TODO not sure if this is necessary, nodes can tell if stream is finished by End frame
    fn finished(&self) -> bool {
        let guard = self.buffer.read().unwrap();
        let state = &guard.0;

        *state == EdgeChannelState::FinishedWriting
    }
}
