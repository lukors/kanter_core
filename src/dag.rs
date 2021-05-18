use crate::{
    error::{Result, TexProError},
    node::{EmbeddedNodeDataId, Node, NodeType, Side},
    node_graph::*,
    process::*,
    slot_data::*,
};
use image::ImageBuffer;
use std::{
    collections::BTreeMap,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc, Arc, RwLock, RwLockReadGuard, RwLockWriteGuard,
    },
    thread,
};

use crate::shared::*;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum NodeState {
    Clean,
    Dirty,
    Requested,
    Prioritised,
    Processing,
}

impl Default for NodeState {
    fn default() -> Self {
        Self::Dirty
    }
}

impl NodeState {
    fn is_dirty(&self) -> bool {
        *self != Self::Clean
    }
}

#[derive(Default)]
pub struct TexProInt {
    pub node_graph: NodeGraph,
    pub slot_datas: Vec<Arc<SlotData>>,
    pub embedded_node_datas: Vec<Arc<EmbeddedNodeData>>,
    pub input_node_datas: Vec<Arc<SlotData>>,
    node_states: BTreeMap<NodeId, NodeState>,
    one_shot: bool,
    state_generation: Generation,
    node_generation: Generation,
    edge_generation: Generation,
}

#[derive(Clone, Copy, Debug, Default)]
struct Generation(usize);

impl Generation {
    fn add(&mut self) {
        self.0 = self.0.wrapping_add(1);
    }
}

impl TexProInt {
    pub fn new() -> Self {
        Self {
            node_graph: NodeGraph::new(),
            slot_datas: Vec::new(),
            embedded_node_datas: Vec::new(),
            input_node_datas: Vec::new(),
            node_states: BTreeMap::new(),
            one_shot: false,
            state_generation: Default::default(),
            node_generation: Default::default(),
            edge_generation: Default::default(),
        }
    }

    pub(crate) fn process_loop(tex_pro: Arc<RwLock<TexProInt>>, shutdown: Arc<AtomicBool>) {
        struct ThreadMessage {
            node_id: NodeId,
            slot_datas: Result<Vec<Arc<SlotData>>>,
        }
        let (send, recv) = mpsc::channel::<ThreadMessage>();

        loop {
            if shutdown.load(Ordering::Relaxed) {
                return;
            }

            if let Ok(mut tex_pro) = tex_pro.write() {
                // Handle messages received from node processing threads.
                for message in recv.try_iter() {
                    let node_id = message.node_id;
                    let slot_datas = message.slot_datas;

                    match slot_datas {
                        Ok(mut slot_datas) => tex_pro.slot_datas.append(&mut slot_datas),
                        Err(e) => {
                            shutdown.store(true, Ordering::Relaxed);
                            panic!(
                                "Error when processing '{:?}' node with id '{}': {}",
                                tex_pro.node_graph.node_with_id(node_id).unwrap().node_type,
                                node_id,
                                e
                            );
                        }
                    }

                    tex_pro.node_states.insert(node_id, NodeState::Clean);
                }

                // Get requested nodes
                let requested = tex_pro
                    .node_states
                    .iter()
                    .filter(|(_, node_state)| {
                        matches!(**node_state, NodeState::Requested | NodeState::Prioritised)
                    })
                    .map(|(node_id, _)| *node_id)
                    .collect::<Vec<NodeId>>();

                // Get the closest non-clean parents
                let mut closest_processable = Vec::new();
                for node_id in requested {
                    closest_processable.append(&mut tex_pro.get_closest_processable(node_id));
                }
                closest_processable.sort_unstable();
                closest_processable.dedup();

                // Attempt to process all non-clean parents
                for node_id in closest_processable {
                    *tex_pro.node_state_mut(node_id).unwrap() = NodeState::Processing;

                    let node = tex_pro.node_graph.node_with_id(node_id).unwrap();

                    let embedded_node_datas: Vec<Arc<EmbeddedNodeData>> = tex_pro
                        .embedded_node_datas
                        .iter()
                        .map(|end| Arc::clone(&end))
                        .collect();

                    let input_node_datas: Vec<Arc<SlotData>> = tex_pro
                        .input_node_datas
                        .iter()
                        .map(|nd| Arc::clone(&nd))
                        .collect();

                    let edges = tex_pro
                        .edges()
                        .iter()
                        .filter(|edge| edge.input_id == node_id)
                        .copied()
                        .collect::<Vec<Edge>>();

                    let input_data = {
                        let input_data = edges
                            .iter()
                            .map(|edge| {
                                tex_pro.slot_datas.iter().find(|slot_data| {
                                    slot_data.slot_id == edge.output_slot
                                        && slot_data.node_id == edge.output_id
                                })
                            })
                            .collect::<Vec<Option<&Arc<SlotData>>>>();

                        if input_data.contains(&None) {
                            continue;
                        } else {
                            input_data
                                .into_iter()
                                .map(|slot_data| Arc::clone(slot_data.unwrap()))
                                .collect::<Vec<Arc<SlotData>>>()
                        }
                    };

                    assert_eq!(
                        edges.len(),
                        input_data.len(),
                        "NodeType: {:?}",
                        node.node_type
                    );

                    let send = send.clone();

                    thread::spawn(move || {
                        let slot_datas: Result<Vec<Arc<SlotData>>> = process_node(
                            node,
                            &input_data,
                            &embedded_node_datas,
                            &input_node_datas,
                            &edges,
                        );

                        match send.send(ThreadMessage {
                            node_id,
                            slot_datas,
                        }) {
                            Ok(_) => (),
                            Err(e) => println!("{:?}", e),
                        };
                    });
                }

                // If the tex_pro is set to one_shot and all nodes are clean, shut it down.
                if tex_pro.one_shot && tex_pro
                    .node_states
                    .iter()
                    .all(|(_, node_state)| *node_state == NodeState::Clean)
                {
                    shutdown.store(true, Ordering::Relaxed);
                    break;
                }
            }

            // Sleeping to reduce CPU load.
            thread::sleep(std::time::Duration::from_micros(1));
        }
    }

    pub fn process_then_kill(&mut self) {
        self.one_shot = true;
        for node_id in self.node_graph.output_ids() {
            self.request(node_id).unwrap();
        }
    }

    /// Waits until a certain NodeId has a certain state, and when it does it returns the
    /// `RwLockWriteGuard` so changes can be made while the `NodeState` the state remains the same.
    pub fn wait_for_state_write(
        tpi: &Arc<RwLock<Self>>,
        node_id: NodeId,
        node_state: NodeState,
    ) -> Result<RwLockWriteGuard<TexProInt>> {
        loop {
            if let Ok(mut tpi) = tpi.write() {
                if node_state == tpi.node_state(node_id)? {
                    return Ok(tpi);
                } else {
                    tpi.prioritise(node_id)?;
                }
            }
        }
    }

    /// Waits until a certain NodeId has a certain state, and when it does it returns the
    /// `RwLockReadGuard` so reads can be made while the `NodeState` remains the same.
    pub fn wait_for_state_read(
        tpi: &Arc<RwLock<Self>>,
        node_id: NodeId,
        node_state: NodeState,
    ) -> Result<RwLockReadGuard<TexProInt>> {
        loop {
            if let Ok(tpi) = tpi.read() {
                if node_state == tpi.node_state(node_id)? {
                    return Ok(tpi);
                }
            }

            tpi.write().unwrap().prioritise(node_id)?;
        }
    }

    pub fn request(&mut self, node_id: NodeId) -> Result<()> {
        let node_state = self.node_state_mut(node_id)?;

        if *node_state == NodeState::Dirty {
            *node_state = NodeState::Requested;
        }

        Ok(())
    }

    pub fn prioritise(&mut self, node_id: NodeId) -> Result<()> {
        let node_state = self.node_state_mut(node_id)?;

        if matches!(node_state, NodeState::Dirty | NodeState::Requested) {
            *node_state = NodeState::Prioritised;
        }

        Ok(())
    }

    pub fn node_state(&self, node_id: NodeId) -> Result<NodeState> {
        if let Some(node_state) = self.node_states.get(&node_id) {
            Ok(*node_state)
        } else {
            Err(TexProError::InvalidNodeId)
        }
    }

    pub fn node_state_mut(&mut self, node_id: NodeId) -> Result<&mut NodeState> {
        if let Some(node_state) = self.node_states.get_mut(&node_id) {
            Ok(node_state)
        } else {
            Err(TexProError::InvalidNodeId)
        }
    }

    pub fn get_dirty(&self) -> Vec<NodeId> {
        self.node_states
            .iter()
            .filter(|(_, node_state)| node_state.is_dirty())
            .map(|(node_id, _)| *node_id)
            .collect()
    }

    /// Returns all `NodeId`s with the given `NodeState`.
    pub fn node_ids_with_state(&self, node_state: NodeState) -> Vec<NodeId> {
        self.node_states
            .iter()
            .filter(|(_, ns)| **ns == node_state)
            .map(|(id, _)| *id)
            .collect()
    }

    /// Sets a node and all its children as dirty.
    fn set_dirty(&mut self, node_id: NodeId) -> Result<Vec<NodeId>> {
        let children = self.get_children_recursive(node_id)?;

        for node_id in children.iter().chain(vec![node_id].iter()) {
            self.node_states.insert(*node_id, NodeState::Dirty);
        }

        self.state_generation.add();
        Ok(children)
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

    /// Returns the NodeIds of all immediate children of this node (not recursive).
    pub fn get_children(&self, node_id: NodeId) -> Result<Vec<NodeId>> {
        self.node_graph.has_node_with_id(node_id)?;

        Ok(self
            .node_graph
            .edges
            .iter()
            .filter(|edge| edge.output_id == node_id)
            .map(|edge| edge.input_id)
            .collect())
    }

    /// Returns the NodeIds of all children of this node.
    pub fn get_children_recursive(&self, node_id: NodeId) -> Result<Vec<NodeId>> {
        let children = self.get_children(node_id)?;
        let mut output = children.clone();

        for child in children {
            output.append(&mut self.get_children_recursive(child)?);
        }

        Ok(output)
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
    pub fn get_ancestors(&self, node_id: NodeId) -> Vec<NodeId> {
        let parents = self.get_parents(node_id);
        let mut output = parents.clone();

        for parent in parents {
            output.append(&mut self.get_ancestors(parent));
        }

        output
    }

    /// Returns the NodeIds of the closest ancestors matching any of the given states, including self.
    pub fn get_closest_processable(&self, node_id: NodeId) -> Vec<NodeId> {
        let mut closest_processable = Vec::new();

        // Put dirty and processing parents in their own vectors.
        let mut dirty = Vec::new();
        let mut processing = Vec::new();
        for node_id in self.get_parents(node_id) {
            match *self.node_states.get(&node_id).unwrap() {
                NodeState::Processing => processing.push(node_id),
                NodeState::Dirty | NodeState::Requested | NodeState::Prioritised => {
                    dirty.push(node_id)
                }
                NodeState::Clean => (),
            }
        }

        // If there are no dirty parents, and no parents currently being processed that means all
        // potential parents for this node have been processed, meaning this node can be processed.
        if dirty.is_empty() && processing.is_empty() {
            closest_processable.push(node_id);
        } else {
            // If there are dirty parents, recurse into them and keep looking for the closest
            // processable node.
            for node_id in dirty {
                closest_processable.append(&mut self.get_closest_processable(node_id));
            }
        }

        closest_processable.sort_unstable();
        closest_processable.dedup();

        closest_processable
    }

    /// Returns the `Size` of the `SlotData` for the given `NodeId` and `SlotId`.
    pub fn get_slot_data_size(&self, node_id: NodeId, slot_id: SlotId) -> Option<Size> {
        if let Some(node_data) = self
            .slot_datas
            .iter()
            .find(|nd| nd.node_id == node_id && nd.slot_id == slot_id)
        {
            Some(node_data.size)
        } else {
            None
        }
    }

    // pub fn get_node_data_size(&self, node_id: NodeId) -> Option<Size> {
    //     if let Some(node_data) = self.slot_datas.iter().find(|nd| nd.node_id == node_id) {
    //         Some(node_data.size)
    //     } else {
    //         None
    //     }
    // }

    /// Embeds a `SlotData` in the `TextureProcessor` with an associated `EmbeddedNodeDataId`.
    /// The `EmbeddedNodeDataId` can be referenced using the assigned `EmbeddedNodeDataId` in a
    /// `NodeType::NodeData` node. This is useful when you want to transfer and use 'NodeData'
    /// between several `TextureProcessor`s.
    ///
    /// Get the `SlotData`s from a `Node` in a `TextureProcessor` by using the `get_node_data()`
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
    // pub(crate) fn remove_nodes_data(&mut self, id: NodeId) {
    //     for i in (0..self.slot_datas.len()).rev() {
    //         if self.slot_datas[i].node_id == id {
    //             self.slot_datas.remove(i);
    //         }
    //     }
    // }

    /// Gets all `SlotData`s in this `TextureProcessor`.
    pub fn slot_datas(&self) -> Vec<Arc<SlotData>> {
        self.slot_datas.clone()
    }

    /// Gets any `SlotData`s associated with a given `NodeId`.
    pub fn node_slot_datas(&self, id: NodeId) -> Vec<Arc<SlotData>> {
        self.slot_datas
            .iter()
            .filter(|nd| nd.node_id == id)
            .map(|nd| Arc::clone(&nd))
            .collect()
    }

    pub fn get_output(&self, node_id: NodeId) -> Result<Vec<u8>> {
        let node_datas = self.node_slot_datas(node_id);
        if node_datas.is_empty() {
            return Err(TexProError::Generic);
        }

        let output_vecs = match self.node_graph.node_with_id(node_id)?.node_type {
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

    pub fn add_node_with_id(&mut self, node: Node, node_id: NodeId) -> Result<NodeId> {
        let node_id = self.node_graph.add_node_with_id(node, node_id)?;
        self.node_generation.add();
        Ok(node_id)
    }

    pub fn add_node(&mut self, node: Node) -> Result<NodeId> {
        let node_id = self.node_graph.add_node(node)?;

        self.node_generation.add();
        self.node_states.insert(node_id, NodeState::default());

        Ok(node_id)
    }

    pub fn remove_node(&mut self, node_id: NodeId) -> Result<Vec<Edge>> {
        let (_, edges) = self.node_graph.remove_node(node_id)?;

        self.node_generation.add();
        self.node_states.remove(&node_id);

        if !edges.is_empty() {
            self.edge_generation.add();
        }

        Ok(edges)
    }

    pub fn connect(
        &mut self,
        output_node: NodeId,
        input_node: NodeId,
        output_slot: SlotId,
        input_slot: SlotId,
    ) -> Result<()> {
        self.node_graph
            .connect(output_node, input_node, output_slot, input_slot)?;

        self.set_dirty(input_node)?;

        Ok(())
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

    pub fn disconnect_slot(
        &mut self,
        node_id: NodeId,
        side: Side,
        slot_id: SlotId,
    ) -> Result<Vec<Edge>> {
        let edges = self.node_graph.disconnect_slot(node_id, side, slot_id)?;

        if !edges.is_empty() {
            self.node_states.insert(node_id, NodeState::Dirty);
        }

        Ok(edges)
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
