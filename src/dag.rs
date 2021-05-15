use crate::{
    error::{Result, TexProError},
    node::{EmbeddedNodeDataId, Node, NodeType, Side},
    node_graph::*,
    process::*,
    slot_data::*,
};
use image::ImageBuffer;
use std::{
    collections::{BTreeMap, HashSet, VecDeque},
    sync::{mpsc, Arc, RwLock},
    thread,
};

use crate::shared::*;

#[derive(Clone, Copy, Debug, PartialEq)]
enum NodeState {
    Clean,
    Touched,
    Dirty,
}

impl Default for NodeState {
    fn default() -> Self {
        Self::Dirty
    }
}

#[derive(Default)]
pub struct TexProInt {
    pub node_graph: NodeGraph,
    pub slot_datas: Vec<Arc<SlotData>>,
    pub embedded_node_datas: Vec<Arc<EmbeddedNodeData>>,
    pub input_node_datas: Vec<Arc<SlotData>>,
    pub task_finished: bool,
    node_states: BTreeMap<NodeId, NodeState>,
}

impl TexProInt {
    pub fn new() -> Self {
        Self {
            node_graph: NodeGraph::new(),
            slot_datas: Vec::new(),
            embedded_node_datas: Vec::new(),
            input_node_datas: Vec::new(),
            task_finished: false,
            node_states: BTreeMap::new(),
        }
    }

    pub fn process(tex_pro: Arc<RwLock<TexProInt>>) {
        thread::spawn(move || {
            struct ThreadMessage {
                node_id: NodeId,
                node_datas: Result<Vec<Arc<SlotData>>>,
            }

            let nodes_to_process = if let Ok(mut tex_pro) = tex_pro.write() {
                let dirty_node_ids = tex_pro.get_dirty();
                
                let mut dirty_and_children = dirty_node_ids.clone();
                for node_id in &dirty_node_ids {
                    dirty_and_children.append(&mut tex_pro.get_children_recursive(*node_id));
                }

                dirty_and_children.sort_unstable();
                dirty_and_children.dedup();

                for node_id in &dirty_and_children {
                    tex_pro.remove_nodes_data(*node_id);
                }

                dirty_and_children
            } else {
                unreachable!();
            };

            let (send, recv) = mpsc::channel::<ThreadMessage>();
            let mut finished_nodes: HashSet<NodeId> =
                HashSet::with_capacity(tex_pro.read().unwrap().node_graph.nodes().len());
            let mut started_nodes: HashSet<NodeId> =
                HashSet::with_capacity(tex_pro.read().unwrap().node_graph.nodes().len());

            let mut queued_ids: VecDeque<NodeId> = VecDeque::from(nodes_to_process);
            for node_id in &queued_ids {
                started_nodes.insert(*node_id);
            }

            if queued_ids.is_empty() {
                return;
            }

            'outer: while finished_nodes.len() < started_nodes.len() {
                for message in recv.try_iter() {
                    tex_pro.write().unwrap().set_node_finished(
                        message.node_id,
                        &mut Some(message.node_datas.unwrap()),
                        &mut started_nodes,
                        &mut finished_nodes,
                        &mut queued_ids,
                    );
                }

                let current_id = match queued_ids.pop_front() {
                    Some(id) => id,
                    None => continue,
                };

                let parent_node_ids = tex_pro
                    .read()
                    .unwrap()
                    .node_graph
                    .edges
                    .iter()
                    .filter(|edge| edge.input_id == current_id)
                    .map(|edge| edge.output_id)
                    .collect::<Vec<NodeId>>();

                for parent_node_id in parent_node_ids {
                    if !finished_nodes.contains(&parent_node_id) {
                        queued_ids.push_back(current_id);
                        continue 'outer;
                    }
                }

                let mut relevant_ids: Vec<NodeId> = Vec::new();
                for node_data in tex_pro.read().unwrap().slot_datas.iter() {
                    for edge in &tex_pro.read().unwrap().node_graph.edges {
                        if edge.output_id != node_data.node_id {
                            continue;
                        } else {
                            relevant_ids.push(node_data.node_id);
                        }
                    }
                }

                // Put the `Arc<Buffer>`s and `Edge`s relevant for the calculation of this node into
                // lists.
                let mut relevant_edges: Vec<Edge> = Vec::new();
                let mut input_data: Vec<Arc<SlotData>> = Vec::new();
                for node_data in tex_pro.read().unwrap().slot_datas.iter() {
                    if !relevant_ids.contains(&node_data.node_id) {
                        continue;
                    }
                    for edge in &tex_pro.read().unwrap().node_graph.edges {
                        if node_data.slot_id == edge.output_slot
                            && node_data.node_id == edge.output_id
                            && current_id == edge.input_id
                        {
                            input_data.push(Arc::clone(node_data));
                            relevant_edges.push(*edge);
                        }
                    }
                }

                // Spawn a thread and calculate the node in it and send back the new `node_data`s for
                // each slot in the node.
                let current_node = tex_pro
                    .read()
                    .unwrap()
                    .node_graph
                    .node_with_id(current_id)
                    .unwrap()
                    .clone();
                let send = send.clone();

                let embedded_node_datas: Vec<Arc<EmbeddedNodeData>> = tex_pro
                    .read()
                    .unwrap()
                    .embedded_node_datas
                    .iter()
                    .map(|end| Arc::clone(&end))
                    .collect();
                let input_node_datas: Vec<Arc<SlotData>> = tex_pro
                    .read()
                    .unwrap()
                    .input_node_datas
                    .iter()
                    .map(|nd| Arc::clone(&nd))
                    .collect();

                thread::spawn(move || {
                    let node_datas: Result<Vec<Arc<SlotData>>> = process_node(
                        current_node,
                        &input_data,
                        &embedded_node_datas,
                        &input_node_datas,
                        &relevant_edges,
                    );

                    match send.send(ThreadMessage {
                        node_id: current_id,
                        node_datas,
                    }) {
                        Ok(_) => (),
                        Err(e) => println!("{:?}", e),
                    };
                });
            }

            tex_pro.write().unwrap().task_finished = true;
        });
    }

    fn set_all_node_state(&mut self, node_state: NodeState) {
        for ns in self.node_states.values_mut() {
            *ns = node_state;
        }
    }

    pub fn get_all_clean(&mut self) -> Vec<NodeId> {
        let mut output = Vec::new();

        for (id, state) in self.node_states.iter_mut() {
            if *state == NodeState::Clean {
                *state = NodeState::Touched;
                output.push(*id);
            }
        }

        output
    }

    /// Returns all nodes that are marked dirty.
    pub fn get_dirty(&self) -> Vec<NodeId> {
        self.node_states
            .iter()
            .filter(|(_, ns)| **ns == NodeState::Dirty)
            .map(|(id, _)| *id)
            .collect::<Vec<NodeId>>()
    }

    pub fn get_root_ids(&self) -> Vec<NodeId> {
        self.node_graph
            .nodes()
            .iter()
            .filter(|node| {
                self.node_graph
                    .edges
                    .iter()
                    .map(|edge| edge.output_id)
                    .any(|x| x == node.node_id)
            })
            .map(|node| node.node_id)
            .collect::<Vec<NodeId>>()
    }

    /// Takes a node and the data it generated, marks it as finished and puts the data in the
    /// `TextureProcessor`'s data vector.
    /// Then it adds any child `NodeId`s of the input `NodeId` to the list of `NodeId`s to process.
    fn set_node_finished(
        &mut self,
        id: NodeId,
        node_datas: &mut Option<Vec<Arc<SlotData>>>,
        started_nodes: &mut HashSet<NodeId>,
        finished_nodes: &mut HashSet<NodeId>,
        queued_ids: &mut VecDeque<NodeId>,
    ) {
        finished_nodes.insert(id);
        self.node_states.insert(id, NodeState::Clean);

        if let Some(node_datas) = node_datas {
            self.slot_datas.append(node_datas);
        }

        // Add any child node of the input `NodeId` to the list of nodes to potentially process.
        for edge in &self.node_graph.edges {
            let input_id = edge.input_id;
            if edge.output_id == id && !started_nodes.contains(&input_id) {
                queued_ids.push_back(input_id);
                started_nodes.insert(input_id);
            }
        }
    }

    /// Returns the NodeIds of all immediate children of this node (not recursive).
    pub fn get_children(&self, node_id: NodeId) -> Vec<NodeId> {
        self.node_graph
            .edges
            .iter()
            .filter(|edge| edge.output_id == node_id)
            .map(|edge| edge.input_id)
            .collect()
    }

    /// Returns the NodeIds of all children of this node.
    pub fn get_children_recursive(&self, node_id: NodeId) -> Vec<NodeId> {
        let children = self.get_children(node_id);
        let mut output = children.clone();

        for child in children {
            output.append(&mut self.get_children_recursive(child));
        }

        output
    }

    /// Returns the NodeIds of all immediate parents of this node (not recursive).
    pub fn get_parents(&self, node_id: NodeId) -> Vec<NodeId> {
        self.node_graph
            .edges
            .iter()
            .filter(|edge| edge.input_id == node_id)
            .map(|edge| edge.output_id)
            .collect()
    }

    /// Returns the NodeIds of all parents of this node.
    pub fn get_parents_recursive(&self, node_id: NodeId) -> Vec<NodeId> {
        let parents = self.get_parents(node_id);
        let mut output = parents.clone();

        for parent in parents {
            output.append(&mut self.get_parents_recursive(parent));
        }

        output
    }

    /// Returns the width and height of the `NodeData` for the given `NodeId` as a `Size`.
    pub fn get_node_data_size(&self, node_id: NodeId) -> Option<Size> {
        if let Some(node_data) = self.slot_datas.iter().find(|nd| nd.node_id == node_id) {
            Some(node_data.size)
        } else {
            None
        }
    }

    /// Embeds a `NodeData` in the `TextureProcessor` with an associated `EmbeddedNodeDataId`.
    /// The `EmbeddedNodeDataId` can be referenced using the assigned `EmbeddedNodeDataId` in a
    /// `NodeType::NodeData` node. This is useful when you want to transfer and use 'NodeData'
    /// between several `TextureProcessor`s.
    ///
    /// Get the `NodeData`s from a `Node` in a `TextureProcessor` by using the `get_node_data()`
    /// function.
    pub fn embed_node_data_with_id(
        &mut self,
        node_data: Arc<SlotData>,
        id: EmbeddedNodeDataId,
    ) -> Result<EmbeddedNodeDataId> {
        if self
            .embedded_node_datas
            .iter()
            .all(|end| end.node_data_id != id)
        {
            self.embedded_node_datas
                .push(Arc::new(EmbeddedNodeData::from_node_data(node_data, id)));
            Ok(id)
        } else {
            Err(TexProError::InvalidSlotId)
        }
    }

    /// Removes all the `slot_data` associated with the given `NodeId`.
    pub(crate) fn remove_nodes_data(&mut self, id: NodeId) {
        for i in (0..self.slot_datas.len()).rev() {
            if self.slot_datas[i].node_id == id {
                self.slot_datas.remove(i);
            }
        }
    }

    /// Gets all `NodeData`s in this `TextureProcessor`.
    pub fn node_datas(&self, id: NodeId) -> Vec<Arc<SlotData>> {
        self.slot_datas
            .iter()
            .filter(|&x| x.node_id == id)
            .map(|x| Arc::clone(x))
            .collect()
    }

    /// Gets any `NodeData`s associated with a given `NodeId`.
    pub fn get_node_data(&self, id: NodeId) -> Vec<Arc<SlotData>> {
        self.slot_datas
            .iter()
            .filter(|nd| nd.node_id == id)
            .map(|nd| Arc::clone(&nd))
            .collect()
    }

    pub fn get_output(&self, node_id: NodeId) -> Result<Vec<u8>> {
        let node_datas = self.node_datas(node_id);
        if node_datas.is_empty() {
            return Err(TexProError::Generic);
        }

        let output_vecs = match self
            .node_graph
            .node_with_id(node_id)
            .ok_or(TexProError::InvalidNodeId)?
            .node_type
        {
            NodeType::OutputRgba => self.get_output_rgba(&node_datas)?,
            NodeType::OutputGray => self.get_output_gray(&node_datas)?,
            _ => return Err(TexProError::InvalidNodeType),
        };

        channels_to_rgba(&output_vecs)
    }

    fn get_output_rgba(&self, node_datas: &[Arc<SlotData>]) -> Result<Vec<Arc<Buffer>>> {
        let empty_buffer: Arc<Buffer> = Arc::new(Box::new(ImageBuffer::new(0, 0)));
        let mut sorted_value_vecs: Vec<Arc<Buffer>> = vec![Arc::clone(&empty_buffer); 4];

        for node_data in node_datas.iter() {
            sorted_value_vecs[node_data.slot_id.0 as usize] = Arc::clone(&node_data.buffer);
        }

        let (width, height) = (node_datas[0].size.width, node_datas[0].size.height);
        let size = (width * height) as usize;

        for (i, value_vec) in sorted_value_vecs.iter_mut().enumerate() {
            if !value_vec.is_empty() {
                continue;
            }

            // Should be black if R, G or B channel, and white if A.
            let buf_vec = if i == 3 {
                vec![1.; size]
            } else {
                vec![0.; size]
            };

            *value_vec = Arc::new(Box::new(
                ImageBuffer::from_raw(width, height, buf_vec).ok_or(TexProError::Generic)?,
            ))
        }

        Ok(sorted_value_vecs)
    }

    fn get_output_gray(&self, node_datas: &[Arc<SlotData>]) -> Result<Vec<Arc<Buffer>>> {
        assert_eq!(node_datas.len(), 1);
        let (width, height) = (node_datas[0].size.width, node_datas[0].size.height);
        let size = (width * height) as usize;

        let mut sorted_value_vecs: Vec<Arc<Buffer>> = vec![Arc::clone(&node_datas[0].buffer); 3];
        sorted_value_vecs.push(Arc::new(Box::new(
            ImageBuffer::from_raw(width, height, vec![1.; size]).ok_or(TexProError::Generic)?,
        )));

        Ok(sorted_value_vecs)
    }

    pub fn add_node(&mut self, node: Node) -> Result<NodeId> {
        let result = self.node_graph.add_node(node);

        if let Ok(node_id) = result {
            self.node_states.insert(node_id, NodeState::Dirty);
        }

        result
    }

    pub fn remove_node(&mut self, node_id: NodeId) -> Result<()> {
        self.node_states.remove(&node_id);
        self.node_graph.remove_node(node_id)
    }

    pub fn connect(
        &mut self,
        output_node: NodeId,
        input_node: NodeId,
        output_slot: SlotId,
        input_slot: SlotId,
    ) -> Result<()> {
        let result = self
            .node_graph
            .connect(output_node, input_node, output_slot, input_slot);

        if result.is_ok() {
            self.node_states.insert(input_node, NodeState::Dirty);
        }

        result
    }

    pub fn connect_arbitrary(
        &mut self,
        a_node: NodeId,
        a_side: Side,
        a_slot: SlotId,
        b_node: NodeId,
        b_side: Side,
        b_slot: SlotId,
    ) -> Result<()> {
        let result = self
            .node_graph
            .connect_arbitrary(a_node, a_side, a_slot, b_node, b_side, b_slot);

        if result.is_ok() {
            self.node_states.insert(b_node, NodeState::Dirty);
        }

        result
    }

    pub fn disconnect_slot(&mut self, node_id: NodeId, side: Side, slot_id: SlotId) {
        self.node_graph.disconnect_slot(node_id, side, slot_id);

        if side == Side::Input {
            self.node_states.insert(node_id, NodeState::Dirty);
        }
    }

    pub fn set_node_graph(&mut self, node_graph: NodeGraph) {
        self.node_states.clear();
        for node_id in node_graph.node_ids() {
            self.node_states.insert(node_id, NodeState::Dirty);
        }

        self.node_graph = node_graph;
    }

    pub fn node_ids(&self) -> Vec<NodeId> {
        self.node_graph.node_ids()
    }

    pub fn edges(&self) -> Vec<Edge> {
        self.node_graph.edges.to_owned()
    }
}
