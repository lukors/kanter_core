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
    pub slot_data_id: EmbeddedSlotDataId,
    pub slot_id: SlotId,
    pub size: Size,
    pub(crate) image: SlotImage,
}

impl EmbeddedSlotData {
    pub fn from_slot_data(slot_data: Arc<SlotData>, slot_data_id: EmbeddedSlotDataId) -> Self {
        Self {
            slot_data_id,
            slot_id: slot_data.slot_id,
            size: slot_data.size,
            image: slot_data.image.clone(),
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
        .find(|end| end.slot_data_id == embedded_node_data_id)
    {
        Ok(vec![Arc::new(SlotData::new(
            node.node_id,
            SlotId(0),
            enode_data.size,
            enode_data.image.clone(),
        ))])
    } else {
        Err(TexProError::NodeProcessing)
    }
}
