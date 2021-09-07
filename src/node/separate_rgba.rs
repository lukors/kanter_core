use std::sync::Arc;

use crate::{error::Result, node_graph::SlotId, slot_data::SlotData, slot_image::SlotImage};

use super::Node;

pub(crate) fn process(slot_datas: &[Arc<SlotData>], node: &Node) -> Result<Vec<Arc<SlotData>>> {
    if let Some(slot_data) = slot_datas.get(0) {
        if let SlotImage::Rgba(buf) = &slot_data.image {
            Ok(vec![
                Arc::new(SlotData::new(
                    node.node_id,
                    SlotId(0),
                    SlotImage::Gray(Arc::clone(&buf[0])),
                )),
                Arc::new(SlotData::new(
                    node.node_id,
                    SlotId(1),
                    SlotImage::Gray(Arc::clone(&buf[1])),
                )),
                Arc::new(SlotData::new(
                    node.node_id,
                    SlotId(2),
                    SlotImage::Gray(Arc::clone(&buf[2])),
                )),
                Arc::new(SlotData::new(
                    node.node_id,
                    SlotId(3),
                    SlotImage::Gray(Arc::clone(&buf[3])),
                )),
            ])
        } else {
            Ok(Vec::new())
        }
    } else {
        Ok(Vec::new())
    }
}
