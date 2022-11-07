use std::{
    collections::{HashMap, HashSet},
    marker::PhantomData,
};

use bytes::Bytes;

use crate::{Frame, InputChannel, OutputChannel, VData, VLeftJoin, VSink, VSource, VTransform, KV};

pub type VNodeId = u32;

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct VEdge {
    pub source_node_id: VNodeId,
    pub source_port: u32,
    pub dest_node_id: VNodeId,
    pub dest_port: u32,
}

pub struct VGraphTopology {
    labels: HashMap<VNodeId, String>,
    edges: HashMap<VNodeId, HashMap<VNodeId, HashSet<VEdge>>>,
    source_ids: HashSet<VNodeId>,
}

pub struct VGraph {
    nodes: HashMap<VNodeId, Box<dyn VNode + Sync + Send>>,
    topo: VGraphTopology,
}

impl VGraph {
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            topo: VGraphTopology {
                labels: HashMap::new(),
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
        Box::new(self.topo.labels.keys().map(|k| *k))
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

    pub fn node_mut(&mut self, node_id: &VNodeId) -> Option<&mut Box<dyn VNode + Sync + Send>> {
        self.nodes.get_mut(node_id)
    }

    pub fn node(&self, node_id: &VNodeId) -> Option<&Box<dyn VNode + Sync + Send>> {
        self.nodes.get(node_id)
    }

    pub fn node_label(&self, node_id: &VNodeId) -> Option<&str> {
        self.topo.labels.get(node_id).map(|s| s.as_str())
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

    pub fn insert<N: VNode + Sync + Send + 'static>(
        &mut self,
        node: N,
        upstream_ids: Option<&[(VNodeId, u32, u32)]>,
    ) -> VNodeId {
        let next_id = self.nodes.len() as u32;

        let node_type_name = std::any::type_name::<N>();
        let node_label = format!("{}[{}]", node_type_name, next_id);

        self.nodes.insert(next_id, Box::new(node));
        self.topo.labels.insert(next_id, node_label);

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
        let node_id = self.insert(source_node, None);

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

        let node_id = self.insert(join, Some(&[left_edge, right_edge]));

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
    pub fn map<MapO: VData, F: Fn(Out) -> MapO + Sync + Send + 'static>(
        &self,
        g: &mut VGraph,
        f: F,
    ) -> VNodeRef<Out, MapO> {
        self.transform(g, crate::transform::map(f))
    }

    pub fn transform<TO, T: VTransform<Input = Out, Output = TO> + Sync + Send + 'static>(
        &self,
        g: &mut VGraph,
        transform: T,
    ) -> VNodeRef<Out, TO> {
        let transform_id = g.insert(transform, Some(&[(self.node_id, 0, 0)]));

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
        let sink_id = g.insert(sink, Some(&[(self.node_id, 0, 0)]));

        VNodeRef::<Out, ()> {
            node_id: sink_id,
            _in: Default::default(),
            _out: Default::default(),
        }
    }
}

pub trait VNode {
    fn tick(&mut self, ctx: &mut VNodeCtx) -> ();
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
            println!("invalid output index"); //TODO return error
            ()
        }
    }

    pub(crate) fn recv(&mut self, input_idx: u32) -> Option<Frame<Bytes>> {
        if let Some(input) = self.inputs.get_mut(&input_idx) {
            input.recv()
        } else {
            println!("invalid input index"); //TODO return error
            None
        }
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
    pub input_ch: Option<Box<dyn InputChannel>>,
}

impl Input {
    pub fn recv(&mut self) -> Option<Frame<Bytes>> {
        if let Some(output) = self.input_ch.as_mut() {
            output.recv()
        } else {
            None
        }
    }
}
