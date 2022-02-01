use crate::{
    edge::Edge,
    error::{Result, TexProError},
    node::{
        embed::{EmbeddedSlotData, EmbeddedSlotDataId},
        Node, Side,
    },
    node_graph::*,
    priority::{Priority, PriorityPropagator},
    slot_data::*,
    transient_buffer::{TransientBufferContainer, TransientBufferQueue},
};
use std::{
    collections::{BTreeMap, BTreeSet, VecDeque},
    fmt::Display,
    sync::{atomic::Ordering, Arc, RwLock, RwLockReadGuard, RwLockWriteGuard},
    thread,
    time::Duration,
};

/// Indicates what is going on with the node.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum NodeState {
    // The node's outputs match the inputs and settings
    Clean,
    // Some input or setting was changed, so the outputs do not match them
    Dirty,
    // Node is in processing queue
    Requested,
    // Node is in priority processing queue (this is not used)
    Prioritised,
    // Node is being processed
    Processing,
    // Some input or setting was changed while the node was being processed, it will be processed
    // again when it's finished.
    ProcessingDirty,
}

impl Display for NodeState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Clean => "Clean",
                Self::Dirty => "Dirty",
                Self::Requested => "Requested",
                Self::Prioritised => "Prioritised",
                Self::Processing => "Processing",
                Self::ProcessingDirty => "ProcessingDirty",
            }
        )
    }
}

impl Default for NodeState {
    fn default() -> Self {
        Self::Dirty
    }
}

#[derive(Debug)]
pub struct LiveGraph {
    pub(crate) node_graph: NodeGraph,
    pub(crate) slot_datas: VecDeque<Arc<SlotData>>,
    embedded_slot_datas: Vec<Arc<EmbeddedSlotData>>,
    input_slot_datas: Vec<Arc<SlotData>>,
    node_state: BTreeMap<NodeId, NodeState>,
    changed: BTreeSet<NodeId>,
    priority_propagator: PriorityPropagator,
    pub auto_update: bool,
    pub use_cache: bool,
    pub(crate) add_buffer_queue: Arc<RwLock<Vec<Arc<TransientBufferContainer>>>>,
}

impl LiveGraph {
    pub fn new(add_buffer_queue: Arc<RwLock<Vec<Arc<TransientBufferContainer>>>>) -> Self {
        Self {
            node_graph: NodeGraph::new(),
            slot_datas: VecDeque::new(),
            embedded_slot_datas: Vec::new(),
            input_slot_datas: Vec::new(),
            node_state: BTreeMap::new(),
            changed: BTreeSet::new(),
            priority_propagator: PriorityPropagator::new(),
            auto_update: false,
            use_cache: false,
            add_buffer_queue,
        }
    }

    /// Return a SlotData as u8.
    pub fn buffer_rgba(&self, node_id: NodeId, slot_id: SlotId) -> Result<Vec<u8>> {
        self.slot_data(node_id, slot_id)?.image.to_u8()
    }

    /// Tries to get the output of a node. If it can't it submits a request for it.
    pub fn try_buffer_rgba(
        live_graph: &Arc<RwLock<LiveGraph>>,
        node_id: NodeId,
        slot_id: SlotId,
    ) -> Result<Vec<u8>> {
        let result = if let Ok(live_graph) = live_graph.try_write() {
            if let Ok(node_state) = live_graph.node_state(node_id) {
                if node_state == NodeState::Clean {
                    live_graph.slot_data(node_id, slot_id)?.image.to_u8()
                } else {
                    Err(TexProError::InvalidNodeId)
                }
            } else {
                Err(TexProError::InvalidNodeId)
            }
        } else {
            Err(TexProError::UnableToLock)
        };

        if result.is_err() {
            // This is blocking, should probably make requests go through an
            // `RwLock<BTreeSet<NodeId>>`.
            live_graph.write().unwrap().request(node_id)?
        }

        result
    }

    /// Tries to get the output of a node. If it can't it submits a request for it.
    pub fn try_buffer_srgba(
        live_graph: &Arc<RwLock<LiveGraph>>,
        node_id: NodeId,
        slot_id: SlotId,
    ) -> Result<Vec<u8>> {
        let result = if let Ok(live_graph) = live_graph.try_write() {
            if let Ok(node_state) = live_graph.node_state(node_id) {
                if node_state == NodeState::Clean {
                    live_graph.slot_data(node_id, slot_id)?.image.to_u8_srgb()
                } else {
                    Err(TexProError::InvalidNodeId)
                }
            } else {
                Err(TexProError::InvalidNodeId)
            }
        } else {
            Err(TexProError::UnableToLock)
        };

        if result.is_err() {
            // This is blocking, should probably make requests go through an
            // `RwLock<BTreeSet<NodeId>>`.
            live_graph.write().unwrap().request(node_id)?
        }

        result
    }

    /// Return all changed `NodeId`s.
    pub fn changed_consume(&mut self) -> Vec<NodeId> {
        let output = self.changed.iter().copied().collect();
        self.changed.clear();
        output
    }

    /// Waits until a certain NodeId has a certain state, and when it does it returns the
    /// `RwLockWriteGuard` so changes can be made while the `NodeState` the state remains the same.
    pub fn await_clean_write(
        live_graph: &Arc<RwLock<Self>>,
        node_id: NodeId,
    ) -> Result<RwLockWriteGuard<LiveGraph>> {
        loop {
            if let Ok(mut live_graph) = live_graph.write() {
                if live_graph.node_state(node_id)? == NodeState::Clean {
                    return Ok(live_graph);
                } else {
                    live_graph.prioritise(node_id)?;
                }
            }

            thread::sleep(Duration::from_millis(1));
        }
    }

    pub fn await_clean_read(
        live_graph: &Arc<RwLock<Self>>,
        node_id: NodeId,
    ) -> Result<RwLockReadGuard<LiveGraph>> {
        loop {
            if let Ok(live_graph) = live_graph.read() {
                if live_graph.node_state(node_id)? == NodeState::Clean {
                    return Ok(live_graph);
                }
            }

            live_graph.write().unwrap().prioritise(node_id)?;
            thread::sleep(Duration::from_millis(1));
        }
    }

    pub(crate) fn propagate_priorities(&mut self) {
        self.priority_propagator.update(&self.node_graph);
    }

    /// Waits until a certain NodeId has a certain state, and when it does it returns the
    /// `RwLockReadGuard` so reads can be made while the `NodeState` remains the same.
    // pub fn await_state_read(
    //     live_graph: &Arc<RwLock<Self>>,
    //     node_id: NodeId,
    //     node_state: NodeState,
    // ) -> Result<RwLockReadGuard<LiveGraph>> {
    //     loop {
    //         if let Ok(live_graph) = live_graph.read() {
    //             if node_state == live_graph.node_state(node_id)? {
    //                 return Ok(live_graph);
    //             }
    //         }

    //         thread::sleep(Duration::from_millis(1));
    //     }
    // }

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

    pub(crate) fn node_states(&self) -> &BTreeMap<NodeId, NodeState> {
        &self.node_state
    }

    /// Gets the NodeState of the node with the given `NodeId`.
    pub fn node_state(&self, node_id: NodeId) -> Result<NodeState> {
        if let Some(node_state) = self.node_state.get(&node_id) {
            Ok(*node_state)
        } else {
            Err(TexProError::InvalidNodeId)
        }
    }

    /// Gets a mutable reference to the NodeState of the node with the given `NodeId`.
    pub fn node_state_mut(&mut self, node_id: NodeId) -> Result<&mut NodeState> {
        Ok(&mut *self
            .node_state
            .get_mut(&node_id)
            .ok_or(TexProError::InvalidNodeId)?)
    }

    /// Returns all `NodeId`s that are not in the given `NodeState`.
    pub fn node_ids_without_state(&self, node_state: NodeState) -> Vec<NodeId> {
        self.node_state
            .iter()
            .filter(|(_, node_state_iter)| **node_state_iter != node_state)
            .map(|(id, _)| *id)
            .collect()
    }

    /// Returns all `NodeId`s with the given `NodeState`.
    pub fn node_ids_with_state(&self, node_state: NodeState) -> Vec<NodeId> {
        self.node_state
            .iter()
            .filter(|(_, node_state_iter)| **node_state_iter == node_state)
            .map(|(id, _)| *id)
            .collect()
    }

    /// Returns the `NodeId`s of the closest ancestors that are ready to be processed, including self.
    pub fn get_closest_processable(&self, node_id: NodeId) -> Vec<NodeId> {
        let mut closest_processable = Vec::new();

        // Put dirty and processing parents in their own vectors.
        let mut dirty = Vec::new();
        let mut processing = Vec::new();
        for node_id in self.node_graph.get_parents(node_id) {
            match self.node_state(node_id).unwrap() {
                NodeState::Processing | NodeState::ProcessingDirty => processing.push(node_id),
                NodeState::Dirty | NodeState::Requested | NodeState::Prioritised => {
                    dirty.push(node_id)
                }
                NodeState::Clean => (),
            }
        }

        if dirty.is_empty() && processing.is_empty() {
            // If there are no dirty parents, and no parents currently being processed that means all
            // potential parents for this node have been processed, meaning this node can be processed.
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

    pub(crate) fn embedded_slot_datas(&self) -> &Vec<Arc<EmbeddedSlotData>> {
        &self.embedded_slot_datas
    }

    /// Embeds a `SlotData` in the `LiveGraph` with an associated `EmbeddedNodeDataId`.
    /// The `EmbeddedNodeDataId` can be referenced using the assigned `EmbeddedNodeDataId` in a
    /// `NodeType::Embed` node. This is useful when you want to transfer and use 'NodeData'
    /// between several `LiveGraph`s.
    ///
    /// Get the `SlotData`s from a `Node` in a `LiveGraph` by using `node_slot_datas_new()`
    /// function.
    pub fn embed_slot_data_with_id(
        &mut self,
        slot_data: Arc<SlotData>,
        id: EmbeddedSlotDataId,
    ) -> Result<EmbeddedSlotDataId> {
        if self
            .embedded_slot_datas
            .iter()
            .all(|end| end.slot_data_id != id)
        {
            TransientBufferQueue::add_slot_data(&self.add_buffer_queue, &slot_data);
            self.embedded_slot_datas
                .push(Arc::new(EmbeddedSlotData::from_slot_data(slot_data, id)));
            Ok(id)
        } else {
            Err(TexProError::InvalidSlotId)
        }
    }

    pub(crate) fn input_slot_datas(&self) -> &Vec<Arc<SlotData>> {
        &self.input_slot_datas
    }

    pub fn add_input_slot_data(&mut self, slot_data: Arc<SlotData>) {
        TransientBufferQueue::add_slot_data(&self.add_buffer_queue, &slot_data);
        self.input_slot_datas.push(slot_data);
    }

    /// Removes all the `SlotData` associated with the given `NodeId`.
    pub(crate) fn remove_nodes_data(&mut self, id: NodeId) {
        for i in (0..self.slot_datas.len()).rev() {
            if self.slot_datas[i].node_id == id {
                self.slot_datas.remove(i);
            }
        }
    }

    pub fn has_node(&self, node_id: NodeId) -> Result<()> {
        self.node_graph.has_node_with_id(node_id)
    }

    pub fn node(&self, node_id: NodeId) -> Result<Node> {
        self.node_graph.node(node_id)
    }

    pub fn node_mut(&mut self, node_id: NodeId) -> Result<&mut Node> {
        self.set_state(node_id, NodeState::Dirty)?;
        self.node_graph
            .node_with_id_mut(node_id)
            .ok_or(TexProError::InvalidNodeId)
    }

    pub fn set_node_with_id(&mut self, node_id: NodeId, node: Node) -> Result<()> {
        let found_node = self
            .node_graph
            .nodes
            .iter_mut()
            .find(|node| node.node_id == node_id)
            .ok_or(TexProError::InvalidNodeId)?;
        *found_node = node;

        Ok(())
    }

    /// Gets all `SlotData`s associated with a given `NodeId`.
    pub fn node_slot_datas(&self, node_id: NodeId) -> Result<Vec<Arc<SlotData>>> {
        let mut output: Vec<Arc<SlotData>> = Vec::new();

        let slot_ids: Vec<SlotId> = self
            .slot_datas
            .iter()
            .filter(|slot_data| slot_data.node_id == node_id)
            .map(|slot_data| slot_data.slot_id)
            .collect();

        for slot_id in slot_ids {
            output.push(Arc::clone(self.slot_data(node_id, slot_id)?));
        }

        Ok(output)
    }

    pub fn slot_data_size(&self, node_id: NodeId, slot_id: SlotId) -> Result<Size> {
        self.slot_data(node_id, slot_id)?.size()
    }

    pub fn slot_in_memory(&self, node_id: NodeId, slot_id: SlotId) -> Result<bool> {
        self.slot_data(node_id, slot_id)?.in_memory()
    }

    /// Warning: Using the `Arc<SlotData>` in another `TextureProcessor` would cause a memory leak.
    pub fn slot_data(&self, node_id: NodeId, slot_id: SlotId) -> Result<&Arc<SlotData>> {
        self.slot_datas
            .iter()
            .find(|slot_data| slot_data.node_id == node_id && slot_data.slot_id == slot_id)
            .ok_or(TexProError::NoSlotData)
    }

    pub fn new_id(&mut self) -> NodeId {
        self.node_graph.new_id()
    }

    pub fn add_node(&mut self, node: Node) -> Result<NodeId> {
        let priority = Arc::clone(&node.priority);
        let node_id = self.node_graph.add_node(node)?;

        self.add_node_internal(priority, node_id);

        Ok(node_id)
    }

    pub fn add_node_with_id(&mut self, node: Node) -> Result<()> {
        let priority = Arc::clone(&node.priority);
        let node_id = node.node_id;

        self.node_graph.add_node_with_id(node)?;

        self.add_node_internal(priority, node_id);

        Ok(())
    }

    fn add_node_internal(&mut self, priority: Arc<Priority>, node_id: NodeId) {
        self.changed.insert(node_id);
        self.node_state.insert(node_id, NodeState::Dirty);
        self.priority_propagator.push_priority(node_id, priority);
    }

    pub fn remove_node(&mut self, node_id: NodeId) -> Result<Vec<Edge>> {
        let (_, edges) = self.node_graph.remove_node(node_id)?;

        self.changed.insert(node_id);

        {
            // Also mark anything that had this node as input as changed.
            let mut node_ids = edges
                .iter()
                .map(|edge| edge.input_id)
                .collect::<Vec<NodeId>>();
            node_ids.sort_unstable();
            node_ids.dedup();
            for node_id in node_ids {
                self.changed.insert(node_id);
            }
        }

        self.remove_nodes_data(node_id);

        self.node_state.remove(&node_id);

        Ok(edges)
    }

    pub fn can_connect(
        &self,
        output_node_id: NodeId,
        input_node_id: NodeId,
        output_slot_id: SlotId,
        input_slot_id: SlotId,
    ) -> Result<()> {
        self.node_graph
            .can_connect(output_node_id, input_node_id, output_slot_id, input_slot_id)
    }

    pub fn connect(
        &mut self,
        output_node: NodeId,
        input_node: NodeId,
        output_slot: SlotId,
        input_slot: SlotId,
    ) -> Result<Edge> {
        let edge = *self
            .node_graph
            .connect(output_node, input_node, output_slot, input_slot)?;

        self.changed.insert(input_node);
        self.node(output_node)?.priority.touch();
        self.set_state(input_node, NodeState::Dirty)?;

        if let Ok(node) = self.node(input_node) {
            node.cancel.store(true, Ordering::Relaxed);
        } else {
            // Assume the node has been removed.
            return Err(TexProError::InvalidNodeId);
        }

        Ok(edge)
    }

    /// Sets the state of a node and adds it to the `changed` list. This function should be used
    /// any time a `Node`'s state is changed to keep it up to date.
    pub(crate) fn set_state(&mut self, node_id: NodeId, node_state: NodeState) -> Result<()> {
        let node_state_old = self.node_state(node_id)?;

        if node_state != node_state_old {
            // If the state becomes dirty, propagate it to all children.
            if node_state == NodeState::Dirty {
                for node_id in self.node_graph.get_children(node_id)? {
                    self.set_state(node_id, node_state)?;
                }
            }

            *self.node_state_mut(node_id)? =
                if node_state == NodeState::Dirty && node_state_old == NodeState::Processing {
                    NodeState::ProcessingDirty
                } else {
                    node_state
                };

            self.changed.insert(node_id);
        }

        Ok(())
    }

    /// Both sets the state as usual, and forces the node to be in the current state. This is
    /// uesful for instance when going from `ProcessingDirty` to `Dirty`, as that transition will
    /// just become `ProcessingDirty` again unless forced.
    pub(crate) fn force_state(&mut self, node_id: NodeId, node_state: NodeState) -> Result<()> {
        self.set_state(node_id, node_state)?;

        let node_state_mut = self.node_state_mut(node_id)?;
        *node_state_mut = node_state;

        Ok(())
    }

    pub fn remove_edge(&mut self, edge: Edge) -> Result<Edge> {
        let mut dirty_nodes = self.node_graph.get_children_recursive(edge.input_id)?;
        dirty_nodes.push(edge.input_id);
        dirty_nodes.sort_unstable();
        dirty_nodes.dedup();

        let edge = self.node_graph.remove_edge(edge)?;

        for node_id in dirty_nodes {
            self.set_state(node_id, NodeState::Dirty)?;
            self.node(edge.output_id)?.priority.touch();
            self.remove_nodes_data(node_id);
        }

        Ok(edge)
    }

    pub fn disconnect_slot(
        &mut self,
        node_id: NodeId,
        side: Side,
        slot_id: SlotId,
    ) -> Result<Vec<Edge>> {
        let edges = self.node_graph.disconnect_slot(node_id, side, slot_id)?;

        let mut dirty_nodes = Vec::new();
        for edge in &edges {
            dirty_nodes.append(&mut self.node_graph.get_children_recursive(edge.input_id)?);
            self.node(edge.output_id)?.priority.touch();
        }
        if side == Side::Input {
            dirty_nodes.push(node_id);
        } else {
            self.changed.insert(node_id);
        }
        dirty_nodes.sort_unstable();
        dirty_nodes.dedup();

        for node_id in dirty_nodes.into_iter() {
            self.set_state(node_id, NodeState::Dirty)?;
        }

        Ok(edges)
    }

    pub fn connected_edges(
        &self,
        node_id: NodeId,
        side: Side,
        slot_id: SlotId,
    ) -> Result<Vec<Edge>> {
        self.node_graph.connected_edges(node_id, side, slot_id)
    }

    pub fn set_node_graph(&mut self, node_graph: NodeGraph) {
        self.node_graph = node_graph;
        self.reset_node_states();
        self.slot_datas.clear();
    }

    /// Clears all node states and resets them to dirty.
    ///
    /// Note: It's important that this function does not use `set_state()`.
    pub(crate) fn reset_node_states(&mut self) {
        self.node_state.clear();
        for node_id in self.node_ids() {
            self.node_state.insert(node_id, NodeState::default());
        }
    }

    pub fn output_ids(&self) -> Vec<NodeId> {
        self.node_graph.output_ids()
    }

    pub fn rename_output_node(&mut self, node_id: NodeId, new_name: &str) -> Result<String> {
        self.node_graph.rename_output_node(node_id, new_name)
    }

    pub fn node_ids(&self) -> Vec<NodeId> {
        self.node_graph.node_ids()
    }

    pub fn edges(&self) -> Vec<Edge> {
        self.node_graph.edges.to_owned()
    }

    pub(crate) fn drop_unused_live_graphs(live_graphs: &mut Vec<Arc<RwLock<LiveGraph>>>) {
        for i in (0..live_graphs.len()).rev() {
            if Arc::strong_count(&live_graphs[i]) == 1 {
                live_graphs.remove(i);
                continue;
            }
        }
    }
}
