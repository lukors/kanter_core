use crate::{
    error::{Result, TexProError},
    node::{
        embed::{EmbeddedSlotData, EmbeddedSlotDataId},
        node_type::process_node,
        Node, Side, SlotType,
    },
    node_graph::*,
    shared::calculate_size,
    slot_data::*,
};
use image::ImageBuffer;
use std::{
    collections::{BTreeMap, BTreeSet, VecDeque},
    mem::size_of,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc, Arc, RwLock, RwLockReadGuard, RwLockWriteGuard,
    },
    thread,
};

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

#[derive(Default)]
pub struct Engine {
    pub node_graph: NodeGraph,
    pub slot_datas: VecDeque<Arc<SlotData>>,
    pub embedded_slot_datas: Vec<Arc<EmbeddedSlotData>>,
    pub input_slot_datas: Vec<Arc<SlotData>>,
    node_state: BTreeMap<NodeId, NodeState>,
    changed: BTreeSet<NodeId>,
    one_shot: bool,
    pub auto_update: bool,
    pub use_cache: bool,
    pub slot_data_ram_cap: usize,
}

impl Engine {
    pub fn new() -> Self {
        Self {
            node_graph: NodeGraph::new(),
            slot_datas: VecDeque::new(),
            embedded_slot_datas: Vec::new(),
            input_slot_datas: Vec::new(),
            node_state: BTreeMap::new(),
            changed: BTreeSet::new(),
            one_shot: false,
            auto_update: false,
            use_cache: false,
            slot_data_ram_cap: 1_073_742_000, // 1 Gib
        }
    }

    pub(crate) fn process_loop(tex_pro: Arc<RwLock<Engine>>, shutdown: Arc<AtomicBool>) {
        struct ThreadMessage {
            node_id: NodeId,
            slot_datas: Result<Vec<Arc<SlotData>>>,
        }
        let (send, recv) = mpsc::channel::<ThreadMessage>();

        loop {
            if shutdown.load(Ordering::Relaxed) {
                return;
            }

            // PLAN
            // - no plan here yet, maybe loop the async writing from ram to disk here?

            if let Ok(mut tex_pro) = tex_pro.write() {
                // Handle messages received from node processing threads.
                for message in recv.try_iter() {
                    let node_id = message.node_id;
                    let slot_datas = message.slot_datas;

                    match slot_datas {
                        Ok(slot_datas) => {
                            tex_pro.remove_nodes_data(node_id);
                            tex_pro.slot_datas.append(&mut slot_datas.into());
                        }
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

                    if tex_pro.set_state(node_id, NodeState::Clean).is_err() {
                        shutdown.store(true, Ordering::Relaxed);
                        return;
                    }

                    if !tex_pro.use_cache {
                        for parent in tex_pro.get_parents(node_id) {
                            if tex_pro
                                .get_children(parent)
                                .iter()
                                .flatten()
                                .all(|node_id| {
                                    matches![
                                        tex_pro.node_state(*node_id).unwrap(),
                                        NodeState::Clean | NodeState::Processing
                                    ]
                                })
                            {
                                tex_pro.remove_nodes_data(parent);
                            }
                        }
                    }
                }

                // Get requested nodes
                let requested = if tex_pro.auto_update {
                    tex_pro
                        .node_state
                        .iter()
                        .filter(|(_, node_state)| {
                            !matches!(node_state, NodeState::Processing | NodeState::Clean)
                        })
                        .map(|(node_id, _)| *node_id)
                        .collect::<Vec<NodeId>>()
                } else {
                    tex_pro
                        .node_state
                        .iter()
                        .filter(|(_, node_state)| {
                            matches!(node_state, NodeState::Requested | NodeState::Prioritised)
                        })
                        .map(|(node_id, _)| *node_id)
                        .collect::<Vec<NodeId>>()
                };

                // Get the closest non-clean parents
                let mut closest_processable = Vec::new();
                for node_id in requested {
                    closest_processable.append(&mut tex_pro.get_closest_processable(node_id));
                }
                closest_processable.sort_unstable();
                closest_processable.dedup();

                for node_id in closest_processable {
                    *tex_pro.node_state_mut(node_id).unwrap() = NodeState::Processing;

                    let node = tex_pro.node_graph.node_with_id(node_id).unwrap();

                    let embedded_node_datas: Vec<Arc<EmbeddedSlotData>> = tex_pro
                        .embedded_slot_datas
                        .iter()
                        .map(|end| Arc::clone(&end))
                        .collect();

                    let input_node_datas: Vec<Arc<SlotData>> = tex_pro
                        .input_slot_datas
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
                        edges
                            .iter()
                            .map(|edge| {
                                let output = tex_pro
                                    .slot_datas
                                    .iter()
                                    .find(|slot_data| {
                                        slot_data.slot_id == edge.output_slot
                                            && slot_data.node_id == edge.output_id
                                    })
                                    .cloned();

                                output.unwrap_or({
                                    Arc::new(SlotData::from_slot_image(
                                        edge.output_id,
                                        edge.output_slot,
                                        Size::new(1, 1),
                                        SlotImage::Gray(Arc::new(Box::new(
                                            ImageBuffer::from_raw(1, 1, vec![0.0]).unwrap(),
                                        ))),
                                    ))
                                })
                            })
                            .collect::<Vec<Arc<SlotData>>>()
                    };

                    while tex_pro.slot_data_bytes_total() > tex_pro.slot_data_ram_cap {
                        if let Some(slot_data_in_ram) = tex_pro
                            .slot_datas
                            .iter()
                            .find(|slot_data| slot_data.image_cache().read().unwrap().is_in_ram())
                        {
                            slot_data_in_ram.store().unwrap();
                        }
                    }

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
                if tex_pro.one_shot
                    && tex_pro
                        .node_state
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

    // fn total_slot_data_size(&self) -> usize {
    //     self.slot_datas
    //         .iter()
    //         .map(|slot_data| slot_data.size.pixel_count())
    //         .sum::<usize>()
    //         * size_of::<ChannelPixel>()
    // }

    fn slot_data_bytes_total(&self) -> usize {
        // self.node_state
        //     .values()
        //     .map(|node_info| node_info.slot_data_bytes)
        //     .sum()
        self.slot_datas()
            .iter()
            .filter(|slot_data| slot_data.image_cache().read().unwrap().is_in_ram())
            .map(|slot_data| slot_data.bytes())
            .sum()
    }

    // fn bytes_needed_for_node(&self, node_id: NodeId) -> Result<usize> {
    //     let node = self.node_graph.node_with_id(node_id)?;

    //     let slot_datas = self
    //         .get_parents(node_id)
    //         .iter()
    //         .map(|node_id| self.node_slot_datas(*node_id))
    //         .flatten()
    //         .collect::<Vec<Arc<SlotData>>>();
    //     let edges = self.node_graph.input_edges(node_id);
    //     let policy = node.resize_policy;

    //     let channel_count: usize = node
    //         .output_slots()
    //         .iter()
    //         .map(|slot| {
    //             match slot.slot_type {
    //                 SlotType::Gray => 1,
    //                 SlotType::Rgba => 4,
    //                 SlotType::GrayOrRgba => 4, // Assume it's RGBA if it can be.
    //             }
    //         })
    //         .sum();

    //     Ok(calculate_size(&slot_datas, &edges, policy).pixel_count()
    //         * channel_count
    //         * size_of::<ChannelPixel>())
    // }

    /// Return a SlotData as u8.
    pub fn buffer_rgba(&self, node_id: NodeId, slot_id: SlotId) -> Result<Vec<u8>> {
        // Ok((*self.slot_data(node_id, slot_id)?.image.read().unwrap()).get().to_u8())
        Ok(self
            .slot_data(node_id, slot_id)?
            .image_cache()
            .write()
            .unwrap()
            .get()
            .to_u8())
    }

    /// Return all changed `NodeId`s.
    pub fn changed_consume(&mut self) -> Vec<NodeId> {
        let output = self.changed.iter().copied().collect();
        self.changed.clear();
        output
    }

    pub fn has_node_with_id(&self, node_id: NodeId) -> Result<()> {
        self.node_graph.has_node_with_id(node_id)
    }

    /// Waits until a certain NodeId has a certain state, and when it does it returns the
    /// `RwLockWriteGuard` so changes can be made while the `NodeState` the state remains the same.
    pub fn wait_for_state_write(
        engine: &Arc<RwLock<Self>>,
        node_id: NodeId,
        node_state: NodeState,
    ) -> Result<RwLockWriteGuard<Engine>> {
        loop {
            if let Ok(mut engine) = engine.write() {
                if node_state == engine.node_state(node_id)? {
                    return Ok(engine);
                } else {
                    engine.prioritise(node_id)?;
                }
            }
        }
    }

    /// Waits until a certain NodeId has a certain state, and when it does it returns the
    /// `RwLockReadGuard` so reads can be made while the `NodeState` remains the same.
    pub fn wait_for_state_read(
        engine: &Arc<RwLock<Self>>,
        node_id: NodeId,
        node_state: NodeState,
    ) -> Result<RwLockReadGuard<Engine>> {
        loop {
            if let Ok(engine) = engine.read() {
                if node_state == engine.node_state(node_id)? {
                    return Ok(engine);
                }
            }

            engine.write().unwrap().prioritise(node_id)?;
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

    /// Gets all `NodeId`s that are not clean.
    pub fn non_clean(&self) -> Vec<NodeId> {
        self.node_state
            .iter()
            .filter(|(_, node_state)| **node_state != NodeState::Clean)
            .map(|(node_id, _)| *node_id)
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

        let mut children = self
            .node_graph
            .edges
            .iter()
            .filter(|edge| edge.output_id == node_id)
            .map(|edge| edge.input_id)
            .collect::<Vec<NodeId>>();

        children.sort_unstable();
        children.dedup();

        Ok(children)
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
        let mut node_ids = self
            .node_graph
            .edges
            .iter()
            .filter(|edge| edge.input_id == node_id)
            .map(|edge| edge.output_id)
            .collect::<Vec<NodeId>>();

        node_ids.sort_unstable();
        node_ids.dedup();
        node_ids
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

    /// Returns the NodeIds of the closest ancestors that are ready to be processed, including self.
    pub fn get_closest_processable(&self, node_id: NodeId) -> Vec<NodeId> {
        let mut closest_processable = Vec::new();

        // Put dirty and processing parents in their own vectors.
        let mut dirty = Vec::new();
        let mut processing = Vec::new();
        for node_id in self.get_parents(node_id) {
            match self.node_state(node_id).unwrap() {
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

    /// Returns the `NodeId`s of all ancestors until a node with the given `NodeState` is found in the first Vec.
    ///
    /// Also returns the nodes with the state in the second Vec.
    ///
    /// Errors if any branch ends without finding a node in the given state.
    pub fn get_ancestors_until_state_strict(
        &self,
        node_id: NodeId,
        node_states: &[NodeState],
    ) -> Result<(Vec<NodeId>, Vec<NodeId>)> {
        let mut node_ids_with_state = Vec::new();
        let mut node_ids_on_way = Vec::new();

        for node_state in node_states.iter() {
            if self.node_state(node_id).unwrap() == *node_state {
                node_ids_with_state.push(node_id);
            }
        }

        if node_ids_with_state.is_empty() {
            node_ids_on_way.push(node_id);

            let parent_node_ids = self.get_parents(node_id);
            if parent_node_ids.is_empty() {
                return Err(TexProError::InvalidNodeId);
            }

            for node_id in parent_node_ids {
                let (on_way, with_state) =
                    &mut self.get_ancestors_until_state_strict(node_id, node_states)?;
                node_ids_on_way.append(on_way);
                node_ids_with_state.append(with_state);
            }
        }

        node_ids_on_way.sort_unstable();
        node_ids_on_way.dedup();

        node_ids_with_state.sort_unstable();
        node_ids_with_state.dedup();

        Ok((node_ids_on_way, node_ids_with_state))
    }

    /// Returns the `Size` of the `SlotData` for the given `NodeId` and `SlotId`.
    pub fn get_slot_data_size(&self, node_id: NodeId, slot_id: SlotId) -> Result<Size> {
        if self.node_state(node_id)? == NodeState::Clean {
            if let Some(node_data) = self
                .slot_datas
                .iter()
                .find(|nd| nd.node_id == node_id && nd.slot_id == slot_id)
            {
                Ok(node_data.size)
            } else {
                Err(TexProError::InvalidBufferCount)
            }
        } else {
            Err(TexProError::NodeDirty)
        }
    }

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
        id: EmbeddedSlotDataId,
    ) -> Result<EmbeddedSlotDataId> {
        if self
            .embedded_slot_datas
            .iter()
            .all(|end| end.slot_data_id != id)
        {
            self.embedded_slot_datas
                .push(Arc::new(EmbeddedSlotData::from_slot_data(node_data, id)));
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

    pub fn node_with_id_mut(&mut self, node_id: NodeId) -> Result<&mut Node> {
        self.set_state(node_id, NodeState::Dirty)?;
        self.node_graph
            .node_with_id_mut(node_id)
            .ok_or(TexProError::InvalidNodeId)
    }

    /// Gets all `SlotData`s in this `TextureProcessor`.
    pub fn slot_datas(&self) -> Vec<Arc<SlotData>> {
        self.slot_datas.clone().into()
    }

    /// Gets all output `SlotData`s in this `TextureProcessor`.
    pub fn slot_datas_output(&self) -> Vec<Arc<SlotData>> {
        self.slot_datas
            .iter()
            .filter(|slot_data| {
                if let Ok(node) = self.node_graph.node_with_id(slot_data.node_id) {
                    node.node_type.is_output()
                } else {
                    false
                }
            })
            .cloned()
            .collect()
    }

    /// Gets any `SlotData`s associated with a given `NodeId`.
    pub fn node_slot_datas(&self, node_id: NodeId) -> Vec<Arc<SlotData>> {
        self.slot_datas
            .iter()
            .filter(|nd| nd.node_id == node_id)
            .map(|nd| Arc::clone(&nd))
            .collect()
    }

    pub fn slot_data(&self, node_id: NodeId, slot_id: SlotId) -> Result<Arc<SlotData>> {
        self.node_slot_datas(node_id)
            .iter()
            .find(|slot_data| slot_data.slot_id == slot_id)
            .cloned()
            .ok_or(TexProError::InvalidSlotId)
    }

    pub fn add_node_with_id(&mut self, node: Node, node_id: NodeId) -> Result<NodeId> {
        let node_id = self.node_graph.add_node_with_id(node, node_id)?;
        self.changed.insert(node_id);
        // self.node_generation_add(node_id)?;
        Ok(node_id)
    }

    pub fn add_node(&mut self, node: Node) -> Result<NodeId> {
        let node_id = self.node_graph.add_node(node)?;

        self.changed.insert(node_id);
        self.node_state.insert(node_id, NodeState::Dirty);

        Ok(node_id)
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

        self.node_state.remove(&node_id);

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

        self.changed.insert(input_node);
        self.set_state(input_node, NodeState::Dirty)?;

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
    ) -> Result<Edge> {
        let new_edge = *self
            .node_graph
            .connect_arbitrary(a_node, a_side, a_slot, b_node, b_side, b_slot)?;

        self.changed.insert(new_edge.input_id);
        self.set_state(new_edge.input_id, NodeState::Dirty)?;

        Ok(new_edge)
    }

    /// Sets the state of a node and updates the `state_generation`. This function should be used
    /// any time a `Node`'s state is changed to ensure the node's `state_generation` is kept up to
    /// date.
    fn set_state(&mut self, node_id: NodeId, node_state: NodeState) -> Result<()> {
        let node_state_old = self.node_state(node_id)?;

        if node_state != node_state_old {
            // If the state becomes dirty, propagate it to all children.
            if node_state == NodeState::Dirty {
                for node_id in self.get_children(node_id)? {
                    self.set_state(node_id, node_state)?;
                }
            }

            self.changed.insert(node_id);
            *self.node_state_mut(node_id)? = node_state;
        }

        Ok(())
    }

    pub fn disconnect_slot(
        &mut self,
        node_id: NodeId,
        side: Side,
        slot_id: SlotId,
    ) -> Result<Vec<Edge>> {
        let edges = self.node_graph.disconnect_slot(node_id, side, slot_id)?;

        let mut disconnected_children = Vec::new();
        for edge in &edges {
            disconnected_children.append(&mut self.get_children_recursive(edge.input_id)?);
        }
        disconnected_children.sort_unstable();
        disconnected_children.dedup();

        for node_id in disconnected_children.into_iter().chain(vec![node_id]) {
            self.set_state(node_id, NodeState::Dirty)?;
        }

        Ok(edges)
    }

    pub fn set_node_graph(&mut self, node_graph: NodeGraph) {
        self.node_graph = node_graph;
        self.reset_node_states();
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

    pub fn node_ids(&self) -> Vec<NodeId> {
        self.node_graph.node_ids()
    }

    pub fn edges(&self) -> Vec<Edge> {
        self.node_graph.edges.to_owned()
    }
}
