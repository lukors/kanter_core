use crate::{
    engine,
    error::Result,
    live_graph::*,
    node_graph::*,
    process_pack::ProcessPackManager,
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

pub struct TextureProcessor {
    live_graph: Arc<RwLock<Vec<Arc<RwLock<LiveGraph>>>>>,
    pub shutdown: Arc<AtomicBool>,
    pub add_buffer_queue: Arc<RwLock<Vec<Arc<TransientBufferContainer>>>>,
    pub memory_threshold: Arc<AtomicUsize>,
    pub(crate) process_pack_manager: RwLock<ProcessPackManager>,
}

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

        let output = Arc::new(Self {
            live_graph: Arc::new(RwLock::new(Vec::new())),
            shutdown: Arc::clone(&shutdown),
            memory_threshold,
            add_buffer_queue,
            process_pack_manager: RwLock::new(ProcessPackManager::new()),
        });
        let output_send = Arc::clone(&output);

        thread::spawn(move || {
            engine::process_loop(output_send);
        });

        thread::spawn(move || {
            transient_buffer_queue.thread_loop();
        });

        output
    }

    pub fn new_live_graph(&self) -> Result<Arc<RwLock<LiveGraph>>> {
        let live_graph = Arc::new(RwLock::new(LiveGraph::new(Arc::clone(
            &self.add_buffer_queue,
        ))));
        self.live_graph.write()?.push(Arc::clone(&live_graph));
        Ok(live_graph)
    }

    pub fn push_live_graph(&self, live_graph: Arc<RwLock<LiveGraph>>) -> Result<()> {
        self.live_graph.write()?.push(live_graph);
        Ok(())
    }

    pub fn live_graph(&self) -> &Arc<RwLock<Vec<Arc<RwLock<LiveGraph>>>>> {
        &self.live_graph
    }

    pub fn buffer_rgba(
        live_graph: &Arc<RwLock<LiveGraph>>,
        node_id: NodeId,
        slot_id: SlotId,
    ) -> Result<Vec<u8>> {
        LiveGraph::await_clean_write(live_graph, node_id)?.buffer_rgba(node_id, slot_id)
    }

    pub fn node_slot_datas_new(
        live_graph: &Arc<RwLock<LiveGraph>>,
        node_id: NodeId,
    ) -> Result<Vec<SlotData>> {
        LiveGraph::await_clean_write(live_graph, node_id)?.node_slot_datas_new(node_id)
    }

    /// Returns the size of a given `SlotData`.
    pub fn await_slot_data_size(
        live_graph: &Arc<RwLock<LiveGraph>>,
        node_id: NodeId,
        slot_id: SlotId,
    ) -> Result<Size> {
        live_graph.write().unwrap().prioritise(node_id)?;

        loop {
            if let Ok(live_graph) = live_graph.try_read() {
                if let Ok(size) = live_graph.slot_data_size(node_id, slot_id) {
                    return Ok(size);
                }
            }
        }
    }

    pub fn processing_node_count(&self) -> Result<usize> {
        Ok(self.process_pack_manager.read()?.process_packs().len())
    }
}
