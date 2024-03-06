use std::collections::HashMap;

pub mod ai;
pub mod io;
pub mod mixdb;
pub mod sink;
pub mod source;

pub use http;
pub use valence_graph as graph;
pub use valence_runtime_ffi::{ByteBuffer, FFIMessage};

pub use graph::{
    Frame, Input, InputChannel, Output, OutputChannel, MxlEdge, MxlGraph, MxlNodeId, MxlNodeRef, MxlNodeType,
};

pub use valence_data::{JsonObject, JsonMxlData, JsonValue};
pub use valence_macros::builder;

pub use anyhow::Result;

use log::error;
use valence_runtime_ffi::protos::{self, VEdgeProto, VGraphProto, VNodeTypeProto};

extern "C" {
    /// Logs a message on the WebAssembly host.
    ///
    /// # Arguments
    ///
    /// * `msg` a `ByteBuffer` containing a utf-8 encoded string to be logged.
    pub fn _valence_log(msg: *const ByteBuffer) -> ();

    /// Sends data into an edge channel.
    ///
    /// # Arguments
    ///
    /// * `id` a `ByteBuffer` containing a protobuf-encoded VEdge
    /// * `data` a `ByteBuffer` containing a `Frame` to be sent
    pub fn _valence_edge_channel_send(id: *const ByteBuffer, data: *const ByteBuffer) -> ();
    pub fn _valence_edge_channel_recv(id: *const ByteBuffer) -> *mut ByteBuffer;
    pub fn _valence_edge_is_finished(id: *const ByteBuffer) -> i32;

    pub fn _valence_unixtime() -> i32;

    pub fn _valence_uuid_v4() -> *mut ByteBuffer;
}

pub fn valence_unixtime() -> i32 {
    unsafe { _valence_unixtime() }
}

pub fn valence_uuid_v4() -> String {
    unsafe {
        let buf = Box::from_raw(_valence_uuid_v4());
        String::from_utf8_unchecked(buf.into_vec())
    }
}

#[macro_export]
macro_rules! vlog {
  () => {
      $crate::vlog!("\n")
  };
  ($($arg:tt)*) => {{
      use $crate::ByteBuffer;
      let s = std::fmt::format(std::format_args!($($arg)*));
      let bb: ByteBuffer = s.into();
      let b = Box::into_raw(Box::new(bb));

      unsafe {
        $crate::_valence_log(b);
      }
  }};
}

pub struct FFIEdgeChannel {
    edge: VEdgeProto,
}

impl FFIEdgeChannel {
    pub fn for_edge(edge: VEdgeProto) -> Self {
        Self { edge }
    }
}

impl OutputChannel for FFIEdgeChannel {
    fn send(&self, data: graph::Frame<valence_runtime_ffi::prost::bytes::Bytes>) -> () {
        let edge_buf: ByteBuffer = FFIMessage(&self.edge).try_into().unwrap();
        let frame_buf: ByteBuffer = data.into_bytes().into();

        unsafe { _valence_edge_channel_send(&edge_buf, &frame_buf) }
    }
}

impl InputChannel for FFIEdgeChannel {
    fn finished(&self) -> bool {
        let edge_buf: ByteBuffer = FFIMessage(&self.edge).try_into().unwrap();
        let is_finished = unsafe { _valence_edge_is_finished(&edge_buf) };
        is_finished > 0
    }

    fn finished_writing(&self) -> bool {
        todo!()
    }

    fn recv(&self) -> Option<graph::Frame<valence_runtime_ffi::prost::bytes::Bytes>> {
        let edge_buf: ByteBuffer = FFIMessage(&self.edge).try_into().unwrap();
        let frame_buf = unsafe {
            let buf = _valence_edge_channel_recv(&edge_buf);

            if buf.is_null() {
                return None;
            }

            Box::from_raw(buf)
        };

        let frame_buf = frame_buf.into_bytes();
        let frame = Frame::from_bytes(frame_buf);

        Some(frame)
    }
}

#[no_mangle]
extern "C" fn _valence_tick_node(graph: *mut MxlGraph, node_id: u32) -> () {
    let graph = unsafe { Box::leak(Box::from_raw(graph)) };

    let inputs = inputs_for_node(&graph, &node_id);
    let outputs = outputs_for_node(&graph, &node_id);

    if let Some(node) = graph.node_mut(&node_id) {
        let mut ctx = graph::MxlNodeCtx::new();

        ctx.inputs = inputs;
        ctx.outputs = outputs;

        //TODO error recovery, classification, retries, etc
        match node.tick(&mut ctx) {
            Ok(_) => (),
            Err(err) => error!("node error: {}", err),
        }
    } else {
        error!("node {} not found", node_id);
    }
}

fn to_edge_proto(ed: &MxlEdge) -> VEdgeProto {
    protos::VEdgeProto {
        source_node_id: ed.source_node_id,
        source_output_port: ed.source_port as u32,
        dest_input_port: ed.dest_port as u32,
        dest_node_id: ed.dest_node_id,
    }
}

/// allows the runtime to free a graph so the Drop traits run on all of the nodes
#[no_mangle]
extern "C" fn _valence_free_graph(graph: *mut MxlGraph) -> () {
    unsafe {
        drop(Box::from_raw(graph));
    }
}

#[no_mangle]
extern "C" fn _valence_export_graph(graph: *mut MxlGraph) -> *const ByteBuffer {
    let graph = unsafe { graph.as_ref().unwrap() };

    let edges: Vec<protos::VEdgeProto> = graph
        .edges()
        .map(|ed| protos::VEdgeProto {
            source_node_id: ed.source_node_id,
            source_output_port: ed.source_port as u32,
            dest_input_port: ed.dest_port as u32,
            dest_node_id: ed.dest_node_id,
        })
        .collect();

    let nodes: HashMap<u32, protos::VNodeInfo> = graph
        .node_ids()
        .map(|node_id| {
            let metadata = graph.node_metadata(&node_id).unwrap();

            let node_type = match metadata.node_type {
                MxlNodeType::Source => VNodeTypeProto::NodeTypeSource,
                MxlNodeType::Transform => VNodeTypeProto::NodeTypeTransform,
                MxlNodeType::Sink => VNodeTypeProto::NodeTypeSink,
                _ => VNodeTypeProto::NodeTypeUnknown,
            };

            let info = protos::VNodeInfo {
                node_id,
                node_type: node_type as i32,
                node_operation: metadata.operation.clone(),
                node_label: metadata.label.clone(),
                input_type: metadata.input_type.clone(),
                output_type: metadata.output_type.clone(),
            };
            (node_id, info)
        })
        .collect();

    let export = VGraphProto {
        metadata: nodes,
        edges,
    };

    let buf = FFIMessage(&export).try_into().unwrap();

    &buf
}

pub fn edge_channel(edge: &MxlEdge) -> FFIEdgeChannel {
    let edge = to_edge_proto(edge);
    FFIEdgeChannel { edge }
}

fn inputs_for_node(graph: &MxlGraph, node_id: &MxlNodeId) -> HashMap<u32, Input> {
    let upstream_edges = graph.upstream_edges(node_id);

    let mut inputs: HashMap<u32, Vec<Box<dyn InputChannel>>> = HashMap::new();

    for edge in upstream_edges {
        let edge_ch: Box<dyn InputChannel> = Box::new(edge_channel(&edge));
        inputs.entry(edge.dest_port).or_insert(vec![]).push(edge_ch);
    }

    inputs
        .into_iter()
        .map(|(k, input_chs)| (k, Input::new(input_chs)))
        .collect()
}

fn outputs_for_node(graph: &MxlGraph, node_id: &MxlNodeId) -> HashMap<u32, Output> {
    let downstream_edges = graph.downstream_edges(node_id);

    let mut outputs: HashMap<u32, Vec<Box<dyn OutputChannel>>> = HashMap::new();

    for edge in downstream_edges {
        let edge_ch: Box<dyn OutputChannel> = Box::new(edge_channel(&edge));

        if !outputs.contains_key(&edge.source_port) {
            outputs.insert(edge.source_port, Vec::new());
        }

        outputs.get_mut(&edge.source_port).unwrap().push(edge_ch);
    }

    outputs
        .into_iter()
        .map(|(k, output_chs)| (k, Output { output_chs }))
        .collect()
}

#[no_mangle]
extern "C" fn _valence_malloc(len: usize) -> *const ByteBuffer {
    let buf = vec![0u8; len];
    Box::into_raw(Box::new(buf.into()))
}
