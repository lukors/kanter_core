use std::sync::Arc;

use crate::{
    error::{Result, TexProError},
    node_graph::SlotId,
    slot_data::{Size, SlotData, SlotImage},
};

use super::Node;

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, PartialOrd, Serialize)]
pub struct EmbeddedSlotDataId(pub u32);
#[derive(Debug, Clone)]
pub struct EmbeddedSlotData {
    pub node_data_id: EmbeddedSlotDataId,
    pub slot_id: SlotId,
    pub size: Size,
    pub image: Arc<SlotImage>,
}

impl EmbeddedSlotData {
    pub fn from_node_data(node_data: Arc<SlotData>, node_data_id: EmbeddedSlotDataId) -> Self {
        Self {
            node_data_id,
            slot_id: node_data.slot_id,
            size: node_data.size,
            image: Arc::clone(&node_data.image),
        }
    }
}

pub(crate) fn process(
    node: &Node,
    embedded_node_datas: &[Arc<EmbeddedSlotData>],
    embedded_node_data_id: EmbeddedSlotDataId,
) -> Result<Vec<Arc<SlotData>>> {
    if let Some(enode_data) = embedded_node_datas
        .iter()
        .find(|end| end.node_data_id == embedded_node_data_id)
    {
        Ok(vec![Arc::new(SlotData::new(
            node.node_id,
            SlotId(0),
            enode_data.size,
            Arc::clone(&enode_data.image),
        ))])
    } else {
        Err(TexProError::NodeProcessing)
    }
}
