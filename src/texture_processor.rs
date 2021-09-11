use crate::{
    edge::Edge,
    engine::*,
    error::{Result, TexProError},
    node::{Node, Side},
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
        let engine = Arc::new(RwLock::new(Engine::new(Arc::clone(&shutdown))));

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

    pub fn buffer_rgba(&mut self, node_id: NodeId, slot_id: SlotId) -> Result<Vec<u8>> {
        self.wait_for_state_write(node_id, NodeState::Clean)?
            .buffer_rgba(node_id, slot_id)
    }

    /// Tries to get the output of a node. If it can't it submits a request for it.
    pub fn try_buffer_rgba(&self, node_id: NodeId, slot_id: SlotId) -> Result<Vec<u8>> {
        let result = if let Ok(engine) = self.engine.try_write() {
            if let Ok(node_state) = engine.node_state(node_id) {
                if node_state == NodeState::Clean {
                    engine.slot_data(node_id, slot_id)?.image.to_u8()
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

    /// Tries to get the output of a node. If it can't it submits a request for it.
    pub fn try_buffer_srgba(&self, node_id: NodeId, slot_id: SlotId) -> Result<Vec<u8>> {
        let result = if let Ok(engine) = self.engine.try_write() {
            if let Ok(node_state) = engine.node_state(node_id) {
                if node_state == NodeState::Clean {
                    engine.slot_data(node_id, slot_id)?.image.to_u8_srgb()
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

    pub(crate) fn node_slot_datas(&self, node_id: NodeId) -> Result<Vec<Arc<SlotData>>> {
        self.wait_for_state_write(node_id, NodeState::Clean)?
            .node_slot_datas(node_id)
    }

    pub fn node_slot_datas_new(&self, node_id: NodeId) -> Result<Vec<SlotData>> {
        self.wait_for_state_write(node_id, NodeState::Clean)?
            .node_slot_datas_new(node_id)
    }

    /// Adds a `Node` to the `TextureProcessor`'s `Engine`.
    ///
    /// Returns the `NodeId` of the created `Node`.
    pub fn add_node(&self, node: Node) -> Result<NodeId> {
        self.engine.write().unwrap().add_node(node)
    }

    /// Removes a `Node` from the `TextureProcessor`'s `Engine`.
    ///
    /// Returns all `Edge`s that were connected to the node.
    pub fn remove_node(&self, node_id: NodeId) -> Result<Vec<Edge>> {
        self.engine.write().unwrap().remove_node(node_id)
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

    pub fn slot_data_new(&self, node_id: NodeId, slot_id: SlotId) -> Result<SlotData> {
        self.wait_for_state_write(node_id, NodeState::Clean)?
            .slot_data_new(node_id, slot_id)
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

    /// Returns the size of a given `SlotData`.
    pub fn await_slot_data_size(&self, node_id: NodeId, slot_id: SlotId) -> Result<Size> {
        self.engine.write().unwrap().prioritise(node_id)?;

        loop {
            if let Ok(engine) = self.engine.try_read() {
                if let Ok(size) = engine.slot_data_size(node_id, slot_id) {
                    return Ok(size);
                }
            }
        }
    }

    /// Returns the size of a given `SlotData`.
    pub fn try_get_slot_data_size(&self, node_id: NodeId, slot_id: SlotId) -> Result<Size> {
        self.engine.write().unwrap().request(node_id)?;
        let engine = self.engine.try_read()?;
        engine.slot_data_size(node_id, slot_id)
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
}
