use crate::{
    dag::*,
    error::{Result, TexProError},
    node::{EmbeddedNodeDataId, Node, Side},
    node_graph::*,
    slot_data::*,
};
use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, RwLock,
    },
    thread,
};

#[derive(Default)]
pub struct TextureProcessor {
    tpi: Arc<RwLock<TexProInt>>,
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
        let tpi = Arc::new(RwLock::new(TexProInt::new()));

        let output = Self {
            tpi: Arc::clone(&tpi),
            shutdown: Arc::clone(&shutdown),
        };

        thread::spawn(move || {
            TexProInt::process_loop(tpi, shutdown);
        });

        output
    }

    pub fn tex_pro_int(&self) -> Arc<RwLock<TexProInt>> {
        Arc::clone(&self.tpi)
    }

    // pub fn process(&self) {
    //     TexProInt::process(Arc::clone(&self.tpi));
    // }

    pub fn get_output(&self, node_id: NodeId) -> Result<Vec<u8>> {
        self.tex_pro_int().write().unwrap().prioritise(node_id)?;
        
        loop {
            if let Ok(tpi) = self.tpi.try_read() {
                if let Ok(output) = tpi.get_output(node_id) {
                    return Ok(output);
                }
            }
        }
    }

    pub fn try_get_output(&self, node_id: NodeId) -> Result<Vec<u8>> {
        if let Ok(tpi) = self.tpi.try_read() {
            if tpi.node_state(node_id) == NodeState::Clean {
                tpi.get_output(node_id) // TODO: need to make a request here if unable to get the output
            }
        } else {
            Err(TexProError::UnableToLock)
        }
    }

    pub fn input_mapping(&self, external_slot: SlotId) -> Result<(NodeId, SlotId)> {
        self.tpi
            .read()
            .unwrap()
            .node_graph
            .input_mapping(external_slot)
    }

    // pub fn processing(&self) -> bool {
    //     self.tpi.read().unwrap().processing
    // }

    pub fn external_output_ids(&self) -> Vec<NodeId> {
        self.tpi.read().unwrap().node_graph.external_output_ids()
    }

    pub fn set_node_graph(&self, node_graph: NodeGraph) {
        self.tpi.write().unwrap().set_node_graph(node_graph);
    }

    pub fn input_slot_datas_push(&self, node_data: Arc<SlotData>) {
        self.tpi.write().unwrap().input_node_datas.push(node_data);
    }

    pub fn slot_datas(&self) -> Vec<Arc<SlotData>> {
        self.tpi.read().unwrap().slot_datas()
    }

    pub fn node_slot_datas(&self, node_id: NodeId) -> Vec<Arc<SlotData>> {
        loop {
            if let Ok(tpi) = self.tpi.read() {
                if let Ok(node_state) = tpi.node_state(node_id) {
                    if node_state == NodeState::Clean {
                        return tpi.node_slot_datas(node_id);
                    }
                }
            }
        }
    }

    pub fn add_node(&self, node: Node) -> Result<NodeId> {
        self.tpi.write().unwrap().add_node(node)
    }

    pub fn add_node_with_id(&self, node: Node, node_id: NodeId) -> Result<NodeId> {
        self.tpi.write().unwrap().add_node_with_id(node, node_id)
    }

    pub fn remove_node(&self, node_id: NodeId) -> Result<()> {
        self.tpi.write().unwrap().remove_node(node_id)
    }

    /// Returns a vector of `NodeId`s that have been processed and not checked (are clean).
    // pub fn get_clean(&self) -> Vec<NodeId> {
    //     self.tpi.write().unwrap().get_all_clean()
    // }

    /// Returns a vector of `NodeId`s that are not clean. That is, not up to date compared to the
    /// state of the graph.
    pub fn get_dirty(&self) -> Vec<NodeId> {
        self.tpi.read().unwrap().get_dirty()
    }

    pub fn node_ids(&self) -> Vec<NodeId> {
        self.tpi.read().unwrap().node_ids()
    }

    pub fn edges(&self) -> Vec<Edge> {
        self.tpi.read().unwrap().edges()
    }

    pub fn connect_arbitrary(
        &self,
        a_node: NodeId,
        a_side: Side,
        a_slot: SlotId,
        b_node: NodeId,
        b_side: Side,
        b_slot: SlotId,
    ) -> Result<()> {
        self.tpi
            .write()
            .unwrap()
            .connect_arbitrary(a_node, a_side, a_slot, b_node, b_side, b_slot)
    }

    pub fn disconnect_slot(&self, node_id: NodeId, side: Side, slot_id: SlotId) {
        self.tpi
            .write()
            .unwrap()
            .disconnect_slot(node_id, side, slot_id)
    }

    pub fn node_slot_data(&self, id: NodeId) -> Vec<Arc<SlotData>> {
        self.tpi.read().unwrap().node_slot_datas(id)
    }

    // pub fn wait_until_finished(&self) {
    //     loop {
    //         if !self.processing() {
    //             return;
    //         }
    //     }
    // }

    pub fn embed_slot_data_with_id(
        &self,
        slot_data: Arc<SlotData>,
        id: EmbeddedNodeDataId,
    ) -> Result<EmbeddedNodeDataId> {
        self.tpi
            .write()
            .unwrap()
            .embed_node_data_with_id(slot_data, id)
    }

    pub fn get_node_data_size(&self, node_id: NodeId) -> Option<Size> {
        self.tpi.read().unwrap().get_node_data_size(node_id)
    }

    pub fn connect(
        &self,
        output_node: NodeId,
        input_node: NodeId,
        output_slot: SlotId,
        input_slot: SlotId,
    ) -> Result<()> {
        self.tpi
            .write()
            .unwrap()
            .connect(output_node, input_node, output_slot, input_slot)
    }

    pub fn node_with_id(&self, node_id: NodeId) -> Option<Node> {
        self.tpi.read().unwrap().node_graph.node_with_id(node_id)
    }

    pub fn set_node_with_id(&self, node_id: NodeId, node: Node) -> Result<()> {
        let mut tpi = self.tpi.write().unwrap();
        let found_node = tpi
            .node_graph
            .nodes
            .iter_mut()
            .find(|node| node.node_id == node_id)
            .ok_or(TexProError::InvalidNodeId)?;
        *found_node = node;

        Ok(())
    }
}
