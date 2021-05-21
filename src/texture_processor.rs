use crate::{
    engine::*,
    error::{Result, TexProError},
    node::{EmbeddedNodeDataId, Node, Side},
    node_graph::*,
    slot_data::*,
};
use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, RwLock, RwLockReadGuard, RwLockWriteGuard,
    },
    thread,
};

#[derive(Default)]
pub struct TextureProcessor {
    engine: Arc<RwLock<Engine>>,
    shutdown: Arc<AtomicBool>,
}

impl Drop for TextureProcessor {
    fn drop(&mut self) {
        self.shutdown.store(true, Ordering::Relaxed);
    }
}

impl TextureProcessor {
    pub fn new() -> Self {
        let shutdown = Arc::new(AtomicBool::new(false));
        let engine = Arc::new(RwLock::new(Engine::new()));

        let output = Self {
            engine: Arc::clone(&engine),
            shutdown: Arc::clone(&shutdown),
        };

        thread::spawn(move || {
            Engine::process_loop(engine, shutdown);
        });

        output
    }

    pub fn engine(&self) -> Arc<RwLock<Engine>> {
        Arc::clone(&self.engine)
    }

    pub fn get_output_rgba(&self, node_id: NodeId, slot_id: SlotId) -> Result<Vec<u8>> {
        Ok(self
            .wait_for_state_read(node_id, NodeState::Clean)?
            .slot_data(node_id, slot_id)
            .ok_or(TexProError::InvalidSlotId)?
            .image
            .to_rgba())
    }

    /// Tries to get the output of a node. If it can't it submits a request for it.
    pub fn try_get_output_rgba(&self, node_id: NodeId, slot_id: SlotId) -> Result<Vec<u8>> {
        let result = if let Ok(engine) = self.engine.try_read() {
            if let Ok(node_state) = engine.node_state(node_id) {
                if node_state == NodeState::Clean {
                    Ok(engine
                        .slot_data(node_id, slot_id)
                        .ok_or(TexProError::InvalidSlotId)?
                        .image
                        .to_rgba())
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
            self.engine.write().unwrap().request(node_id)?
        }

        result
    }

    pub fn process_then_kill(&self) {
        self.engine.write().unwrap().process_then_kill();
    }

    pub fn input_mapping(&self, external_slot: SlotId) -> Result<(NodeId, SlotId)> {
        self.engine
            .read()
            .unwrap()
            .node_graph
            .input_mapping(external_slot)
    }

    pub fn external_output_ids(&self) -> Vec<NodeId> {
        self.engine.read().unwrap().node_graph.output_ids()
    }

    pub fn set_node_graph(&self, node_graph: NodeGraph) -> Result<()> {
        self.engine.write()?.set_node_graph(node_graph);

        Ok(())
    }

    pub fn input_slot_datas_push(&self, node_data: Arc<SlotData>) {
        self.engine
            .write()
            .unwrap()
            .input_node_datas
            .push(node_data);
    }

    pub fn slot_datas(&self) -> Vec<Arc<SlotData>> {
        self.engine.read().unwrap().slot_datas()
    }

    pub fn node_slot_datas(&self, node_id: NodeId) -> Result<Vec<Arc<SlotData>>> {
        Ok(self
            .wait_for_state_read(node_id, NodeState::Clean)?
            .node_slot_datas(node_id))
    }

    pub fn add_node(&self, node: Node) -> Result<NodeId> {
        self.engine.write().unwrap().add_node(node)
    }

    pub fn add_node_with_id(&self, node: Node, node_id: NodeId) -> Result<NodeId> {
        self.engine.write().unwrap().add_node_with_id(node, node_id)
    }

    pub fn remove_node(&self, node_id: NodeId) -> Result<Vec<Edge>> {
        self.engine.write().unwrap().remove_node(node_id)
    }

    /// Returns a vector of `NodeId`s that are not clean. That is, not up to date compared to the
    /// state of the graph.
    pub fn non_clean(&self) -> Vec<NodeId> {
        self.engine.read().unwrap().non_clean()
    }

    pub fn node_ids(&self) -> Vec<NodeId> {
        self.engine.read().unwrap().node_ids()
    }

    pub fn edges(&self) -> Vec<Edge> {
        self.engine.read().unwrap().edges()
    }

    pub fn connect_arbitrary(
        &self,
        a_node: NodeId,
        a_side: Side,
        a_slot: SlotId,
        b_node: NodeId,
        b_side: Side,
        b_slot: SlotId,
    ) -> Result<Edge> {
        self.engine
            .write()
            .unwrap()
            .connect_arbitrary(a_node, a_side, a_slot, b_node, b_side, b_slot)
    }

    pub fn disconnect_slot(
        &self,
        node_id: NodeId,
        side: Side,
        slot_id: SlotId,
    ) -> Result<Vec<Edge>> {
        self.engine
            .write()
            .unwrap()
            .disconnect_slot(node_id, side, slot_id)
    }

    pub fn node_slot_data(&self, node_id: NodeId) -> Result<Vec<Arc<SlotData>>> {
        Ok(self
            .wait_for_state_read(node_id, NodeState::Clean)?
            .node_slot_datas(node_id))
    }

    pub fn wait_for_state_write(
        &self,
        node_id: NodeId,
        node_state: NodeState,
    ) -> Result<RwLockWriteGuard<Engine>> {
        Engine::wait_for_state_write(&self.engine, node_id, node_state)
    }

    pub fn wait_for_state_read(
        &self,
        node_id: NodeId,
        node_state: NodeState,
    ) -> Result<RwLockReadGuard<Engine>> {
        Engine::wait_for_state_read(&self.engine, node_id, node_state)
    }

    pub fn node_state(&self, node_id: NodeId) -> Result<NodeState> {
        self.engine.read()?.node_state(node_id)
    }

    /// Returns all `NodeId`s with the given `NodeState`.
    pub fn node_ids_with_state(&self, node_state: NodeState) -> Vec<NodeId> {
        self.engine.read().unwrap().node_ids_with_state(node_state)
    }

    pub fn embed_slot_data_with_id(
        &self,
        slot_data: Arc<SlotData>,
        id: EmbeddedNodeDataId,
    ) -> Result<EmbeddedNodeDataId> {
        self.engine
            .write()
            .unwrap()
            .embed_node_data_with_id(slot_data, id)
    }

    /// Returns the size of a given `SlotData`.
    pub fn await_slot_data_size(&self, node_id: NodeId, slot_id: SlotId) -> Result<Size> {
        // This mgiht be able to work without any actual existing `SlotData`. It might be possible
        // to calculate what the output size would be if the `SlotData` existed, without looking
        // at an actual `SlotData`.
        self.engine.write().unwrap().prioritise(node_id)?;

        loop {
            if let Ok(engine) = self.engine.try_read() {
                if let Ok(size) = engine.get_slot_data_size(node_id, slot_id) {
                    return Ok(size);
                }
            }
        }
    }

    /// Returns the size of a given `SlotData`.
    pub fn try_get_slot_data_size(&self, node_id: NodeId, slot_id: SlotId) -> Result<Size> {
        // This mgiht be able to work without any actual existing `SlotData`. It might be possible
        // to calculate what the output size would be if the `SlotData` existed, without looking
        // at an actual `SlotData`.
        self.engine.write().unwrap().request(node_id)?;
        let engine = self.engine.try_read()?;
        engine.get_slot_data_size(node_id, slot_id)
    }

    pub fn connect(
        &self,
        output_node: NodeId,
        input_node: NodeId,
        output_slot: SlotId,
        input_slot: SlotId,
    ) -> Result<()> {
        self.engine
            .write()
            .unwrap()
            .connect(output_node, input_node, output_slot, input_slot)
    }

    pub fn node_with_id(&self, node_id: NodeId) -> Result<Node> {
        self.engine.read().unwrap().node_graph.node_with_id(node_id)
    }

    pub fn set_node_with_id(&self, node_id: NodeId, node: Node) -> Result<()> {
        let mut engine = self.engine.write().unwrap();
        let found_node = engine
            .node_graph
            .nodes
            .iter_mut()
            .find(|node| node.node_id == node_id)
            .ok_or(TexProError::InvalidNodeId)?;
        *found_node = node;

        Ok(())
    }

    /// Return all changed `NodeId`s.
    pub fn changed_consume(&self) -> Vec<NodeId> {
        self.engine.write().unwrap().changed_consume()
    }

    pub fn has_node_with_id(&self, node_id: NodeId) -> Result<()> {
        self.engine.read().unwrap().has_node_with_id(node_id)
    }
}
