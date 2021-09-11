use crate::{
    engine::*,
    error::{Result, TexProError},
    node_graph::*,
    slot_data::*,
    transient_buffer::{TransientBufferContainer, TransientBufferQueue},
};
use std::{
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc, RwLock,
    },
    thread,
};

// #[derive(Default)]
pub struct TextureProcessor {
    engine: Arc<RwLock<Vec<Arc<RwLock<Engine>>>>>,
    pub shutdown: Arc<AtomicBool>,
    pub add_buffer_queue: Arc<RwLock<Vec<Arc<TransientBufferContainer>>>>,
    pub memory_threshold: Arc<AtomicUsize>,
}

// impl Default for TextureProcessor {
//     fn default() -> Self {
//         const ONE_GB: usize = 1_000_000_000;
//         Self::new(Arc::new(ONE_GB.into()))
//     }
// }

impl Drop for TextureProcessor {
    fn drop(&mut self) {
        self.shutdown.store(true, Ordering::Relaxed);
    }
}

impl TextureProcessor {
    pub fn new(memory_threshold: Arc<AtomicUsize>) -> Arc<Self> {
        let shutdown = Arc::new(AtomicBool::new(false));

        let mut transient_buffer_queue =
            TransientBufferQueue::new(Arc::clone(&memory_threshold), Arc::clone(&shutdown));
        let add_buffer_queue = Arc::clone(&transient_buffer_queue.incoming_buffers);

        // let engine = Arc::new(RwLock::new(Engine::new(Arc::clone(&shutdown), Arc::clone(&add_buffer_queue))));

        let output = Arc::new(Self {
            engine: Arc::new(RwLock::new(Vec::new())),
            shutdown: Arc::clone(&shutdown),
            memory_threshold,
            add_buffer_queue,
        });
        let output_send = Arc::clone(&output);

        thread::spawn(move || {
            Engine::process_loop(output_send);
        });

        thread::spawn(move || {
            transient_buffer_queue.thread_loop();
        });

        output
    }

    pub fn new_engine(&self) -> Result<Arc<RwLock<Engine>>> {
        let engine = Arc::new(RwLock::new(Engine::new(Arc::clone(&self.add_buffer_queue))));
        self.engine.write()?.push(Arc::clone(&engine));
        Ok(engine)
    }

    pub fn add_engine(&self, engine: Arc<RwLock<Engine>>) -> Result<()> {
        self.engine.write()?.push(engine);
        Ok(())
    }

    pub fn engine(&self) -> &Arc<RwLock<Vec<Arc<RwLock<Engine>>>>> {
        &self.engine
    }

    pub fn buffer_rgba(
        engine: &Arc<RwLock<Engine>>,
        node_id: NodeId,
        slot_id: SlotId,
    ) -> Result<Vec<u8>> {
        Engine::wait_for_state_write(engine, node_id, NodeState::Clean)?
            .buffer_rgba(node_id, slot_id)
    }

    pub(crate) fn node_slot_datas(
        engine: &Arc<RwLock<Engine>>,
        node_id: NodeId,
    ) -> Result<Vec<Arc<SlotData>>> {
        Engine::wait_for_state_write(engine, node_id, NodeState::Clean)?.node_slot_datas(node_id)
    }

    pub fn node_slot_datas_new(
        engine: &Arc<RwLock<Engine>>,
        node_id: NodeId,
    ) -> Result<Vec<SlotData>> {
        Engine::wait_for_state_write(engine, node_id, NodeState::Clean)?
            .node_slot_datas_new(node_id)
    }

    // /// Adds a `Node` to the `TextureProcessor`'s `Engine`.
    // ///
    // /// Returns the `NodeId` of the created `Node`.
    // pub fn add_node(&self, node: Node) -> Result<NodeId> {
    //     self.engine.write().unwrap().add_node(node)
    // }

    // /// Removes a `Node` from the `TextureProcessor`'s `Engine`.
    // ///
    // /// Returns all `Edge`s that were connected to the node.
    // pub fn remove_node(&self, node_id: NodeId) -> Result<Vec<Edge>> {
    //     self.engine.write().unwrap().remove_node(node_id)
    // }

    // pub fn connect_arbitrary(
    //     &self,
    //     a_node: NodeId,
    //     a_side: Side,
    //     a_slot: SlotId,
    //     b_node: NodeId,
    //     b_side: Side,
    //     b_slot: SlotId,
    // ) -> Result<Edge> {
    //     self.engine
    //         .write()
    //         .unwrap()
    //         .connect_arbitrary(a_node, a_side, a_slot, b_node, b_side, b_slot)
    // }

    // pub fn disconnect_slot(
    //     &self,
    //     node_id: NodeId,
    //     side: Side,
    //     slot_id: SlotId,
    // ) -> Result<Vec<Edge>> {
    //     self.engine
    //         .write()
    //         .unwrap()
    //         .disconnect_slot(node_id, side, slot_id)
    // }

    // pub fn slot_data_new(
    //     engine: &Arc<RwLock<Engine>>,
    //     node_id: NodeId,
    //     slot_id: SlotId,
    // ) -> Result<SlotData> {
    //     Engine::wait_for_state_write(engine, node_id, NodeState::Clean)?
    //         .slot_data_new(node_id, slot_id)
    // }

    // pub fn wait_for_state_read(
    //     &self,
    //     node_id: NodeId,
    //     node_state: NodeState,
    // ) -> Result<RwLockReadGuard<Engine>> {
    //     Engine::wait_for_state_read(&self.engine, node_id, node_state)
    // }

    // pub fn node_state(&self, node_id: NodeId) -> Result<NodeState> {
    //     self.engine.read()?.node_state(node_id)
    // }

    // /// Returns all `NodeId`s with the given `NodeState`.
    // pub fn node_ids_with_state(&self, node_state: NodeState) -> Vec<NodeId> {
    //     self.engine.read().unwrap().node_ids_with_state(node_state)
    // }

    /// Returns the size of a given `SlotData`.
    pub fn await_slot_data_size(
        engine: &Arc<RwLock<Engine>>,
        node_id: NodeId,
        slot_id: SlotId,
    ) -> Result<Size> {
        engine.write().unwrap().prioritise(node_id)?;

        loop {
            if let Ok(engine) = engine.try_read() {
                if let Ok(size) = engine.slot_data_size(node_id, slot_id) {
                    return Ok(size);
                }
            }
        }
    }

    // /// Returns the size of a given `SlotData`.
    // pub fn try_get_slot_data_size(&self, node_id: NodeId, slot_id: SlotId) -> Result<Size> {
    //     self.engine.write().unwrap().request(node_id)?;
    //     let engine = self.engine.try_read()?;
    //     engine.slot_data_size(node_id, slot_id)
    // }

    // pub fn connect(
    //     &self,
    //     output_node: NodeId,
    //     input_node: NodeId,
    //     output_slot: SlotId,
    //     input_slot: SlotId,
    // ) -> Result<()> {
    //     self.engine
    //         .write()
    //         .unwrap()
    //         .connect(output_node, input_node, output_slot, input_slot)
    // }
}
