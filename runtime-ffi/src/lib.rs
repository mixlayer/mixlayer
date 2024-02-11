pub mod protos {
    include!(concat!(env!("OUT_DIR"), "/valence.rs"));
}

mod buffer;

use std::collections::HashMap;

pub use prost;

pub use buffer::{ByteBuffer, FFIMessage};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub enum NodeState {
    Running,
    Finished,
}

#[derive(Debug, Clone, Serialize)]
pub enum EdgeState {
    Running,
    Finished,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct GraphRunState {
    pub nodes: HashMap<u32, NodeState>,
    pub edges: HashMap<String, EdgeState>,
}
