use std::{
    collections::{HashMap, HashSet},
    marker::PhantomData,
};

use anyhow::Result;
use bytes::Bytes;
use log::error;
use serde::Serialize;
use valence_data::JsonObject;

use crate::{
    transform, Frame, InputChannel, OutputChannel, VData, VLeftJoin, VSink, VSource, VTransform, KV,
};

pub type VNodeId = u32;

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct VEdge {
    pub source_node_id: VNodeId,
    pub source_port: u32,
    pub dest_node_id: VNodeId,
    pub dest_port: u32,
}

pub struct VGraphTopology {
    metadata: HashMap<VNodeId, VNodeMetadata>,
    edges: HashMap<VNodeId, HashMap<VNodeId, HashSet<VEdge>>>,
    source_ids: HashSet<VNodeId>,
}

pub enum VNodeType {
    Source,
    Transform,
    Sink,
    Join,
}

pub struct VNodeMetadata {
    pub operation: String,
    pub label: Option<String>,
    pub node_type: VNodeType,
    pub input_type: String,
    pub output_type: String,
}

pub struct VGraph {
    nodes: HashMap<VNodeId, Box<dyn VNode + Send>>,
    topo: VGraphTopology,
}

impl VGraph {
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            topo: VGraphTopology {
                metadata: HashMap::new(),
                edges: HashMap::new(),
                source_ids: HashSet::new(),
            },
        }
    }

    pub fn edges<'a>(&'a self) -> Box<dyn Iterator<Item = &'a VEdge> + 'a> {
        let it = self
            .topo
            .edges
            .iter()
            .map(|(_, v)| v.iter().map(|(_, v)| v.iter()).flatten())
            .flatten();

        Box::new(it)
    }

    pub fn node_ids<'a>(&'a self) -> Box<dyn Iterator<Item = VNodeId> + 'a> {
        Box::new(self.topo.metadata.keys().map(|k| *k))
    }

    /// Returns any node ids that output directly into the specified node id
    pub fn upstream_edges(&self, node_id: &VNodeId) -> Vec<VEdge> {
        self.topo
            .edges
            .iter()
            .flat_map(|(_k, v)| v.get(node_id))
            .flatten()
            .map(|v| v.clone())
            .collect()
    }

    // Returns all direct edges originating from node_id
    pub fn downstream_edges(&self, node_id: &VNodeId) -> Vec<VEdge> {
        if let Some(edges) = self.topo.edges.get(node_id) {
            edges.iter().map(|(_, es)| es.clone()).flatten().collect()
        } else {
            vec![]
        }
    }

    // TODO would like to return a Set that preserves insertion order, but there isn't one in Rust std
    // TODO this topological sort is probably suboptimal for dataflow, inspect further
    pub fn sort_from_sources(&self) -> Vec<VNodeId> {
        pub fn inner(
            node_ids: &HashSet<VNodeId>,
            edges: &HashMap<VNodeId, HashMap<VNodeId, HashSet<VEdge>>>,
            out: &mut Vec<VNodeId>,
        ) {
            for node_id in node_ids {
                //have to dedupe node ids manually because we aren't using a Set type
                if !out.contains(node_id) {
                    out.push(*node_id);
                }
            }

            for node_id in node_ids {
                match edges.get(node_id) {
                    Some(es) => {
                        let ks: HashSet<VNodeId> = es.keys().map(|k| *k).collect();
                        inner(&ks, edges, out)
                    }
                    None => (),
                }
            }
        }

        let mut out = Vec::new();
        inner(&self.topo.source_ids, &self.topo.edges, &mut out);
        out
    }

    pub fn node_mut(&mut self, node_id: &VNodeId) -> Option<&mut Box<dyn VNode + Send>> {
        self.nodes.get_mut(node_id)
    }

    pub fn node(&self, node_id: &VNodeId) -> Option<&Box<dyn VNode + Send>> {
        self.nodes.get(node_id)
    }

    pub fn node_operation(&self, node_id: &VNodeId) -> Option<&str> {
        self.topo
            .metadata
            .get(node_id)
            .map(|s| s.operation.as_str())
    }

    pub fn node_metadata(&self, node_id: &VNodeId) -> Option<&VNodeMetadata> {
        self.topo.metadata.get(node_id)
    }

    pub fn label(&mut self, node_id: &VNodeId, label: String) -> () {
        if let Some(metadata) = self.topo.metadata.get_mut(node_id) {
            metadata.label = Some(label);
        }
    }

    fn insert_edge(&mut self, edge: VEdge) -> () {
        if self.topo.edges.get(&edge.source_node_id).is_none() {
            self.topo.edges.insert(edge.source_node_id, HashMap::new());
        }

        let edge_map = self.topo.edges.get_mut(&edge.source_node_id).unwrap();

        if edge_map.get(&edge.dest_node_id).is_none() {
            edge_map.insert(edge.dest_node_id, HashSet::new());
        }

        let edge_set = edge_map.get_mut(&edge.dest_node_id).unwrap();

        edge_set.insert(edge);
    }

    pub fn sink<T: VData, N: VSink<Input = T> + Send + 'static>(
        &mut self,
        sink: N,
    ) -> VNodeRef<T, ()> {
        let node_id = self.insert::<N::Input, (), _>(sink, None, None, VNodeType::Sink);

        VNodeRef::<T, ()> {
            node_id,
            _in: Default::default(),
            _out: Default::default(),
        }
    }

    pub fn transform<I: VData, O: VData, N: VTransform<Input = I, Output = O> + Send + 'static>(
        &mut self,
        xform: N,
    ) -> VNodeRef<I, O> {
        let node_id = self.insert::<N::Input, (), _>(xform, None, None, VNodeType::Transform);

        VNodeRef::<I, O> {
            node_id,
            _in: Default::default(),
            _out: Default::default(),
        }
    }

    pub fn insert<I, O, N: VNode + Send + 'static>(
        &mut self,
        node: N,
        upstream_ids: Option<&[(VNodeId, u32, u32)]>,
        label: Option<String>,
        node_type: VNodeType,
    ) -> VNodeId {
        let next_id = self.nodes.len() as u32;

        let node_type_name = std::any::type_name::<N>();

        let label = label.or_else(|| node.default_label());
        let operation = format_node_type(node_type_name);

        //TODO `format_node_type` here will probably truncate useful generic type info
        let input_type = format_node_type(std::any::type_name::<I>());
        let output_type = format_node_type(std::any::type_name::<O>());

        self.nodes.insert(next_id, Box::new(node));

        let metadata = VNodeMetadata {
            label,
            operation,
            node_type,
            input_type,
            output_type,
        };

        self.topo.metadata.insert(next_id, metadata);

        if let Some(upstream_ids) = upstream_ids {
            for (upstream_id, upstream_port, dest_port) in upstream_ids {
                let edge = VEdge {
                    dest_node_id: next_id,
                    dest_port: *dest_port,
                    source_node_id: *upstream_id,
                    source_port: *upstream_port,
                };

                self.insert_edge(edge);
            }
        }

        next_id
    }

    pub fn source<T: VData, S: VSource<Output = T> + Sync + Send + 'static>(
        &mut self,
        source_node: S,
    ) -> VNodeRef<(), T> {
        let node_id = self.insert::<(), S::Output, _>(source_node, None, None, VNodeType::Source);

        self.topo.source_ids.insert(node_id);

        VNodeRef::<(), T> {
            node_id,
            _in: Default::default(),
            _out: Default::default(),
        }
    }

    pub fn left_join<LI, RI, K, LV, RV>(
        &mut self,
        left: &VNodeRef<LI, KV<K, LV>>,
        right: &VNodeRef<RI, KV<K, RV>>,
    ) -> VNodeRef<(KV<K, LV>, KV<K, RV>), KV<K, KV<LV, RV>>>
    where
        K: VData + PartialEq,
        LV: VData,
        RV: VData,
    {
        let join: VLeftJoin<K, LV, RV> = VLeftJoin::new();

        let left_edge = (left.node_id, 0, crate::join::LEFT_INPUT);
        let right_edge = (right.node_id, 0, crate::join::RIGHT_INPUT);

        //TODO figure out how to describe join types in node metadata, using () for now
        let node_id =
            self.insert::<(), (), _>(join, Some(&[left_edge, right_edge]), None, VNodeType::Join);

        VNodeRef {
            node_id,
            _in: Default::default(),
            _out: Default::default(),
        }
    }
}

pub struct VNodeRef<In, Out> {
    node_id: VNodeId,
    _in: PhantomData<In>,
    _out: PhantomData<Out>,
}

impl<In, Out: VData> VNodeRef<In, Out> {
    pub fn id(&self) -> VNodeId {
        self.node_id
    }

    pub fn map<MapO: VData, F: Fn(Out) -> MapO + Sync + Send + 'static>(
        &self,
        g: &mut VGraph,
        f: F,
    ) -> VNodeRef<Out, MapO> {
        self.transform(g, crate::transform::map(f))
    }

    pub fn try_map<MapO: VData, F: Fn(Out) -> Result<MapO> + Sync + Send + 'static>(
        &self,
        g: &mut VGraph,
        f: F,
    ) -> VNodeRef<Out, MapO> {
        self.transform(g, crate::transform::try_map(f))
    }

    //TODO probably put behind a feature so not forced to import serde_json for everyone
    // or separate crate
    pub fn to_json(&self, g: &mut VGraph) -> VNodeRef<Out, JsonObject>
    where
        Out: Serialize + Serialize,
    {
        self.transform(g, crate::transform::to_json())
    }

    pub fn filter<F: Fn(&Out) -> bool + Send + Sync + 'static>(
        &self,
        g: &mut VGraph,
        f: F,
    ) -> VNodeRef<Out, Out> {
        self.transform(g, crate::transform::filter(f))
    }

    pub fn collect(&self, g: &mut VGraph) -> VNodeRef<Out, Vec<Out>> {
        self.transform(g, transform::collect())
    }

    pub fn transform<TO, T: VTransform<Input = Out, Output = TO> + Sync + Send + 'static>(
        &self,
        g: &mut VGraph,
        transform: T,
    ) -> VNodeRef<Out, TO> {
        let transform_id = g.insert::<T::Input, T::Output, _>(
            transform,
            Some(&[(self.node_id, 0, 0)]),
            None,
            VNodeType::Transform,
        );

        VNodeRef {
            node_id: transform_id,
            _in: Default::default(),
            _out: Default::default(),
        }
    }

    pub fn sink<S: VSink<Input = Out> + Sync + Send + 'static>(
        &self,
        g: &mut VGraph,
        sink: S,
    ) -> VNodeRef<Out, ()> {
        let sink_id =
            g.insert::<S::Input, (), _>(sink, Some(&[(self.node_id, 0, 0)]), None, VNodeType::Sink);

        VNodeRef::<Out, ()> {
            node_id: sink_id,
            _in: Default::default(),
            _out: Default::default(),
        }
    }

    pub fn label(self, g: &mut VGraph, label: impl AsRef<str>) -> Self {
        let label = label.as_ref().to_owned();
        g.label(&self.node_id, label);
        self
    }

    //TODO probably just get rid of this in favor of connect()
    pub fn connect_sink(&self, g: &mut VGraph, sink: &VNodeRef<Out, ()>) -> () {
        g.insert_edge(VEdge {
            source_node_id: self.node_id,
            source_port: 0,
            dest_node_id: sink.node_id,
            dest_port: 0,
        })
    }

    pub fn connect<Any>(&self, g: &mut VGraph, next: &VNodeRef<Out, Any>) -> () {
        g.insert_edge(VEdge {
            source_node_id: self.node_id,
            source_port: 0,
            dest_node_id: next.node_id,
            dest_port: 0,
        })
    }

    pub fn batch(&self, g: &mut VGraph, batch_size: usize) -> VNodeRef<Out, Vec<Out>> {
        self.transform(g, transform::batch(batch_size))
    }
}

impl<In, Out: VData> VNodeRef<In, Vec<Out>> {
    pub fn flatten(&self, g: &mut VGraph) -> VNodeRef<Vec<Out>, Out> {
        self.transform(g, transform::flatten())
    }
}

pub trait VNode {
    //TODO probably make this required for node impls
    fn default_label(&self) -> Option<String> {
        None
    }

    fn tick(&mut self, ctx: &mut VNodeCtx) -> Result<(), anyhow::Error>;
}

pub struct VNodeCtx {
    //TODO make private
    pub outputs: HashMap<u32, Output>,
    pub inputs: HashMap<u32, Input>,
}

impl VNodeCtx {
    pub fn new() -> Self {
        Self {
            outputs: HashMap::new(),
            inputs: HashMap::new(),
        }
    }

    pub(crate) fn send(&mut self, output_idx: u32, data: Frame<Bytes>) -> () {
        if let Some(output) = self.outputs.get_mut(&output_idx) {
            output.send(data)
        } else {
            error!("invalid output index :{}", output_idx); //TODO return error
            ()
        }
    }

    pub(crate) fn recv(&mut self, input_idx: u32) -> Option<Frame<Bytes>> {
        if let Some(input) = self.inputs.get_mut(&input_idx) {
            input.recv()
        } else {
            error!("invalid input index"); //TODO return error
            None
        }
    }

    pub fn recv_finished(&self) -> bool {
        self.inputs.values().all(|i| i.finished())
    }
}

pub struct Output {
    pub output_chs: Vec<Box<dyn OutputChannel>>,
}

impl Output {
    pub fn send(&mut self, data: Frame<Bytes>) -> () {
        for ch in self.output_chs.iter_mut() {
            ch.send(data.clone())
        }
    }
}

pub struct Input {
    // input channels associated with this input,
    // there is typically one input channel per edge in the graph
    input_chs: Vec<Box<dyn InputChannel>>,
}

impl Input {
    pub fn new(input_chs: Vec<Box<dyn InputChannel>>) -> Self {
        Self { input_chs }
    }

    pub fn recv(&mut self) -> Option<Frame<Bytes>> {
        //find the first input that has a Frame waiting and return it
        //TODO we might want to implement some kind of round robin/fairness scheme here
        //  but it's made difficult beacuse this state is reconstructed for every call to tick()
        for ch in self.input_chs.iter_mut() {
            if let Some(frame) = ch.recv() {
                return Some(frame);
            }
        }

        None
    }

    pub fn finished(&self) -> bool {
        self.input_chs.iter().all(|ch| ch.finished())
    }
}

pub fn format_node_type(ty: &str) -> String {
    let generic = ty.split("<").next().unwrap();
    let parts = generic.split("::");
    let ty = parts.last().unwrap();

    match ty.split_once("<") {
        Some((prefix, _)) => match prefix.split("::").last() {
            Some(ty) => return ty.to_owned(),
            _ => {}
        },
        _ => {}
    }

    //fall through here and return input if failed
    ty.to_owned()
}
