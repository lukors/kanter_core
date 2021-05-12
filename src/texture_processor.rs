use crate::{
    dag::*,
    error::{Result, TexProError},
    node::{EmbeddedNodeDataId, Node, Side},
    node_data::*,
    node_graph::*,
};
use std::sync::{Arc, RwLock};

#[derive(Default)]
pub struct TextureProcessor {
    tpi: Arc<RwLock<TexProInt>>,
}

impl TextureProcessor {
    pub fn new() -> Self {
        Self {
            tpi: Arc::new(RwLock::new(TexProInt::new())),
        }
    }

    pub fn process(&self) {
        TexProInt::process(Arc::clone(&self.tpi));
    }

    pub fn get_output(&self, node_id: NodeId) -> Vec<u8> {
        loop {
            if let Ok(tpi) = self.tpi.try_read() {
                if let Ok(output) = tpi.get_output(node_id) {
                    return output;
                }
            }
        }
    }

    pub fn try_get_output(&self, node_id: NodeId) -> Result<Vec<u8>> {
        if let Ok(tpi) = self.tpi.try_read() {
            return tpi.get_output(node_id);
        } else {
            return Err(TexProError::UnableToLock);
        }
    }

    pub fn input_mapping(&self, external_slot: SlotId) -> Result<(NodeId, SlotId)> {
        self.tpi
            .read()
            .unwrap()
            .node_graph
            .input_mapping(external_slot)
    }

    pub fn external_output_ids(&self) -> Vec<NodeId> {
        self.tpi.read().unwrap().node_graph.external_output_ids()
    }

    pub fn node_graph_set(&self, node_graph: NodeGraph) {
        self.tpi.write().unwrap().node_graph = node_graph;
    }

    pub fn input_node_datas_push(&self, node_data: Arc<NodeData>) {
        &mut self.tpi.write().unwrap().input_node_datas.push(node_data);
    }

    pub fn node_datas(&self, id: NodeId) -> Vec<Arc<NodeData>> {
        self.tpi.read().unwrap().node_datas(id)
    }

    pub fn add_node(&self, node: Node) -> Result<NodeId> {
        self.tpi.write().unwrap().node_graph.add_node(node)
    }

    pub fn remove_node(&self, node_id: NodeId) -> Result<()> {
        self.tpi.write().unwrap().node_graph.remove_node(node_id)
    }

    pub fn node_ids(&self) -> Vec<NodeId> {
        self.tpi.read().unwrap().node_graph.node_ids()
    }

    pub fn edges(&self) -> Vec<Edge> {
        self.tpi.read().unwrap().node_graph.edges.to_owned()
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
            .node_graph
            .connect_arbitrary(a_node, a_side, a_slot, b_node, b_side, b_slot)
    }

    pub fn disconnect_slot(&self, node_id: NodeId, side: Side, slot_id: SlotId) {
        self.tpi
            .write()
            .unwrap()
            .node_graph
            .disconnect_slot(node_id, side, slot_id)
    }

    pub fn get_node_data(&self, id: NodeId) -> Vec<Arc<NodeData>> {
        self.tpi.read().unwrap().get_node_data(id)
    }
    pub fn embed_node_data_with_id(
        &self,
        node_data: Arc<NodeData>,
        id: EmbeddedNodeDataId,
    ) -> Result<EmbeddedNodeDataId> {
        self.tpi
            .write()
            .unwrap()
            .embed_node_data_with_id(node_data, id)
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
        self.tpi.write().unwrap().node_graph.connect(
            output_node,
            input_node,
            output_slot,
            input_slot,
        )
    }

    pub fn node_with_id(&self, node_id: NodeId) -> Option<Node> {
        self.tpi.read().unwrap().node_graph.node_with_id(node_id)
    }

    pub fn node_with_id_set(&self, node_id: NodeId, node: Node) -> Result<()> {
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

    pub fn node_with_id_mut(&self, node_id: NodeId) -> Option<Node> {
        self.tpi.write().unwrap().node_graph.node_with_id(node_id)
    }
}
